use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::modules::projects::{self, NewProject, Project, ProjectDossier, ProjectPatch};

/// The workspace's projects: active first, then on hold, then done.
#[tauri::command]
#[specta::specta]
pub async fn list_projects(
    hub: State<'_, WorkspaceHub>,
    slug: String,
) -> Result<Vec<Project>, AppError> {
    Ok(projects::list(&hub.pool(&slug)?).await?)
}

/// A project's page: the project, the updates waiting, its history and meetings.
#[tauri::command]
#[specta::specta]
pub async fn get_project(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    id: String,
) -> Result<ProjectDossier, AppError> {
    Ok(projects::dossier(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn create_project(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    project: NewProject,
) -> Result<Project, AppError> {
    Ok(projects::create(&hub.pool(&slug)?, project).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn update_project(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    id: String,
    patch: ProjectPatch,
) -> Result<Project, AppError> {
    Ok(projects::update(&hub.pool(&slug)?, &id, patch).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_project(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    id: String,
) -> Result<(), AppError> {
    Ok(projects::delete(&hub.pool(&slug)?, &id).await?)
}

/// Apply: the proposed change goes into the project.
#[tauri::command]
#[specta::specta]
pub async fn apply_project_update(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    id: String,
) -> Result<Project, AppError> {
    Ok(projects::apply(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn dismiss_project_update(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    id: String,
) -> Result<Project, AppError> {
    Ok(projects::dismiss(&hub.pool(&slug)?, &id).await?)
}

/// "Find project updates": reads a meeting's summary against the projects now. Returns how many
/// updates it proposed.
#[tauri::command]
#[specta::specta]
pub async fn find_project_updates(
    app: AppHandle,
    slug: String,
    meeting_id: String,
) -> Result<u32, AppError> {
    Ok(projects::propose(&app, &slug, &meeting_id, true).await?)
}
