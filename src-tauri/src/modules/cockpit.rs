//! The Cockpit (D-057): what needs the owner today, across every open workspace — tasks due,
//! projects whose milestone is near, commitments to follow up, this week's meetings and the
//! proposals waiting. Each workspace is read from its own pool through the hub; nothing is
//! stored and nothing is asked of a model.

use anyhow::Result;
use chrono::{Datelike, Duration, Local, NaiveDate, NaiveTime, TimeZone, Utc};
use serde::Serialize;
use specta::Type;
use sqlx::SqlitePool;

use crate::kernel::hub::WorkspaceHub;
use crate::modules::crm::{self, Commitment};
use crate::modules::meetings::MeetingStatus;
use crate::modules::projects::{self, Project, ProjectStatus};
use crate::modules::tasks::{self, TaskView};

/// A project shows when its next milestone is due within this many days, or is overdue.
const WATCH_DAYS: i64 = 7;
/// Commitments shown per workspace; the rest are counted.
const COMMITMENTS: usize = 8;

#[derive(Debug, Clone, Serialize, Type)]
pub struct Today {
    /// The day on this Mac, YYYY-MM-DD.
    pub day: String,
    /// The current workspace first, then the others.
    pub workspaces: Vec<Day>,
    /// Workspaces that couldn't be read (not open, or an error), by name.
    pub unread: Vec<String>,
}

/// One workspace's part of the day.
#[derive(Debug, Clone, Serialize, Type)]
pub struct Day {
    pub slug: String,
    pub name: String,
    /// Open tasks due today or earlier, as Tasks lists them under Today.
    pub tasks: Vec<TaskView>,
    /// Active projects whose next milestone is due within a week or overdue, soonest first.
    pub projects: Vec<Project>,
    /// Commitments the owner confirmed and follows up, newest first (the first 8).
    pub commitments: Vec<Commitment>,
    /// How many there are in all.
    pub open_commitments: u32,
    /// This week's meetings, Monday to Sunday, in order.
    pub meetings: Vec<WeekMeeting>,
    /// Proposals waiting for the owner.
    pub waiting: Waiting,
}

#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct WeekMeeting {
    pub id: String,
    pub title: String,
    pub held_on: String,
    pub status: MeetingStatus,
}

#[derive(Debug, Clone, Default, Serialize, Type, sqlx::FromRow)]
pub struct Waiting {
    /// Project updates.
    pub updates: u32,
    pub commitments: u32,
    /// Tasks proposed from emails.
    pub email_tasks: u32,
}

pub async fn today(hub: &WorkspaceHub) -> Result<Today> {
    let day = Local::now().date_naive();
    let current = hub.current_workspace().await?.map(|w| w.slug);
    let (mut workspaces, mut unread) = (Vec::new(), Vec::new());
    // Each active workspace from its own pool in the hub. One that can't be read (it failed to
    // open at launch, or a query failed) is named; the others still show.
    for w in hub.list().await?.into_iter().filter(|w| !w.archived) {
        let day = match hub.pool(&w.slug) {
            Ok(pool) => read(&pool, day, w.slug.clone(), w.name.clone()).await,
            Err(err) => Err(err),
        };
        match day {
            Ok(d) => workspaces.push(d),
            Err(err) => {
                tracing::warn!("the cockpit couldn't read {}: {err:#}", w.slug);
                unread.push(w.name);
            }
        }
    }
    // The current workspace first; the others keep the registry's order.
    workspaces.sort_by_key(|d| Some(&d.slug) != current.as_ref());
    Ok(Today {
        day: day.to_string(),
        workspaces,
        unread,
    })
}

async fn read(pool: &SqlitePool, day: NaiveDate, slug: String, name: String) -> Result<Day> {
    let tasks = tasks::list(pool).await?.today;
    let soon = (day + Duration::days(WATCH_DAYS)).to_string();
    let mut projects: Vec<Project> = projects::list(pool)
        .await?
        .into_iter()
        .filter(|p| {
            p.status == ProjectStatus::Active
                && p.milestone_due
                    .as_deref()
                    .is_some_and(|d| d <= soon.as_str())
        })
        .collect();
    projects.sort_by(|a, b| a.milestone_due.cmp(&b.milestone_due));
    let mut commitments = crm::open(pool).await?;
    let open_commitments = commitments.len() as u32;
    commitments.truncate(COMMITMENTS);
    let (from, to) = week(day);
    let meetings = sqlx::query_as(
        "SELECT id, title, held_on, status FROM meeting WHERE held_on >= ? AND held_on < ? \
         ORDER BY held_on",
    )
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    let waiting = sqlx::query_as(
        "SELECT (SELECT count(*) FROM project_update WHERE status = 'proposed') AS updates, \
         (SELECT count(*) FROM commitment WHERE status = 'proposed') AS commitments, \
         (SELECT count(*) FROM email_task WHERE status = 'proposed') AS email_tasks",
    )
    .fetch_one(pool)
    .await?;
    Ok(Day {
        slug,
        name,
        tasks,
        projects,
        commitments,
        open_commitments,
        meetings,
        waiting,
    })
}

/// This week on this Mac — Monday 00:00 to the next Monday — as the UTC times `held_on` holds.
fn week(day: NaiveDate) -> (String, String) {
    let monday = day - Duration::days(i64::from(day.weekday().num_days_from_monday()));
    let utc = |d: NaiveDate| {
        Local
            .from_local_datetime(&d.and_time(NaiveTime::MIN))
            .earliest()
            .map(|t| {
                t.with_timezone(&Utc)
                    .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                    .to_string()
            })
            .unwrap_or_else(|| format!("{d}T00:00:00.000Z"))
    };
    (utc(monday), utc(monday + Duration::days(7)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_day_reads_what_needs_the_owner() {
        let root = std::env::temp_dir().join(format!("newdawn-cockpit-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let day = Local::now().date_naive();
            let at = |d: NaiveDate| format!("{d}");
            for (title, due) in [
                ("Overdue", Some(day - Duration::days(2))),
                ("Due today", Some(day)),
                ("Later", Some(day + Duration::days(3))),
                ("No date", None),
            ] {
                sqlx::query(
                    "INSERT INTO task (id, title, origin, due_on) VALUES (?, ?, 'typed', ?)",
                )
                .bind(uuid::Uuid::now_v7().to_string())
                .bind(title)
                .bind(due.map(at))
                .execute(&pool)
                .await
                .unwrap();
            }
            for (name, status, due) in [
                ("Soon", "active", Some(day + Duration::days(3))),
                ("Overdue", "active", Some(day - Duration::days(1))),
                ("Far", "active", Some(day + Duration::days(30))),
                ("On hold", "on_hold", Some(day)),
                ("No milestone", "active", None),
            ] {
                sqlx::query(
                    "INSERT INTO project (id, name, status, milestone_due) VALUES (?, ?, ?, ?)",
                )
                .bind(uuid::Uuid::now_v7().to_string())
                .bind(name)
                .bind(status)
                .bind(due.map(at))
                .execute(&pool)
                .await
                .unwrap();
            }
            let (monday, next_monday) = week(day);
            for (id, held_on) in [
                ("this", monday.clone()),
                ("before", "2000-01-01T00:00:00.000Z".into()),
                ("next", next_monday),
            ] {
                sqlx::query("INSERT INTO meeting (id, title, held_on) VALUES (?, ?, ?)")
                    .bind(id)
                    .bind(id)
                    .bind(held_on)
                    .execute(&pool)
                    .await
                    .unwrap();
            }
            // A second workspace, made current: it comes first.
            hub.create_workspace("Majoris", "other").await.unwrap();
            hub.set_current_workspace("other").await.unwrap();
            let today = today(&hub).await.unwrap();
            let slugs: Vec<&str> = today.workspaces.iter().map(|d| d.slug.as_str()).collect();
            assert_eq!(slugs, ["other", "test"], "the current workspace first");
            assert!(today.unread.is_empty());
            let d = &today.workspaces[1];
            let titles: Vec<&str> = d.tasks.iter().map(|t| t.title.as_str()).collect();
            assert_eq!(titles.len(), 2, "due today or earlier: {titles:?}");
            let names: Vec<&str> = d.projects.iter().map(|p| p.name.as_str()).collect();
            assert_eq!(
                names,
                ["Overdue", "Soon"],
                "active, within a week, soonest first"
            );
            let meetings: Vec<&str> = d.meetings.iter().map(|m| m.id.as_str()).collect();
            assert_eq!(meetings, ["this"], "Monday counts; next Monday doesn't");
            hub.close_all().await;
        });
        let _ = std::fs::remove_dir_all(root);
    }
}
