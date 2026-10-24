use tauri::AppHandle;

use crate::error::AppError;
use crate::kernel::windows::{self, SettingsTab};

#[tauri::command]
#[specta::specta]
pub async fn open_settings(app: AppHandle, tab: SettingsTab) -> Result<(), AppError> {
    Ok(windows::open_settings(&app, tab)?)
}
