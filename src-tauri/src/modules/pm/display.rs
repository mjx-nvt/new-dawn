//! A project's display (D-076, Plane's display options): the layout — List, Board, Calendar,
//! Table, Timeline —, how the items are grouped, sub-grouped (Board) and ordered, the properties
//! a row or card shows, whether sub-items and empty groups show, the calendar's span, the
//! timeline's zoom and the board's folded columns; remembered per project (one owner — Plane
//! keeps them per user).

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;

use crate::error::AppError;

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";

pub const LAYOUTS: [&str; 5] = ["list", "board", "calendar", "table", "timeline"];
pub const GROUPS: [&str; 8] = ["state", "group", "priority", "labels", "assignees", "cycle", "created_by", "none"];
pub const ORDERS: [&str; 6] = ["manual", "created", "updated", "start", "due", "priority"];
/// Plane's display properties (Table adds its created and updated columns).
pub const PROPERTIES: [&str; 15] = ["id", "state", "priority", "assignees", "labels", "start", "due", "estimate", "cycle", "sub_items", "attachments", "links", "created", "created_by", "updated"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct PmDisplay {
    pub layout: String,
    pub group: String,
    /// The Board's rows ("none": none).
    pub sub_group: String,
    pub order: String,
    pub properties: Vec<String>,
    pub sub_items: bool,
    pub empty_groups: bool,
    /// "month" or "week".
    pub calendar: String,
    /// "week", "month" or "quarter".
    pub zoom: String,
    /// The Board's folded columns (their group keys).
    pub collapsed: Vec<String>,
}

impl Default for PmDisplay {
    fn default() -> Self {
        Self {
            layout: "list".into(),
            group: "state".into(),
            sub_group: "none".into(),
            order: "manual".into(),
            properties: ["id", "state", "priority", "assignees", "labels", "due", "sub_items"].map(String::from).to_vec(),
            sub_items: true,
            empty_groups: true,
            calendar: "month".into(),
            zoom: "month".into(),
            collapsed: Vec::new(),
        }
    }
}

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

#[derive(sqlx::FromRow)]
struct Row {
    layout: String,
    group_by: String,
    sub_group_by: String,
    order_by: String,
    properties_json: String,
    sub_items: bool,
    empty_groups: bool,
    calendar: String,
    zoom: String,
    collapsed_json: String,
}

pub async fn get(pool: &SqlitePool, project_id: &str) -> Result<PmDisplay> {
    let row: Option<Row> = sqlx::query_as(
        "SELECT layout, group_by, sub_group_by, order_by, properties_json, sub_items, empty_groups, calendar, zoom, collapsed_json FROM pm_display WHERE project_id = ?",
    )
    .bind(project_id)
    .fetch_optional(pool)
    .await?;
    Ok(row
        .map(|r| PmDisplay {
            layout: r.layout,
            group: r.group_by,
            sub_group: r.sub_group_by,
            order: r.order_by,
            properties: serde_json::from_str(&r.properties_json).unwrap_or_default(),
            sub_items: r.sub_items,
            empty_groups: r.empty_groups,
            calendar: r.calendar,
            zoom: r.zoom,
            collapsed: serde_json::from_str(&r.collapsed_json).unwrap_or_default(),
        })
        .unwrap_or_default())
}

/// A display checked: known layout, grouping, order, span, zoom and properties; a sub-group that
/// repeats the grouping (or state with state group) is none. Views keep displays too (D-077).
pub fn check(display: PmDisplay) -> Result<PmDisplay> {
    let one_of = |what: &str, value: &str, known: &[&str]| if known.contains(&value) { Ok(()) } else { Err(invalid(format!("Unknown {what} \"{value}\""))) };
    one_of("layout", &display.layout, &LAYOUTS)?;
    one_of("grouping", &display.group, &GROUPS)?;
    one_of("sub-grouping", &display.sub_group, &GROUPS)?;
    one_of("order", &display.order, &ORDERS)?;
    one_of("calendar span", &display.calendar, &["month", "week"])?;
    one_of("zoom", &display.zoom, &["week", "month", "quarter"])?;
    for p in &display.properties {
        one_of("property", p, &PROPERTIES)?;
    }
    if display.collapsed.len() > 100 {
        return Err(invalid("Too many folded columns"));
    }
    let mut display = display;
    let pair = |a: &str, b: &str| a == b || (["state", "group"].contains(&a) && ["state", "group"].contains(&b));
    if pair(&display.sub_group, &display.group) {
        display.sub_group = "none".into();
    }
    Ok(display)
}

pub async fn set(pool: &SqlitePool, project_id: &str, display: PmDisplay) -> Result<PmDisplay> {
    super::get(pool, project_id).await?;
    let display = check(display)?;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO pm_display (project_id, layout, group_by, sub_group_by, order_by, properties_json, sub_items, empty_groups, calendar, zoom, collapsed_json) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT (project_id) DO UPDATE SET layout = excluded.layout, group_by = excluded.group_by, \
         sub_group_by = excluded.sub_group_by, order_by = excluded.order_by, properties_json = excluded.properties_json, sub_items = excluded.sub_items, \
         empty_groups = excluded.empty_groups, calendar = excluded.calendar, zoom = excluded.zoom, collapsed_json = excluded.collapsed_json, updated_at = {NOW}"
    )))
    .bind(project_id)
    .bind(&display.layout)
    .bind(&display.group)
    .bind(&display.sub_group)
    .bind(&display.order)
    .bind(serde_json::to_string(&display.properties)?)
    .bind(display.sub_items)
    .bind(display.empty_groups)
    .bind(&display.calendar)
    .bind(&display.zoom)
    .bind(serde_json::to_string(&display.collapsed)?)
    .execute(pool)
    .await?;
    get(pool, project_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::hub::WorkspaceHub;
    use crate::modules::pm::{self, Features, NewPmProject, Network};

    #[test]
    fn a_projects_display_is_remembered_and_checked() {
        let root = std::env::temp_dir().join(format!("newdawn-pm-display-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let new = NewPmProject { name: "VIFC Fintech Hub".into(), identifier: "VIFC".into(), description: String::new(), emoji: String::new(), cover: String::new(), network: Network::Private, lead: None, features: Features::default() };
            let p = pm::create(&pool, new).await.unwrap();
            assert_eq!(get(&pool, &p.id).await.unwrap(), PmDisplay::default(), "none kept: the defaults");
            let board = PmDisplay { layout: "board".into(), sub_group: "assignees".into(), collapsed: vec!["done".into()], ..PmDisplay::default() };
            assert_eq!(set(&pool, &p.id, board.clone()).await.unwrap(), board);
            assert!(set(&pool, &p.id, PmDisplay { layout: "gantt".into(), ..PmDisplay::default() }).await.is_err());
            assert!(set(&pool, &p.id, PmDisplay { properties: vec!["colour".into()], ..PmDisplay::default() }).await.is_err());
            // A sub-group repeating the grouping is none.
            let kept = set(&pool, &p.id, PmDisplay { group: "assignees".into(), sub_group: "assignees".into(), ..board.clone() }).await.unwrap();
            assert_eq!(kept.sub_group, "none");
            // Merged people move their folded column; one deleted for good leaves none (the trigger).
            for (id, name) in [("p1", "Lan Pham"), ("p2", "Lan P.")] {
                sqlx::query("INSERT INTO person (id, name, name_first) VALUES (?, ?, 'Lan')").bind(id).bind(name).execute(&pool).await.unwrap();
            }
            set(&pool, &p.id, PmDisplay { group: "assignees".into(), sub_group: "none".into(), collapsed: vec!["p1".into(), "me".into()], ..board.clone() }).await.unwrap();
            crate::modules::people::merge(&pool, "p2", "p1").await.unwrap();
            assert_eq!(get(&pool, &p.id).await.unwrap().collapsed, vec!["p2".to_string(), "me".to_string()]);
            sqlx::query("DELETE FROM person WHERE id = 'p2'").execute(&pool).await.unwrap();
            assert_eq!(get(&pool, &p.id).await.unwrap().collapsed, vec!["me".to_string()]);
            hub.close_all().await;
        });
        let _ = std::fs::remove_dir_all(root);
    }
}
