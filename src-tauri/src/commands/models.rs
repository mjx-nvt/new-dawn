use tauri::{AppHandle, State};

use crate::kernel::ai::whisper::{self, WhisperEngine};
use crate::kernel::hub::WorkspaceHub;

use crate::error::AppError;
use crate::kernel::ai::models::{self, ModelInfo, ModelStore, SpeechChoice, SpeechJob};
use crate::kernel::ai::queue::{Activity, AiQueue};
use crate::kernel::ai::resources::{self, Pace};

#[tauri::command]
#[specta::specta]
pub fn list_models(store: State<'_, ModelStore>) -> Result<Vec<ModelInfo>, AppError> {
    Ok(store.list())
}

/// Starts (or resumes) a download in the background; progress and the result arrive as events.
#[tauri::command]
#[specta::specta]
pub fn start_model_download(app: AppHandle, id: String) -> Result<(), AppError> {
    Ok(models::start_download(&app, &id)?)
}

#[tauri::command]
#[specta::specta]
pub fn cancel_model_download(store: State<'_, ModelStore>, id: String) -> Result<(), AppError> {
    store.cancel(&id);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn delete_model(store: State<'_, ModelStore>, id: String) -> Result<(), AppError> {
    Ok(store.delete(&id)?)
}

#[tauri::command]
#[specta::specta]
pub fn reveal_models_folder(store: State<'_, ModelStore>) -> Result<(), AppError> {
    tauri_plugin_opener::reveal_item_in_dir(store.dir()).map_err(anyhow::Error::from)?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn get_whisper_idle_unload(engine: State<'_, WhisperEngine>) -> Result<u32, AppError> {
    Ok(engine.idle_unload_secs() as u32)
}

/// How long the Whisper model stays in memory after a job, in seconds (saved in registry.db).
#[tauri::command]
#[specta::specta]
pub async fn set_whisper_idle_unload(
    hub: State<'_, WorkspaceHub>,
    engine: State<'_, WhisperEngine>,
    secs: u32,
) -> Result<(), AppError> {
    Ok(whisper::set_idle_unload(&hub, &engine, secs).await?)
}

/// Each speech job's model: the owner's choice and what runs now.
#[tauri::command]
#[specta::specta]
pub async fn get_speech_choices(
    hub: State<'_, WorkspaceHub>,
    store: State<'_, ModelStore>,
) -> Result<Vec<SpeechChoice>, AppError> {
    Ok(models::speech_choices(&hub, &store).await?)
}

/// The Whisper model for meetings or Live (None = automatic).
#[tauri::command]
#[specta::specta]
pub async fn set_speech_model(
    hub: State<'_, WorkspaceHub>,
    job: SpeechJob,
    model: Option<String>,
) -> Result<(), AppError> {
    Ok(models::choose_speech_model(&hub, job, model.as_deref()).await?)
}

/// What the AI is doing and what waits for it, in the order it will run (D-048).
#[tauri::command]
#[specta::specta]
pub fn ai_activity(queue: State<'_, AiQueue>) -> Result<Vec<Activity>, AppError> {
    Ok(queue.activity())
}

/// Gentle (the default) or full speed while you work.
#[tauri::command]
#[specta::specta]
pub async fn get_ai_pace(hub: State<'_, WorkspaceHub>) -> Result<Pace, AppError> {
    Ok(resources::read_pace(&hub).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn set_ai_pace(hub: State<'_, WorkspaceHub>, pace: Pace) -> Result<(), AppError> {
    Ok(resources::set_pace(&hub, pace).await?)
}
