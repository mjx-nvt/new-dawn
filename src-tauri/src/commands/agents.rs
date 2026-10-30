use std::path::Path;

use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::modules::agents::{self, Agent, AgentGuide};
use crate::modules::lessons;

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

/// "Check now": reads the owner's newest decisions about the agent's work without waiting for a
/// free Mac (D-052). Returns how many lines it proposes.
#[tauri::command]
#[specta::specta]
pub async fn learn_now(app: AppHandle, slug: String, agent: Agent) -> Result<u32, AppError> {
    Ok(lessons::run(&app, &slug, agent, true).await?)
}

/// Approve: the proposed line is added to the agent's guide.
#[tauri::command]
#[specta::specta]
pub async fn approve_lesson(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    id: String,
) -> Result<Vec<AgentGuide>, AppError> {
    let workspace = hub.workspace(&slug).await?;
    let (pool, folder) = (hub.pool(&slug)?, Path::new(&workspace.folder_path));
    lessons::approve(&pool, folder, &id).await?;
    Ok(agents::list(&pool, folder).await?)
}

/// Dismiss: the proposed line is set aside and not proposed again.
#[tauri::command]
#[specta::specta]
pub async fn dismiss_lesson(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    id: String,
) -> Result<Vec<AgentGuide>, AppError> {
    let workspace = hub.workspace(&slug).await?;
    let pool = hub.pool(&slug)?;
    lessons::dismiss(&pool, &id).await?;
    Ok(agents::list(&pool, Path::new(&workspace.folder_path)).await?)
}

/// Stops using a line approved from "What I learned"; it isn't proposed again.
#[tauri::command]
#[specta::specta]
pub async fn forget_learned(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    agent: Agent,
    line: String,
) -> Result<Vec<AgentGuide>, AppError> {
    let workspace = hub.workspace(&slug).await?;
    let (pool, folder) = (hub.pool(&slug)?, Path::new(&workspace.folder_path));
    lessons::forget(&pool, folder, agent, &line).await?;
    Ok(agents::list(&pool, folder).await?)
}
