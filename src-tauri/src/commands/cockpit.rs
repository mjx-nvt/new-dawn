use tauri::State;

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::modules::cockpit::{self, Today};

/// What needs the owner today, across every open workspace (D-057).
#[tauri::command]
#[specta::specta]
pub async fn cockpit_today(hub: State<'_, WorkspaceHub>) -> Result<Today, AppError> {
    Ok(cockpit::today(&hub).await?)
}
