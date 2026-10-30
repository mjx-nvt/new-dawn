//! Tasks (0.7.3): the owner's to-dos in this workspace, as simple as Reminders — typed here,
//! added from a meeting's action items ("Add to Tasks"), or found after a summary by the tasks
//! agent (`meetings::tasks`). A linked action item carries its task's id in the summary
//! (`ActionItem.task_id`); the tick follows both ways. Deleting leaves a tombstone, so a deleted
//! task never comes back by itself.

pub mod due;

use anyhow::{Context, Result, bail};
use chrono::NaiveDate;
use serde::{Deserialize, Deserializer, Serialize};
use specta::Type;
use sqlx::{SqliteConnection, SqlitePool};

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::kernel::profile;
use crate::modules::feedback::{self, Feedback, Kind, Source};
use crate::modules::meetings::summarise::{self, ActionItem, SummaryDoc, SummaryView};
use crate::modules::meetings::tasks::{Who, shown_owner};
use crate::modules::meetings::{self, Meeting, MeetingStatus, review};
use crate::modules::people;
use due::Due;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum TaskOrigin {
    /// Found after a summary (the rules or the tasks agent).
    Auto,
    /// "Add to Tasks" on an action item.
    Button,
    /// Typed in Tasks.
    Typed,
}

/// The meeting a task came from, as it is now (a renamed meeting shows its new title).
#[derive(Debug, Clone, Serialize, Type)]
pub struct TaskMeeting {
    pub id: String,
    pub title: String,
    pub held_on: String,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct TaskView {
    pub id: String,
    pub title: String,
    /// Whose item it is when it isn't yours ("Minh"); "" when it is.
    pub owner: String,
    /// The due phrase as said ("end of month"); "" when none was.
    pub due_text: String,
    /// The day it is due, YYYY-MM-DD (a local calendar day).
    pub due_on: Option<String>,
    pub done_at: Option<String>,
    pub meeting: Option<TaskMeeting>,
    pub origin: TaskOrigin,
    /// Why it was found to be yours (automatic tasks).
    pub reason: String,
    /// Reminders' levels: 0 none, 1 low (!), 2 medium (!!), 3 high (!!!).
    pub priority: u8,
}

/// The list in Reminders' order ("Highest first").
#[derive(Debug, Clone, Default, Serialize, Type)]
pub struct TaskList {
    /// Open and due today or earlier: highest priority first, then overdue first.
    pub today: Vec<TaskView>,
    /// Open and due later: earliest first, then highest priority.
    pub upcoming: Vec<TaskView>,
    /// Open with no date: highest priority first, then oldest.
    pub no_date: Vec<TaskView>,
    /// Done, most recent first (the last 200).
    pub completed: Vec<TaskView>,
}

/// What `update` changes; a field left out stays as it is.
#[derive(Debug, Clone, Deserialize, Type)]
pub struct TaskPatch {
    pub title: Option<String>,
    /// The day it is due, YYYY-MM-DD; `null` removes the date.
    #[serde(default, deserialize_with = "present")]
    #[specta(type = Option<String>)]
    pub due_on: Option<Option<String>>,
    pub done: Option<bool>,
    /// 0 (none) to 3 (high).
    pub priority: Option<u8>,
}

/// A field that is there, even as `null` (`Some(None)`); left out, serde's default (`None`).
fn present<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Option<String>>, D::Error> {
    Option::deserialize(d).map(Some)
}

#[derive(sqlx::FromRow)]
struct Row {
    id: String,
    title: String,
    owner: String,
    due_text: String,
    due_on: Option<String>,
    done_at: Option<String>,
    origin: TaskOrigin,
    reason: String,
    priority: u8,
    meeting_id: Option<String>,
    meeting_title: Option<String>,
    meeting_held_on: Option<String>,
}

impl From<Row> for TaskView {
    fn from(r: Row) -> Self {
        let meeting = match (r.meeting_id, r.meeting_title, r.meeting_held_on) {
            (Some(id), Some(title), Some(held_on)) => Some(TaskMeeting { id, title, held_on }),
            _ => None,
        };
        TaskView {
            id: r.id,
            title: r.title,
            owner: r.owner,
            due_text: r.due_text,
            due_on: r.due_on,
            done_at: r.done_at,
            meeting,
            origin: r.origin,
            reason: r.reason,
            priority: r.priority,
        }
    }
}

/// Task rows with their meeting's title and date read live; `rest` (a constant) filters and orders.
fn select(rest: &'static str) -> sqlx::AssertSqlSafe<String> {
    sqlx::AssertSqlSafe(format!(
        "SELECT t.id, t.title, t.owner, t.due_text, t.due_on, t.done_at, t.origin, t.reason, t.priority, \
         m.id AS meeting_id, m.title AS meeting_title, m.held_on AS meeting_held_on \
         FROM task t LEFT JOIN meeting m ON m.id = t.meeting_id {rest}"
    ))
}

/// Today where the Mac is, YYYY-MM-DD (what `due_on` holds).
fn today() -> String {
    chrono::Local::now().date_naive().to_string()
}

/// A task's text (its title, the due phrase): one line, single spaces, at most `max` characters.
/// Unlike a name it keeps "<" and ">" ("revenue > 5bn"): it is only shown as text, never prompted.
fn one_line(text: &str, max: usize) -> String {
    text.split(|c: char| c.is_whitespace() || c.is_control())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max)
        .collect()
}

pub async fn list(pool: &SqlitePool) -> Result<TaskList> {
    let today = today();
    let open: Vec<Row> = sqlx::query_as(select(
        "WHERE t.deleted_at IS NULL AND t.done_at IS NULL \
         ORDER BY t.due_on, t.priority DESC, t.created_at",
    ))
    .fetch_all(pool)
    .await?;
    let mut list = TaskList::default();
    for task in open.into_iter().map(TaskView::from) {
        match task.due_on.as_deref() {
            None => list.no_date.push(task),
            Some(day) if day <= today.as_str() => list.today.push(task),
            Some(_) => list.upcoming.push(task),
        }
    }
    // Today: highest first, and the sort is stable, so equal priorities stay overdue first. No date
    // is already highest first (its rows share a NULL day); Upcoming stays by day, so a high task
    // three weeks out doesn't jump above tomorrow's.
    list.today.sort_by_key(|t| std::cmp::Reverse(t.priority));
    let completed: Vec<Row> = sqlx::query_as(select(
        "WHERE t.deleted_at IS NULL AND t.done_at IS NOT NULL ORDER BY t.done_at DESC LIMIT 200",
    ))
    .fetch_all(pool)
    .await?;
    list.completed = completed.into_iter().map(TaskView::from).collect();
    Ok(list)
}

/// Open tasks due today or earlier: the sidebar badge.
pub async fn open_count(pool: &SqlitePool) -> Result<u32> {
    let (count,): (u32,) = sqlx::query_as(
        "SELECT count(*) FROM task WHERE deleted_at IS NULL AND done_at IS NULL AND due_on <= ?",
    )
    .bind(today())
    .fetch_one(pool)
    .await?;
    Ok(count)
}

async fn get(pool: &SqlitePool, id: &str) -> Result<TaskView> {
    let row: Option<Row> = sqlx::query_as(select("WHERE t.id = ? AND t.deleted_at IS NULL"))
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row
        .map(TaskView::from)
        .ok_or_else(|| AppError::not_found("No such task"))?)
}

/// A task typed in Tasks: no date, no meeting.
pub async fn add(pool: &SqlitePool, title: &str) -> Result<TaskView> {
    let title = one_line(title, 500);
    if title.is_empty() {
        bail!(AppError::invalid("A task needs a title"));
    }
    let id = meetings::new_id();
    sqlx::query("INSERT INTO task (id, title, origin) VALUES (?, ?, 'typed')")
        .bind(&id)
        .bind(&title)
        .execute(pool)
        .await?;
    get(pool, &id).await
}

/// Renames, dates, ticks or prioritises a task. A tick follows to the action item it came from.
pub async fn update(pool: &SqlitePool, id: &str, patch: TaskPatch) -> Result<TaskView> {
    let title = patch.title.as_deref().map(|t| one_line(t, 500));
    if title.as_deref() == Some("") {
        bail!(AppError::invalid("A task needs a title"));
    }
    if patch.priority.is_some_and(|p| p > 3) {
        bail!(AppError::invalid("A priority is 0 (none) to 3 (high)"));
    }
    let before = agent_task(pool, id).await?;
    let due_on = match patch.due_on {
        Some(Some(day)) => Some(Some(
            NaiveDate::parse_from_str(day.trim(), "%Y-%m-%d")
                .map_err(|_| AppError::invalid("A due date is a day: YYYY-MM-DD"))?
                .to_string(),
        )),
        other => other,
    };
    let done = sqlx::query(
        "UPDATE task SET title = coalesce(?1, title), \
           due_on = CASE WHEN ?2 THEN ?3 ELSE due_on END, \
           done_at = CASE WHEN ?4 IS NULL THEN done_at \
             WHEN ?4 THEN coalesce(done_at, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')) END, \
           priority = coalesce(?6, priority), \
           updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE id = ?5 AND deleted_at IS NULL",
    )
    .bind(title.as_deref())
    .bind(due_on.is_some())
    .bind(due_on.flatten())
    .bind(patch.done)
    .bind(id)
    .bind(patch.priority)
    .execute(pool)
    .await?;
    if done.rows_affected() == 0 {
        bail!(AppError::not_found("No such task"));
    }
    // A task the AI added, reworded by the owner.
    if let (Some((old, meeting_id)), Some(new)) = (before, title.as_deref()) {
        if old != new {
            feedback::record(
                pool,
                Feedback {
                    meeting_id: meeting_id.as_deref(),
                    source: Source::Tasks,
                    kind: Kind::TaskReworded,
                    before: &old,
                    after: new,
                    context: serde_json::json!({}),
                },
            )
            .await;
        }
    }
    if let Some(done) = patch.done {
        tick_item(pool, id, done).await?;
    }
    get(pool, id).await
}

/// The title and meeting of a task the AI added (None for one added by hand).
async fn agent_task(pool: &SqlitePool, id: &str) -> Result<Option<(String, Option<String>)>> {
    Ok(sqlx::query_as(
        "SELECT title, meeting_id FROM task WHERE id = ? AND origin = 'auto' AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?)
}

/// Hides a task for good (a tombstone): a summary never makes it again, and "Add to Tasks" on
/// its action item brings it back. The item keeps its link.
pub async fn delete(pool: &SqlitePool, id: &str) -> Result<()> {
    let before = agent_task(pool, id).await?;
    let done = sqlx::query(
        "UPDATE task SET deleted_at = coalesce(deleted_at, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')), \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?",
    )
    .bind(id)
    .execute(pool)
    .await?;
    if done.rows_affected() == 0 {
        bail!(AppError::not_found("No such task"));
    }
    // A task the AI added that the owner didn't want.
    if let Some((title, meeting_id)) = before {
        feedback::record(
            pool,
            Feedback {
                meeting_id: meeting_id.as_deref(),
                source: Source::Tasks,
                kind: Kind::TaskDeleted,
                before: &title,
                after: "",
                context: serde_json::json!({}),
            },
        )
        .await;
    }
    Ok(())
}

/// The task for action item `item` of `meeting`: made, or brought back when it was deleted. The
/// same owner and text in the same meeting (`source_key`) are one task, so a repeat — or a crash
/// between saving the task and the item's link — never makes a second. Its tick is the item's.
/// The due phrase becomes a day when it names one; the agent's day (`ai_due`) only when it
/// doesn't, and only near the meeting — a wrong date is worse than the phrase. Marks the meeting
/// as having fed Tasks. Returns the task's id.
pub(crate) async fn insert_item(
    conn: &mut SqliteConnection,
    meeting: &Meeting,
    item: &ActionItem,
    owner: &str,
    origin: TaskOrigin,
    reason: &str,
    ai_due: Option<NaiveDate>,
) -> Result<String> {
    let title = one_line(&item.item, 500);
    if title.is_empty() {
        bail!(AppError::invalid("This action item has no text"));
    }
    let said = one_line(&item.due, 200);
    let (due_text, due_on) = match due::local_day(&meeting.held_on) {
        None => (said, None),
        Some(day) => match due::parse_due(&said, day) {
            Due::None => (String::new(), None),
            Due::On(on) => (said, Some(on)),
            Due::Text => (
                said,
                ai_due.filter(|on| (-1..=366).contains(&(*on - day).num_days())),
            ),
        },
    };
    let (id,): (String,) = sqlx::query_as(
        "INSERT INTO task (id, title, owner, due_text, due_on, done_at, meeting_id, source_key, origin, reason) \
         VALUES (?1, ?2, ?3, ?4, ?5, CASE WHEN ?6 THEN strftime('%Y-%m-%dT%H:%M:%fZ', 'now') END, ?7, ?8, ?9, ?10) \
         ON CONFLICT (meeting_id, source_key) DO UPDATE SET deleted_at = NULL, \
           done_at = CASE WHEN ?6 THEN coalesce(task.done_at, excluded.done_at) END, \
           updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         RETURNING id",
    )
    .bind(meetings::new_id())
    .bind(&title)
    .bind(owner)
    .bind(&due_text)
    .bind(due_on.map(|d| d.to_string()))
    .bind(item.done)
    .bind(&meeting.id)
    .bind(format!(
        "{}|{}",
        people::fold(&item.owner),
        people::fold(&item.item)
    ))
    .bind(origin)
    .bind(reason)
    .fetch_one(&mut *conn)
    .await?;
    sqlx::query(
        "UPDATE meeting SET tasks_at = coalesce(tasks_at, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')) WHERE id = ?",
    )
    .bind(&meeting.id)
    .execute(&mut *conn)
    .await?;
    Ok(id)
}

/// "Add to Tasks" on action item `index` of a meeting's summary: its task is made — or the one
/// it had comes back after a delete — and linked.
pub async fn add_from_item(
    hub: &WorkspaceHub,
    slug: &str,
    meeting_id: &str,
    index: usize,
) -> Result<SummaryView> {
    let pool = hub.pool(slug)?;
    let meeting = meetings::get(&pool, meeting_id).await?;
    if meeting.status == MeetingStatus::Summarising {
        bail!(AppError::invalid(
            "A summary is being written — add it when it's done"
        ));
    }
    let mut doc = summarise::view(&pool, meeting_id)
        .await?
        .ok_or_else(|| AppError::not_found("This meeting has no summary yet"))?
        .doc;
    let item = doc
        .action_items
        .get(index)
        .cloned()
        .ok_or_else(|| AppError::invalid("No such action item"))?;
    let owner = profile::owner(hub, slug).await?;
    let people = people::list(&pool).await?;
    let who = owner.as_ref().map(|o| Who::new(o, &people, &doc.attendees));
    let mut tx = pool.begin().await?;
    let restored = match item.task_id.as_deref() {
        Some(id) => {
            sqlx::query(
                "UPDATE task SET deleted_at = NULL, \
                   done_at = CASE WHEN ?1 THEN coalesce(done_at, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')) END, \
                   updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
                 WHERE id = ?2 AND meeting_id = ?3",
            )
            .bind(item.done)
            .bind(id)
            .bind(meeting_id)
            .execute(&mut *tx)
            .await?
            .rows_affected()
                > 0
        }
        None => false,
    };
    let id = match item.task_id.clone().filter(|_| restored) {
        Some(id) => id,
        None => {
            let shown = shown_owner(&item.owner, who.as_ref());
            insert_item(
                &mut tx,
                &meeting,
                &item,
                &shown,
                TaskOrigin::Button,
                "",
                None,
            )
            .await?
        }
    };
    tx.commit().await?;
    // The owner found this action item worth a task: the summary got it right.
    feedback::record(
        &pool,
        Feedback {
            meeting_id: Some(meeting_id),
            source: Source::Summary,
            kind: Kind::TaskAdded,
            before: "",
            after: &item.item,
            context: serde_json::json!({ "owner": item.owner }),
        },
    )
    .await;
    if item.task_id.as_deref() != Some(id.as_str()) {
        // The same owner and text are one task, and a task follows one item: an identical item
        // linked to it lets it go, so it follows the item added here (and its tick, set above).
        for a in &mut doc.action_items {
            if a.task_id.as_deref() == Some(id.as_str()) {
                a.task_id = None;
            }
        }
        doc.action_items[index].task_id = Some(id);
        meetings::save_sections(&pool, meeting_id, &serde_json::to_string(&doc)?, false).await?;
    }
    summarise::view(&pool, meeting_id)
        .await?
        .context("summary vanished while saving")
}

/// A tick in the summary ticks the linked tasks (`update_section`).
pub async fn follow(pool: &SqlitePool, meeting_id: &str, items: &[ActionItem]) -> Result<()> {
    for item in items {
        let Some(id) = &item.task_id else { continue };
        sqlx::query(
            "UPDATE task SET done_at = CASE WHEN ?1 THEN coalesce(done_at, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')) END, \
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
             WHERE id = ?2 AND meeting_id = ?3 AND (done_at IS NOT NULL) <> ?1",
        )
        .bind(item.done)
        .bind(id)
        .bind(meeting_id)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// A tick in Tasks ticks the action item the task came from. A tick isn't an edit: the section
/// keeps its flag and the email isn't marked older. (A summary being written meanwhile takes the
/// task's tick when it saves its links, `meetings::tasks::find`.)
async fn tick_item(pool: &SqlitePool, task_id: &str, done: bool) -> Result<()> {
    let row: Option<(Option<String>,)> = sqlx::query_as("SELECT meeting_id FROM task WHERE id = ?")
        .bind(task_id)
        .fetch_optional(pool)
        .await?;
    let Some((Some(meeting_id),)) = row else {
        return Ok(());
    };
    let Some(summary) = meetings::summary(pool, &meeting_id).await? else {
        return Ok(());
    };
    let mut doc: SummaryDoc =
        serde_json::from_str(&summary.sections_json).context("reading the stored summary")?;
    let mut changed = false;
    for item in doc
        .action_items
        .iter_mut()
        .filter(|a| a.task_id.as_deref() == Some(task_id) && a.done != done)
    {
        item.done = done;
        changed = true;
    }
    if changed {
        review::show_ticks(&mut doc);
        meetings::save_sections(pool, &meeting_id, &serde_json::to_string(&doc)?, false).await?;
    }
    Ok(())
}

/// A name corrected in a meeting (confirmed, edited with the attendees or by Corrections) is
/// corrected in its tasks too: their owner and title were copied from the action item.
pub async fn rename(pool: &SqlitePool, meeting_id: &str, pairs: &[(String, String)]) -> Result<()> {
    let tasks: Vec<(String, String, String)> =
        sqlx::query_as("SELECT id, owner, title FROM task WHERE meeting_id = ?")
            .bind(meeting_id)
            .fetch_all(pool)
            .await?;
    for (id, mut owner, mut title) in tasks {
        if people::rename_in_text(&mut owner, pairs) + people::rename_in_text(&mut title, pairs) > 0
        {
            sqlx::query(
                "UPDATE task SET owner = ?, title = ?, \
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?",
            )
            .bind(&owner)
            .bind(&title)
            .bind(&id)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::ai::whisper::Language;
    use crate::kernel::profile::ProfilePatch;
    use crate::modules::meetings::summarise::{Attendee, Section, default_structure};

    fn item(owner: &str, text: &str, due: &str) -> ActionItem {
        ActionItem {
            owner: owner.into(),
            item: text.into(),
            due: due.into(),
            done: false,
            task_id: None,
        }
    }

    #[test]
    fn tasks_follow_their_action_items() {
        let root = std::env::temp_dir().join(format!("newdawn-tasks-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            profile::set(
                &hub,
                ProfilePatch {
                    name: Some("Tien Nguyen".into()),
                    everyone_attends: None,
                    summary_language: None,
                },
            )
            .await
            .unwrap();
            let pool = hub.pool("test").unwrap();
            meetings::create(
                &pool,
                meetings::NewMeeting {
                    id: "m1",
                    meeting_type_id: None,
                    title: "Partner call",
                    held_at_unix: 1_792_983_600, // Mon 26 Oct 2026, 03:00 UTC
                    language: Language::En,
                    audio_path: "/nowhere.wav",
                    status: MeetingStatus::Summarised,
                },
            )
            .await
            .unwrap();
            let doc = SummaryDoc {
                sections: default_structure(Language::En)
                    .into_iter()
                    .map(|spec| Section {
                        kind: spec.kind,
                        title: spec.title,
                        markdown: String::new(),
                        edited: false,
                    })
                    .collect(),
                attendees: ["Tien Nguyen", "Minh"]
                    .map(|name| Attendee {
                        name: name.into(),
                        ..Attendee::default()
                    })
                    .to_vec(),
                action_items: vec![
                    item("Tien", "Send the MoU", "Friday"),
                    item("Minh", "Book the room", "Not set"),
                ],
                ..SummaryDoc::default()
            };
            let json = serde_json::to_string(&doc).unwrap();
            meetings::save_summary(&pool, "m1", &json, "S", "Body", None, false, None)
                .await
                .unwrap();

            // "Add to Tasks": made and linked; the meeting has fed Tasks.
            let v = add_from_item(&hub, "test", "m1", 1).await.unwrap();
            let minh = v.doc.action_items[1].task_id.clone().unwrap();
            assert_eq!(v.in_tasks, vec![minh.clone()]);
            assert!(meetings::get(&pool, "m1").await.unwrap().tasks_at.is_some());
            let v = add_from_item(&hub, "test", "m1", 0).await.unwrap();
            let mine = v.doc.action_items[0].task_id.clone().unwrap();
            let t = get(&pool, &mine).await.unwrap();
            assert_eq!(
                (t.owner.as_str(), t.due_text.as_str(), t.due_on.as_deref()),
                ("", "Friday", Some("2026-10-30")),
                "yours: no owner shown; Friday counted from the meeting"
            );
            assert_eq!(
                (t.origin, t.priority),
                (TaskOrigin::Button, 0),
                "added from an item: no priority"
            );
            assert_eq!(t.meeting.unwrap().title, "Partner call");
            let t = get(&pool, &minh).await.unwrap();
            assert_eq!(
                (t.owner.as_str(), t.due_text.as_str(), t.due_on),
                ("Minh", "", None)
            );

            // Sections by the local day; the badge counts today and overdue.
            let day = |offset: i64| {
                (chrono::Local::now().date_naive() + chrono::Duration::days(offset)).to_string()
            };
            let patch = |due_on: Option<Option<String>>, done: Option<bool>| TaskPatch {
                title: None,
                due_on,
                done,
                priority: None,
            };
            update(&pool, &mine, patch(Some(Some(day(-1))), None))
                .await
                .unwrap();
            update(&pool, &minh, patch(Some(Some(day(3))), None))
                .await
                .unwrap();
            let typed = add(&pool, "  Check revenue >\n 5bn ").await.unwrap();
            assert_eq!(
                (typed.title.as_str(), typed.origin),
                ("Check revenue > 5bn", TaskOrigin::Typed)
            );
            let ids = |tasks: &[TaskView]| tasks.iter().map(|t| t.id.clone()).collect::<Vec<_>>();
            let l = list(&pool).await.unwrap();
            assert_eq!(ids(&l.today), vec![mine.clone()], "overdue is in Today");
            assert_eq!(ids(&l.upcoming), vec![minh.clone()]);
            assert_eq!(ids(&l.no_date), vec![typed.id.clone()]);
            assert_eq!(open_count(&pool).await.unwrap(), 1);

            // A tick in Tasks ticks the item (not an edit); unticking the item unticks the task.
            update(&pool, &minh, patch(None, Some(true))).await.unwrap();
            let v = summarise::view(&pool, "m1").await.unwrap().unwrap();
            assert!(v.doc.action_items[1].done && v.edited_at.is_none());
            assert!(
                v.doc.sections[3]
                    .markdown
                    .contains("- [x] **Minh** — Book the room")
            );
            assert_eq!(
                ids(&list(&pool).await.unwrap().completed),
                vec![minh.clone()]
            );
            let unticked = v.doc.sections[3].markdown.replace("- [x]", "- [ ]");
            summarise::update_section(&pool, "VIFC", "m1", 3, &unticked)
                .await
                .unwrap();
            assert!(get(&pool, &minh).await.unwrap().done_at.is_none());

            // Delete is a tombstone: hidden, the link stays; "Add to Tasks" brings it back.
            delete(&pool, &minh).await.unwrap();
            let v = summarise::view(&pool, "m1").await.unwrap().unwrap();
            assert_eq!(v.in_tasks, vec![mine.clone()]);
            assert_eq!(
                v.doc.action_items[1].task_id.as_deref(),
                Some(minh.as_str())
            );
            assert!(!ids(&list(&pool).await.unwrap().upcoming).contains(&minh));
            let v = add_from_item(&hub, "test", "m1", 1).await.unwrap();
            assert_eq!(
                v.doc.action_items[1].task_id.as_deref(),
                Some(minh.as_str())
            );
            assert!(v.in_tasks.contains(&minh), "the same task came back");

            // An identical item is the same task, which follows one item: added from the
            // other, it moves there.
            let mut twins = v.doc.clone();
            twins.action_items.push(ActionItem {
                task_id: None,
                ..twins.action_items[1].clone()
            });
            let json = serde_json::to_string(&twins).unwrap();
            meetings::save_sections(&pool, "m1", &json, false)
                .await
                .unwrap();
            let v = add_from_item(&hub, "test", "m1", 2).await.unwrap();
            assert_eq!(
                (
                    v.doc.action_items[1].task_id.as_deref(),
                    v.doc.action_items[2].task_id.as_deref()
                ),
                (None, Some(minh.as_str()))
            );

            // Edits: a date removed, a bad date or an empty title refused.
            let t = update(&pool, &minh, patch(Some(None), None)).await.unwrap();
            assert_eq!(t.due_on, None);
            assert!(
                update(&pool, &minh, patch(Some(Some("30/10".into())), None))
                    .await
                    .is_err()
            );
            let rename = TaskPatch {
                title: Some("  ".into()),
                due_on: None,
                done: None,
                priority: None,
            };
            assert!(update(&pool, &minh, rename).await.is_err());
            assert!(delete(&pool, "nope").await.is_err());
            let patch: TaskPatch = serde_json::from_str(r#"{"due_on": null}"#).unwrap();
            assert_eq!(patch.due_on, Some(None), "null removes the date");
            let patch: TaskPatch = serde_json::from_str("{}").unwrap();
            assert_eq!(patch.due_on, None, "left out keeps it");
            hub.close_all().await;
        });
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn priority_sorts_today_and_no_date_upcoming_stays_by_day() {
        let root = std::env::temp_dir().join(format!("newdawn-tasks-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let day = |offset: i64| {
                (chrono::Local::now().date_naive() + chrono::Duration::days(offset)).to_string()
            };
            // (title, due in days, priority)
            for (title, due, priority) in [
                ("overdue", Some(-2), 0),
                ("yesterday, low", Some(-1), 1),
                ("today", Some(0), 0),
                ("today, high", Some(0), 3),
                ("in two days", Some(2), 0),
                ("tomorrow", Some(1), 0),
                ("in two days, high", Some(2), 3),
                ("no date", None, 0),
                ("no date, medium", None, 2),
            ] {
                let t = add(&pool, title).await.unwrap();
                assert_eq!(t.priority, 0, "typed: no priority");
                let patch = TaskPatch {
                    title: None,
                    due_on: due.map(|d| Some(day(d))),
                    done: None,
                    priority: Some(priority),
                };
                assert_eq!(
                    update(&pool, &t.id, patch).await.unwrap().priority,
                    priority
                );
            }
            let titles =
                |tasks: &[TaskView]| tasks.iter().map(|t| t.title.clone()).collect::<Vec<_>>();
            let l = list(&pool).await.unwrap();
            assert_eq!(
                titles(&l.today),
                ["today, high", "yesterday, low", "overdue", "today"],
                "highest first, then overdue first"
            );
            assert_eq!(
                titles(&l.upcoming),
                ["tomorrow", "in two days, high", "in two days"],
                "by day, then highest first"
            );
            assert_eq!(titles(&l.no_date), ["no date, medium", "no date"]);

            // Left out keeps it; out of range is refused.
            let high = &l.today[0].id;
            let rename: TaskPatch = serde_json::from_str(r#"{"title": "Today, urgent"}"#).unwrap();
            assert_eq!(update(&pool, high, rename).await.unwrap().priority, 3);
            let wrong: TaskPatch = serde_json::from_str(r#"{"priority": 4}"#).unwrap();
            assert!(update(&pool, high, wrong).await.is_err());
            hub.close_all().await;
        });
        std::fs::remove_dir_all(root).unwrap();
    }
}
