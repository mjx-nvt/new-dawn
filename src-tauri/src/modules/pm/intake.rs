//! Plane's intake (D-081): requests waiting to become work. An intake item is a work item in its
//! project's Triage state — numbered, with its description, priority, assignees, labels and due
//! date, its comments and activity — that only Intake shows, with how it stands: pending; accepted
//! (it moved into a state of the project's work); declined; snoozed until a day (from that day it
//! reads pending again); or a duplicate of another item. Made in the app, or filed from an email
//! by the Email reading agent.

use anyhow::{Result, anyhow};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::{Sqlite, SqlitePool, Transaction};

use super::items::{self, ItemInput, PmItem, Priority};
use super::relations::{self, PmRelationKind};
use crate::error::AppError;

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum IntakeStatus {
    Pending,
    Accepted,
    Declined,
    Snoozed,
    Duplicate,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct PmIntakeItem {
    pub item: PmItem,
    /// As it stands today: a snooze whose day has come reads pending.
    pub status: IntakeStatus,
    pub snoozed_until: Option<String>,
    pub duplicate_of: Option<String>,
    pub duplicate_key: Option<String>,
    /// "in_app" or "email".
    pub source: String,
    /// The email it was filed from (its subject and sender), when it was.
    pub email_subject: Option<String>,
    pub email_sender: Option<String>,
}

#[derive(sqlx::FromRow)]
struct Row {
    item_id: String,
    status: IntakeStatus,
    snoozed_until: Option<String>,
    duplicate_of: Option<String>,
    duplicate_key: Option<String>,
    source: String,
    email_subject: Option<String>,
    email_sender: Option<String>,
}

/// The status a row reads today.
fn today_status(status: IntakeStatus, snoozed_until: Option<&str>, today: NaiveDate) -> IntakeStatus {
    match (status, snoozed_until) {
        (IntakeStatus::Snoozed, Some(day)) if day <= today.to_string().as_str() => IntakeStatus::Pending,
        _ => status,
    }
}

const SELECT: &str = "SELECT x.item_id, x.status, x.snoozed_until, x.duplicate_of, m.identifier || '-' || d.sequence AS duplicate_key, x.source, \
     e.subject AS email_subject, e.sender AS email_sender \
     FROM pm_intake_item x JOIN pm_item i ON i.id = x.item_id LEFT JOIN pm_item d ON d.id = x.duplicate_of \
     LEFT JOIN pm_project m ON m.project_id = d.project_id LEFT JOIN email_item e ON e.id = x.email_item_id";

async fn read(pool: &SqlitePool, r: Row, today: NaiveDate) -> Result<PmIntakeItem> {
    Ok(PmIntakeItem {
        item: items::get(pool, &r.item_id).await?,
        status: today_status(r.status, r.snoozed_until.as_deref(), today),
        snoozed_until: r.snoozed_until,
        duplicate_of: r.duplicate_of,
        duplicate_key: r.duplicate_key,
        source: r.source,
        email_subject: r.email_subject,
        email_sender: r.email_sender,
    })
}

/// A project's intake items, newest first.
pub async fn list(pool: &SqlitePool, project_id: &str, today: NaiveDate) -> Result<Vec<PmIntakeItem>> {
    let rows: Vec<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} WHERE i.project_id = ? ORDER BY i.created_at DESC, i.id DESC"))).bind(project_id).fetch_all(pool).await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        out.push(read(pool, r, today).await?);
    }
    Ok(out)
}

pub async fn get(pool: &SqlitePool, id: &str, today: NaiveDate) -> Result<PmIntakeItem> {
    let row: Option<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} WHERE x.item_id = ?"))).bind(id).fetch_optional(pool).await?;
    let r = row.ok_or_else(|| anyhow!(AppError::not_found("That request is gone")))?;
    read(pool, r, today).await
}

/// How many wait in a project's Intake today (the tab's count).
pub async fn open_count(pool: &SqlitePool, project_id: &str, today: NaiveDate) -> Result<u32> {
    let (n,): (u32,) = sqlx::query_as(
        "SELECT count(*) FROM pm_intake_item x JOIN pm_item i ON i.id = x.item_id WHERE i.project_id = ? \
         AND (x.status = 'pending' OR (x.status = 'snoozed' AND x.snoozed_until <= ?))",
    )
    .bind(project_id)
    .bind(today.to_string())
    .fetch_one(pool)
    .await?;
    Ok(n)
}

/// The project's Triage state, made the first time (Plane's, hidden from the project's states).
pub(crate) async fn ensure_triage(tx: &mut Transaction<'_, Sqlite>, project_id: &str) -> Result<String> {
    let there: Option<(String,)> = sqlx::query_as("SELECT id FROM pm_state WHERE project_id = ? AND state_group = 'triage' ORDER BY sequence LIMIT 1").bind(project_id).fetch_optional(&mut **tx).await?;
    if let Some((id,)) = there {
        return Ok(id);
    }
    // "Triage", unless the project named a state of its own so (names are once a project).
    let taken: Vec<(String,)> = sqlx::query_as("SELECT name FROM pm_state WHERE project_id = ?").bind(project_id).fetch_all(&mut **tx).await?;
    let free = |n: &str| !taken.iter().any(|(t,)| super::settings::same(t, n));
    let name = ["Triage", "Intake triage"].into_iter().map(str::to_owned).chain((2..).map(|n| format!("Triage {n}"))).find(|n| free(n)).unwrap_or_default();
    let id = uuid::Uuid::now_v7().to_string();
    sqlx::query("INSERT INTO pm_state (id, project_id, name, color, state_group, sequence, is_default) VALUES (?, ?, ?, 'gray', 'triage', 5000, 0)").bind(&id).bind(project_id).bind(&name).execute(&mut **tx).await?;
    Ok(id)
}

/// A request: Plane's intake form — a title, a description (the editor's JSON text), a priority,
/// assignees, labels, a due date.
#[derive(Debug, Clone, Deserialize, Type)]
pub struct IntakeInput {
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    pub priority: Option<Priority>,
    pub assignees: Vec<String>,
    pub labels: Vec<String>,
    pub target_date: Option<String>,
}

pub async fn create(pool: &SqlitePool, input: IntakeInput, actor: &str, today: NaiveDate) -> Result<PmIntakeItem> {
    let made = items::create_with(
        pool,
        ItemInput { project_id: input.project_id, name: input.name, description: input.description, priority: input.priority, assignees: input.assignees, labels: input.labels, target_date: input.target_date, ..ItemInput::default() },
        actor,
        Some(("in_app", None)),
    )
    .await?;
    get(pool, &made.id, today).await
}

/// One that is still open (pending or snoozed) in a project that isn't archived, or a refusal
/// naming why not.
async fn open_one(pool: &SqlitePool, id: &str, today: NaiveDate) -> Result<PmIntakeItem> {
    let it = get(pool, id, today).await?;
    if super::get(pool, &it.item.project_id).await?.archived_at.is_some() {
        return Err(invalid("The project is archived — restore it to act on its requests"));
    }
    match it.status {
        IntakeStatus::Pending | IntakeStatus::Snoozed => Ok(it),
        IntakeStatus::Accepted => Err(invalid(format!("{} was accepted — it's a work item now", it.item.key))),
        _ => Err(invalid(format!("{} is closed", it.item.key))),
    }
}

/// Closes or snoozes an open request — only if it still is (two actions at once: the second is
/// refused).
async fn set_status(tx: &mut Transaction<'_, Sqlite>, id: &str, status: IntakeStatus, snoozed_until: Option<&str>, duplicate_of: Option<&str>) -> Result<()> {
    let done = sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE pm_intake_item SET status = ?, snoozed_until = ?, duplicate_of = ?, updated_at = {NOW} WHERE item_id = ? AND status IN ('pending', 'snoozed')")))
        .bind(status)
        .bind(snoozed_until)
        .bind(duplicate_of)
        .bind(id)
        .execute(&mut **tx)
        .await?;
    if done.rows_affected() != 1 {
        return Err(invalid("That request changed meanwhile — look at it again"));
    }
    Ok(())
}

/// How a request stood before an action, for its activity row.
fn was(it: &PmIntakeItem) -> &'static str {
    if it.status == IntakeStatus::Snoozed { "Snoozed" } else { "Pending" }
}

/// Accepted: into a state of the project's work (its default when none is chosen) — a work item
/// like any other from now on.
pub async fn accept(pool: &SqlitePool, id: &str, state_id: Option<&str>, actor: &str, today: NaiveDate) -> Result<PmIntakeItem> {
    let it = open_one(pool, id, today).await?;
    let target = items::work_state(pool, &it.item.project_id, state_id).await?;
    let mut tx = pool.begin().await?;
    items::leave_triage(&mut tx, &it.item, &target, actor).await?;
    set_status(&mut tx, id, IntakeStatus::Accepted, None, None).await?;
    items::activity(&mut tx, id, "updated", "intake", was(&it), "Accepted", (None, None), actor).await?;
    tx.commit().await?;
    get(pool, id, today).await
}

pub async fn decline(pool: &SqlitePool, id: &str, actor: &str, today: NaiveDate) -> Result<PmIntakeItem> {
    let it = open_one(pool, id, today).await?;
    let mut tx = pool.begin().await?;
    set_status(&mut tx, id, IntakeStatus::Declined, None, None).await?;
    items::activity(&mut tx, id, "updated", "intake", was(&it), "Declined", (None, None), actor).await?;
    tx.commit().await?;
    get(pool, id, today).await
}

/// Out of sight until a day after today; it reads pending again from that day.
pub async fn snooze(pool: &SqlitePool, id: &str, until: &str, actor: &str, today: NaiveDate) -> Result<PmIntakeItem> {
    let it = open_one(pool, id, today).await?;
    let day = items::clean_day(Some(until))?.ok_or_else(|| invalid("Snooze until a day"))?;
    if day.as_str() <= today.to_string().as_str() {
        return Err(invalid("Snooze until a day after today"));
    }
    let mut tx = pool.begin().await?;
    set_status(&mut tx, id, IntakeStatus::Snoozed, Some(&day), None).await?;
    items::activity(&mut tx, id, "updated", "intake", was(&it), &format!("Snoozed until {day}"), (None, None), actor).await?;
    tx.commit().await?;
    get(pool, id, today).await
}

/// A duplicate of a work item of the project: a Duplicate of relation to it, and closed.
pub async fn duplicate(pool: &SqlitePool, id: &str, of: &str, actor: &str, today: NaiveDate) -> Result<PmIntakeItem> {
    let it = open_one(pool, id, today).await?;
    let original = items::get(pool, of).await?;
    if original.project_id != it.item.project_id || original.state_group == super::settings::StateGroup::Triage || original.id == it.item.id {
        return Err(invalid("A request duplicates a work item of its project"));
    }
    // The relation and the status are one change.
    let mut tx = pool.begin().await?;
    set_status(&mut tx, id, IntakeStatus::Duplicate, None, Some(of)).await?;
    relations::relate_in(&mut tx, &it.item, PmRelationKind::Duplicate, &original, actor).await?;
    items::activity(&mut tx, id, "updated", "intake", was(&it), &format!("Duplicate of {}", original.key), (None, None), actor).await?;
    tx.commit().await?;
    get(pool, id, today).await
}

/// Deleted with its comments and files — not once accepted (it's a work item then).
pub async fn delete(pool: &SqlitePool, folder: &str, id: &str, today: NaiveDate) -> Result<()> {
    let it = get(pool, id, today).await?;
    if it.status == IntakeStatus::Accepted {
        return Err(invalid(format!("{} was accepted — delete it as a work item", it.item.key)));
    }
    items::delete(pool, folder, id).await
}

/// A request the Email reading agent found for a project with Intake on: an intake item quoting
/// the email; once a title an email. Whether it was filed.
pub(crate) async fn file_from_email(pool: &SqlitePool, project_id: &str, title: &str, evidence: &str, email_item_id: &str, subject: &str, sender: &str) -> Result<bool> {
    let title: String = title.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(items::NAME_MAX).collect();
    if title.is_empty() {
        return Ok(false);
    }
    let there: Option<(String,)> = sqlx::query_as("SELECT x.item_id FROM pm_intake_item x JOIN pm_item i ON i.id = x.item_id WHERE x.email_item_id = ? AND i.name = ?").bind(email_item_id).bind(&title).fetch_optional(pool).await?;
    if there.is_some() {
        return Ok(false);
    }
    let paragraph = |text: String| serde_json::json!({ "type": "paragraph", "content": [{ "type": "text", "text": text }] });
    let from = if subject.trim().is_empty() { format!("From {}", sender.trim()) } else { format!("From {} — {}", sender.trim(), subject.trim()) };
    let doc = serde_json::json!({ "type": "doc", "content": [paragraph(format!("“{}”", crate::modules::tasks::one_line(evidence, 1_000))), paragraph(from)] });
    items::create_with(pool, ItemInput { project_id: project_id.to_owned(), name: title, description: Some(doc.to_string()), ..ItemInput::default() }, "Email reading agent", Some(("email", Some(email_item_id)))).await?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::hub::WorkspaceHub;
    use crate::modules::pm::items::ItemPatch;
    use crate::modules::pm::{self, Features, NewPmProject, Network, settings};
    use chrono::Duration;

    #[test]
    fn an_accepted_request_is_work_in_its_state_and_a_snoozed_one_returns_on_its_day() {
        let root = std::env::temp_dir().join(format!("newdawn-pm-intake-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let folder = hub.workspace("test").await.unwrap().folder_path;
            let p = pm::create(&pool, NewPmProject { name: "VIFC Fintech Hub".into(), identifier: "VIFC".into(), description: String::new(), emoji: String::new(), cover: String::new(), network: Network::Private, lead: None, features: Features { intake: true, ..Features::default() } })
                .await
                .unwrap();
            let today = super::super::cycles::today();
            let todo = settings::states(&pool, &p.id).await.unwrap().into_iter().find(|s| s.name == "Todo").unwrap().id;
            let ask = |name: &str| IntakeInput { project_id: p.id.clone(), name: name.into(), description: None, priority: Some(Priority::High), assignees: vec![], labels: vec![], target_date: None };

            let work = items::create(&pool, ItemInput { project_id: p.id.clone(), name: "Draft the rules".into(), ..ItemInput::default() }, "me").await.unwrap();
            let a = create(&pool, ask("Add the pilot list to the site"), "me", today).await.unwrap();
            let b = create(&pool, ask("Translate the rules"), "me", today).await.unwrap();
            let c = create(&pool, ask("Draft the rules again"), "me", today).await.unwrap();
            assert_eq!((a.status, a.item.key.as_str(), a.item.state_name.as_str()), (IntakeStatus::Pending, "VIFC-2", "Triage"));
            // Not work yet: no list, no search, no state of the project's.
            assert_eq!(items::list(&pool, &p.id, false).await.unwrap().len(), 1);
            assert!(relations::search(&pool, "pilot list", None, None).await.unwrap().is_empty());
            assert!(settings::states(&pool, &p.id).await.unwrap().iter().any(|s| s.name == "Triage"));
            assert!(items::update(&pool, &a.item.id, ItemPatch { state_id: Some(todo.clone()), ..ItemPatch::default() }, "me").await.is_err(), "accepted, not moved");
            assert_eq!(open_count(&pool, &p.id, today).await.unwrap(), 3);

            // Accepted into Todo: VIFC-2, a work item in its state, in the lists.
            let accepted = accept(&pool, &a.item.id, Some(&todo), "me", today).await.unwrap();
            assert_eq!((accepted.status, accepted.item.key.as_str(), accepted.item.state_name.as_str()), (IntakeStatus::Accepted, "VIFC-2", "Todo"));
            assert_eq!(items::list(&pool, &p.id, false).await.unwrap().len(), 2);
            assert!(accept(&pool, &a.item.id, None, "me", today).await.is_err(), "once");
            assert!(delete(&pool, &folder, &a.item.id, today).await.is_err(), "a work item now");

            // Snoozed until the day after tomorrow: closed today and tomorrow, pending on its day.
            assert!(snooze(&pool, &b.item.id, &today.to_string(), "me", today).await.is_err(), "a day after today");
            let day = today + Duration::days(2);
            assert_eq!(snooze(&pool, &b.item.id, &day.to_string(), "me", today).await.unwrap().status, IntakeStatus::Snoozed);
            assert_eq!(get(&pool, &b.item.id, today + Duration::days(1)).await.unwrap().status, IntakeStatus::Snoozed);
            assert_eq!(get(&pool, &b.item.id, day).await.unwrap().status, IntakeStatus::Pending);
            assert_eq!((open_count(&pool, &p.id, today).await.unwrap(), open_count(&pool, &p.id, day).await.unwrap()), (1, 2));

            // A duplicate of the work it repeats; declined; deleted.
            let dup = duplicate(&pool, &c.item.id, &work.id, "me", today).await.unwrap();
            assert_eq!((dup.status, dup.duplicate_key.as_deref()), (IntakeStatus::Duplicate, Some("VIFC-1")));
            assert_eq!(relations::relations(&pool, &c.item.id).await.unwrap().len(), 1);
            assert_eq!(decline(&pool, &b.item.id, "me", day).await.unwrap().status, IntakeStatus::Declined);
            delete(&pool, &folder, &b.item.id, today).await.unwrap();
            assert_eq!(list(&pool, &p.id, today).await.unwrap().len(), 2);

            // From an email: once a title an email.
            sqlx::query("INSERT INTO email_item (id, message_id, subject, sender, source) VALUES ('e1', 'm1', 'Pilot cohort', 'Minh <minh@example.com>', 'file')").execute(&pool).await.unwrap();
            assert!(file_from_email(&pool, &p.id, "List the pilot fintechs", "Could you list the pilot fintechs by Friday?", "e1", "Pilot cohort", "Minh").await.unwrap());
            assert!(!file_from_email(&pool, &p.id, "List the pilot fintechs", "Could you list the pilot fintechs by Friday?", "e1", "Pilot cohort", "Minh").await.unwrap());
            let filed = list(&pool, &p.id, today).await.unwrap().into_iter().find(|x| x.source == "email").unwrap();
            assert_eq!((filed.status, filed.email_subject.as_deref(), filed.item.created_by.as_str()), (IntakeStatus::Pending, Some("Pilot cohort"), "Email reading agent"));
            assert!(filed.item.description_text.contains("by Friday"));
            hub.close_all().await;
        });
        let _ = std::fs::remove_dir_all(root);
    }
}
