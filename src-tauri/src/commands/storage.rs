use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::kernel::storage::{self, StorageReport};

#[tauri::command]
#[specta::specta]
pub async fn get_storage_report(app: AppHandle) -> Result<StorageReport, AppError> {
    Ok(storage::report(&app).await?)
}

/// 0 keeps recordings forever.
#[tauri::command]
#[specta::specta]
pub async fn set_retention_days(hub: State<'_, WorkspaceHub>, days: u32) -> Result<(), AppError> {
    Ok(storage::set_retention_days(&hub, days).await?)
}

/// Compresses every original recording in the workspace; returns how many.
#[tauri::command]
#[specta::specta]
pub async fn compress_recordings(
    hub: State<'_, WorkspaceHub>,
    slug: String,
) -> Result<u32, AppError> {
    Ok(storage::compress_all(&hub, &slug).await?)
}

/// Trashes recordings of summarised meetings older than `days`; returns how many.
#[tauri::command]
#[specta::specta]
pub async fn trash_old_recordings(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    days: u32,
) -> Result<u32, AppError> {
    if days == 0 {
        return Err(AppError::invalid("Choose a number of days"));
    }
    Ok(storage::trash_old_recordings(&hub, &slug, days).await?)
}
