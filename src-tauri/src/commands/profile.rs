use tauri::{AppHandle, State};
use tauri_specta::Event;

use crate::error::AppError;
use crate::events::{FormatsChanged, notify_workspaces_changed};
use crate::kernel::formats::{self, Formats};
use crate::kernel::hub::WorkspaceHub;
use crate::kernel::profile::{self, Profile, ProfilePatch};

/// The owner's formats: time zone, date, time, number, week start (D-072).
#[tauri::command]
#[specta::specta]
pub async fn get_formats(hub: State<'_, WorkspaceHub>) -> Result<Formats, AppError> {
    Ok(formats::get(&hub).await?)
}

/// Saves the formats; every window redraws its dates and numbers (`FormatsChanged`).
#[tauri::command]
#[specta::specta]
pub async fn set_formats(app: AppHandle, hub: State<'_, WorkspaceHub>, formats: Formats) -> Result<Formats, AppError> {
    let saved = formats::set(&hub, formats).await?;
    let _ = FormatsChanged { formats: saved.clone() }.emit(&app);
    Ok(saved)
}

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
