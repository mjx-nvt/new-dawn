//! Plane's projects and their settings (D-073): each command reads its workspace's pool from the
//! hub and calls `modules::pm`.

use tauri::State;

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::modules::pm::settings::{self, EstimateInput, LabelInput, PmEstimate, PmLabel, PmState, StateInput, StatePatch};
use crate::modules::pm::{self, NewPmProject, PmProject, PmProjectPatch};

#[tauri::command]
#[specta::specta]
pub async fn list_pm_projects(hub: State<'_, WorkspaceHub>, slug: String) -> Result<Vec<PmProject>, AppError> {
    Ok(pm::list(&hub.pool(&slug)?).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn get_pm_project(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<PmProject, AppError> {
    Ok(pm::get(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn create_pm_project(hub: State<'_, WorkspaceHub>, slug: String, project: NewPmProject) -> Result<PmProject, AppError> {
    Ok(pm::create(&hub.pool(&slug)?, project).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn update_pm_project(hub: State<'_, WorkspaceHub>, slug: String, id: String, patch: PmProjectPatch) -> Result<PmProject, AppError> {
    Ok(pm::update(&hub.pool(&slug)?, &id, patch).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn archive_pm_project(hub: State<'_, WorkspaceHub>, slug: String, id: String, archived: bool) -> Result<PmProject, AppError> {
    Ok(pm::archive(&hub.pool(&slug)?, &id, archived).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_pm_states(hub: State<'_, WorkspaceHub>, slug: String, project_id: String) -> Result<Vec<PmState>, AppError> {
    Ok(settings::states(&hub.pool(&slug)?, &project_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn create_pm_state(hub: State<'_, WorkspaceHub>, slug: String, project_id: String, input: StateInput) -> Result<Vec<PmState>, AppError> {
    Ok(settings::create_state(&hub.pool(&slug)?, &project_id, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn update_pm_state(hub: State<'_, WorkspaceHub>, slug: String, id: String, patch: StatePatch) -> Result<Vec<PmState>, AppError> {
    Ok(settings::update_state(&hub.pool(&slug)?, &id, patch).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn set_default_pm_state(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<Vec<PmState>, AppError> {
    Ok(settings::set_default_state(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_pm_state(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<Vec<PmState>, AppError> {
    Ok(settings::delete_state(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn move_pm_state(hub: State<'_, WorkspaceHub>, slug: String, id: String, up: bool) -> Result<Vec<PmState>, AppError> {
    Ok(settings::move_state(&hub.pool(&slug)?, &id, up).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_pm_labels(hub: State<'_, WorkspaceHub>, slug: String, project_id: String) -> Result<Vec<PmLabel>, AppError> {
    Ok(settings::labels(&hub.pool(&slug)?, &project_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn create_pm_label(hub: State<'_, WorkspaceHub>, slug: String, project_id: String, input: LabelInput) -> Result<Vec<PmLabel>, AppError> {
    Ok(settings::create_label(&hub.pool(&slug)?, &project_id, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn update_pm_label(hub: State<'_, WorkspaceHub>, slug: String, id: String, input: LabelInput) -> Result<Vec<PmLabel>, AppError> {
    Ok(settings::update_label(&hub.pool(&slug)?, &id, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_pm_label(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<Vec<PmLabel>, AppError> {
    Ok(settings::delete_label(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_pm_estimates(hub: State<'_, WorkspaceHub>, slug: String, project_id: String) -> Result<Vec<PmEstimate>, AppError> {
    Ok(settings::estimates(&hub.pool(&slug)?, &project_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn create_pm_estimate(hub: State<'_, WorkspaceHub>, slug: String, project_id: String, input: EstimateInput) -> Result<Vec<PmEstimate>, AppError> {
    Ok(settings::create_estimate(&hub.pool(&slug)?, &project_id, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn update_pm_estimate(hub: State<'_, WorkspaceHub>, slug: String, id: String, input: EstimateInput) -> Result<Vec<PmEstimate>, AppError> {
    Ok(settings::update_estimate(&hub.pool(&slug)?, &id, input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn disable_pm_estimates(hub: State<'_, WorkspaceHub>, slug: String, project_id: String) -> Result<Vec<PmEstimate>, AppError> {
    Ok(settings::disable_estimates(&hub.pool(&slug)?, &project_id).await?)
}
