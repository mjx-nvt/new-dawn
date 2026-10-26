use tauri::State;

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::modules::meetings::summarise::SummaryView;
use crate::modules::tasks::{self, TaskList, TaskPatch, TaskView};

/// The workspace's tasks: today (overdue included), upcoming, no date, completed.
#[tauri::command]
#[specta::specta]
pub async fn list_tasks(hub: State<'_, WorkspaceHub>, slug: String) -> Result<TaskList, AppError> {
    Ok(tasks::list(&hub.pool(&slug)?).await?)
}

/// A task typed in Tasks (no date).
#[tauri::command]
#[specta::specta]
pub async fn add_task(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    title: String,
) -> Result<TaskView, AppError> {
    Ok(tasks::add(&hub.pool(&slug)?, &title).await?)
}

/// Renames, dates (`due_on: null` removes the date) or ticks a task; a tick follows to the
/// meeting's action item.
#[tauri::command]
#[specta::specta]
pub async fn update_task(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    id: String,
    patch: TaskPatch,
) -> Result<TaskView, AppError> {
    Ok(tasks::update(&hub.pool(&slug)?, &id, patch).await?)
}

/// Deletes a task; "Add to Tasks" on its action item brings it back.
#[tauri::command]
#[specta::specta]
pub async fn delete_task(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    id: String,
) -> Result<(), AppError> {
    Ok(tasks::delete(&hub.pool(&slug)?, &id).await?)
}

/// "Add to Tasks" on action item `index` of a meeting's summary.
#[tauri::command]
#[specta::specta]
pub async fn add_action_item_to_tasks(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    meeting_id: String,
    index: u32,
) -> Result<SummaryView, AppError> {
    Ok(tasks::add_from_item(&hub, &slug, &meeting_id, index as usize).await?)
}

/// Open tasks due today or earlier: the sidebar badge.
#[tauri::command]
#[specta::specta]
pub async fn open_task_count(hub: State<'_, WorkspaceHub>, slug: String) -> Result<u32, AppError> {
    Ok(tasks::open_count(&hub.pool(&slug)?).await?)
}
