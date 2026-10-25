use tauri::{AppHandle, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::error::AppError;
use crate::kernel::ai::ollama::{self, OllamaStatus};
use crate::kernel::hub::WorkspaceHub;
use crate::modules::meetings::review;
use crate::modules::meetings::summarise::{self, SummaryJobs, SummaryView};
use crate::modules::people::AttendeeEdit;

#[tauri::command]
#[specta::specta]
pub async fn get_summary(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    meeting_id: String,
) -> Result<Option<SummaryView>, AppError> {
    Ok(summarise::view(&hub.pool(&slug)?, &meeting_id).await?)
}

/// (Re)writes the executive summary; `deep` turns Qwen's thinking on (slower, D-021);
/// `keep_edits` keeps hand-edited sections (D-025).
#[tauri::command]
#[specta::specta]
pub async fn summarise_meeting(
    app: AppHandle,
    slug: String,
    meeting_id: String,
    deep: bool,
    keep_edits: bool,
) -> Result<(), AppError> {
    Ok(summarise::start(&app, &slug, &meeting_id, deep, keep_edits).await?)
}

#[tauri::command]
#[specta::specta]
pub fn cancel_summary(jobs: State<'_, SummaryJobs>, meeting_id: String) -> Result<(), AppError> {
    jobs.cancel(&meeting_id);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn update_summary_section(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    meeting_id: String,
    index: u32,
    markdown: String,
) -> Result<SummaryView, AppError> {
    let workspace = hub.workspace(&slug).await?;
    Ok(summarise::update_section(
        &hub.pool(&slug)?,
        &workspace.name,
        &meeting_id,
        index,
        &markdown,
    )
    .await?)
}

/// Puts the email on the clipboard as HTML, with the Markdown text as the plain-text fallback.
#[tauri::command]
#[specta::specta]
pub async fn copy_summary_email(
    app: AppHandle,
    hub: State<'_, WorkspaceHub>,
    slug: String,
    meeting_id: String,
) -> Result<String, AppError> {
    let (subject, plain, html) = summarise::email_parts(&hub.pool(&slug)?, &meeting_id).await?;
    app.clipboard()
        .write_html(html, Some(plain))
        .map_err(anyhow::Error::from)?;
    Ok(subject)
}

#[tauri::command]
#[specta::specta]
pub async fn open_summary_in_mail(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    meeting_id: String,
) -> Result<(), AppError> {
    let (subject, plain, _) = summarise::email_parts(&hub.pool(&slug)?, &meeting_id).await?;
    tauri_plugin_opener::open_url(summarise::mailto(&subject, &plain), None::<&str>)
        .map_err(anyhow::Error::from)?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn ollama_status() -> Result<OllamaStatus, AppError> {
    Ok(ollama::status().await)
}

#[tauri::command]
#[specta::specta]
pub fn open_ollama_app() -> Result<(), AppError> {
    tauri_plugin_opener::open_path("/Applications/Ollama.app", None::<&str>)
        .map_err(anyhow::Error::from)?;
    Ok(())
}

/// Starts downloading a model through Ollama (`None` = the selected one); progress arrives as
/// events.
#[tauri::command]
#[specta::specta]
pub fn pull_ollama_model(app: AppHandle, model: Option<String>) -> Result<(), AppError> {
    let model = model.unwrap_or_else(ollama::model);
    Ok(ollama::start_pull(&app, &model)?)
}

#[tauri::command]
#[specta::specta]
pub async fn remove_ollama_model(app: AppHandle, model: String) -> Result<(), AppError> {
    Ok(ollama::remove(&app, &model).await?)
}

/// The text model for every summary from now on.
#[tauri::command]
#[specta::specta]
pub async fn select_ollama_model(app: AppHandle, model: String) -> Result<OllamaStatus, AppError> {
    Ok(ollama::select(&app, &model).await?)
}

/// Drops the text model from memory now.
#[tauri::command]
#[specta::specta]
pub async fn unload_ollama_model() -> Result<(), AppError> {
    ollama::unload().await;
    Ok(())
}

/// Saves corrected attendees; `remember` teaches the people memory (D-028).
#[tauri::command]
#[specta::specta]
pub async fn update_attendees(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    meeting_id: String,
    attendees: Vec<AttendeeEdit>,
    remember: bool,
) -> Result<SummaryView, AppError> {
    let workspace = hub.workspace(&slug).await?;
    let pool = hub.pool(&slug)?;
    Ok(review::update_attendees(&pool, &workspace.name, &meeting_id, &attendees, remember).await?)
}
