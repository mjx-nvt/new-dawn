//! Plane's analytics (D-083): an overview of the open projects (or one), work items created and
//! resolved day by day, and the work counted by a property ("customized insights"). Intake's
//! requests and archived items are left out; days are this Mac's.

use anyhow::{Result, anyhow};
use chrono::{Duration, NaiveDate};
use serde::Serialize;
use specta::Type;
use sqlx::SqlitePool;

use super::cycles::Share;
use crate::error::AppError;

/// The work items counted: open projects', not archived, not Intake's (`?1` a project or null).
const SCOPE: &str = "i.archived_at IS NULL AND s.state_group <> 'triage' AND m.archived_at IS NULL AND (?1 IS NULL OR i.project_id = ?1)";
const FROM: &str = "FROM pm_item i JOIN pm_state s ON s.id = i.state_id JOIN pm_project m ON m.project_id = i.project_id";

#[derive(Debug, Clone, Serialize, Type)]
pub struct PmOverview {
    pub projects: u32,
    pub items: u32,
    pub open_items: u32,
    pub active_cycles: u32,
    /// Intake's requests waiting today.
    pub requests: u32,
}

pub async fn overview(pool: &SqlitePool, project: Option<&str>, today: NaiveDate) -> Result<PmOverview> {
    let day = today.to_string();
    let (projects,): (u32,) = sqlx::query_as("SELECT count(*) FROM pm_project WHERE archived_at IS NULL AND (?1 IS NULL OR project_id = ?1)").bind(project).fetch_one(pool).await?;
    let (items, open_items): (u32, u32) = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT count(*), coalesce(sum(s.state_group NOT IN ('completed', 'cancelled')), 0) {FROM} WHERE {SCOPE}"))).bind(project).fetch_one(pool).await?;
    let (active_cycles,): (u32,) = sqlx::query_as(
        "SELECT count(*) FROM pm_cycle c JOIN pm_project m ON m.project_id = c.project_id \
         WHERE m.archived_at IS NULL AND m.cycles = 1 AND c.archived_at IS NULL AND c.completed_at IS NULL AND c.start_date <= ?2 AND c.end_date >= ?2 AND (?1 IS NULL OR c.project_id = ?1)",
    )
    .bind(project)
    .bind(&day)
    .fetch_one(pool)
    .await?;
    let (requests,): (u32,) = sqlx::query_as(
        "SELECT count(*) FROM pm_intake_item x JOIN pm_item i ON i.id = x.item_id JOIN pm_project m ON m.project_id = i.project_id \
         WHERE m.archived_at IS NULL AND m.intake = 1 AND (?1 IS NULL OR i.project_id = ?1) AND (x.status = 'pending' OR (x.status = 'snoozed' AND x.snoozed_until <= ?2))",
    )
    .bind(project)
    .bind(&day)
    .fetch_one(pool)
    .await?;
    Ok(PmOverview { projects, items, open_items, active_cycles, requests })
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct DayCount {
    pub day: String,
    pub created: u32,
    /// Moved into a completed state that day.
    pub resolved: u32,
}

/// Each day from `from` to `to` (at most a year): the work items made and the ones resolved.
pub async fn created_vs_resolved(pool: &SqlitePool, project: Option<&str>, from: NaiveDate, to: NaiveDate) -> Result<Vec<DayCount>> {
    if to < from || (to - from).num_days() > 366 {
        return Err(anyhow!(AppError::invalid("Choose a period of a year at most")));
    }
    let count = |column: &'static str| async move {
        let rows: Vec<(String, u32)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT date({column}, 'localtime') AS d, count(*) {FROM} WHERE {SCOPE} AND {column} IS NOT NULL AND date({column}, 'localtime') BETWEEN ?2 AND ?3 GROUP BY d"
        )))
        .bind(project)
        .bind(from.to_string())
        .bind(to.to_string())
        .fetch_all(pool)
        .await?;
        Ok::<_, anyhow::Error>(rows)
    };
    let (made, resolved) = (count("i.created_at").await?, count("i.completed_at").await?);
    let on = |rows: &[(String, u32)], day: &str| rows.iter().find(|(d, _)| d == day).map(|(_, n)| *n).unwrap_or(0);
    let mut out = Vec::new();
    let mut day = from;
    while day <= to {
        let d = day.to_string();
        out.push(DayCount { created: on(&made, &d), resolved: on(&resolved, &d), day: d });
        day += Duration::days(1);
    }
    Ok(out)
}

/// The work items counted by a property, and how many of each are done (an item with several
/// labels, assignees or modules counts in each). Across the projects a state, a label, a cycle or
/// a module is one bar by its name (each project's "Done" is one "Done").
pub async fn insights(pool: &SqlitePool, project: Option<&str>, axis: &str) -> Result<Vec<Share>> {
    let done = "sum(s.state_group = 'completed')";
    // A thing's key: its id in one project, its name (any case) across them.
    let key = |id: &str, name: &str| format!("CASE WHEN ?1 IS NULL THEN lower({name}) ELSE {id} END");
    let sql = match axis {
        "state" => format!(
            "SELECT {}, min(s.name), count(*), {done} {FROM} WHERE {SCOPE} GROUP BY 1 \
             ORDER BY min(CASE s.state_group WHEN 'backlog' THEN 0 WHEN 'unstarted' THEN 1 WHEN 'started' THEN 2 WHEN 'completed' THEN 3 ELSE 4 END), min(s.sequence)",
            key("s.id", "s.name")
        ),
        "state_group" => format!(
            "SELECT s.state_group, s.state_group, count(*), {done} {FROM} WHERE {SCOPE} GROUP BY 1 \
             ORDER BY CASE s.state_group WHEN 'backlog' THEN 0 WHEN 'unstarted' THEN 1 WHEN 'started' THEN 2 WHEN 'completed' THEN 3 ELSE 4 END"
        ),
        "priority" => format!(
            "SELECT i.priority, i.priority, count(*), {done} {FROM} WHERE {SCOPE} GROUP BY 1 \
             ORDER BY CASE i.priority WHEN 'urgent' THEN 0 WHEN 'high' THEN 1 WHEN 'medium' THEN 2 WHEN 'low' THEN 3 ELSE 4 END"
        ),
        "assignee" => format!(
            "SELECT coalesce(a.who, 'none'), CASE WHEN a.who IS NULL THEN 'No assignee' WHEN a.who = 'me' THEN 'You' \
             ELSE coalesce(nullif(trim(p.name_first || ' ' || p.name_last), ''), 'Someone') END, count(*), {done} \
             {FROM} LEFT JOIN pm_item_assignee a ON a.item_id = i.id LEFT JOIN person p ON p.id = a.who WHERE {SCOPE} GROUP BY 1 ORDER BY 3 DESC"
        ),
        "label" => format!("SELECT coalesce({}, 'none'), coalesce(min(l.name), 'No label'), count(*), {done} {FROM} LEFT JOIN pm_item_label il ON il.item_id = i.id LEFT JOIN pm_label l ON l.id = il.label_id WHERE {SCOPE} GROUP BY 1 ORDER BY 3 DESC", key("l.id", "l.name")),
        "cycle" => format!("SELECT coalesce({}, 'none'), coalesce(min(c.name), 'No cycle'), count(*), {done} {FROM} LEFT JOIN pm_cycle_item ci ON ci.item_id = i.id LEFT JOIN pm_cycle c ON c.id = ci.cycle_id WHERE {SCOPE} GROUP BY 1 ORDER BY 3 DESC", key("c.id", "c.name")),
        "module" => format!("SELECT coalesce({}, 'none'), coalesce(min(d.name), 'No module'), count(*), {done} {FROM} LEFT JOIN pm_module_item x ON x.item_id = i.id LEFT JOIN pm_module d ON d.id = x.module_id WHERE {SCOPE} GROUP BY 1 ORDER BY 3 DESC", key("d.id", "d.name")),
        _ => return Err(anyhow!(AppError::invalid("Work items can't be counted by that"))),
    };
    let rows: Vec<(String, String, u32, u32)> = sqlx::query_as(sqlx::AssertSqlSafe(sql)).bind(project).fetch_all(pool).await?;
    Ok(rows.into_iter().map(|(key, label, total, done)| Share { key, label, total, done }).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::hub::WorkspaceHub;
    use crate::modules::pm::items::{self, ItemInput, ItemPatch};
    use crate::modules::pm::{self, Features, NewPmProject, Network, settings};

    #[test]
    fn created_and_resolved_count_by_day_and_insights_by_property() {
        let root = std::env::temp_dir().join(format!("newdawn-pm-analytics-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let p = pm::create(&pool, NewPmProject { name: "VIFC Fintech Hub".into(), identifier: "VIFC".into(), description: String::new(), emoji: String::new(), cover: String::new(), network: Network::Private, lead: None, features: Features::default() })
                .await
                .unwrap();
            let done = settings::states(&pool, &p.id).await.unwrap().into_iter().find(|s| s.name == "Done").unwrap().id;
            let today = super::super::cycles::today();
            let mut ids = Vec::new();
            for n in 1..=3 {
                ids.push(items::create(&pool, ItemInput { project_id: p.id.clone(), name: format!("Task {n}"), assignees: vec!["me".into()], ..ItemInput::default() }, "me").await.unwrap().id);
            }
            items::update(&pool, &ids[0], ItemPatch { state_id: Some(done), ..ItemPatch::default() }, "me").await.unwrap();
            let week = created_vs_resolved(&pool, Some(&p.id), today - Duration::days(6), today).await.unwrap();
            assert_eq!(week.len(), 7);
            assert_eq!((week[6].created, week[6].resolved), (3, 1), "today: three made, one resolved");
            assert_eq!(week[0].created, 0);
            let o = overview(&pool, Some(&p.id), today).await.unwrap();
            assert_eq!((o.projects, o.items, o.open_items), (1, 3, 2));
            let by = insights(&pool, None, "state_group").await.unwrap();
            assert_eq!(by.iter().map(|s| (s.key.as_str(), s.total)).collect::<Vec<_>>(), [("backlog", 2), ("completed", 1)]);
            assert_eq!(insights(&pool, None, "assignee").await.unwrap()[0].label, "You");
            assert!(insights(&pool, None, "colour").await.is_err());
            hub.close_all().await;
        });
        let _ = std::fs::remove_dir_all(root);
    }
}
