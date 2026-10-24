//! Import an audio file into a meeting and transcribe it as a background job:
//! copy → (gateway: decode → VAD → Whisper) → save `transcript_segment` rows + `transcript.json`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::UNIX_EPOCH;

use anyhow::{Context, Result, bail};
use sqlx::SqlitePool;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;
use tauri_specta::Event;
use tokio::io::AsyncWriteExt;

use super::{Meeting, MeetingStatus, NewMeeting, audio};
use crate::error::AppError;
use crate::events::{TranscriptionFinished, TranscriptionProgress};
use crate::kernel::ai::whisper::{self, Language, Progress, Segment, Stage, TranscribeJob};
use crate::kernel::hub::WorkspaceHub;

/// Cancel flags of running jobs, by meeting id.
#[derive(Default)]
pub struct TranscriptionJobs(Mutex<HashMap<String, Arc<AtomicBool>>>);

impl TranscriptionJobs {
    pub fn cancel(&self, meeting_id: &str) {
        if let Some(flag) = self
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(meeting_id)
        {
            flag.store(true, Ordering::Relaxed);
        }
    }
}

pub struct ImportRequest {
    pub slug: String,
    pub source: PathBuf,
    pub title: String,
    pub language: Language,
    pub meeting_type_id: Option<String>,
}

/// Copies the file into `recordings/<meeting-id>/`, creates the meeting and starts its job.
pub async fn import(app: &AppHandle, request: ImportRequest) -> Result<Meeting> {
    let ext = request
        .source
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase)
        .filter(|e| audio::SUPPORTED_EXTENSIONS.contains(&e.as_str()))
        .ok_or_else(|| {
            AppError::invalid("Choose an audio file (m4a, mp3, wav, aiff, flac, ogg…)")
        })?;
    let title = request.title.trim();
    if title.is_empty() {
        bail!(AppError::invalid("Title is required"));
    }
    let held_at_unix = std::fs::metadata(&request.source)
        .with_context(|| format!("reading {}", request.source.display()))?
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs() as i64);

    let hub = app.state::<WorkspaceHub>();
    let workspace = hub.workspace(&request.slug).await?;
    let pool = hub.pool(&request.slug)?;
    let id = super::new_id();
    let folder = Path::new(&workspace.folder_path)
        .join("recordings")
        .join(&id);
    tokio::fs::create_dir_all(&folder).await?;
    let audio_path = folder.join(format!("audio.{ext}"));
    tokio::fs::copy(&request.source, &audio_path)
        .await
        .context("copying the audio file")?;

    let meeting = super::create(
        &pool,
        NewMeeting {
            id: &id,
            meeting_type_id: request.meeting_type_id.as_deref(),
            title,
            held_at_unix,
            language: request.language,
            audio_path: &audio_path.to_string_lossy(),
            status: MeetingStatus::Recorded,
        },
    )
    .await?;
    start(app, &request.slug, &id).await?;
    Ok(meeting)
}

/// Starts (or re-runs) transcription of a stored meeting in the background.
pub async fn start(app: &AppHandle, slug: &str, meeting_id: &str) -> Result<()> {
    let pool = app.state::<WorkspaceHub>().pool(slug)?;
    let meeting = super::get(&pool, meeting_id).await?;
    if !matches!(
        meeting.status,
        MeetingStatus::Recorded | MeetingStatus::Failed | MeetingStatus::Transcribed
    ) {
        bail!(AppError::invalid(
            "This meeting can't be transcribed right now"
        ));
    }
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let jobs = app.state::<TranscriptionJobs>();
        let mut jobs = jobs.0.lock().unwrap_or_else(PoisonError::into_inner);
        if jobs.contains_key(meeting_id) {
            bail!(AppError::invalid(
                "This meeting is already being transcribed"
            ));
        }
        jobs.insert(meeting_id.to_owned(), cancel.clone());
    }
    let (app, slug, meeting_id) = (app.clone(), slug.to_owned(), meeting_id.to_owned());
    tauri::async_runtime::spawn(async move {
        let title = meeting.title.clone();
        let result = run(&app, &slug, pool, meeting, cancel).await;
        app.state::<TranscriptionJobs>()
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&meeting_id);
        if let Err(err) = &result {
            tracing::warn!("transcription of {meeting_id} ended: {err:#}");
        }
        notify_finished(&app, &title, &result);
        // The executive summary follows the transcript automatically (D-021).
        if result.is_ok() {
            if let Err(err) = super::summarise::start(&app, &slug, &meeting_id, false).await {
                tracing::warn!("could not queue the summary: {err:#}");
            }
        }
        let error = result.err().map(AppError::from);
        let _ = TranscriptionFinished {
            workspace_slug: slug,
            meeting_id,
            error,
        }
        .emit(&app);
    });
    Ok(())
}

/// macOS notification when a job ends (not on cancel — the user asked for that).
fn notify_finished(app: &AppHandle, meeting_title: &str, result: &Result<()>) {
    let heading = match result {
        Ok(()) => "Transcript ready",
        Err(err) if whisper::is_cancelled(err) => return,
        Err(_) => "Transcription failed",
    };
    if let Err(err) = app
        .notification()
        .builder()
        .title(heading)
        .body(meeting_title)
        .show()
    {
        tracing::warn!("could not show a notification: {err}");
    }
}

/// Marks `transcribing`, runs the gateway, saves; on any failure after that point the status is
/// set exactly once: cancel restores the previous status, an error becomes `failed` unless an
/// earlier transcript exists.
async fn run(
    app: &AppHandle,
    slug: &str,
    pool: SqlitePool,
    meeting: Meeting,
    cancel: Arc<AtomicBool>,
) -> Result<()> {
    let previous = meeting.status;
    let audio_path = PathBuf::from(
        meeting
            .audio_path
            .clone()
            .context("meeting has no audio file")?,
    );
    let progress: Progress = {
        let (app, slug, meeting_id) = (app.clone(), slug.to_owned(), meeting.id.clone());
        Arc::new(move |stage, percent| {
            let event = TranscriptionProgress {
                workspace_slug: slug.clone(),
                meeting_id: meeting_id.clone(),
                stage,
                percent,
            };
            let _ = event.emit(&app);
        })
    };
    super::set_status(&pool, &meeting.id, MeetingStatus::Transcribing).await?;

    let outcome: Result<()> = async {
        let job = TranscribeJob {
            meeting_id: &meeting.id,
            language: meeting.language,
            cancel,
        };
        let source = audio_path.clone();
        let transcript = whisper::transcribe(
            app,
            &pool,
            job,
            move || audio::load_16k_mono(&source),
            progress.clone(),
        )
        .await?;
        progress(Stage::Saving, 97);
        super::save_transcript(
            &pool,
            &meeting.id,
            &transcript.segments,
            transcript.duration_ms,
        )
        .await?;
        // Side file only; the database is the source of truth, so a failure here is not fatal.
        if let Some(folder) = audio_path.parent() {
            if let Err(err) =
                write_transcript_json(folder, &meeting.id, transcript.model, &transcript.segments)
                    .await
            {
                tracing::warn!("could not write transcript.json: {err:#}");
            }
        }
        Ok(())
    }
    .await;

    if let Err(err) = &outcome {
        let had_transcript = matches!(
            previous,
            MeetingStatus::Transcribed | MeetingStatus::Summarising | MeetingStatus::Summarised
        );
        let status = if whisper::is_cancelled(err) || had_transcript {
            previous
        } else {
            MeetingStatus::Failed
        };
        if let Err(e) = super::set_status(&pool, &meeting.id, status).await {
            tracing::warn!("could not reset meeting status: {e:#}");
        }
    }
    outcome
}

/// Side file next to the audio: temp file, fsync, then rename (atomic and durable).
async fn write_transcript_json(
    folder: &Path,
    meeting_id: &str,
    model: &str,
    segments: &[Segment],
) -> Result<()> {
    let body = serde_json::to_vec_pretty(&serde_json::json!({
        "meeting_id": meeting_id,
        "model": model,
        "segments": segments,
    }))?;
    let tmp = folder.join("transcript.json.tmp");
    let mut file = tokio::fs::File::create(&tmp).await?;
    file.write_all(&body).await?;
    file.sync_all().await?;
    tokio::fs::rename(&tmp, folder.join("transcript.json")).await?;
    Ok(())
}
