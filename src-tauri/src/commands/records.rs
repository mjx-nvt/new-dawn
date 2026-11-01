use tauri::State;

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::kernel::profile;
use crate::kernel::records::types::Json;
use crate::kernel::records::{self, query, Hit, ObjectDef, ObjectMeta, Page, Query};

/// The owner, writing by hand (Twenty's actor on the record and its activities).
async fn actor(hub: &WorkspaceHub, slug: &str) -> Result<records::Actor, AppError> {
    let name = profile::owner(hub, slug).await?.map(|o| o.name).unwrap_or_default();
    Ok(records::owner_actor(&hub.pool(slug)?, &name).await?)
}

/// Every object of the workspace's CRM, system ones included (D-060).
#[tauri::command]
#[specta::specta]
pub async fn list_objects(hub: State<'_, WorkspaceHub>, slug: String) -> Result<Vec<ObjectMeta>, AppError> {
    Ok(records::objects(&hub.pool(&slug)?).await?)
}

/// An object with its fields, by singular name, plural name or id.
#[tauri::command]
#[specta::specta]
pub async fn get_object(hub: State<'_, WorkspaceHub>, slug: String, object: String) -> Result<ObjectDef, AppError> {
    Ok(records::object(&hub.pool(&slug)?, &object).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_records(hub: State<'_, WorkspaceHub>, slug: String, object: String, query: Query) -> Result<Page, AppError> {
    let pool = hub.pool(&slug)?;
    let object = records::object(&pool, &object).await?;
    Ok(query::list(&pool, &object, &query).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn get_record(hub: State<'_, WorkspaceHub>, slug: String, object: String, id: String) -> Result<Json, AppError> {
    let pool = hub.pool(&slug)?;
    let object = records::object(&pool, &object).await?;
    Ok(Json(query::get(&pool, &object, &id, true).await?))
}

#[tauri::command]
#[specta::specta]
pub async fn create_record(hub: State<'_, WorkspaceHub>, slug: String, object: String, data: Json) -> Result<Json, AppError> {
    let actor = actor(&hub, &slug).await?;
    let pool = hub.pool(&slug)?;
    let object = records::object(&pool, &object).await?;
    Ok(Json(query::create(&pool, &object, &data.0, &actor).await?))
}

#[tauri::command]
#[specta::specta]
pub async fn update_record(hub: State<'_, WorkspaceHub>, slug: String, object: String, id: String, data: Json) -> Result<Json, AppError> {
    let actor = actor(&hub, &slug).await?;
    let pool = hub.pool(&slug)?;
    let object = records::object(&pool, &object).await?;
    Ok(Json(query::update(&pool, &object, &id, &data.0, &actor).await?))
}

/// A soft delete: the record waits under Deleted records.
#[tauri::command]
#[specta::specta]
pub async fn delete_record(hub: State<'_, WorkspaceHub>, slug: String, object: String, id: String) -> Result<(), AppError> {
    let actor = actor(&hub, &slug).await?;
    let pool = hub.pool(&slug)?;
    let object = records::object(&pool, &object).await?;
    Ok(query::delete(&pool, &object, &id, &actor).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn restore_record(hub: State<'_, WorkspaceHub>, slug: String, object: String, id: String) -> Result<Json, AppError> {
    let actor = actor(&hub, &slug).await?;
    let pool = hub.pool(&slug)?;
    let object = records::object(&pool, &object).await?;
    Ok(Json(query::restore(&pool, &object, &id, &actor).await?))
}

/// Deletes for good.
#[tauri::command]
#[specta::specta]
pub async fn destroy_record(hub: State<'_, WorkspaceHub>, slug: String, object: String, id: String) -> Result<(), AppError> {
    let pool = hub.pool(&slug)?;
    let object = records::object(&pool, &object).await?;
    Ok(query::destroy(&pool, &object, &id).await?)
}

/// Records of every object whose words match, best first (`/`).
#[tauri::command]
#[specta::specta]
pub async fn search_records(hub: State<'_, WorkspaceHub>, slug: String, text: String, limit: u32) -> Result<Vec<Hit>, AppError> {
    Ok(records::search::find(&hub.pool(&slug)?, &text, limit.clamp(1, 200)).await?)
}
