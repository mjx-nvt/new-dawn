use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::kernel::ai::whisper::Language;
use crate::kernel::permissions::{self, PermissionState};
use crate::modules::meetings::Meeting;
use crate::recorder::session::{self, Recorder, RecorderStatus, StartRequest};

#[tauri::command]
#[specta::specta]
pub fn recorder_status(recorder: State<'_, Recorder>) -> Result<RecorderStatus, AppError> {
    Ok(recorder.status())
}

#[tauri::command]
#[specta::specta]
pub async fn start_recording(
    app: AppHandle,
    slug: String,
    title: String,
    language: Language,
    meeting_type_id: Option<String>,
) -> Result<Meeting, AppError> {
    Ok(session::start(&app, StartRequest { slug, title, language, meeting_type_id }).await?)
}

#[tauri::command]
#[specta::specta]
pub fn set_recording_paused(app: AppHandle, paused: bool) -> Result<RecorderStatus, AppError> {
    Ok(session::pause(&app, paused)?)
}

/// Finalises the WAV, marks the meeting `recorded` and queues transcription.
#[tauri::command]
#[specta::specta]
pub async fn stop_recording(app: AppHandle) -> Result<Meeting, AppError> {
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
    let url = if system_audio { permissions::SYSTEM_AUDIO_SETTINGS } else { permissions::MICROPHONE_SETTINGS };
    tauri_plugin_opener::open_url(url, None::<&str>).map_err(anyhow::Error::from)?;
    Ok(())
}
