use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::kernel::ai::live::{LiveDirection, LiveLines};
use crate::kernel::ai::whisper::Language;
use crate::kernel::permissions::{self, PermissionState};
use crate::modules::meetings::{Meeting, TranscriptSegment};
use crate::recorder::session::{self, Recorder, RecorderStatus, StartRequest};

#[tauri::command]
#[specta::specta]
pub fn recorder_status(recorder: State<'_, Recorder>) -> Result<RecorderStatus, AppError> {
    Ok(recorder.status())
}

/// `live`: transcribe and translate as people speak (D-041), the way `direction` says; the
/// meeting's language follows it (both ways: auto). `save: false` (live only): a quick
/// translation — no meeting (None), recording or summary; its lines are in `live_lines`. Live
/// refuses with `ai_busy` while another AI job runs, `model_missing` without a Whisper model.
#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)] // the bridge passes each field
pub async fn start_recording(
    app: AppHandle,
    slug: String,
    title: String,
    language: Language,
    meeting_type_id: Option<String>,
    presentation: bool,
    presenter: Option<String>,
    live: bool,
    save: bool,
    direction: LiveDirection,
) -> Result<Option<Meeting>, AppError> {
    Ok(session::start(
        &app,
        StartRequest {
            slug,
            title,
            language,
            meeting_type_id,
            presenter,
            presentation,
            live,
            save,
            direction,
        },
    )
    .await?)
}

/// The current or last quick translation's lines, in memory only until the next live session
/// starts or the app quits.
#[tauri::command]
#[specta::specta]
pub fn live_lines(lines: State<'_, LiveLines>) -> Result<Vec<TranscriptSegment>, AppError> {
    Ok(lines.get())
}

#[tauri::command]
#[specta::specta]
pub fn set_recording_paused(app: AppHandle, paused: bool) -> Result<RecorderStatus, AppError> {
    Ok(session::pause(&app, paused)?)
}

/// Finalises the WAV, marks the meeting `recorded` and queues transcription — or, after a live
/// session, waits for its last lines and marks it `transcribed`. None after a quick translation.
#[tauri::command]
#[specta::specta]
pub async fn stop_recording(app: AppHandle) -> Result<Option<Meeting>, AppError> {
    Ok(session::stop(&app).await?)
}

#[tauri::command]
#[specta::specta]
pub fn microphone_permission() -> Result<PermissionState, AppError> {
    Ok(permissions::microphone()?)
}

#[tauri::command]
#[specta::specta]
pub async fn request_microphone_permission() -> Result<PermissionState, AppError> {
    Ok(permissions::request_microphone().await?)
}

/// Opens System Settings at Microphone or at Screen & System Audio Recording.
#[tauri::command]
#[specta::specta]
pub fn open_privacy_settings(system_audio: bool) -> Result<(), AppError> {
    let url = if system_audio {
        permissions::SYSTEM_AUDIO_SETTINGS
    } else {
        permissions::MICROPHONE_SETTINGS
    };
    tauri_plugin_opener::open_url(url, None::<&str>).map_err(anyhow::Error::from)?;
    Ok(())
}
