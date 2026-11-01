//! Views (D-061): a saved way of looking at an object's records — Twenty's table, later kanban
//! and calendar: which fields, in what order and width, with which aggregate; filters as rule
//! groups (AND / OR, nested); sorts; a group-by select field with its own order and hidden
//! groups; compact; where a record opens; who sees it. Each object has an "All <X>" view
//! (key INDEX) that can't be deleted.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use specta::Type;
use sqlx::SqlitePool;

use super::query::{Query, Sort};
use super::types::{FieldType, Json, ObjectDef};
use crate::error::AppError;

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";
pub const INDEX: &str = "INDEX";

#[derive(Debug, Clone, Serialize, Deserialize, Type, sqlx::FromRow)]
pub struct ViewRow {
    pub id: String,
    pub object_id: String,
    pub name: String,
    pub icon: String,
    /// TABLE · KANBAN · CALENDAR
    pub r#type: String,
    /// INDEX for the object's default view, else none.
    pub key: Option<String>,
    pub position: f64,
    pub is_compact: bool,
    /// SIDE_PANEL · RECORD_PAGE
    pub open_record_in: String,
    /// WORKSPACE · UNLISTED (one owner: a flag)
    pub visibility: String,
    pub group_by_field_id: Option<String>,
    /// MANUAL · ASC · DESC
    pub group_sort: String,
    pub kanban_field_id: Option<String>,
    pub kanban_aggregate_operation: Option<String>,
    pub kanban_aggregate_field_id: Option<String>,
    pub calendar_field_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, sqlx::FromRow)]
pub struct ViewField {
    pub id: String,
    pub view_id: String,
    pub field_id: String,
    pub is_visible: bool,
    pub position: f64,
    /// The column's width in points.
    pub size: i32,
    /// COUNT · COUNT_EMPTY · COUNT_NOT_EMPTY · COUNT_UNIQUE · PERCENT_EMPTY · PERCENT_NOT_EMPTY
    /// · SUM · AVG · MIN · MAX, or none.
    pub aggregate_operation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, sqlx::FromRow)]
pub struct ViewFilterGroup {
    pub id: String,
    pub view_id: String,
    pub parent_id: Option<String>,
    /// AND · OR · NOT
    pub logical: String,
    pub position: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ViewFilter {
    pub id: String,
    pub view_id: String,
    pub field_id: String,
    /// A composite's part ('' = the whole field).
    pub sub: String,
    /// The engine's operator names (is, contains, isRelative…).
    pub operand: String,
    pub value: Json,
    /// None = the view's own top-level AND.
    pub group_id: Option<String>,
    pub position: f64,
}

#[derive(Debug, sqlx::FromRow)]
struct FilterRow {
    id: String,
    view_id: String,
    field_id: String,
    sub: String,
    operand: String,
    value_json: String,
    group_id: Option<String>,
    position: f64,
}

impl From<FilterRow> for ViewFilter {
    fn from(r: FilterRow) -> Self {
        Self {
            id: r.id,
            view_id: r.view_id,
            field_id: r.field_id,
            sub: r.sub,
            operand: r.operand,
            value: Json(serde_json::from_str(&r.value_json).unwrap_or(Value::Null)),
            group_id: r.group_id,
            position: r.position,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, sqlx::FromRow)]
pub struct ViewSort {
    pub id: String,
    pub view_id: String,
    pub field_id: String,
    pub sub: String,
    /// ASC · DESC
    pub direction: String,
    pub position: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, sqlx::FromRow)]
pub struct ViewGroup {
    pub id: String,
    pub view_id: String,
    /// A select option's value ('' = no value).
    pub field_value: String,
    pub is_visible: bool,
    pub position: f64,
}

/// A view with everything it saves.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ViewDef {
    pub view: ViewRow,
    pub fields: Vec<ViewField>,
    pub filter_groups: Vec<ViewFilterGroup>,
    pub filters: Vec<ViewFilter>,
    pub sorts: Vec<ViewSort>,
    pub groups: Vec<ViewGroup>,
}

const VIEW_COLUMNS: &str = "id, object_id, name, icon, type, key, position, is_compact, open_record_in, visibility, \
     group_by_field_id, group_sort, kanban_field_id, kanban_aggregate_operation, kanban_aggregate_field_id, calendar_field_id";

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

pub async fn list(pool: &SqlitePool, object_id: &str) -> Result<Vec<ViewRow>> {
    Ok(sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT {VIEW_COLUMNS} FROM view WHERE object_id = ? ORDER BY position, created_at")))
        .bind(object_id)
        .fetch_all(pool)
        .await?)
}

async fn row(pool: &SqlitePool, id: &str) -> Result<ViewRow> {
    let row: Option<ViewRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT {VIEW_COLUMNS} FROM view WHERE id = ?")))
        .bind(id)
        .fetch_optional(pool)
        .await?;
    row.ok_or_else(|| anyhow!(AppError::not_found("No such view")))
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<ViewDef> {
    let view = row(pool, id).await?;
    let fields = sqlx::query_as("SELECT id, view_id, field_id, is_visible, position, size, aggregate_operation FROM view_field WHERE view_id = ? ORDER BY position")
        .bind(id)
        .fetch_all(pool)
        .await?;
    let filter_groups = sqlx::query_as("SELECT id, view_id, parent_id, logical, position FROM view_filter_group WHERE view_id = ? ORDER BY position")
        .bind(id)
        .fetch_all(pool)
        .await?;
    let filters: Vec<FilterRow> = sqlx::query_as("SELECT id, view_id, field_id, sub, operand, value_json, group_id, position FROM view_filter WHERE view_id = ? ORDER BY position")
        .bind(id)
        .fetch_all(pool)
        .await?;
    let sorts = sqlx::query_as("SELECT id, view_id, field_id, sub, direction, position FROM view_sort WHERE view_id = ? ORDER BY position")
        .bind(id)
        .fetch_all(pool)
        .await?;
    let groups = sqlx::query_as("SELECT id, view_id, field_value, is_visible, position FROM view_group WHERE view_id = ? ORDER BY position")
        .bind(id)
        .fetch_all(pool)
        .await?;
    Ok(ViewDef { view, fields, filter_groups, filters: filters.into_iter().map(ViewFilter::from).collect(), sorts, groups })
}

/// The fields a table shows by default: the label first, then by position; never system fields,
/// the position, or a one-to-many relation.
fn default_fields(object: &ObjectDef) -> Vec<&super::types::FieldMeta> {
    let mut fields: Vec<_> = object
        .fields
        .iter()
        .filter(|f| f.is_active && !f.is_system && f.r#type != FieldType::Position && !super::types::columns(f).is_empty())
        .collect();
    if let Some(i) = fields.iter().position(|f| Some(f.id.as_str()) == object.meta.label_identifier_field_id.as_deref()) {
        let label = fields.remove(i);
        fields.insert(0, label);
    }
    fields
}

/// A column's default width, by type.
pub fn default_size(t: FieldType) -> i64 {
    match t {
        FieldType::FullName | FieldType::Links | FieldType::Domain | FieldType::Emails | FieldType::Phones => 200,
        FieldType::LongText | FieldType::RichText | FieldType::Address => 240,
        FieldType::Number | FieldType::Currency | FieldType::Rating | FieldType::Boolean | FieldType::Date => 130,
        FieldType::DateTime | FieldType::Select | FieldType::Actor => 150,
        _ => 180,
    }
}

/// The object's "All <X>" view, made when missing, with every table field visible. Returns its
/// id. Fields added to the object later join the view hidden, at the end.
pub async fn ensure_index(pool: &SqlitePool, object: &ObjectDef) -> Result<String> {
    let existing: Option<(String,)> = sqlx::query_as("SELECT id FROM view WHERE object_id = ? AND key = ?")
        .bind(&object.meta.id)
        .bind(INDEX)
        .fetch_optional(pool)
        .await?;
    let id = match existing {
        Some((id,)) => id,
        None => {
            let id = uuid::Uuid::now_v7().to_string();
            sqlx::query("INSERT INTO view (id, object_id, name, icon, type, key, position) VALUES (?, ?, ?, ?, 'TABLE', ?, 0)")
                .bind(&id)
                .bind(&object.meta.id)
                .bind(format!("All {}", object.meta.label_plural))
                .bind(&object.meta.icon)
                .bind(INDEX)
                .execute(pool)
                .await?;
            id
        }
    };
    let known: Vec<(String,)> = sqlx::query_as("SELECT field_id FROM view_field WHERE view_id = ?").bind(&id).fetch_all(pool).await?;
    let fresh = known.is_empty();
    let known: Vec<String> = known.into_iter().map(|(f,)| f).collect();
    for (i, field) in default_fields(object).into_iter().enumerate() {
        if known.contains(&field.id) {
            continue;
        }
        sqlx::query("INSERT INTO view_field (id, view_id, field_id, is_visible, position, size) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(uuid::Uuid::now_v7().to_string())
            .bind(&id)
            .bind(&field.id)
            .bind(fresh)
            .bind(if fresh { i as f64 } else { 1_000.0 + i as f64 })
            .bind(default_size(field.r#type))
            .execute(pool)
            .await?;
    }
    Ok(id)
}

/// A new view of the object, with the INDEX view's columns.
pub async fn create(pool: &SqlitePool, object: &ObjectDef, name: &str, icon: &str, r#type: &str) -> Result<ViewDef> {
    let name = name.trim();
    if name.is_empty() {
        return Err(invalid("A view needs a name"));
    }
    if !matches!(r#type, "TABLE" | "KANBAN" | "CALENDAR") {
        return Err(invalid("A view is a table, a kanban or a calendar"));
    }
    let index = ensure_index(pool, object).await?;
    let id = uuid::Uuid::now_v7().to_string();
    let (next,): (f64,) = sqlx::query_as("SELECT coalesce(max(position), 0.0) + 1.0 FROM view WHERE object_id = ?")
        .bind(&object.meta.id)
        .fetch_one(pool)
        .await?;
    sqlx::query("INSERT INTO view (id, object_id, name, icon, type, position) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(&object.meta.id)
        .bind(name)
        .bind(icon)
        .bind(r#type)
        .bind(next)
        .execute(pool)
        .await?;
    copy_fields(pool, &index, &id).await?;
    get(pool, &id).await
}

async fn copy_fields(pool: &SqlitePool, from: &str, to: &str) -> Result<()> {
    let fields: Vec<ViewField> = sqlx::query_as("SELECT id, view_id, field_id, is_visible, position, size, aggregate_operation FROM view_field WHERE view_id = ?")
        .bind(from)
        .fetch_all(pool)
        .await?;
    for f in fields {
        sqlx::query("INSERT INTO view_field (id, view_id, field_id, is_visible, position, size, aggregate_operation) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(uuid::Uuid::now_v7().to_string())
            .bind(to)
            .bind(&f.field_id)
            .bind(f.is_visible)
            .bind(f.position)
            .bind(f.size)
            .bind(&f.aggregate_operation)
            .execute(pool)
            .await?;
    }
    Ok(())
}

/// A copy of a view — "Copy of <name>" unless named — with its fields, filters, sorts and groups.
pub async fn duplicate(pool: &SqlitePool, id: &str, name: Option<&str>) -> Result<ViewDef> {
    let def = get(pool, id).await?;
    let new_id = uuid::Uuid::now_v7().to_string();
    let name = name.map(str::trim).filter(|n| !n.is_empty()).map(str::to_owned).unwrap_or_else(|| format!("Copy of {}", def.view.name));
    let (next,): (f64,) = sqlx::query_as("SELECT coalesce(max(position), 0.0) + 1.0 FROM view WHERE object_id = ?")
        .bind(&def.view.object_id)
        .fetch_one(pool)
        .await?;
    sqlx::query(
        "INSERT INTO view (id, object_id, name, icon, type, position, is_compact, open_record_in, visibility, group_by_field_id, group_sort, \
         kanban_field_id, kanban_aggregate_operation, kanban_aggregate_field_id, calendar_field_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&new_id)
    .bind(&def.view.object_id)
    .bind(name)
    .bind(&def.view.icon)
    .bind(&def.view.r#type)
    .bind(next)
    .bind(def.view.is_compact)
    .bind(&def.view.open_record_in)
    .bind(&def.view.visibility)
    .bind(&def.view.group_by_field_id)
    .bind(&def.view.group_sort)
    .bind(&def.view.kanban_field_id)
    .bind(&def.view.kanban_aggregate_operation)
    .bind(&def.view.kanban_aggregate_field_id)
    .bind(&def.view.calendar_field_id)
    .execute(pool)
    .await?;
    copy_fields(pool, id, &new_id).await?;
    set_filters(
        pool,
        &new_id,
        FiltersInput {
            groups: def.filter_groups.iter().map(|g| GroupInput { id: g.id.clone(), parent_id: g.parent_id.clone(), logical: g.logical.clone(), position: g.position }).collect(),
            filters: def
                .filters
                .iter()
                .map(|f| FilterInput { field_id: f.field_id.clone(), sub: f.sub.clone(), operand: f.operand.clone(), value: f.value.clone(), group_id: f.group_id.clone(), position: f.position })
                .collect(),
        },
    )
    .await?;
    set_sorts(pool, &new_id, def.sorts.iter().map(|s| SortInput { field_id: s.field_id.clone(), sub: s.sub.clone(), direction: s.direction.clone(), position: s.position }).collect()).await?;
    set_groups(pool, &new_id, def.groups.iter().map(|g| GroupValueInput { field_value: g.field_value.clone(), is_visible: g.is_visible, position: g.position }).collect()).await?;
    get(pool, &new_id).await
}

/// What Edit, Options and the group menu change on a view. A missing field is left alone;
/// `group_by_field_id: Some(None)` clears the grouping.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct ViewPatch {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub is_compact: Option<bool>,
    #[serde(default)]
    pub open_record_in: Option<String>,
    #[serde(default)]
    pub visibility: Option<String>,
    #[serde(default)]
    pub group_by_field_id: Option<Option<String>>,
    #[serde(default)]
    pub group_sort: Option<String>,
    #[serde(default)]
    pub kanban_field_id: Option<Option<String>>,
    #[serde(default)]
    pub kanban_aggregate_operation: Option<Option<String>>,
    #[serde(default)]
    pub kanban_aggregate_field_id: Option<Option<String>>,
    #[serde(default)]
    pub calendar_field_id: Option<Option<String>>,
}

pub async fn update(pool: &SqlitePool, id: &str, patch: ViewPatch) -> Result<ViewDef> {
    let current = row(pool, id).await?;
    if let Some(name) = patch.name.as_deref().map(str::trim) {
        if name.is_empty() {
            return Err(invalid("A view needs a name"));
        }
    }
    if let Some(v) = patch.open_record_in.as_deref() {
        if !matches!(v, "SIDE_PANEL" | "RECORD_PAGE") {
            return Err(invalid("A record opens in the side panel or on its page"));
        }
    }
    if let Some(v) = patch.visibility.as_deref() {
        if !matches!(v, "WORKSPACE" | "UNLISTED") {
            return Err(invalid("A view is for the workspace or unlisted"));
        }
        if current.key.as_deref() == Some(INDEX) {
            return Err(invalid("The default view's visibility can't change"));
        }
    }
    if let Some(v) = patch.group_sort.as_deref() {
        if !matches!(v, "MANUAL" | "ASC" | "DESC") {
            return Err(invalid("Groups sort by hand, A to Z or Z to A"));
        }
    }
    if let Some(Some(op)) = patch.kanban_aggregate_operation.as_ref() {
        if !AGGREGATES.contains(&op.as_str()) {
            return Err(invalid(format!("No aggregate named {op}")));
        }
    }
    if let Some(Some(field_id)) = &patch.group_by_field_id {
        let object = super::object(pool, &current.object_id).await?;
        match object.fields.iter().find(|f| &f.id == field_id) {
            Some(f) if f.r#type == FieldType::Select => {}
            _ => return Err(invalid("A table groups by a select field")),
        }
    }
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE view SET name = coalesce(?1, name), icon = coalesce(?2, icon), is_compact = coalesce(?3, is_compact), \
         open_record_in = coalesce(?4, open_record_in), visibility = coalesce(?5, visibility), \
         group_by_field_id = CASE WHEN ?6 THEN ?7 ELSE group_by_field_id END, group_sort = coalesce(?8, group_sort), \
         kanban_field_id = CASE WHEN ?9 THEN ?10 ELSE kanban_field_id END, \
         kanban_aggregate_operation = CASE WHEN ?11 THEN ?12 ELSE kanban_aggregate_operation END, \
         kanban_aggregate_field_id = CASE WHEN ?13 THEN ?14 ELSE kanban_aggregate_field_id END, \
         calendar_field_id = CASE WHEN ?15 THEN ?16 ELSE calendar_field_id END, \
         updated_at = {NOW} WHERE id = ?17"
    )))
    .bind(patch.name.as_deref().map(str::trim))
    .bind(patch.icon)
    .bind(patch.is_compact)
    .bind(patch.open_record_in)
    .bind(patch.visibility)
    .bind(patch.group_by_field_id.is_some())
    .bind(patch.group_by_field_id.flatten())
    .bind(patch.group_sort)
    .bind(patch.kanban_field_id.is_some())
    .bind(patch.kanban_field_id.flatten())
    .bind(patch.kanban_aggregate_operation.is_some())
    .bind(patch.kanban_aggregate_operation.flatten())
    .bind(patch.kanban_aggregate_field_id.is_some())
    .bind(patch.kanban_aggregate_field_id.flatten())
    .bind(patch.calendar_field_id.is_some())
    .bind(patch.calendar_field_id.flatten())
    .bind(id)
    .execute(pool)
    .await?;
    get(pool, id).await
}

pub async fn delete(pool: &SqlitePool, id: &str) -> Result<()> {
    let current = row(pool, id).await?;
    if current.key.as_deref() == Some(INDEX) {
        return Err(invalid("The default view can't be deleted"));
    }
    sqlx::query("DELETE FROM view WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

/// The views of an object in the order given (the dropdown's drag).
pub async fn reorder(pool: &SqlitePool, object_id: &str, ids: &[String]) -> Result<()> {
    for (i, id) in ids.iter().enumerate() {
        sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE view SET position = ?, updated_at = {NOW} WHERE id = ? AND object_id = ?")))
            .bind(i as f64)
            .bind(id)
            .bind(object_id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ViewFieldInput {
    pub field_id: String,
    pub is_visible: bool,
    pub position: f64,
    pub size: i32,
    #[serde(default)]
    pub aggregate_operation: Option<String>,
}

const AGGREGATES: [&str; 10] = ["COUNT", "COUNT_EMPTY", "COUNT_NOT_EMPTY", "COUNT_UNIQUE", "PERCENT_EMPTY", "PERCENT_NOT_EMPTY", "SUM", "AVG", "MIN", "MAX"];

/// The columns of a view, one row per field: visibility, order, width and footer aggregate.
async fn write_fields(conn: &mut sqlx::SqliteConnection, id: &str, fields: &[ViewFieldInput]) -> Result<()> {
    for f in fields {
        if let Some(op) = f.aggregate_operation.as_deref() {
            if !AGGREGATES.contains(&op) {
                return Err(invalid(format!("No aggregate named {op}")));
            }
        }
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "INSERT INTO view_field (id, view_id, field_id, is_visible, position, size, aggregate_operation) VALUES (?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT (view_id, field_id) DO UPDATE SET is_visible = excluded.is_visible, position = excluded.position, \
             size = excluded.size, aggregate_operation = excluded.aggregate_operation, updated_at = {NOW}"
        )))
        .bind(uuid::Uuid::now_v7().to_string())
        .bind(id)
        .bind(&f.field_id)
        .bind(f.is_visible)
        .bind(f.position)
        .bind(f.size.clamp(40, 1_200))
        .bind(&f.aggregate_operation)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

pub async fn set_fields(pool: &SqlitePool, id: &str, fields: Vec<ViewFieldInput>) -> Result<ViewDef> {
    row(pool, id).await?;
    let mut tx = pool.begin().await?;
    write_fields(&mut tx, id, &fields).await?;
    tx.commit().await?;
    get(pool, id).await
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct GroupInput {
    /// The UI's id for the group, so rules can point at it; replaced by a fresh one on save.
    pub id: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    pub logical: String,
    pub position: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct FilterInput {
    pub field_id: String,
    #[serde(default)]
    pub sub: String,
    pub operand: String,
    #[serde(default)]
    pub value: Json,
    #[serde(default)]
    pub group_id: Option<String>,
    pub position: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct FiltersInput {
    #[serde(default)]
    pub groups: Vec<GroupInput>,
    #[serde(default)]
    pub filters: Vec<FilterInput>,
}

/// The view's filters, whole: the groups (with the ids the UI used, mapped to new ones, parents
/// first so the self-reference holds) and the rules. Replaces what was there.
async fn write_filters(conn: &mut sqlx::SqliteConnection, id: &str, input: &FiltersInput) -> Result<()> {
    sqlx::query("DELETE FROM view_filter WHERE view_id = ?").bind(id).execute(&mut *conn).await?;
    sqlx::query("DELETE FROM view_filter_group WHERE view_id = ?").bind(id).execute(&mut *conn).await?;
    let mut ids: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for g in &input.groups {
        if ids.insert(g.id.clone(), uuid::Uuid::now_v7().to_string()).is_some() {
            return Err(invalid("Two rule groups share an id"));
        }
    }
    let mut pending: Vec<&GroupInput> = input.groups.iter().collect();
    let mut done = std::collections::HashSet::new();
    while !pending.is_empty() {
        let before = pending.len();
        let mut rest = Vec::new();
        for g in pending {
            let parent_ok = g.parent_id.as_ref().is_none_or(|p| done.contains(p));
            if !parent_ok {
                rest.push(g);
                continue;
            }
            if !matches!(g.logical.as_str(), "AND" | "OR" | "NOT") {
                return Err(invalid("A rule group is And, Or or Not"));
            }
            sqlx::query("INSERT INTO view_filter_group (id, view_id, parent_id, logical, position) VALUES (?, ?, ?, ?, ?)")
                .bind(&ids[&g.id])
                .bind(id)
                .bind(g.parent_id.as_ref().and_then(|p| ids.get(p)))
                .bind(&g.logical)
                .bind(g.position)
                .execute(&mut *conn)
                .await?;
            done.insert(g.id.clone());
        }
        if rest.len() == before {
            return Err(invalid("A rule group points at a parent that isn't there, or at itself"));
        }
        pending = rest;
    }
    for f in &input.filters {
        if f.group_id.as_ref().is_some_and(|g| !ids.contains_key(g)) {
            return Err(invalid("A rule points at a group that isn't there"));
        }
        sqlx::query("INSERT INTO view_filter (id, view_id, field_id, sub, operand, value_json, group_id, position) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(uuid::Uuid::now_v7().to_string())
            .bind(id)
            .bind(&f.field_id)
            .bind(&f.sub)
            .bind(&f.operand)
            .bind(f.value.0.to_string())
            .bind(f.group_id.as_ref().and_then(|g| ids.get(g)))
            .bind(f.position)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

pub async fn set_filters(pool: &SqlitePool, id: &str, input: FiltersInput) -> Result<ViewDef> {
    row(pool, id).await?;
    let mut tx = pool.begin().await?;
    write_filters(&mut tx, id, &input).await?;
    tx.commit().await?;
    get(pool, id).await
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SortInput {
    pub field_id: String,
    #[serde(default)]
    pub sub: String,
    pub direction: String,
    pub position: f64,
}

async fn write_sorts(conn: &mut sqlx::SqliteConnection, id: &str, sorts: &[SortInput]) -> Result<()> {
    sqlx::query("DELETE FROM view_sort WHERE view_id = ?").bind(id).execute(&mut *conn).await?;
    for s in sorts {
        if !matches!(s.direction.as_str(), "ASC" | "DESC") {
            return Err(invalid("A sort is ascending or descending"));
        }
        sqlx::query("INSERT INTO view_sort (id, view_id, field_id, sub, direction, position) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(uuid::Uuid::now_v7().to_string())
            .bind(id)
            .bind(&s.field_id)
            .bind(&s.sub)
            .bind(&s.direction)
            .bind(s.position)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

pub async fn set_sorts(pool: &SqlitePool, id: &str, sorts: Vec<SortInput>) -> Result<ViewDef> {
    row(pool, id).await?;
    let mut tx = pool.begin().await?;
    write_sorts(&mut tx, id, &sorts).await?;
    tx.commit().await?;
    get(pool, id).await
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct GroupValueInput {
    pub field_value: String,
    pub is_visible: bool,
    pub position: f64,
}

/// The order and visibility of the groups (a select field's options) of a grouped view.
async fn write_groups(conn: &mut sqlx::SqliteConnection, id: &str, groups: &[GroupValueInput]) -> Result<()> {
    sqlx::query("DELETE FROM view_group WHERE view_id = ?").bind(id).execute(&mut *conn).await?;
    for g in groups {
        sqlx::query("INSERT INTO view_group (id, view_id, field_value, is_visible, position) VALUES (?, ?, ?, ?, ?)")
            .bind(uuid::Uuid::now_v7().to_string())
            .bind(id)
            .bind(&g.field_value)
            .bind(g.is_visible)
            .bind(g.position)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

pub async fn set_groups(pool: &SqlitePool, id: &str, groups: Vec<GroupValueInput>) -> Result<ViewDef> {
    row(pool, id).await?;
    let mut tx = pool.begin().await?;
    write_groups(&mut tx, id, &groups).await?;
    tx.commit().await?;
    get(pool, id).await
}

/// Everything "Update view" writes, in one go.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct SaveInput {
    #[serde(default)]
    pub fields: Vec<ViewFieldInput>,
    #[serde(default)]
    pub filters: FiltersInput,
    #[serde(default)]
    pub sorts: Vec<SortInput>,
    #[serde(default)]
    pub groups: Vec<GroupValueInput>,
    #[serde(default)]
    pub group_by_field_id: Option<String>,
    #[serde(default)]
    pub group_sort: Option<String>,
}

/// The working copy written into the view — fields, filters, sorts, groups and the grouping — in
/// one transaction: a failure changes nothing (D-061).
pub async fn save(pool: &SqlitePool, id: &str, input: SaveInput) -> Result<ViewDef> {
    let current = row(pool, id).await?;
    if let Some(v) = input.group_sort.as_deref() {
        if !matches!(v, "MANUAL" | "ASC" | "DESC") {
            return Err(invalid("Groups sort by hand, A to Z or Z to A"));
        }
    }
    if let Some(field_id) = &input.group_by_field_id {
        let object = super::object(pool, &current.object_id).await?;
        match object.fields.iter().find(|f| &f.id == field_id) {
            Some(f) if f.r#type == FieldType::Select => {}
            _ => return Err(invalid("A table groups by a select field")),
        }
    }
    let mut tx = pool.begin().await?;
    write_fields(&mut tx, id, &input.fields).await?;
    write_filters(&mut tx, id, &input.filters).await?;
    write_sorts(&mut tx, id, &input.sorts).await?;
    write_groups(&mut tx, id, &input.groups).await?;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE view SET group_by_field_id = ?, group_sort = coalesce(?, group_sort), updated_at = {NOW} WHERE id = ?"
    )))
    .bind(&input.group_by_field_id)
    .bind(&input.group_sort)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    get(pool, id).await
}

/// A rule without the value its operator needs (just added, not yet typed) is left out.
fn needs_value(op: &str) -> bool {
    !matches!(op, "isEmpty" | "isNotEmpty" | "isToday" | "isInPast" | "isInFuture")
}

fn is_blank(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::String(s) => s.trim().is_empty(),
        Value::Array(items) => items.is_empty(),
        _ => false,
    }
}

/// The engine's query for a view: its rules as the filter JSON (top-level rules and groups under
/// one AND; a group's rules under its own AND / OR / NOT; unfinished rules and empty groups left
/// out), its sorts in order. A relation rule's value is the chosen records' ids; a currency's
/// amount is in units and becomes micros — as the view bar sends them.
pub fn query_of(object: &ObjectDef, def: &ViewDef) -> Result<Query> {
    let field_of = |field_id: &str| object.fields.iter().find(|f| f.id == field_id);
    let rule = |f: &ViewFilter| -> Option<Value> {
        let field = field_of(&f.field_id)?;
        if needs_value(&f.operand) && is_blank(&f.value.0) {
            return None;
        }
        let value = match (&f.value.0, field.r#type) {
            (Value::Array(items), FieldType::Relation) => Value::Array(items.iter().map(|v| v.get("id").cloned().unwrap_or_else(|| v.clone())).collect()),
            (Value::Number(n), FieldType::Currency) if f.sub == "amountMicros" => Value::from((n.as_f64().unwrap_or(0.0) * 1_000_000.0).round() as i64),
            (v, _) => v.clone(),
        };
        Some(json!({ "field": field.name, "sub": if f.sub.is_empty() { Value::Null } else { Value::String(f.sub.clone()) }, "op": f.operand, "value": value }))
    };
    fn group(def: &ViewDef, id: Option<&str>, rule: &dyn Fn(&ViewFilter) -> Option<Value>) -> Vec<Value> {
        let mut items: Vec<Value> = def.filters.iter().filter(|f| f.group_id.as_deref() == id).filter_map(rule).collect();
        for g in def.filter_groups.iter().filter(|g| g.parent_id.as_deref() == id) {
            let inner = group(def, Some(&g.id), rule);
            if inner.is_empty() {
                continue;
            }
            items.push(match g.logical.as_str() {
                "OR" => json!({ "or": inner }),
                "NOT" => json!({ "not": { "and": inner } }),
                _ => json!({ "and": inner }),
            });
        }
        items
    }
    let items = group(def, None, &rule);
    let sort = def
        .sorts
        .iter()
        .filter_map(|s| Some(Sort { field: field_of(&s.field_id)?.name.clone(), sub: if s.sub.is_empty() { None } else { Some(s.sub.clone()) }, desc: s.direction == "DESC" }))
        .collect();
    Ok(Query { filter: if items.is_empty() { None } else { Some(Json(json!({ "and": items }))) }, sort, ..Query::default() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::hub::WorkspaceHub;

    /// A view's rules, nested, survive a save, a duplicate and the trip into the engine's query.
    #[test]
    fn rules_round_trip_through_save_duplicate_and_query() {
        let root = std::env::temp_dir().join(format!("newdawn-views-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let people = super::super::object(&pool, "person").await.unwrap();
            let city = people.field("city").unwrap().id.clone();
            let company = people.field("company").unwrap().id.clone();
            let made = create(&pool, &people, "Hanoi or Keppel", "", "TABLE").await.unwrap();
            let saved = save(
                &pool,
                &made.view.id,
                SaveInput {
                    fields: Vec::new(),
                    filters: FiltersInput {
                        groups: vec![GroupInput { id: "g1".into(), parent_id: None, logical: "OR".into(), position: 0.0 }],
                        filters: vec![
                            FilterInput { field_id: city.clone(), sub: String::new(), operand: "contains".into(), value: Json(json!("Hanoi")), group_id: Some("g1".into()), position: 0.0 },
                            FilterInput { field_id: company.clone(), sub: String::new(), operand: "is".into(), value: Json(json!([{ "id": "c1", "label": "Keppel Land" }])), group_id: Some("g1".into()), position: 1.0 },
                            // Added but never given a value: left out of the query.
                            FilterInput { field_id: city.clone(), sub: String::new(), operand: "contains".into(), value: Json(Value::Null), group_id: None, position: 2.0 },
                        ],
                    },
                    sorts: vec![SortInput { field_id: people.field("name").unwrap().id.clone(), sub: "lastName".into(), direction: "DESC".into(), position: 0.0 }],
                    groups: Vec::new(),
                    group_by_field_id: None,
                    group_sort: None,
                },
            )
            .await
            .unwrap();
            assert_eq!(saved.filter_groups.len(), 1);
            assert_eq!(saved.filters.iter().filter(|f| f.group_id.is_some()).count(), 2);
            let copy = duplicate(&pool, &saved.view.id, None).await.unwrap();
            assert_eq!(copy.view.name, "Copy of Hanoi or Keppel");
            let query = query_of(&people, &copy).unwrap();
            let filter = query.filter.clone().unwrap().0;
            assert_eq!(filter["and"].as_array().unwrap().len(), 1, "the unfinished rule is left out");
            let or = &filter["and"][0]["or"];
            assert_eq!(or[0]["field"], "city");
            assert_eq!(or[1]["field"], "company");
            assert_eq!(or[1]["value"], json!(["c1"]), "a relation rule carries ids");
            assert_eq!(query.sort[0].field, "name");
            assert_eq!(query.sort[0].sub.as_deref(), Some("lastName"));
            assert!(query.sort[0].desc);
            // The engine accepts the shape.
            let page = super::super::query::list(&pool, &people, &query).await.unwrap();
            assert_eq!(page.total, 0);
            hub.close_all().await;
        });
        let _ = std::fs::remove_dir_all(root);
    }
}
