use std::path::PathBuf;

use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::kernel::ai::whisper::Language;
use crate::kernel::hub::WorkspaceHub;
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
    hub: State<'_, WorkspaceHub>,
    slug: String,
) -> Result<Vec<MeetingTypeSummary>, AppError> {
    Ok(meetings::meeting_types(&hub.pool(&slug)?).await?)
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
