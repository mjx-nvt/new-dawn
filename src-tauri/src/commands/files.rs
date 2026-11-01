//! A record's files (0.10.6): add, save as, delete — the workspace's `files/` folder through the hub.

use std::path::Path;

use tauri::State;

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::kernel::records;
use crate::kernel::records::files;
use crate::kernel::records::types::Json;

/// Copies a file into the workspace and attaches it to the record.
#[tauri::command]
#[specta::specta]
pub async fn add_attachment(hub: State<'_, WorkspaceHub>, slug: String, object: String, id: String, path: String) -> Result<Json, AppError> {
    let actor = records::actor_of(&hub, &slug).await?;
    let folder = hub.workspace(&slug).await?.folder_path;
    let pool = hub.pool(&slug)?;
    let object = records::object(&pool, &object).await?;
    Ok(Json(files::add(&pool, &folder, &object, &id, Path::new(&path), &actor).await?))
}

/// A picture pasted or dropped into the editor, as bytes.
#[tauri::command]
#[specta::specta]
pub async fn add_attachment_bytes(hub: State<'_, WorkspaceHub>, slug: String, object: String, id: String, name: String, bytes: Vec<u8>) -> Result<Json, AppError> {
    let actor = records::actor_of(&hub, &slug).await?;
    let folder = hub.workspace(&slug).await?.folder_path;
    let pool = hub.pool(&slug)?;
    let object = records::object(&pool, &object).await?;
    Ok(Json(files::add_bytes(&pool, &folder, &object, &id, &name, &bytes, &actor).await?))
}

/// Copies an attachment's file to where the owner chose.
#[tauri::command]
#[specta::specta]
pub async fn save_attachment(hub: State<'_, WorkspaceHub>, slug: String, id: String, to: String) -> Result<(), AppError> {
    let folder = hub.workspace(&slug).await?.folder_path;
    Ok(files::save_as(&hub.pool(&slug)?, &folder, &id, Path::new(&to)).await?)
}

/// The attachment's file on disk (for Open and Reveal in Finder).
#[tauri::command]
#[specta::specta]
pub async fn attachment_path(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<String, AppError> {
    let folder = hub.workspace(&slug).await?.folder_path;
    Ok(files::path_of(&hub.pool(&slug)?, &folder, &id).await?.to_string_lossy().into_owned())
}

#[tauri::command]
#[specta::specta]
pub async fn delete_attachment(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<(), AppError> {
    let actor = records::actor_of(&hub, &slug).await?;
    Ok(files::remove(&hub.pool(&slug)?, &id, &actor).await?)
}
