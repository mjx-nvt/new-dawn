use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::events::notify_workspaces_changed;
use crate::kernel::hub::WorkspaceHub;
use crate::kernel::profile::{self, Profile, ProfilePatch};

#[tauri::command]
#[specta::specta]
pub async fn get_profile(hub: State<'_, WorkspaceHub>) -> Result<Profile, AppError> {
    Ok(profile::get(&hub).await?)
}

/// Saves the profile; the main window refreshes through `WorkspacesChanged` (the owner shows
/// in every workspace).
#[tauri::command]
#[specta::specta]
pub async fn set_profile(
    app: AppHandle,
    hub: State<'_, WorkspaceHub>,
    patch: ProfilePatch,
) -> Result<Profile, AppError> {
    let saved = profile::set(&hub, patch).await?;
    notify_workspaces_changed(&app);
    Ok(saved)
}
