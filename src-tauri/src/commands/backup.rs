use tauri::State;

use crate::error::AppError;
use crate::kernel::backup::{self, BackupStatus};
use crate::kernel::hub::WorkspaceHub;

#[tauri::command]
#[specta::specta]
pub async fn get_backup_status(hub: State<'_, WorkspaceHub>) -> Result<BackupStatus, AppError> {
    Ok(backup::status(&hub).await?)
}

/// `None` turns daily backups off.
#[tauri::command]
#[specta::specta]
pub async fn set_backup_folder(
    hub: State<'_, WorkspaceHub>,
    folder: Option<String>,
) -> Result<BackupStatus, AppError> {
    Ok(backup::set_folder(&hub, folder.as_deref()).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn backup_now(hub: State<'_, WorkspaceHub>) -> Result<BackupStatus, AppError> {
    Ok(backup::run(&hub).await?)
}
