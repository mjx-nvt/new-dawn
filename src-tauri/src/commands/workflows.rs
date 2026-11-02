//! Workflows (D-068, 0.10.11): versions and the builder's draft, activation, runs by hand, the
//! runs' pages, forms, retries.

use tauri::State;

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::kernel::records::types::Json;
use crate::modules::workflows::run::{self, PendingCode, RunDetail};
use crate::modules::workflows::{self, ManualWorkflow, Version};

#[tauri::command]
#[specta::specta]
pub async fn list_workflow_versions(hub: State<'_, WorkspaceHub>, slug: String, workflow_id: String) -> Result<Vec<Version>, AppError> {
    Ok(workflows::versions(&hub.pool(&slug)?, &workflow_id).await?)
}

/// The version the builder shows: the draft, else the live one (reading makes no version).
#[tauri::command]
#[specta::specta]
pub async fn workflow_draft(hub: State<'_, WorkspaceHub>, slug: String, workflow_id: String) -> Result<Version, AppError> {
    Ok(workflows::editable(&hub.pool(&slug)?, &workflow_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn save_workflow_draft(hub: State<'_, WorkspaceHub>, slug: String, workflow_id: String, definition: Json) -> Result<Version, AppError> {
    Ok(workflows::save_draft(&hub.pool(&slug)?, &workflow_id, &definition.0).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn activate_workflow_version(hub: State<'_, WorkspaceHub>, slug: String, version_id: String) -> Result<Version, AppError> {
    Ok(workflows::activate(&hub.pool(&slug)?, &version_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn deactivate_workflow_version(hub: State<'_, WorkspaceHub>, slug: String, version_id: String) -> Result<Version, AppError> {
    Ok(workflows::deactivate(&hub.pool(&slug)?, &version_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn discard_workflow_draft(hub: State<'_, WorkspaceHub>, slug: String, version_id: String) -> Result<(), AppError> {
    Ok(workflows::discard_draft(&hub.pool(&slug)?, &version_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn use_workflow_version_as_draft(hub: State<'_, WorkspaceHub>, slug: String, version_id: String) -> Result<Version, AppError> {
    Ok(workflows::use_as_draft(&hub.pool(&slug)?, &version_id).await?)
}

/// By hand: the active version on these records (the draft when `test`). The run ids.
#[tauri::command]
#[specta::specta]
pub async fn run_workflow(hub: State<'_, WorkspaceHub>, slug: String, workflow_id: String, record_ids: Vec<String>, test: bool) -> Result<Vec<String>, AppError> {
    Ok(workflows::start_manual(&hub.pool(&slug)?, &workflow_id, &record_ids, test).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn workflow_run_detail(hub: State<'_, WorkspaceHub>, slug: String, run_id: String) -> Result<RunDetail, AppError> {
    Ok(run::detail(&hub.pool(&slug)?, &run_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn answer_workflow_form(hub: State<'_, WorkspaceHub>, slug: String, run_id: String, step_id: String, answers: Json) -> Result<RunDetail, AppError> {
    Ok(run::answer_form(&hub.pool(&slug)?, &run_id, &step_id, answers.0).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn retry_workflow_run(hub: State<'_, WorkspaceHub>, slug: String, run_id: String) -> Result<RunDetail, AppError> {
    Ok(run::retry(&hub.pool(&slug)?, &run_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn cancel_workflow_run(hub: State<'_, WorkspaceHub>, slug: String, run_id: String) -> Result<RunDetail, AppError> {
    Ok(run::cancel(&hub.pool(&slug)?, &run_id).await?)
}

/// The code steps waiting for the window to run them (D-069).
#[tauri::command]
#[specta::specta]
pub async fn pending_workflow_code(hub: State<'_, WorkspaceHub>, slug: String) -> Result<Vec<PendingCode>, AppError> {
    Ok(run::pending_code(&hub.pool(&slug)?).await?)
}

/// What the window's run of a code step gave: its output, or the error.
#[tauri::command]
#[specta::specta]
pub async fn answer_workflow_code(hub: State<'_, WorkspaceHub>, slug: String, run_id: String, step_id: String, attempt: String, output: Option<Json>, error: Option<String>) -> Result<RunDetail, AppError> {
    let outcome = match error {
        Some(e) => Err(e),
        None => Ok(output.map(|o| o.0).unwrap_or(serde_json::Value::Null)),
    };
    Ok(run::answer_code(&hub.pool(&slug)?, &run_id, &step_id, &attempt, outcome).await?)
}

/// The manual workflows for ⌘K (none) or a record's page (its object).
#[tauri::command]
#[specta::specta]
pub async fn list_manual_workflows(hub: State<'_, WorkspaceHub>, slug: String, object: Option<String>) -> Result<Vec<ManualWorkflow>, AppError> {
    Ok(workflows::manual(&hub.pool(&slug)?, object.as_deref()).await?)
}
