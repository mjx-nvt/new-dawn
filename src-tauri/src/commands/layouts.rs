//! The record page (D-062): its layout, timeline, neighbours, and the tasks, notes and files
//! that hang off a record.

use tauri::State;

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::kernel::records::layouts::{self, TimelineEvent, Layout, LayoutInput, Neighbours};
use crate::kernel::records::linked::{self, Attachment};
use crate::kernel::records::types::Json;
use crate::kernel::records;

/// The object's record page layout — tabs of widgets — made from Twenty's default when missing.
#[tauri::command]
#[specta::specta]
pub async fn get_record_layout(hub: State<'_, WorkspaceHub>, slug: String, object: String) -> Result<Layout, AppError> {
    let pool = hub.pool(&slug)?;
    let object = records::object(&pool, &object).await?;
    Ok(layouts::record_layout(&pool, &object).await?)
}

/// The layout editor's Save: every tab and widget, in one transaction.
#[tauri::command]
#[specta::specta]
pub async fn save_record_layout(hub: State<'_, WorkspaceHub>, slug: String, object: String, input: LayoutInput) -> Result<Layout, AppError> {
    let pool = hub.pool(&slug)?;
    let object = records::object(&pool, &object).await?;
    Ok(layouts::save(&pool, &object, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn reset_record_layout(hub: State<'_, WorkspaceHub>, slug: String, object: String) -> Result<Layout, AppError> {
    let pool = hub.pool(&slug)?;
    let object = records::object(&pool, &object).await?;
    Ok(layouts::reset(&pool, &object).await?)
}

/// A record's timeline, newest first, with the field labels on each change.
#[tauri::command]
#[specta::specta]
pub async fn list_timeline(hub: State<'_, WorkspaceHub>, slug: String, object: String, id: String, limit: u32) -> Result<Vec<TimelineEvent>, AppError> {
    let pool = hub.pool(&slug)?;
    let object = records::object(&pool, &object).await?;
    Ok(layouts::timeline(&pool, &object, &id, limit).await?)
}

/// The records before and after this one, for ‹ › on the page.
#[tauri::command]
#[specta::specta]
pub async fn record_neighbours(hub: State<'_, WorkspaceHub>, slug: String, object: String, id: String) -> Result<Neighbours, AppError> {
    let pool = hub.pool(&slug)?;
    let object = records::object(&pool, &object).await?;
    Ok(layouts::neighbours(&pool, &object, &id).await?)
}

/// The tasks or notes (`kind`) that target a record, newest first.
#[tauri::command]
#[specta::specta]
pub async fn list_linked(hub: State<'_, WorkspaceHub>, slug: String, object: String, id: String, kind: String) -> Result<Vec<Json>, AppError> {
    let pool = hub.pool(&slug)?;
    let object = records::object(&pool, &object).await?;
    Ok(linked::list(&pool, &object, &id, &kind).await?.into_iter().map(Json).collect())
}

/// A new task or note with this title, targeting the record.
#[tauri::command]
#[specta::specta]
pub async fn add_linked(hub: State<'_, WorkspaceHub>, slug: String, object: String, id: String, kind: String, title: String) -> Result<Json, AppError> {
    let actor = records::actor_of(&hub, &slug).await?;
    let pool = hub.pool(&slug)?;
    let object = records::object(&pool, &object).await?;
    Ok(Json(linked::add(&pool, &object, &id, &kind, &title, &actor).await?))
}

#[tauri::command]
#[specta::specta]
pub async fn list_attachments(hub: State<'_, WorkspaceHub>, slug: String, object: String, id: String) -> Result<Vec<Attachment>, AppError> {
    let pool = hub.pool(&slug)?;
    let object = records::object(&pool, &object).await?;
    Ok(linked::attachments(&pool, &object, &id).await?)
}
