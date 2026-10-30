use tauri::{AppHandle, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::error::AppError;
use crate::kernel::ai::ollama::{self, OllamaStatus};
use crate::kernel::ai::translate::SummaryLang;
use crate::kernel::hub::WorkspaceHub;
use crate::modules::meetings::corrections::{self, CorrectionsOutcome};
use crate::modules::meetings::naming::{self, NameDecision};
use crate::modules::meetings::review;
use crate::modules::meetings::summarise::{self, SummaryJobs, SummaryView};
use crate::modules::meetings::translations::{self, SummaryTranslation};
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

/// Applies the owner's answers to the naming agent: renames everywhere, teaches the memory,
/// clears the pending list.
#[tauri::command]
#[specta::specta]
pub async fn confirm_names(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    meeting_id: String,
    decisions: Vec<NameDecision>,
) -> Result<SummaryView, AppError> {
    let workspace = hub.workspace(&slug).await?;
    Ok(naming::confirm(&hub.pool(&slug)?, &workspace.name, &meeting_id, &decisions).await?)
}

/// Applies the owner's corrections (names, titles, terms, facts) to the summary, email and
/// transcript with the corrections agent, and remembers the people and terms (D-037).
#[tauri::command]
#[specta::specta]
pub async fn apply_corrections(
    app: AppHandle,
    slug: String,
    meeting_id: String,
    text: String,
) -> Result<CorrectionsOutcome, AppError> {
    Ok(corrections::apply(&app, &slug, &meeting_id, &text).await?)
}

/// Drops the pending list without changing anything.
#[tauri::command]
#[specta::specta]
pub async fn dismiss_names(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    meeting_id: String,
) -> Result<Option<SummaryView>, AppError> {
    let pool = hub.pool(&slug)?;
    crate::modules::meetings::set_naming(&pool, &meeting_id, None).await?;
    Ok(summarise::view(&pool, &meeting_id).await?)
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
    app: AppHandle,
    hub: State<'_, WorkspaceHub>,
    slug: String,
    meeting_id: String,
    index: u32,
    markdown: String,
) -> Result<SummaryView, AppError> {
    let workspace = hub.workspace(&slug).await?;
    let view = summarise::update_section(
        &hub.pool(&slug)?,
        &workspace.name,
        &meeting_id,
        index,
        &markdown,
    )
    .await?;
    // Edited action items: the commitments follow, in the background (D-055).
    crate::modules::crm::follow(&app, &slug, &meeting_id);
    Ok(view)
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

/// The summary in `lang`, if it was translated (D-046).
#[tauri::command]
#[specta::specta]
pub async fn get_summary_translation(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    meeting_id: String,
    lang: SummaryLang,
) -> Result<Option<SummaryTranslation>, AppError> {
    Ok(translations::get(&hub.pool(&slug)?, &meeting_id, lang).await?)
}

/// Translates the summary into `lang` on this Mac and keeps it.
#[tauri::command]
#[specta::specta]
pub async fn translate_summary(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    meeting_id: String,
    lang: SummaryLang,
) -> Result<SummaryTranslation, AppError> {
    Ok(translations::translate(&hub.pool(&slug)?, &meeting_id, lang).await?)
}

/// Writes the cover note again with the email agent.
#[tauri::command]
#[specta::specta]
pub async fn rewrite_email(
    app: AppHandle,
    slug: String,
    meeting_id: String,
) -> Result<SummaryView, AppError> {
    Ok(summarise::rewrite_email(&app, &slug, &meeting_id).await?)
}

/// A new message in Mail with the cover note and the latest minutes export attached (Apple
/// Events, like the PDF converters); `mailto:` when Mail can't be driven. Returns whether a
/// file was attached.
#[tauri::command]
#[specta::specta]
pub async fn open_summary_in_mail(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    meeting_id: String,
) -> Result<bool, AppError> {
    Ok(summarise::open_in_mail(&hub.pool(&slug)?, &meeting_id).await?)
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
    Ok(review::update_attendees(&hub.pool(&slug)?, &meeting_id, &attendees, remember).await?)
}
