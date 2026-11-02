//! The CRM's assistant (D-070, 0.10.13): agents, chats, a question, proposals.

use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::modules::assistant::{self, AssistantAgent, AssistantAgentInput, ChatMessage, ChatThread};

#[tauri::command]
#[specta::specta]
pub async fn list_assistant_agents(hub: State<'_, WorkspaceHub>, slug: String) -> Result<Vec<AssistantAgent>, AppError> {
    Ok(assistant::agents(&hub.pool(&slug)?).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn save_assistant_agent(hub: State<'_, WorkspaceHub>, slug: String, input: AssistantAgentInput) -> Result<AssistantAgent, AppError> {
    Ok(assistant::save_agent(&hub.pool(&slug)?, &input).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_assistant_agent(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<(), AppError> {
    Ok(assistant::delete_agent(&hub.pool(&slug)?, &id).await?)
}

/// The chats, newest first — all, or a record's (`record_object` + `record_id`).
#[tauri::command]
#[specta::specta]
pub async fn list_chat_threads(hub: State<'_, WorkspaceHub>, slug: String, record_object: Option<String>, record_id: Option<String>) -> Result<Vec<ChatThread>, AppError> {
    let on = record_object.as_deref().zip(record_id.as_deref()).filter(|(o, i)| !o.is_empty() && !i.is_empty());
    Ok(assistant::threads(&hub.pool(&slug)?, on).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn new_chat_thread(hub: State<'_, WorkspaceHub>, slug: String, agent_id: Option<String>, record_object: String, record_id: String) -> Result<ChatThread, AppError> {
    Ok(assistant::new_thread(&hub.pool(&slug)?, agent_id.as_deref(), &record_object, &record_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_chat_thread(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<(), AppError> {
    Ok(assistant::delete_thread(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn chat_messages(hub: State<'_, WorkspaceHub>, slug: String, thread_id: String) -> Result<Vec<ChatMessage>, AppError> {
    Ok(assistant::messages(&hub.pool(&slug)?, &thread_id).await?)
}

/// The owner asks in a chat: the thread after the agent's turn.
#[tauri::command]
#[specta::specta]
pub async fn ask_assistant(app: AppHandle, hub: State<'_, WorkspaceHub>, slug: String, thread_id: String, text: String) -> Result<Vec<ChatMessage>, AppError> {
    Ok(assistant::ask(&app, &hub.pool(&slug)?, &thread_id, &text).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn apply_chat_proposal(hub: State<'_, WorkspaceHub>, slug: String, message_id: String) -> Result<ChatMessage, AppError> {
    Ok(assistant::apply(&hub.pool(&slug)?, &message_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn dismiss_chat_proposal(hub: State<'_, WorkspaceHub>, slug: String, message_id: String) -> Result<ChatMessage, AppError> {
    Ok(assistant::dismiss(&hub.pool(&slug)?, &message_id).await?)
}
