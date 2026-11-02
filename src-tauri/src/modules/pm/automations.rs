//! Plane's project automations (D-075, Settings → Automations): once a day while New Dawn is open
//! — the first hour it is, a missed day caught up at launch — in each workspace with Projects on:
//! closed items archived after the project's months, idle open ones closed into its cancelled
//! state, and due-date reminders in the Inbox the day before (on the day, when that was missed).
//! Everything goes through the items' own functions as "automation": its activity, its notices.

use std::time::Duration;

use anyhow::Result;
use chrono::{DateTime, Local, NaiveDate, Utc};
use sqlx::SqlitePool;
use tauri::{AppHandle, Manager};

use super::items::{self, ItemPatch};
use crate::kernel::hub::WorkspaceHub;
use crate::modules::switches;

/// The actor the automations write as.
pub const AUTOMATION: &str = "automation";
/// The local day the automations last ran in a workspace (a workspace setting).
const DAY_KEY: &str = "pm_automations_day";

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Ran {
    pub archived: u32,
    pub closed: u32,
    pub reminded: u32,
}

/// The items of a project whose state is in `groups` and that nothing happened to — no change, no
/// comment — for `months`.
async fn idle(pool: &SqlitePool, project: &str, groups: &str, months: u32, at: &str) -> Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT i.id FROM pm_item i JOIN pm_state s ON s.id = i.state_id WHERE i.project_id = ? AND i.archived_at IS NULL \
         AND s.state_group IN ({groups}) \
         AND max(i.updated_at, coalesce((SELECT max(a.created_at) FROM pm_activity a WHERE a.item_id = i.id), '')) \
             <= strftime('%Y-%m-%dT%H:%M:%fZ', ?, '-{months} months') ORDER BY i.sequence"
    )))
    .bind(project)
    .bind(at)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|(i,)| i).collect())
}

/// Held by a cycle that hasn't ended (a draft, an upcoming or the running one): Plane's
/// automations leave such items alone (D-078).
async fn held(pool: &SqlitePool, item: &str, today: NaiveDate) -> Result<bool> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT 1 FROM pm_cycle_item x JOIN pm_cycle c ON c.id = x.cycle_id WHERE x.item_id = ? AND c.completed_at IS NULL AND (c.end_date IS NULL OR c.end_date >= ?)",
    )
    .bind(item)
    .bind(today.to_string())
    .fetch_optional(pool)
    .await?;
    Ok(row.is_some())
}

/// One run as of `now` (the months count back from it) for the local day `today`. An item that
/// can't be changed is logged and left; the others go on.
pub async fn run(pool: &SqlitePool, now: DateTime<Utc>, today: NaiveDate) -> Result<Ran> {
    let mut ran = Ran::default();
    let at = now.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    let projects: Vec<(String, u32, u32, Option<String>, bool)> =
        sqlx::query_as("SELECT project_id, archive_in, close_in, close_state_id, due_reminders FROM pm_project WHERE archived_at IS NULL").fetch_all(pool).await?;
    for (project, archive_in, close_in, close_state, reminders) in projects {
        if archive_in > 0 {
            for id in idle(pool, &project, "'completed', 'cancelled'", archive_in, &at).await? {
                if held(pool, &id, today).await? {
                    continue;
                }
                match items::archive(pool, &id, true, AUTOMATION).await {
                    Ok(_) => ran.archived += 1,
                    Err(e) => tracing::warn!("auto-archive {id}: {e:#}"),
                }
            }
        }
        if close_in > 0 {
            // The state chosen in the settings, else the project's first cancelled one.
            let state: Option<(String,)> = sqlx::query_as("SELECT id FROM pm_state WHERE project_id = ? AND state_group = 'cancelled' ORDER BY id = ? DESC, sequence LIMIT 1")
                .bind(&project)
                .bind(close_state.as_deref().unwrap_or(""))
                .fetch_optional(pool)
                .await?;
            if let Some((state,)) = state {
                for id in idle(pool, &project, "'backlog', 'unstarted', 'started'", close_in, &at).await? {
                    if held(pool, &id, today).await? {
                        continue;
                    }
                    match items::update(pool, &id, ItemPatch { state_id: Some(state.clone()), ..ItemPatch::default() }, AUTOMATION).await {
                        Ok(_) => ran.closed += 1,
                        Err(e) => tracing::warn!("auto-close {id}: {e:#}"),
                    }
                }
            }
        }
        if reminders {
            let tomorrow = today.succ_opt().unwrap_or(today);
            // Open items the owner follows, due today or tomorrow; once a due date.
            let due: Vec<(String, String)> = sqlx::query_as(
                "SELECT i.id, i.target_date FROM pm_item i JOIN pm_state s ON s.id = i.state_id WHERE i.project_id = ? AND i.archived_at IS NULL \
                 AND s.state_group NOT IN ('completed', 'cancelled', 'triage') AND i.target_date IN (?, ?) \
                 AND EXISTS (SELECT 1 FROM pm_item_subscriber b WHERE b.item_id = i.id AND b.who = 'me')",
            )
            .bind(&project)
            .bind(today.to_string())
            .bind(tomorrow.to_string())
            .fetch_all(pool)
            .await?;
            for (item, on) in due {
                let done = sqlx::query("INSERT INTO notification (id, kind, item_id, actor, due_on) VALUES (?, 'reminder', ?, '', ?) ON CONFLICT DO NOTHING")
                    .bind(uuid::Uuid::now_v7().to_string())
                    .bind(&item)
                    .bind(&on)
                    .execute(pool)
                    .await?;
                ran.reminded += done.rows_affected() as u32;
            }
        }
    }
    Ok(ran)
}

/// Runs once a local day: nothing when today's run is done.
pub async fn run_due(pool: &SqlitePool) -> Result<Option<Ran>> {
    let today = Local::now().date_naive();
    let last: Option<(String,)> = sqlx::query_as("SELECT value FROM setting WHERE key = ?").bind(DAY_KEY).fetch_optional(pool).await?;
    if last.is_some_and(|(d,)| d == today.to_string()) {
        return Ok(None);
    }
    let ran = run(pool, Utc::now(), today).await?;
    sqlx::query(
        "INSERT INTO setting (key, value) VALUES (?, ?) ON CONFLICT (key) DO UPDATE SET value = excluded.value, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
    )
    .bind(DAY_KEY)
    .bind(today.to_string())
    .execute(pool)
    .await?;
    Ok(Some(ran))
}

/// A minute and a half after launch, then hourly, over the open workspaces with Projects on.
pub fn spawn(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(90)).await;
        loop {
            let hub = app.state::<WorkspaceHub>();
            for ws in hub.list().await.unwrap_or_default() {
                let Ok(pool) = hub.pool(&ws.slug) else { continue };
                if !switches::enabled(&pool).await.map(|m| m.projects).unwrap_or(true) {
                    continue;
                }
                match run_due(&pool).await {
                    Ok(Some(ran)) => tracing::info!("project automations in {}: {ran:?}", ws.slug),
                    Ok(None) => {}
                    Err(e) => tracing::warn!("project automations in {}: {e:#}", ws.slug),
                }
            }
            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use chrono::{Duration as Days, Local, Months, Utc};

    use super::*;
    use crate::modules::pm::bulk::{self, PmBulkAction};
    use crate::modules::pm::inbox::{self, InboxQuery, NoticeKind, InboxShow};
    use crate::modules::pm::items::{ItemInput, Priority};
    use crate::modules::pm::relations::{self, PmRelationKind};
    use crate::modules::pm::{self, Automations, Features, NewPmProject, Network, PmProjectPatch, settings};

    fn inbox_query(mentions: bool, show: InboxShow) -> InboxQuery {
        InboxQuery { mentions, show, assigned: false, created: false, subscribed: false }
    }

    #[test]
    fn work_items_ii_as_plane_says() {
        let root = std::env::temp_dir().join(format!("newdawn-pm-auto-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let folder = hub.workspace("test").await.unwrap().folder_path;
            let new = NewPmProject { name: "VIFC Fintech Hub".into(), identifier: "VIFC".into(), description: String::new(), emoji: String::new(), cover: String::new(), network: Network::Private, lead: None, features: Features::default() };
            let p = pm::create(&pool, new).await.unwrap();
            let states = settings::states(&pool, &p.id).await.unwrap();
            let state = |name: &str| states.iter().find(|s| s.name == name).unwrap().id.clone();
            let make = |name: &str| ItemInput { project_id: p.id.clone(), name: name.into(), ..ItemInput::default() };

            // Bulk: ten items' priority, ten activities; one refused names the item and writes nothing.
            let mut ten = Vec::new();
            for n in 0..10 {
                ten.push(items::create(&pool, make(&format!("Item {n}")), "me").await.unwrap().id);
            }
            let high = items::ItemPatch { priority: Some(Priority::High), ..items::ItemPatch::default() };
            assert_eq!(bulk::update(&pool, &ten, &high, "me").await.unwrap(), 10);
            let rows: (i64,) = sqlx::query_as("SELECT count(*) FROM pm_activity WHERE field = 'priority' AND new_value = 'High'").fetch_one(&pool).await.unwrap();
            assert_eq!(rows.0, 10, "ten activities");
            let backwards = items::ItemPatch { start_date: Some("2026-12-10".into()), target_date: Some("2026-12-01".into()), ..items::ItemPatch::default() };
            let err = bulk::update(&pool, &ten[..2], &backwards, "me").await.unwrap_err().to_string();
            assert!(err.contains("VIFC-1: "), "{err}");
            assert!(items::get(&pool, &ten[1]).await.unwrap().start_date.is_none(), "nothing written");
            let refused = bulk::act(&pool, &folder, &ten[..2], PmBulkAction::Archive, "me").await.unwrap_err().to_string();
            assert!(refused.contains("VIFC-1, VIFC-2 are not"), "{refused}");
            assert_eq!(bulk::copy(&pool, &folder, &ten[..2], &p.id, "me").await.unwrap(), 2);

            // Relations both ways, one a pair; links; votes.
            let blocked = relations::relate(&pool, &ten[0], PmRelationKind::BlockedBy, &ten[1], "me").await.unwrap();
            assert_eq!((blocked.kind, blocked.item.key.as_str()), (PmRelationKind::BlockedBy, "VIFC-2"));
            let other_side = relations::relations(&pool, &ten[1]).await.unwrap();
            assert_eq!((other_side[0].kind, other_side[0].item.key.as_str()), (PmRelationKind::Blocking, "VIFC-1"));
            assert!(relations::relate(&pool, &ten[1], PmRelationKind::RelatesTo, &ten[0], "me").await.unwrap_err().to_string().contains("already related"));
            assert_eq!(relations::search(&pool, "vifc-2", None, None).await.unwrap()[0].id, ten[1]);
            relations::unrelate(&pool, &blocked.id, "me").await.unwrap();
            assert!(relations::relations(&pool, &ten[1]).await.unwrap().is_empty());
            let link = relations::add_link(&pool, &ten[0], "sbv.gov.vn/sandbox", "", "me").await.unwrap();
            assert!(link.url.ends_with("://sbv.gov.vn/sandbox") && link.url.starts_with("https"));
            assert!(relations::add_link(&pool, &ten[0], "ftp://x", "", "me").await.is_err());
            assert!(relations::add_link(&pool, &ten[0], &link.url, "Again", "me").await.unwrap_err().to_string().contains("already"));
            // Attachments: every file checked first — one too large, none attached.
            let small = root.join("terms.pdf");
            let large = root.join("scan.pdf");
            std::fs::write(&small, b"%PDF").unwrap();
            std::fs::write(&large, vec![0u8; 1024 * 1024 + 1]).unwrap();
            let refused = relations::attach_all(&pool, &folder, &ten[0], &[small.as_path(), large.as_path()], 1, "me").await.unwrap_err().to_string();
            assert!(refused.contains("scan.pdf"), "{refused}");
            assert!(relations::attachments(&pool, &folder, &ten[0]).await.unwrap().is_empty(), "none attached");
            let kept = relations::attach_all(&pool, &folder, &ten[0], &[small.as_path()], 1, "me").await.unwrap();
            assert!(std::path::Path::new(&kept[0].path).exists());
            assert_eq!(items::vote(&pool, &ten[0], 1).await.unwrap().vote, 1);
            assert_eq!(items::vote(&pool, &ten[0], 0).await.unwrap().vote, 0);

            // Others' doings on what the owner follows reach the Inbox; the owner's own don't.
            let none = inbox::list(&pool, &inbox_query(false, InboxShow::Inbox), &[]).await.unwrap();
            assert!(none.is_empty(), "the owner's own changes aren't news");
            items::update(&pool, &ten[2], items::ItemPatch { state_id: Some(state("Done")), ..items::ItemPatch::default() }, "Projects agent").await.unwrap();
            let news = inbox::list(&pool, &inbox_query(false, InboxShow::Inbox), &[]).await.unwrap();
            assert_eq!((news.len(), news[0].kind, news[0].actor.as_str(), news[0].new_value.as_str()), (1, NoticeKind::Completed, "Projects agent", "Done"));
            assert_eq!(inbox::counts(&pool, &[]).await.unwrap().all, 1);
            assert_eq!(inbox::counts(&pool, &[NoticeKind::Completed]).await.unwrap().all, 0, "muted");
            inbox::mark(&pool, &news[0].id, true).await.unwrap();
            assert_eq!(inbox::counts(&pool, &[]).await.unwrap().all, 0);
            inbox::archive(&pool, &news[0].id, true).await.unwrap();
            assert_eq!(inbox::list(&pool, &inbox_query(false, InboxShow::Archived), &[]).await.unwrap().len(), 1);
            // A comment by an agent naming the owner is a mention, not also a comment.
            let doc = r#"{"type":"doc","content":[{"type":"paragraph","content":[{"type":"mention","attrs":{"id":"me","label":"You","object":"workspaceMembers"}},{"type":"text","text":" please check"}]}]}"#;
            crate::modules::pm::comments::add(&pool, &ten[3], doc, None, "Projects agent").await.unwrap();
            assert_eq!(inbox::list(&pool, &inbox_query(true, InboxShow::Inbox), &[]).await.unwrap().len(), 1);
            assert_eq!(inbox::counts(&pool, &[]).await.unwrap().all, 0, "not twice");
            let snoozed = inbox::list(&pool, &inbox_query(true, InboxShow::Inbox), &[]).await.unwrap()[0].id.clone();
            let later = (Utc::now() + Days::days(1)).format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
            inbox::snooze(&pool, &snoozed, Some(&later)).await.unwrap();
            assert!(inbox::list(&pool, &inbox_query(true, InboxShow::Inbox), &[]).await.unwrap().is_empty());
            assert_eq!(inbox::list(&pool, &inbox_query(true, InboxShow::Snoozed), &[]).await.unwrap().len(), 1);
            assert!(inbox::snooze(&pool, &snoozed, Some("2020-01-01T00:00:00.000Z")).await.is_err());

            // The automations: reminders the day before, once; an idle Backlog item closed into
            // Cancelled after three months; a closed one archived after one.
            let tied = relations::relate(&pool, &ten[3], PmRelationKind::RelatesTo, &ten[2], "me").await.unwrap();
            let today = Local::now().date_naive();
            let tomorrow = today.succ_opt().unwrap();
            items::update(&pool, &ten[4], items::ItemPatch { target_date: Some(tomorrow.to_string()), ..items::ItemPatch::default() }, "me").await.unwrap();
            let auto = Automations { archive_in: 1, close_in: 3, close_state_id: None, due_reminders: true };
            pm::update(&pool, &p.id, PmProjectPatch { automations: Some(auto), ..PmProjectPatch::default() }, "me").await.unwrap();
            let ran = run(&pool, Utc::now(), today).await.unwrap();
            assert_eq!(ran, Ran { archived: 0, closed: 0, reminded: 1 });
            let reminders = inbox::list(&pool, &inbox_query(false, InboxShow::Inbox), &[]).await.unwrap();
            assert_eq!((reminders[0].kind, reminders[0].due_on.as_deref()), (NoticeKind::Reminder, Some(tomorrow.to_string().as_str())));
            assert_eq!(run(&pool, Utc::now(), today).await.unwrap().reminded, 0, "once a due date");
            // Four months on: the open items idle three months are cancelled, Done ones archived.
            let later = Utc::now().checked_add_months(Months::new(4)).unwrap();
            let ran = run(&pool, later, today).await.unwrap();
            assert!(ran.closed >= 9, "{ran:?}");
            assert_eq!(items::get(&pool, &ten[5]).await.unwrap().state_name, "Cancelled");
            let acts = items::activities(&pool, &ten[5]).await.unwrap();
            assert_eq!(acts.last().map(|a| (a.actor.as_str(), a.field.as_str())), Some((AUTOMATION, "state")));
            assert!(items::get(&pool, &ten[2]).await.unwrap().archived_at.is_some(), "Done for over a month: archived");
            relations::unrelate(&pool, &tied.id, "me").await.unwrap();
            assert!(run_due(&pool).await.unwrap().is_some() && run_due(&pool).await.unwrap().is_none(), "once a day");
            hub.close_all().await;
        });
        let _ = std::fs::remove_dir_all(root);
    }
}
