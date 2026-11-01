use tauri::State;

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::kernel::records::query::{self, AggregateInput, AggregateValue, GroupCount, Query};
use crate::kernel::records::views::{self, FiltersInput, GroupValueInput, SaveInput, SortInput, ViewDef, ViewFieldInput, ViewPatch, ViewRow};
use crate::kernel::records::{self};

/// An object's views, the default "All <X>" first (made if missing).
#[tauri::command]
#[specta::specta]
pub async fn list_views(hub: State<'_, WorkspaceHub>, slug: String, object: String) -> Result<Vec<ViewRow>, AppError> {
    let pool = hub.pool(&slug)?;
    let object = records::object(&pool, &object).await?;
    views::ensure_index(&pool, &object).await?;
    Ok(views::list(&pool, &object.meta.id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn get_view(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<ViewDef, AppError> {
    Ok(views::get(&hub.pool(&slug)?, &id).await?)
}

/// The engine's query for a saved view (its filters and sorts), for kanban and calendar pages.
#[tauri::command]
#[specta::specta]
pub async fn view_query(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<Query, AppError> {
    let pool = hub.pool(&slug)?;
    let def = views::get(&pool, &id).await?;
    let object = records::object(&pool, &def.view.object_id).await?;
    Ok(views::query_of(&object, &def)?)
}

#[tauri::command]
#[specta::specta]
pub async fn create_view(hub: State<'_, WorkspaceHub>, slug: String, object: String, name: String, icon: String, view_type: String) -> Result<ViewDef, AppError> {
    let pool = hub.pool(&slug)?;
    let object = records::object(&pool, &object).await?;
    Ok(views::create(&pool, &object, &name, &icon, &view_type).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn update_view(hub: State<'_, WorkspaceHub>, slug: String, id: String, patch: ViewPatch) -> Result<ViewDef, AppError> {
    Ok(views::update(&hub.pool(&slug)?, &id, patch).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_view(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<(), AppError> {
    Ok(views::delete(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn duplicate_view(hub: State<'_, WorkspaceHub>, slug: String, id: String, name: Option<String>) -> Result<ViewDef, AppError> {
    Ok(views::duplicate(&hub.pool(&slug)?, &id, name.as_deref()).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn reorder_views(hub: State<'_, WorkspaceHub>, slug: String, object: String, ids: Vec<String>) -> Result<(), AppError> {
    let pool = hub.pool(&slug)?;
    let object = records::object(&pool, &object).await?;
    Ok(views::reorder(&pool, &object.meta.id, &ids).await?)
}

/// Everything "Update view" writes — fields, filters, sorts, groups, grouping — in one transaction.
#[tauri::command]
#[specta::specta]
pub async fn save_view(hub: State<'_, WorkspaceHub>, slug: String, id: String, input: SaveInput) -> Result<ViewDef, AppError> {
    Ok(views::save(&hub.pool(&slug)?, &id, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn set_view_fields(hub: State<'_, WorkspaceHub>, slug: String, id: String, fields: Vec<ViewFieldInput>) -> Result<ViewDef, AppError> {
    Ok(views::set_fields(&hub.pool(&slug)?, &id, fields).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn set_view_filters(hub: State<'_, WorkspaceHub>, slug: String, id: String, input: FiltersInput) -> Result<ViewDef, AppError> {
    Ok(views::set_filters(&hub.pool(&slug)?, &id, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn set_view_sorts(hub: State<'_, WorkspaceHub>, slug: String, id: String, sorts: Vec<SortInput>) -> Result<ViewDef, AppError> {
    Ok(views::set_sorts(&hub.pool(&slug)?, &id, sorts).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn set_view_groups(hub: State<'_, WorkspaceHub>, slug: String, id: String, groups: Vec<GroupValueInput>) -> Result<ViewDef, AppError> {
    Ok(views::set_groups(&hub.pool(&slug)?, &id, groups).await?)
}

/// Aggregates over the records a query lists (a view's footer, a group's header).
#[tauri::command]
#[specta::specta]
pub async fn aggregate_records(hub: State<'_, WorkspaceHub>, slug: String, object: String, query: Query, ops: Vec<AggregateInput>) -> Result<Vec<AggregateValue>, AppError> {
    let pool = hub.pool(&slug)?;
    let object = records::object(&pool, &object).await?;
    Ok(query::aggregate(&pool, &object, &query, &ops).await?)
}

/// How many records a query lists per value of a select field (a grouped table's headers).
#[tauri::command]
#[specta::specta]
pub async fn group_counts(hub: State<'_, WorkspaceHub>, slug: String, object: String, query: Query, field: String) -> Result<Vec<GroupCount>, AppError> {
    let pool = hub.pool(&slug)?;
    let object = records::object(&pool, &object).await?;
    Ok(query::group_counts(&pool, &object, &query, &field).await?)
}
