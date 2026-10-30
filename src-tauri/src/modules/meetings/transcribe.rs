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
use tauri_specta::Event;
use tokio::io::AsyncWriteExt;

use super::{Meeting, MeetingFormat, MeetingStatus, NewMeeting, Speaker, audio};
use crate::error::AppError;
use crate::events::{TranscriptionFinished, TranscriptionProgress};
use crate::kernel::ai::live::overheard;
use crate::kernel::ai::queue::{JobKind, Lane, Task};
use crate::kernel::ai::whisper::{self, Language, Progress, Segment, Stage, TranscribeJob};
use crate::kernel::hub::WorkspaceHub;
use crate::modules::vocabulary;

/// A transcription's place, next to its recording (D-048): resumed after a quit, deleted once the
/// transcript is saved.
pub const CHECKPOINT: &str = "transcribing.json";

/// Cancel flags of running jobs, by meeting id.
#[derive(Default)]
pub struct TranscriptionJobs(Mutex<HashMap<String, Arc<AtomicBool>>>);

impl TranscriptionJobs {
    pub fn contains(&self, meeting_id: &str) -> bool {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains_key(meeting_id)
    }

    /// Some meeting is being transcribed (or waits for its turn).
    pub fn any(&self) -> bool {
        !self
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_empty()
    }

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
    /// A presentation by one main presenter (else a discussion), D-038.
    pub presenter: Option<String>,
    pub presentation: bool,
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
    let meeting = if request.presentation {
        super::set_format(
            &pool,
            &id,
            super::MeetingFormat::Presentation,
            request.presenter.as_deref(),
        )
        .await?
    } else {
        meeting
    };
    start(app, &request.slug, &id).await?;
    Ok(meeting)
}

/// Starts (or re-runs) transcription of a stored meeting in the background.
pub async fn start(app: &AppHandle, slug: &str, meeting_id: &str) -> Result<()> {
    let pool = app.state::<WorkspaceHub>().pool(slug)?;
    let meeting = super::get(&pool, meeting_id).await?;
    if !matches!(
        meeting.status,
        MeetingStatus::Recorded
            | MeetingStatus::Failed
            | MeetingStatus::Transcribed
            | MeetingStatus::Summarised
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
            if let Err(err) = super::summarise::start(&app, &slug, &meeting_id, false, true).await {
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
    crate::kernel::notify::show(app, heading, meeting_title);
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
    // In the meeting's own folder, whatever layout its audio has (D-048).
    let checkpoint = Some(super::job_file(app, slug, &meeting.id, CHECKPOINT).await?);
    super::set_status(&pool, &meeting.id, MeetingStatus::Transcribing).await?;

    let outcome: Result<()> = async {
        // The workspace vocabulary guides Whisper and corrects the result (D-036); the owner's
        // name leads the people.
        let owner = crate::kernel::profile::owner(&app.state::<WorkspaceHub>(), slug).await?;
        let people = crate::modules::people::with_owner(
            crate::modules::people::list(&pool).await?,
            owner.as_ref(),
        );
        let terms = vocabulary::list(&pool).await?;
        let prompt = vocabulary::whisper_prompt(
            &meeting.title,
            meeting.presenter.as_deref(),
            &people,
            &terms,
        );
        let task = Task::job(
            JobKind::Transcription,
            format!("Transcribing “{}”", meeting.title),
            slug,
            &meeting.id,
        );
        // A discussion recorded with the call's audio is transcribed as the microphone and the
        // call apart (D-051): who spoke is known, and two people talking at once both count.
        let folder = audio_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let apart = meeting.format == MeetingFormat::Discussion
            && super::CHANNELS
                .iter()
                .all(|(name, _)| super::channel_file(&folder, name).is_some());
        let mut sources = Vec::new();
        if apart {
            for (name, speaker) in super::CHANNELS {
                let place =
                    super::job_file(app, slug, &meeting.id, &channel_checkpoint(name)).await?;
                sources.push((Some(name), Some(speaker), Some(place)));
            }
        } else {
            sources.push((None, None, checkpoint.clone()));
        }
        let parts = sources.len() as u32;
        let mut lines: Vec<(Segment, Option<String>, Option<Speaker>)> = Vec::new();
        let (mut model, mut duration_ms) = ("", 0);
        for (k, (channel, speaker, place)) in sources.into_iter().enumerate() {
            let (report, k) = (progress.clone(), k as u32);
            let share: Progress =
                Arc::new(move |stage, percent| report(stage, (k * 100 + percent) / parts));
            let job = TranscribeJob {
                meeting_id: &meeting.id,
                language: meeting.language,
                cancel: cancel.clone(),
                prompt: Some(prompt.clone()),
                lane: Lane::Foreground,
                task: task.clone(),
                checkpoint: place,
            };
            let (mix, folder) = (audio_path.clone(), folder.clone());
            let load = move || match channel {
                // Found when its turn comes: a compression may have replaced the WAV meanwhile.
                Some(name) => audio::load_16k_mono(
                    &super::channel_file(&folder, name)
                        .with_context(|| format!("the {name} recording is gone"))?,
                ),
                None => audio::load_16k_mono(&mix),
            };
            let mut transcript = whisper::transcribe(app, &pool, job, load, share).await?;
            let raw = polish(&mut transcript.segments, &prompt, &people, &terms);
            model = transcript.model;
            duration_ms = duration_ms.max(transcript.duration_ms);
            lines.extend(
                transcript
                    .segments
                    .into_iter()
                    .zip(raw)
                    .map(|(s, r)| (s, r, speaker)),
            );
        }
        if parts > 1 {
            lines = merge_channels(lines);
        }
        progress(Stage::Saving, 97);
        let segments: Vec<Segment> = lines.iter().map(|(s, ..)| s.clone()).collect();
        let raw: Vec<Option<String>> = lines.iter().map(|(_, r, _)| r.clone()).collect();
        let speakers: Vec<Option<Speaker>> = lines.iter().map(|(.., sp)| *sp).collect();
        super::save_lines(&pool, &meeting.id, &segments, &raw, &speakers, duration_ms).await?;
        forget_all(app, slug, &meeting.id, checkpoint.as_deref()).await;
        let corrected = raw.iter().filter(|r| r.is_some()).count();
        if corrected > 0 {
            tracing::info!(
                "vocabulary corrected {corrected} line(s) of meeting {}",
                meeting.id
            );
        }
        // The transcript is safe: the original can shrink now (D-031). Not fatal either.
        if let Err(err) =
            crate::kernel::storage::compress_meeting(&pool, &meeting.id, &audio_path).await
        {
            tracing::warn!("could not compress the recording: {err:#}");
        }
        // Side file only; the database is the source of truth, so a failure here is not fatal.
        if let Some(folder) = audio_path.parent() {
            if let Err(err) = write_transcript_json(folder, &meeting.id, model, &segments).await {
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
        // Cancelled: the owner stopped it, nothing to resume. An error keeps the place, so
        // Transcribe again goes on from it.
        if whisper::is_cancelled(err) {
            forget_all(app, slug, &meeting.id, checkpoint.as_deref()).await;
        }
        if let Err(e) = super::set_status(&pool, &meeting.id, status).await {
            tracing::warn!("could not reset meeting status: {e:#}");
        }
    }
    outcome
}

/// Less call speech than this in a discussion recorded in two channels: not a call (D-051).
const MIN_CALL_MS: u32 = 20_000;

/// A channel's saved place: `transcribing-mic.json`, `transcribing-call.json` (D-051).
pub fn channel_checkpoint(name: &str) -> String {
    format!("transcribing-{name}.json")
}

/// Every place a transcription of this meeting may have saved (the mix, or each channel).
pub async fn checkpoints(app: &AppHandle, slug: &str, meeting_id: &str) -> Result<Vec<PathBuf>> {
    let mut out = vec![super::job_file(app, slug, meeting_id, CHECKPOINT).await?];
    for (name, _) in super::CHANNELS {
        out.push(super::job_file(app, slug, meeting_id, &channel_checkpoint(name)).await?);
    }
    Ok(out)
}

async fn forget_all(app: &AppHandle, slug: &str, meeting_id: &str, fallback: Option<&Path>) {
    match checkpoints(app, slug, meeting_id).await {
        Ok(all) => all.iter().for_each(|path| forget(Some(path))),
        Err(_) => forget(fallback),
    }
}

/// The microphone's and the call's lines as one transcript (D-051): a microphone line that is the
/// call heard through the speakers goes (Live's test, D-043), then everything in speaking order.
fn merge_channels(
    mut lines: Vec<(Segment, Option<String>, Option<Speaker>)>,
) -> Vec<(Segment, Option<String>, Option<Speaker>)> {
    let them = |speaker: &Option<Speaker>| *speaker == Some(Speaker::Them);
    let calls: Vec<(u32, u32, String)> = lines
        .iter()
        .filter(|(.., speaker)| them(speaker))
        .map(|(s, ..)| (s.start_ms, s.end_ms, s.text.clone()))
        .collect();
    let calls: Vec<(u32, u32, &str)> = calls.iter().map(|(a, b, t)| (*a, *b, t.as_str())).collect();
    lines.retain(|(s, _, speaker)| {
        *speaker != Some(Speaker::You) || !overheard(s.start_ms, s.end_ms, &s.text, &calls)
    });
    // The Mac's sound is recorded whether or not a call runs: with next to no speech from it, this
    // was a meeting in a room, where the microphone hears everyone — no line is anyone's.
    let call_ms: u32 = lines
        .iter()
        .filter(|(.., speaker)| them(speaker))
        .map(|(s, ..)| s.end_ms.saturating_sub(s.start_ms))
        .sum();
    if call_ms < MIN_CALL_MS {
        lines.iter_mut().for_each(|(.., speaker)| *speaker = None);
    }
    lines.sort_by_key(|(s, ..)| s.start_ms);
    lines
}

/// What every transcript gets after Whisper: lines that only echo the prompt dropped, and the
/// vocabulary's spellings applied (D-036). Returns Whisper's text of each corrected line, for Undo.
pub(crate) fn polish(
    segments: &mut Vec<Segment>,
    prompt: &str,
    people: &[crate::modules::people::Person],
    terms: &[vocabulary::Term],
) -> Vec<Option<String>> {
    segments.retain(|s| !vocabulary::echoes_prompt(&s.text, prompt));
    let mut lines: Vec<String> = segments.iter().map(|s| s.text.clone()).collect();
    let raw = vocabulary::correct_lines(&mut lines, &vocabulary::replacements(people, terms));
    for (segment, line) in segments.iter_mut().zip(lines) {
        segment.text = line;
    }
    raw
}

/// Deletes a job's saved place (best effort: a stale one is checked before use anyway).
pub(super) fn forget(checkpoint: Option<&Path>) {
    if let Some(path) = checkpoint {
        if let Err(err) = std::fs::remove_file(path) {
            if err.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!("could not delete {}: {err}", path.display());
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn line(
        start_ms: u32,
        end_ms: u32,
        text: &str,
        speaker: Speaker,
    ) -> (Segment, Option<String>, Option<Speaker>) {
        let segment = Segment {
            start_ms,
            end_ms,
            text: text.into(),
            ..Default::default()
        };
        (segment, None, Some(speaker))
    }

    #[test]
    fn merge_drops_the_call_the_microphone_heard_and_keeps_both_voices() {
        let merged = merge_channels(vec![
            line(
                0,
                3_000,
                "Chào anh, hôm nay mình bàn về hợp đồng",
                Speaker::You,
            ),
            // The call through the speakers, picked up by the microphone.
            line(
                5_000,
                9_000,
                "We should sign the contract next week",
                Speaker::You,
            ),
            // The owner talking over them.
            line(6_000, 6_800, "Đúng rồi", Speaker::You),
            line(
                5_200,
                29_100,
                "We should sign the contract next week",
                Speaker::Them,
            ),
            line(4_000, 4_800, "Yes", Speaker::Them),
        ]);
        let said: Vec<(&str, Option<Speaker>)> = merged
            .iter()
            .map(|(s, _, speaker)| (s.text.as_str(), *speaker))
            .collect();
        assert_eq!(
            said,
            vec![
                ("Chào anh, hôm nay mình bàn về hợp đồng", Some(Speaker::You)),
                ("Yes", Some(Speaker::Them)),
                ("We should sign the contract next week", Some(Speaker::Them)),
                ("Đúng rồi", Some(Speaker::You)),
            ]
        );
    }

    #[test]
    fn a_room_meeting_has_no_speakers() {
        // The tap heard a notification, not a call: the microphone heard everyone in the room.
        let merged = merge_channels(vec![
            line(0, 3_000, "Anh Minh nói trước đi", Speaker::You),
            line(4_000, 5_000, "Thank you", Speaker::Them),
            line(6_000, 9_000, "Em xin trình bày kế hoạch", Speaker::You),
        ]);
        assert_eq!(merged.len(), 3);
        assert!(merged.iter().all(|(.., speaker)| speaker.is_none()));
    }
}
