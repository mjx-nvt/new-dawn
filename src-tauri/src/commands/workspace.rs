use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::events::notify_workspaces_changed;
use crate::kernel::hub::WorkspaceHub;
use crate::kernel::registry::{Workspace, WorkspacePatch};

#[tauri::command]
#[specta::specta]
pub async fn list_workspaces(hub: State<'_, WorkspaceHub>) -> Result<Vec<Workspace>, AppError> {
    Ok(hub.list().await?)
}

#[tauri::command]
#[specta::specta]
pub async fn get_current_workspace(hub: State<'_, WorkspaceHub>) -> Result<Option<Workspace>, AppError> {
    Ok(hub.current_workspace().await?)
}

#[tauri::command]
#[specta::specta]
pub async fn set_current_workspace(
    app: AppHandle,
    hub: State<'_, WorkspaceHub>,
    slug: String,
) -> Result<Workspace, AppError> {
    let workspace = hub.set_current_workspace(&slug).await?;
    notify_workspaces_changed(&app);
    Ok(workspace)
}

#[tauri::command]
#[specta::specta]
pub async fn create_workspace(
    app: AppHandle,
    hub: State<'_, WorkspaceHub>,
    name: String,
    slug: String,
) -> Result<Workspace, AppError> {
    let workspace = hub.create_workspace(&name, &slug).await?;
    notify_workspaces_changed(&app);
    Ok(workspace)
}

#[tauri::command]
#[specta::specta]
pub async fn update_workspace(
    app: AppHandle,
    hub: State<'_, WorkspaceHub>,
    slug: String,
    patch: WorkspacePatch,
) -> Result<Workspace, AppError> {
    let workspace = hub.update_workspace(&slug, patch).await?;
    notify_workspaces_changed(&app);
    Ok(workspace)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_workspace(
    app: AppHandle,
    hub: State<'_, WorkspaceHub>,
    slug: String,
) -> Result<(), AppError> {
    hub.delete_workspace(&slug).await?;
    notify_workspaces_changed(&app);
    Ok(())
}
