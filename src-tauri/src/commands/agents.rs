use std::path::Path;

use tauri::State;

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::modules::agents::{self, Agent, AgentGuide};

#[tauri::command]
#[specta::specta]
pub async fn list_agents(
    hub: State<'_, WorkspaceHub>,
    slug: String,
) -> Result<Vec<AgentGuide>, AppError> {
    let workspace = hub.workspace(&slug).await?;
    Ok(agents::list(&hub.pool(&slug)?, Path::new(&workspace.folder_path)).await?)
}

/// Saves the workspace's own guide for an agent; empty text restores the default.
#[tauri::command]
#[specta::specta]
pub async fn set_agent_guide(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    agent: Agent,
    text: String,
) -> Result<Vec<AgentGuide>, AppError> {
    let workspace = hub.workspace(&slug).await?;
    let folder = Path::new(&workspace.folder_path);
    agents::set_guide(folder, agent, &text)?;
    Ok(agents::list(&hub.pool(&slug)?, folder).await?)
}

/// Chooses the model an agent runs on (None = the app's selected model).
#[tauri::command]
#[specta::specta]
pub async fn set_agent_model(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    agent: Agent,
    model: Option<String>,
) -> Result<Vec<AgentGuide>, AppError> {
    let workspace = hub.workspace(&slug).await?;
    let pool = hub.pool(&slug)?;
    agents::set_model(&pool, agent, model.as_deref()).await?;
    Ok(agents::list(&pool, Path::new(&workspace.folder_path)).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn reset_agent_guide(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    agent: Agent,
) -> Result<Vec<AgentGuide>, AppError> {
    let workspace = hub.workspace(&slug).await?;
    let folder = Path::new(&workspace.folder_path);
    agents::reset(folder, agent)?;
    Ok(agents::list(&hub.pool(&slug)?, folder).await?)
}
