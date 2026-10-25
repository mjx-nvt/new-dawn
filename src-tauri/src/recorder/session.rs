//! Recording session: Idle → Recording ⇄ Paused → Stopping → Idle.
//! A capture thread owns the (non-Send) mic stream and the system tap; a mixer thread writes the
//! WAV. Start creates the meeting (`recording`); Stop finalises the file, marks it `recorded` and
//! queues transcription.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use super::mixer::{self, Shared, Source};
use super::saver::{self, WavWriter};
use super::{mic, system_tap};
use crate::error::AppError;
use crate::events::{RecorderLevel, RecorderStatusChanged};
use crate::kernel::ai::whisper::Language;
use crate::kernel::hub::WorkspaceHub;
use crate::kernel::notify;
use crate::kernel::permissions::{self, PermissionState};
use crate::modules::meetings::{self, Meeting, MeetingStatus, NewMeeting, transcribe};
use sqlx::SqlitePool;

/// Bounded channel size per source: 50 × ~100 ms = 5 s of slack before chunks are dropped.
const CHANNEL_CAP: usize = 50;
/// Stop gives the capture thread this long to finalise the WAV before falling back to repair.
const STOP_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum RecorderPhase {
    Idle,
    Recording,
    Paused,
    Stopping,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SystemAudio {
    /// The tap is delivering audio (or the call is simply quiet).
    Ok,
    /// Exact silence for 3 s while the mic hears speech: System Audio Recording is not allowed.
    Silent,
    /// The tap could not be created (older macOS or a Core Audio error): mic only.
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct RecorderStatus {
    pub phase: RecorderPhase,
    pub workspace_slug: Option<String>,
    pub meeting_id: Option<String>,
    pub elapsed_ms: u32,
    pub system_audio: SystemAudio,
}

pub struct StartRequest {
    pub slug: String,
    pub title: String,
    pub language: Language,
    pub meeting_type_id: Option<String>,
    /// A presentation by one main presenter (else a discussion), D-038.
    pub presenter: Option<String>,
    pub presentation: bool,
}

struct Active {
    slug: String,
    meeting_id: String,
    shared: Arc<Shared>,
    stop: Sender<()>,
    capture: Option<JoinHandle<Result<u64>>>,
    started: Instant,
    paused_since: Option<Instant>,
    paused_total: Duration,
    tap_available: bool,
    stopping: bool,
}

impl Active {
    fn elapsed(&self) -> Duration {
        let paused_now = self.paused_since.map(|t| t.elapsed()).unwrap_or_default();
        self.started
            .elapsed()
            .saturating_sub(self.paused_total + paused_now)
    }

    fn status(&self) -> RecorderStatus {
        let phase = if self.stopping {
            RecorderPhase::Stopping
        } else if self.paused_since.is_some() {
            RecorderPhase::Paused
        } else {
            RecorderPhase::Recording
        };
        let system_audio = if !self.tap_available {
            SystemAudio::Unavailable
        } else if self.shared.system_silent.load(Ordering::Relaxed) {
            SystemAudio::Silent
        } else {
            SystemAudio::Ok
        };
        RecorderStatus {
            phase,
            workspace_slug: Some(self.slug.clone()),
            meeting_id: Some(self.meeting_id.clone()),
            elapsed_ms: self.elapsed().as_millis() as u32,
            system_audio,
        }
    }
}

#[derive(Default)]
pub struct Recorder {
    active: Mutex<Option<Active>>,
    /// Claimed for the whole of `start` (which awaits), so two starts can't both succeed.
    starting: AtomicBool,
}

/// Releases `Recorder::starting` on every exit from `start`.
struct StartClaim<'a>(&'a AtomicBool);

impl Drop for StartClaim<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

impl Recorder {
    fn lock(&self) -> std::sync::MutexGuard<'_, Option<Active>> {
        self.active.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn status(&self) -> RecorderStatus {
        self.lock()
            .as_ref()
            .map(Active::status)
            .unwrap_or(RecorderStatus {
                phase: RecorderPhase::Idle,
                workspace_slug: None,
                meeting_id: None,
                elapsed_ms: 0,
                system_audio: SystemAudio::Ok,
            })
    }

    /// App exit: stop capture and finalise the WAV; the meeting is settled at next launch.
    pub fn shutdown(&self) {
        let active = self.lock().take();
        if let Some(mut active) = active {
            let _ = active.stop.send(());
            if let Some(handle) = active.capture.take() {
                let _ = handle.join();
            }
        }
    }
}

pub async fn start(app: &AppHandle, request: StartRequest) -> Result<Meeting> {
    let recorder = app.state::<Recorder>();
    if recorder.lock().is_some() || recorder.starting.swap(true, Ordering::AcqRel) {
        bail!(AppError::invalid("A recording is already running"));
    }
    let _claim = StartClaim(&recorder.starting);
    let title = request.title.trim().to_owned();
    if title.is_empty() {
        bail!(AppError::invalid("Title is required"));
    }
    match permissions::request_microphone().await? {
        PermissionState::Granted => {}
        _ => bail!(AppError::new(
            "microphone_denied",
            "New Dawn needs microphone access (System Settings → Privacy & Security → Microphone)"
        )),
    }

    let hub = app.state::<WorkspaceHub>();
    let workspace = hub.workspace(&request.slug).await?;
    let pool = hub.pool(&request.slug)?;
    let id = meetings::new_id();
    let folder = Path::new(&workspace.folder_path)
        .join("recordings")
        .join(&id);
    tokio::fs::create_dir_all(&folder)
        .await
        .with_context(|| format!("creating {}", folder.display()))?;
    let path = folder.join("audio.wav");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let mut meeting = meetings::create(
        &pool,
        NewMeeting {
            id: &id,
            meeting_type_id: request.meeting_type_id.as_deref(),
            title: &title,
            held_at_unix: now,
            language: request.language,
            audio_path: &path.to_string_lossy(),
            status: MeetingStatus::Recording,
        },
    )
    .await?;
    if request.presentation {
        meeting = meetings::set_format(
            &pool,
            &id,
            meetings::MeetingFormat::Presentation,
            request.presenter.as_deref(),
        )
        .await?;
    }

    let shared = Arc::new(Shared::default());
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let (ready_tx, ready_rx) = mpsc::channel::<Result<bool>>();
    let capture = spawn_capture(path, shared.clone(), stop_rx, ready_tx);
    let ready = tokio::task::spawn_blocking(move || {
        ready_rx
            .recv()
            .map_err(|_| anyhow!("capture thread exited"))
    })
    .await
    .map_err(anyhow::Error::from)
    .and_then(|r| r)
    .and_then(|r| r);
    let tap_available = match ready {
        Ok(tap) => tap,
        Err(err) => {
            let _ = tokio::task::spawn_blocking(move || capture.join()).await;
            meetings::delete(&pool, &id).await.ok();
            tokio::fs::remove_dir_all(&folder).await.ok();
            return Err(err);
        }
    };

    let active = Active {
        slug: request.slug,
        meeting_id: id,
        shared,
        stop: stop_tx,
        capture: Some(capture),
        started: Instant::now(),
        paused_since: None,
        paused_total: Duration::ZERO,
        tap_available,
        stopping: false,
    };
    let status = active.status();
    *app.state::<Recorder>().lock() = Some(active);
    let _ = RecorderStatusChanged(status).emit(app);
    spawn_meter(app.clone());
    notify::show(app, "Recording started", &meeting.title);
    Ok(meeting)
}

pub fn pause(app: &AppHandle, paused: bool) -> Result<RecorderStatus> {
    let recorder = app.state::<Recorder>();
    let mut guard = recorder.lock();
    let active = guard
        .as_mut()
        .ok_or_else(|| AppError::invalid("Nothing is recording"))?;
    match (paused, active.paused_since) {
        (true, None) => active.paused_since = Some(Instant::now()),
        (false, Some(since)) => {
            active.paused_total += since.elapsed();
            active.paused_since = None;
        }
        _ => {}
    }
    active.shared.paused.store(paused, Ordering::Relaxed);
    let status = active.status();
    drop(guard);
    let _ = RecorderStatusChanged(status.clone()).emit(app);
    Ok(status)
}

pub async fn stop(app: &AppHandle) -> Result<Meeting> {
    let (slug, meeting_id, capture, stop) = {
        let recorder = app.state::<Recorder>();
        let mut guard = recorder.lock();
        let active = guard
            .as_mut()
            .ok_or_else(|| AppError::invalid("Nothing is recording"))?;
        if active.stopping {
            bail!(AppError::invalid("Already stopping"));
        }
        active.stopping = true;
        let _ = RecorderStatusChanged(active.status()).emit(app);
        (
            active.slug.clone(),
            active.meeting_id.clone(),
            active.capture.take(),
            active.stop.clone(),
        )
    };
    let _ = stop.send(());
    let joined: Result<u64> = match capture {
        Some(handle) => match tokio::time::timeout(
            STOP_TIMEOUT,
            tokio::task::spawn_blocking(move || handle.join()),
        )
        .await
        {
            Ok(Ok(Ok(result))) => result,
            Ok(Ok(Err(_))) => Err(anyhow!("capture thread panicked")),
            Ok(Err(err)) => Err(err.into()),
            Err(_) => Err(anyhow!("capture did not stop within {STOP_TIMEOUT:?}")),
        },
        None => Ok(0),
    };
    // Whatever happened above, the session is over.
    app.state::<Recorder>().lock().take();
    let _ = RecorderStatusChanged(app.state::<Recorder>().status()).emit(app);

    let pool = app.state::<WorkspaceHub>().pool(&slug)?;
    let samples = match joined {
        Ok(samples) => samples,
        Err(err) => {
            // The WAV checkpoints every 2 s, so repair what is on disk rather than lose the call.
            tracing::warn!("recorder ended with an error: {err:#}");
            let meeting = meetings::get(&pool, &meeting_id).await?;
            match repair_file(meeting.audio_path).await {
                Ok(samples) => samples,
                Err(repair_err) => {
                    meetings::set_status(&pool, &meeting_id, MeetingStatus::Failed)
                        .await
                        .ok();
                    return Err(repair_err);
                }
            }
        }
    };
    let duration_ms = saver::duration_ms(samples);
    meetings::finish_recording(&pool, &meeting_id, duration_ms).await?;
    let queued = transcribe::start(app, &slug, &meeting_id).await;
    if let Err(err) = &queued {
        tracing::warn!("could not queue transcription: {err:#}");
    }
    let meeting = meetings::get(&pool, &meeting_id).await?;
    let next = if queued.is_ok() {
        " — transcribing"
    } else {
        ""
    };
    notify::show(
        app,
        "Recording stopped",
        &format!("{} · {}{next}", meeting.title, notify::clock(duration_ms)),
    );
    Ok(meeting)
}

/// Recordings cut short by a quit or crash: repair the WAV from its length and mark them
/// `recorded` (at most ~2 s lost); `failed` when nothing usable is left. Runs at launch.
pub async fn recover(pool: &SqlitePool) -> Result<()> {
    for (id, path) in meetings::interrupted_recordings(pool).await? {
        match repair_file(path).await {
            Ok(samples) if samples > 0 => {
                meetings::finish_recording(pool, &id, saver::duration_ms(samples)).await?
            }
            other => {
                tracing::warn!("recording {id} could not be recovered: {:?}", other.err());
                meetings::set_status(pool, &id, MeetingStatus::Failed).await?;
            }
        }
    }
    Ok(())
}

async fn repair_file(path: Option<String>) -> Result<u64> {
    let path = path.context("meeting has no audio file")?;
    tokio::task::spawn_blocking(move || saver::repair(Path::new(&path))).await?
}

/// Capture thread: owns the mic stream and the tap (neither is Send), runs the mixer, and keeps
/// both alive until `stop`. Reports `Ok(tap_available)` or the start error through `ready`.
fn spawn_capture(
    path: PathBuf,
    shared: Arc<Shared>,
    stop: mpsc::Receiver<()>,
    ready: Sender<Result<bool>>,
) -> JoinHandle<Result<u64>> {
    std::thread::Builder::new()
        .name("recorder-capture".into())
        .spawn(move || -> Result<u64> {
            let (mic_tx, mic_rx) = mpsc::sync_channel(CHANNEL_CAP);
            let (sys_tx, sys_rx) = mpsc::sync_channel(CHANNEL_CAP);
            let setup = (|| -> Result<_> {
                let mic = mic::start(mic_tx, shared.clone())?;
                let wav = WavWriter::create(&path)?;
                Ok((mic, wav))
            })();
            let (mic, wav) = match setup {
                Ok(parts) => parts,
                Err(err) => {
                    let _ = ready.send(Err(err));
                    return Ok(0);
                }
            };
            let tap = match system_tap::start(sys_tx, shared.clone()) {
                Ok(tap) => Some(tap),
                Err(err) => {
                    tracing::warn!(
                        "system audio unavailable, recording the microphone only: {err:#}"
                    );
                    None
                }
            };
            let _ = ready.send(Ok(tap.is_some()));

            let mic_source = Source {
                rx: mic_rx,
                rate: mic.sample_rate,
            };
            let sys_source = tap.as_ref().map(|t| Source {
                rx: sys_rx,
                rate: t.sample_rate,
            });
            let mixer_shared = shared.clone();
            let mixer = std::thread::Builder::new()
                .name("recorder-mixer".into())
                .spawn(move || mixer::run(mic_source, sys_source, mixer_shared, wav))?;

            let _ = stop.recv(); // stop requested (or the session vanished)
            drop(tap); // closes the system channel
            drop(mic); // closes the mic channel → the mixer drains, finalises the WAV and returns
            let dropped = shared.dropped.load(Ordering::Relaxed);
            if dropped > 0 {
                tracing::warn!("recorder dropped {dropped} audio chunks (mixer fell behind)");
            }
            mixer.join().map_err(|_| anyhow!("mixer thread panicked"))?
        })
        .expect("spawning the capture thread")
}

/// Emits `RecorderLevel` at 10 Hz while a session is active (also re-emits status when the
/// system-audio state changes).
fn spawn_meter(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last_system = None;
        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let snapshot = {
                let recorder = app.state::<Recorder>();
                let guard = recorder.lock();
                guard.as_ref().map(|a| (a.shared.levels(), a.status()))
            };
            let Some(((you, them), status)) = snapshot else {
                break;
            };
            let _ = RecorderLevel {
                you,
                them,
                elapsed_ms: status.elapsed_ms,
            }
            .emit(&app);
            if last_system != Some(status.system_audio) {
                last_system = Some(status.system_audio);
                let _ = RecorderStatusChanged(status).emit(&app);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;
    use std::sync::{Arc, mpsc};
    use std::time::Duration;

    use super::{Shared, spawn_capture};
    use crate::kernel::permissions;
    use crate::modules::meetings::audio::load_16k_mono;

    /// Live devices (run by hand: `cargo test live_capture -- --ignored --nocapture`): records
    /// ~3 s of mic + system tap while `say` plays through the speakers, then checks the WAV.
    #[test]
    #[ignore]
    fn live_capture() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter("info")
            .with_test_writer()
            .try_init();
        eprintln!(
            "microphone permission: {:?}",
            permissions::microphone().unwrap()
        );
        let path = std::env::temp_dir().join(format!("newdawn-live-{}.wav", uuid::Uuid::now_v7()));
        let shared = Arc::new(Shared::default());
        let (stop_tx, stop_rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let capture = spawn_capture(path.clone(), shared.clone(), stop_rx, ready_tx);
        let tap = ready_rx.recv().unwrap().expect("capture should start");
        eprintln!("system tap available: {tap}");

        let mut speaker = std::process::Command::new("say")
            .arg("New Dawn recorder test")
            .spawn()
            .unwrap();
        let mut peak_them = 0.0f32;
        let mut peak_you = 0.0f32;
        for _ in 0..30 {
            std::thread::sleep(Duration::from_millis(100));
            let (you, them) = shared.levels();
            peak_you = peak_you.max(you);
            peak_them = peak_them.max(them);
        }
        speaker.wait().unwrap();
        stop_tx.send(()).unwrap();
        let samples = capture.join().unwrap().unwrap();
        let decoded = load_16k_mono(&path).unwrap();
        eprintln!(
            "samples={samples} ({:.2}s) decoded={} peak_you={peak_you:.4} peak_them={peak_them:.4} silent_flag={} dropped={}",
            samples as f32 / 16_000.0,
            decoded.len(),
            shared.system_silent.load(Ordering::Relaxed),
            shared.dropped.load(Ordering::Relaxed)
        );
        assert!(samples > 16_000 * 2, "expected ~3 s of audio");
        assert_eq!(decoded.len() as u64, samples);
        std::fs::remove_file(path).unwrap();
    }
}
