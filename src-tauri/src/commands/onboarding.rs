use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;

#[tauri::command]
#[specta::specta]
pub async fn get_onboarding_done(hub: State<'_, WorkspaceHub>) -> Result<bool, AppError> {
    Ok(hub.onboarding_done().await?)
}

#[tauri::command]
#[specta::specta]
pub async fn set_onboarding_done(hub: State<'_, WorkspaceHub>, done: bool) -> Result<(), AppError> {
    Ok(hub.set_onboarding_done(done).await?)
}

/// Shows a sample notification; the first one makes macOS ask whether New Dawn may notify.
#[tauri::command]
#[specta::specta]
pub fn test_notification(app: AppHandle) -> Result<(), AppError> {
    crate::kernel::notify::show(
        &app,
        "New Dawn",
        "Notifications are on: you'll hear when a transcript or summary is ready.",
    );
    Ok(())
}
