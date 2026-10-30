use tauri::{AppHandle, State};

use crate::kernel::ai::whisper::{self, WhisperEngine};
use crate::kernel::hub::WorkspaceHub;

use crate::error::AppError;
use crate::kernel::ai::models::{self, ModelInfo, ModelStore, SpeechChoice, SpeechJob};
use crate::kernel::ai::queue::{Activity, AiQueue};
use crate::kernel::ai::resources::{self, AiPolicy};
use crate::modules::model_check::{self, CheckKind, ModelCheck, Pick, Reference};

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

/// How AI work shares the Mac: the pace (gentle by default) and whether catch-up work may run
/// while you work.
#[tauri::command]
#[specta::specta]
pub async fn get_ai_policy(hub: State<'_, WorkspaceHub>) -> Result<AiPolicy, AppError> {
    Ok(resources::policy(&hub).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn set_ai_policy(hub: State<'_, WorkspaceHub>, policy: AiPolicy) -> Result<(), AppError> {
    Ok(resources::set_policy(&hub, policy).await?)
}

/// Run now for a catch-up job held back by the Mac's state (the AI activity list).
#[tauri::command]
#[specta::specta]
pub fn run_ai_job_now(queue: State<'_, AiQueue>, id: String) -> Result<(), AppError> {
    queue.run_now(&id);
    Ok(())
}

/// The model checks of a workspace, newest first (D-050).
#[tauri::command]
#[specta::specta]
pub async fn list_model_checks(
    hub: State<'_, WorkspaceHub>,
    slug: String,
) -> Result<Vec<ModelCheck>, AppError> {
    Ok(model_check::list(&hub.pool(&slug)?).await?)
}

/// Tries `candidate` (a Whisper catalogue id or an Ollama model) on the reference meetings.
#[tauri::command]
#[specta::specta]
pub async fn start_model_check(
    app: AppHandle,
    slug: String,
    kind: CheckKind,
    candidate: String,
) -> Result<ModelCheck, AppError> {
    Ok(model_check::start(&app, &slug, kind, &candidate).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn cancel_model_check(app: AppHandle, slug: String, id: String) -> Result<(), AppError> {
    Ok(model_check::cancel(&app, &slug, &id).await?)
}

/// The owner's choice on one blind pair.
#[tauri::command]
#[specta::specta]
pub async fn pick_model_check(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    id: String,
    pair_id: String,
    pick: Pick,
) -> Result<ModelCheck, AppError> {
    Ok(model_check::pick(&hub.pool(&slug)?, &id, &pair_id, pick).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn list_references(
    hub: State<'_, WorkspaceHub>,
    slug: String,
) -> Result<Vec<Reference>, AppError> {
    Ok(model_check::references(&hub.pool(&slug)?).await?)
}

/// Marks a meeting as a reference for the model check, or not.
#[tauri::command]
#[specta::specta]
pub async fn set_meeting_reference(
    hub: State<'_, WorkspaceHub>,
    slug: String,
    meeting_id: String,
    on: bool,
) -> Result<(), AppError> {
    Ok(model_check::set_reference(&hub.pool(&slug)?, &meeting_id, on).await?)
}
