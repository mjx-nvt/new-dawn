//! Record page layouts (D-062): Twenty's page layouts — per object, tabs of widgets on a
//! 12-column grid. An object shows the defaults computed from its metadata until the layout
//! editor saves a layout of its own (a whole layout at once); Reset deletes it and the defaults
//! return. Also the record page's other reads: a record's timeline with its field labels, and
//! its neighbours in the list.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use specta::Type;
use sqlx::SqlitePool;

use super::types::{FieldType, Json, ObjectDef, RelationKind};
use crate::error::AppError;

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";
/// Twenty's widget types, and New Dawn's COMMITMENTS and MEETINGS (on a person).
pub const WIDGETS: [&str; 13] = ["FIELDS", "RELATED", "TIMELINE", "TASKS", "NOTES", "FILES", "RICH_TEXT", "CHART", "EMAILS", "CALENDAR", "IFRAME", "COMMITMENTS", "MEETINGS"];

#[derive(Debug, Clone, Serialize, Deserialize, Type, sqlx::FromRow)]
pub struct LayoutRow {
    pub id: String,
    pub object_id: Option<String>,
    pub name: String,
    /// RECORD · DASHBOARD
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, sqlx::FromRow)]
pub struct TabRow {
    pub id: String,
    pub layout_id: String,
    pub title: String,
    pub position: f64,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Widget {
    pub id: String,
    pub tab_id: String,
    pub r#type: String,
    pub title: String,
    pub position: f64,
    pub grid_x: i32,
    pub grid_y: i32,
    pub grid_w: i32,
    pub grid_h: i32,
    /// FIELDS `{fields: [names]}` (none = all) · RELATED `{field, editable}` · RICH_TEXT `{text}`.
    pub config: Json,
}

#[derive(Debug, sqlx::FromRow)]
struct WidgetRow {
    id: String,
    tab_id: String,
    #[sqlx(rename = "type")]
    type_name: String,
    title: String,
    position: f64,
    grid_x: i32,
    grid_y: i32,
    grid_w: i32,
    grid_h: i32,
    config_json: String,
}

impl From<WidgetRow> for Widget {
    fn from(r: WidgetRow) -> Self {
        Self {
            id: r.id,
            tab_id: r.tab_id,
            r#type: r.type_name,
            title: r.title,
            position: r.position,
            grid_x: r.grid_x,
            grid_y: r.grid_y,
            grid_w: r.grid_w,
            grid_h: r.grid_h,
            config: Json(serde_json::from_str(&r.config_json).unwrap_or(Value::Object(Default::default()))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Tab {
    pub tab: TabRow,
    pub widgets: Vec<Widget>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Layout {
    pub layout: LayoutRow,
    pub tabs: Vec<Tab>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct WidgetInput {
    pub r#type: String,
    pub title: String,
    pub position: f64,
    pub grid_x: i32,
    pub grid_y: i32,
    pub grid_w: i32,
    pub grid_h: i32,
    pub config: Json,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TabInput {
    pub title: String,
    pub position: f64,
    pub is_default: bool,
    pub widgets: Vec<WidgetInput>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct LayoutInput {
    pub tabs: Vec<TabInput>,
}

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

fn widget(r#type: &str, title: &str, x: i32, y: i32, w: i32, h: i32, config: Value) -> WidgetInput {
    WidgetInput { r#type: r#type.into(), title: title.into(), position: 0.0, grid_x: x, grid_y: y, grid_w: w, grid_h: h, config: Json(config) }
}

fn tab(title: &str, position: f64, is_default: bool, widgets: Vec<WidgetInput>) -> TabInput {
    let widgets = widgets
        .into_iter()
        .enumerate()
        .map(|(i, mut w)| {
            w.position = i as f64;
            w
        })
        .collect();
    TabInput { title: title.into(), position, is_default, widgets }
}

/// The object's one-to-many relations to records (not the system targets), as Related widgets.
fn related_tabs(object: &ObjectDef, from: f64) -> Vec<TabInput> {
    object
        .fields
        .iter()
        .filter(|f| f.is_active && !f.is_system && f.r#type == FieldType::Relation && f.options.relation.as_ref().is_some_and(|r| r.kind == RelationKind::OneToMany))
        .enumerate()
        .map(|(i, f)| tab(&f.label, from + i as f64, false, vec![widget("RELATED", &f.label, 0, 0, 12, 8, json!({ "field": f.name, "editable": true }))]))
        .collect()
}

/// Twenty's default record page: Overview (the fields beside the timeline), the related
/// records, Tasks, Notes, Files — and, on a person, New Dawn's Commitments and Meetings.
pub fn defaults(object: &ObjectDef) -> Vec<TabInput> {
    let name = object.meta.name_singular.as_str();
    if matches!(name, "note" | "task") {
        // The body's editor wants the width; Twenty's targets — one row per record the note or
        // task is attached to — are the system junction's rows, editable (0.10.6).
        let targets = if name == "note" { "noteTargets" } else { "taskTargets" };
        return vec![
            tab("Overview", 0.0, true, vec![widget("FIELDS", "Fields", 0, 0, 12, 9, json!({ "fields": [] })), widget("TIMELINE", "Timeline", 0, 9, 12, 6, json!({}))]),
            tab("Related to", 1.0, false, vec![widget("RELATED", "Related to", 0, 0, 12, 8, json!({ "field": targets, "editable": true }))]),
            tab("Files", 2.0, false, vec![widget("FILES", "Files", 0, 0, 12, 8, json!({}))]),
        ];
    }
    let mut tabs = vec![tab("Overview", 0.0, true, vec![widget("FIELDS", "Fields", 0, 0, 7, 10, json!({ "fields": [] })), widget("TIMELINE", "Timeline", 7, 0, 5, 10, json!({}))])];
    let mut next = 1.0;
    if name == "person" {
        tabs.push(tab("Commitments", next, false, vec![widget("COMMITMENTS", "Commitments", 0, 0, 12, 8, json!({}))]));
        tabs.push(tab("Meetings", next + 1.0, false, vec![widget("MEETINGS", "Meetings", 0, 0, 12, 8, json!({}))]));
        next += 2.0;
    }
    let related = related_tabs(object, next);
    next += related.len() as f64;
    tabs.extend(related);
    if !matches!(name, "task" | "note") {
        tabs.push(tab("Tasks", next, false, vec![widget("TASKS", "Tasks", 0, 0, 12, 8, json!({}))]));
        tabs.push(tab("Notes", next + 1.0, false, vec![widget("NOTES", "Notes", 0, 0, 12, 8, json!({}))]));
        next += 2.0;
    }
    tabs.push(tab("Files", next, false, vec![widget("FILES", "Files", 0, 0, 12, 8, json!({}))]));
    tabs
}

async fn layout_row(pool: &SqlitePool, object_id: &str) -> Result<Option<LayoutRow>> {
    Ok(sqlx::query_as("SELECT id, object_id, name, kind FROM page_layout WHERE object_id = ? AND kind = 'RECORD'")
        .bind(object_id)
        .fetch_optional(pool)
        .await?)
}

async fn read(pool: &SqlitePool, row: LayoutRow) -> Result<Layout> {
    let tabs: Vec<TabRow> = sqlx::query_as("SELECT id, layout_id, title, position, is_default FROM page_layout_tab WHERE layout_id = ? ORDER BY position")
        .bind(&row.id)
        .fetch_all(pool)
        .await?;
    let mut out = Vec::with_capacity(tabs.len());
    for t in tabs {
        let widgets: Vec<WidgetRow> = sqlx::query_as(
            "SELECT id, tab_id, type, title, position, grid_x, grid_y, grid_w, grid_h, config_json FROM page_layout_widget WHERE tab_id = ? ORDER BY position",
        )
        .bind(&t.id)
        .fetch_all(pool)
        .await?;
        out.push(Tab { tab: t, widgets: widgets.into_iter().map(Widget::from).collect() });
    }
    Ok(Layout { layout: row, tabs: out })
}

/// Replaces a layout's tabs and widgets. The first tab marked default is the default; none
/// marked, the first tab. Widgets are kept inside the 12 columns.
async fn write(conn: &mut sqlx::SqliteConnection, layout_id: &str, tabs: &[TabInput]) -> Result<()> {
    if tabs.is_empty() {
        return Err(invalid("A record page needs at least one tab"));
    }
    sqlx::query("DELETE FROM page_layout_tab WHERE layout_id = ?").bind(layout_id).execute(&mut *conn).await?;
    let default_at = tabs.iter().position(|t| t.is_default).unwrap_or(0);
    for (i, t) in tabs.iter().enumerate() {
        let title = t.title.trim();
        if title.is_empty() {
            return Err(invalid("A tab needs a title"));
        }
        let tab_id = uuid::Uuid::now_v7().to_string();
        sqlx::query("INSERT INTO page_layout_tab (id, layout_id, title, position, is_default) VALUES (?, ?, ?, ?, ?)")
            .bind(&tab_id)
            .bind(layout_id)
            .bind(title)
            .bind(t.position)
            .bind(i == default_at)
            .execute(&mut *conn)
            .await?;
        for w in &t.widgets {
            if !WIDGETS.contains(&w.r#type.as_str()) {
                return Err(invalid(format!("No widget of type {}", w.r#type)));
            }
            let x = w.grid_x.clamp(0, 11);
            sqlx::query(
                "INSERT INTO page_layout_widget (id, tab_id, type, title, position, grid_x, grid_y, grid_w, grid_h, config_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(uuid::Uuid::now_v7().to_string())
            .bind(&tab_id)
            .bind(&w.r#type)
            .bind(w.title.trim())
            .bind(w.position)
            .bind(x)
            .bind(w.grid_y.max(0))
            .bind(w.grid_w.clamp(1, 12 - x))
            .bind(w.grid_h.clamp(1, 60))
            .bind(w.config.0.to_string())
            .execute(&mut *conn)
            .await?;
        }
    }
    Ok(())
}

/// The object's record page layout: the saved one, else Twenty's defaults computed from the
/// metadata with placeholder ids — so a relation added later (0.10.5) has its tab until the
/// owner saves a layout of their own.
pub async fn record_layout(pool: &SqlitePool, object: &ObjectDef) -> Result<Layout> {
    if let Some(row) = layout_row(pool, &object.meta.id).await? {
        return read(pool, row).await;
    }
    let tabs = defaults(object)
        .into_iter()
        .enumerate()
        .map(|(i, t)| {
            let tab_id = format!("default-{i}");
            let widgets = t
                .widgets
                .into_iter()
                .enumerate()
                .map(|(j, w)| Widget { id: format!("default-{i}-{j}"), tab_id: tab_id.clone(), r#type: w.r#type, title: w.title, position: w.position, grid_x: w.grid_x, grid_y: w.grid_y, grid_w: w.grid_w, grid_h: w.grid_h, config: w.config })
                .collect();
            Tab { tab: TabRow { id: tab_id, layout_id: String::new(), title: t.title, position: t.position, is_default: t.is_default }, widgets }
        })
        .collect();
    Ok(Layout { layout: LayoutRow { id: String::new(), object_id: Some(object.meta.id.clone()), name: format!("{} page", object.meta.label_singular), kind: "RECORD".into() }, tabs })
}

/// The layout editor's Save: the row (made on the first save), every tab and widget, in one
/// transaction.
pub async fn save(pool: &SqlitePool, object: &ObjectDef, input: LayoutInput) -> Result<Layout> {
    let mut tx = pool.begin().await?;
    sqlx::query("INSERT OR IGNORE INTO page_layout (id, object_id, name, kind) VALUES (?, ?, ?, 'RECORD')")
        .bind(uuid::Uuid::now_v7().to_string())
        .bind(&object.meta.id)
        .bind(format!("{} page", object.meta.label_singular))
        .execute(&mut *tx)
        .await?;
    let (id,): (String,) = sqlx::query_as("SELECT id FROM page_layout WHERE object_id = ? AND kind = 'RECORD'").bind(&object.meta.id).fetch_one(&mut *tx).await?;
    write(&mut tx, &id, &input.tabs).await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE page_layout SET updated_at = {NOW} WHERE id = ?"))).bind(&id).execute(&mut *tx).await?;
    tx.commit().await?;
    record_layout(pool, object).await
}

/// Back to Twenty's default page for the object.
pub async fn reset(pool: &SqlitePool, object: &ObjectDef) -> Result<Layout> {
    sqlx::query("DELETE FROM page_layout WHERE object_id = ? AND kind = 'RECORD'").bind(&object.meta.id).execute(pool).await?;
    record_layout(pool, object).await
}

/// One line of a record's timeline, as the page shows it.
#[derive(Debug, Clone, Serialize, Type)]
pub struct TimelineEvent {
    pub id: String,
    pub happens_at: String,
    /// `person.created`, `person.updated`, `person.deleted`, `person.restored`…
    pub name: String,
    /// For an update: `{ "diff": { "<field>": { "before", "after", "label", "type" } } }`.
    pub properties: Json,
    /// Who: the workspace member's name, else "New Dawn" (an agent, an import, the carry-over).
    pub actor: String,
}

/// The newest activities of a record, the field labels added to an update's diff.
pub async fn timeline(pool: &SqlitePool, object: &ObjectDef, record_id: &str, limit: u32) -> Result<Vec<TimelineEvent>> {
    let Some(column) = super::timeline::target_column(pool, object).await? else { return Ok(Vec::new()) };
    let rows: Vec<(String, String, String, String, Option<String>)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT a.id, a.happens_at, a.name, a.properties_json, \
         trim(coalesce(m.name_first, '') || ' ' || coalesce(m.name_last, '')) \
         FROM timeline_activity a LEFT JOIN workspace_member m ON m.id = a.workspace_member_id \
         WHERE a.{column} = ? ORDER BY a.happens_at DESC, a.id DESC LIMIT ?"
    )))
    .bind(record_id)
    .bind(limit.clamp(1, 500))
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, happens_at, name, properties, actor)| {
            let mut properties: Value = serde_json::from_str(&properties).unwrap_or(Value::Object(Default::default()));
            if let Some(diff) = properties.get_mut("diff").and_then(Value::as_object_mut) {
                for (field, change) in diff.iter_mut() {
                    if let (Some(meta), Some(change)) = (object.field(field), change.as_object_mut()) {
                        change.insert("label".into(), Value::String(meta.label.clone()));
                        change.insert("type".into(), Value::String(meta.r#type.as_str().into()));
                    }
                }
            }
            TimelineEvent { id, happens_at, name, properties: Json(properties), actor: actor.filter(|a| !a.trim().is_empty()).unwrap_or_else(|| "New Dawn".into()) }
        })
        .collect())
}

/// The record before and after this one in the object's default order (position, then made).
#[derive(Debug, Clone, Serialize, Type)]
pub struct Neighbours {
    pub prev: Option<String>,
    pub next: Option<String>,
}

// ponytail: reads every id of the table per visit; a LAG/LEAD window query if a table grows past a personal CRM.
pub async fn neighbours(pool: &SqlitePool, object: &ObjectDef, id: &str) -> Result<Neighbours> {
    let table = &object.meta.table_name;
    let order = if object.fields.iter().any(|f| f.r#type == FieldType::Position) { "position ASC, created_at ASC" } else { "created_at ASC" };
    let ids: Vec<(String,)> = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT id FROM {table} WHERE deleted_at IS NULL ORDER BY {order}")))
        .fetch_all(pool)
        .await?;
    let at = ids.iter().position(|(x,)| x == id);
    Ok(Neighbours {
        prev: at.and_then(|i| i.checked_sub(1)).and_then(|i| ids.get(i)).map(|(x,)| x.clone()),
        next: at.and_then(|i| ids.get(i + 1)).map(|(x,)| x.clone()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::hub::WorkspaceHub;

    /// A company's page comes with Twenty's defaults, keeps what the editor saves — a widget
    /// clamped into the grid, the chosen default tab — and goes back to the defaults on reset.
    #[test]
    fn a_layout_is_made_saved_and_reset() {
        let root = std::env::temp_dir().join(format!("newdawn-layouts-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let company = super::super::object(&pool, "company").await.unwrap();
            let made = record_layout(&pool, &company).await.unwrap();
            let titles: Vec<&str> = made.tabs.iter().map(|t| t.tab.title.as_str()).collect();
            assert_eq!(titles, ["Overview", "People", "Opportunities", "Tasks", "Notes", "Files"]);
            assert!(made.tabs[0].tab.is_default);
            assert_eq!(made.tabs[1].widgets[0].config.0["field"], "people");
            assert!(made.layout.id.is_empty(), "the defaults aren't saved until the owner edits");

            let saved = save(
                &pool,
                &company,
                LayoutInput {
                    tabs: vec![
                        tab("Fields only", 0.0, false, vec![widget("FIELDS", "Fields", 10, 0, 6, 4, json!({ "fields": ["name"] }))]),
                        tab("Work", 1.0, true, vec![widget("TASKS", "Tasks", 0, 0, 12, 6, json!({}))]),
                    ],
                },
            )
            .await
            .unwrap();
            assert!(!saved.layout.id.is_empty());
            assert_eq!(saved.tabs.len(), 2);
            assert!(!saved.tabs[0].tab.is_default && saved.tabs[1].tab.is_default);
            assert_eq!((saved.tabs[0].widgets[0].grid_x, saved.tabs[0].widgets[0].grid_w), (10, 2), "kept inside the 12 columns");
            assert_eq!(saved.tabs[0].widgets[0].config.0["fields"], json!(["name"]));
            assert!(save(&pool, &company, LayoutInput { tabs: Vec::new() }).await.is_err(), "a page needs a tab");
            assert!(save(&pool, &company, LayoutInput { tabs: vec![tab("Odd", 0.0, true, vec![widget("GADGET", "", 0, 0, 1, 1, json!({}))])] }).await.is_err());

            let back = reset(&pool, &company).await.unwrap();
            assert!(back.layout.id.is_empty(), "the defaults again");
            assert_eq!(back.tabs.len(), 6);
            assert_eq!(back.tabs[0].tab.title, "Overview");
            hub.close_all().await;
        });
        std::fs::remove_dir_all(&root).unwrap();
    }
}
