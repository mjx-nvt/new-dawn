use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::modules::inbox::{self, EmailItem};

/// The emails read, newest first, with what came of each.
#[tauri::command]
#[specta::specta]
pub async fn list_emails(
    hub: State<'_, WorkspaceHub>,
    slug: String,
) -> Result<Vec<EmailItem>, AppError> {
    Ok(inbox::list(&hub.pool(&slug)?).await?)
}

/// Reads an .eml file the owner chose (D-056).
#[tauri::command]
#[specta::specta]
pub async fn read_email_file(
    app: AppHandle,
    slug: String,
    path: String,
) -> Result<EmailItem, AppError> {
    Ok(inbox::read_file(&app, &slug, &path).await?)
}

/// Reads the email selected in Apple Mail.
#[tauri::command]
#[specta::specta]
pub async fn read_mail_selection(app: AppHandle, slug: String) -> Result<EmailItem, AppError> {
    Ok(inbox::read_mail(&app, &slug).await?)
}

/// Forgets an email: its text and the proposals still waiting.
#[tauri::command]
#[specta::specta]
pub async fn forget_email(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    id: String,
) -> Result<(), AppError> {
    Ok(inbox::forget(&hub.pool(&slug)?, &id).await?)
}

/// Add to Tasks: an email's proposed task becomes the owner's.
#[tauri::command]
#[specta::specta]
pub async fn add_email_task(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    id: String,
) -> Result<(), AppError> {
    Ok(inbox::add_task(&hub.pool(&slug)?, &id).await?)
}

/// Dismisses an email's proposed task.
#[tauri::command]
#[specta::specta]
pub async fn dismiss_email_task(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    id: String,
) -> Result<(), AppError> {
    Ok(inbox::dismiss_task(&hub.pool(&slug)?, &id).await?)
}
