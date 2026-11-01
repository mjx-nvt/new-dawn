use tauri::State;

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::modules::switches::{self, Modules};

/// Which modules the workspace has on — Projects, CRM (D-059).
#[tauri::command]
#[specta::specta]
pub async fn get_modules(hub: State<'_, WorkspaceHub>, slug: String) -> Result<Modules, AppError> {
    Ok(switches::enabled(&hub.pool(&slug)?).await?)
}

/// Turns the workspace's modules on or off. Off hides the module and pauses its agents; its data
/// stays.
#[tauri::command]
#[specta::specta]
pub async fn set_modules(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    modules: Modules,
) -> Result<Modules, AppError> {
    switches::set(&hub.pool(&slug)?, modules).await?;
    Ok(modules)
}
