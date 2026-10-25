use tauri::State;

use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::modules::meetings::review;
use crate::modules::vocabulary::{self, Term, TermInput};

#[tauri::command]
#[specta::specta]
pub async fn list_vocabulary(
    hub: State<'_, WorkspaceHub>,
    slug: String,
) -> Result<Vec<Term>, AppError> {
    Ok(vocabulary::list(&hub.pool(&slug)?).await?)
}

/// Adds (no id) or updates a term by hand.
#[tauri::command]
#[specta::specta]
pub async fn save_vocabulary_term(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    term: TermInput,
) -> Result<Term, AppError> {
    Ok(vocabulary::save(&hub.pool(&slug)?, term).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_vocabulary_term(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    id: String,
) -> Result<(), AppError> {
    Ok(vocabulary::delete(&hub.pool(&slug)?, &id).await?)
}

/// What Whisper is told before the next transcription (the title is a placeholder).
#[tauri::command]
#[specta::specta]
pub async fn whisper_prompt_preview(
    hub: State<'_, WorkspaceHub>,
    slug: String,
) -> Result<String, AppError> {
    Ok(vocabulary::preview(&hub.pool(&slug)?).await?)
}

/// Undoes the vocabulary's correction of one transcript line.
#[tauri::command]
#[specta::specta]
pub async fn restore_transcript_segment(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    meeting_id: String,
    seq: u32,
) -> Result<(), AppError> {
    Ok(review::restore_segment(&hub.pool(&slug)?, &meeting_id, seq).await?)
}
