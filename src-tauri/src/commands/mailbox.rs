//! Apple Mail as the connected account (D-066, 0.10.9): a record's email threads and calendar,
//! the mail settings, the blocklist, a person added from a thread.

use tauri::State;

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::kernel::records::types::Json;
use crate::kernel::records;
use crate::modules::crm::CalendarEvent;
use crate::modules::mailbox::{self, Blocked, EmailMessage, EmailThreads, MailSettings};

/// The record's threads from the cache, read from Mail again when `refresh` or when stale.
#[tauri::command]
#[specta::specta]
pub async fn list_email_threads(hub: State<'_, WorkspaceHub>, slug: String, object: String, id: String, refresh: bool) -> Result<EmailThreads, AppError> {
    let pool = hub.pool(&slug)?;
    let def = records::object(&pool, &object).await?;
    Ok(mailbox::threads(&pool, &def, &id, refresh).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_email_messages(hub: State<'_, WorkspaceHub>, slug: String, thread_id: String) -> Result<Vec<EmailMessage>, AppError> {
    Ok(mailbox::messages(&hub.pool(&slug)?, &thread_id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn mail_settings(hub: State<'_, WorkspaceHub>, slug: String) -> Result<MailSettings, AppError> {
    Ok(mailbox::settings(&hub.pool(&slug)?).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn set_mail_settings(hub: State<'_, WorkspaceHub>, slug: String, settings: MailSettings) -> Result<(), AppError> {
    Ok(mailbox::set_settings(&hub.pool(&slug)?, &settings).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_blocklist(hub: State<'_, WorkspaceHub>, slug: String) -> Result<Vec<Blocked>, AppError> {
    Ok(mailbox::blocklist(&hub.pool(&slug)?).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn block_handle(hub: State<'_, WorkspaceHub>, slug: String, handle: String) -> Result<Blocked, AppError> {
    Ok(mailbox::block(&hub.pool(&slug)?, &handle).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn unblock_handle(hub: State<'_, WorkspaceHub>, slug: String, id: String) -> Result<(), AppError> {
    Ok(mailbox::unblock(&hub.pool(&slug)?, &id).await?)
}

/// A person the CRM didn't know, from a thread's participant.
#[tauri::command]
#[specta::specta]
pub async fn add_person_from_email(hub: State<'_, WorkspaceHub>, slug: String, handle: String, name: String) -> Result<Json, AppError> {
    let actor = records::actor_of(&hub, &slug).await?;
    Ok(Json(mailbox::add_person(&hub.pool(&slug)?, &handle, &name, &actor).await?))
}

#[tauri::command]
#[specta::specta]
pub async fn calendar_events(hub: State<'_, WorkspaceHub>, slug: String, object: String, id: String) -> Result<Vec<CalendarEvent>, AppError> {
    let pool = hub.pool(&slug)?;
    let def = records::object(&pool, &object).await?;
    Ok(mailbox::calendar(&pool, &def, &id).await?)
}

/// Whether Mail is running (the Accounts pane's status).
#[tauri::command]
#[specta::specta]
pub async fn mail_running() -> Result<bool, AppError> {
    Ok(mailbox::mail_running().await)
}
