use std::path::PathBuf;

use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::kernel::ai::whisper::Language;
use crate::kernel::hub::WorkspaceHub;
use crate::modules::meetings::review;
use crate::modules::meetings::summarise::SummaryJobs;
use crate::modules::meetings::template;
use crate::modules::meetings::transcribe::{self, ImportRequest, TranscriptionJobs};
use crate::modules::meetings::{self, Meeting, MeetingHit, MeetingTypeSummary, TranscriptSegment};

#[tauri::command]
#[specta::specta]
pub async fn list_meetings(
    hub: State<'_, WorkspaceHub>,
    slug: String,
) -> Result<Vec<Meeting>, AppError> {
    Ok(meetings::list(&hub.pool(&slug)?).await?)
}

/// Title, transcript and summary search in one workspace (FTS5).
#[tauri::command]
#[specta::specta]
pub async fn search_meetings(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    query: String,
) -> Result<Vec<MeetingHit>, AppError> {
    Ok(meetings::search(&hub.pool(&slug)?, &query).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_meeting_types(
    app: AppHandle,
    slug: String,
) -> Result<Vec<MeetingTypeSummary>, AppError> {
    Ok(template::summaries(&app, &slug).await?)
}

/// Changes (or clears) a meeting's type.
#[tauri::command]
#[specta::specta]
pub async fn set_meeting_type(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    meeting_id: String,
    type_id: Option<String>,
) -> Result<Meeting, AppError> {
    Ok(meetings::set_type(&hub.pool(&slug)?, &meeting_id, type_id.as_deref()).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn get_meeting(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    id: String,
) -> Result<Meeting, AppError> {
    Ok(meetings::get(&hub.pool(&slug)?, &id).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn get_transcript(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    meeting_id: String,
) -> Result<Vec<TranscriptSegment>, AppError> {
    Ok(meetings::transcript(&hub.pool(&slug)?, &meeting_id).await?)
}

/// Copies the audio into the workspace, creates the meeting and starts transcription.
#[tauri::command]
#[specta::specta]
pub async fn import_audio(
    app: AppHandle,
    slug: String,
    path: String,
    title: String,
    language: Language,
    meeting_type_id: Option<String>,
) -> Result<Meeting, AppError> {
    let request = ImportRequest {
        slug,
        source: PathBuf::from(path),
        title,
        language,
        meeting_type_id,
    };
    Ok(transcribe::import(&app, request).await?)
}

/// Re-runs transcription (after a failure or cancel, or to retry with another model).
#[tauri::command]
#[specta::specta]
pub async fn transcribe_meeting(
    app: AppHandle,
    slug: String,
    meeting_id: String,
) -> Result<(), AppError> {
    Ok(transcribe::start(&app, &slug, &meeting_id).await?)
}

#[tauri::command]
#[specta::specta]
pub fn cancel_transcription(
    jobs: State<'_, TranscriptionJobs>,
    meeting_id: String,
) -> Result<(), AppError> {
    jobs.cancel(&meeting_id);
    Ok(())
}

/// Corrects one transcript line (the next summary run reads it).
#[tauri::command]
#[specta::specta]
pub async fn update_transcript_segment(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    meeting_id: String,
    seq: u32,
    text: String,
) -> Result<(), AppError> {
    Ok(review::update_segment(&hub.pool(&slug)?, &meeting_id, seq, &text).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn set_meeting_notes(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    meeting_id: String,
    notes: String,
) -> Result<Meeting, AppError> {
    Ok(review::set_notes(&hub.pool(&slug)?, &meeting_id, &notes).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn rename_meeting(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    meeting_id: String,
    title: String,
) -> Result<Meeting, AppError> {
    Ok(review::rename(&hub, &slug, &meeting_id, &title).await?)
}

/// Recording to the Trash; transcript, summary and export rows deleted (exported files stay).
#[tauri::command]
#[specta::specta]
pub async fn delete_meeting(
    hub: State<'_, WorkspaceHub>,
    transcriptions: State<'_, TranscriptionJobs>,
    summaries: State<'_, SummaryJobs>,
    slug: String,
    meeting_id: String,
) -> Result<(), AppError> {
    let running = transcriptions.contains(&meeting_id) || summaries.contains(&meeting_id);
    Ok(review::delete_meeting(&hub, &slug, &meeting_id, running).await?)
}
