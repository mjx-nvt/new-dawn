//! Model files: catalogue, download, verify, delete.
//!
//! The ONLY network code allowed outside localhost (CLAUDE.md rule 3, D-014): files are fetched
//! from huggingface.co (redirects may only go to its *.hf.co CDN) at a pinned commit, only when the
//! user clicks Download, and every file is SHA-256 verified before it can be used.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use specta::Type;
use tokio::io::AsyncWriteExt;

use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use crate::error::AppError;
use crate::events::{ModelDownloadFinished, ModelDownloadProgress};
use crate::kernel::hub::WorkspaceHub;

const WHISPER_REPO: &str = "ggerganov/whisper.cpp";
const WHISPER_COMMIT: &str = "5359861c739e955e79d9a303bcbc70fb988958b1";
/// Silero VAD ships inside the app (src-tauri/resources/models/, bundle.resources), not downloaded.
pub const BUNDLED_VAD: &str = "models/ggml-silero-v5.1.2.bin";

/// A download that delivers no bytes for this long is abandoned (the partial file is kept).
const STALL_TIMEOUT: Duration = Duration::from_secs(30);
const PROGRESS_EVERY: Duration = Duration::from_millis(250);
/// How often a waiting download checks its cancel flag.
const CANCEL_POLL: Duration = Duration::from_secs(1);

pub struct ModelSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub file: &'static str,
    repo: &'static str,
    commit: &'static str,
    pub size: u64,
    sha256: &'static str,
}

impl ModelSpec {
    fn url(&self) -> String {
        format!(
            "https://huggingface.co/{}/resolve/{}/{}",
            self.repo, self.commit, self.file
        )
    }
}

/// Whisper models. Sizes and hashes from the Hugging Face API at the pinned commit (2026-10-24).
pub const CATALOGUE: &[ModelSpec] = &[
    ModelSpec {
        id: "large-v3-turbo",
        label: "Whisper large-v3-turbo",
        description: "Twice as fast as large-v3; Live uses it first.",
        file: "ggml-large-v3-turbo.bin",
        repo: WHISPER_REPO,
        commit: WHISPER_COMMIT,
        size: 1_624_555_275,
        sha256: "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69",
    },
    ModelSpec {
        id: "large-v3-turbo-q5_0",
        label: "Whisper large-v3-turbo (q5_0)",
        description: "Quantised turbo: a third of the size, slightly less accurate.",
        file: "ggml-large-v3-turbo-q5_0.bin",
        repo: WHISPER_REPO,
        commit: WHISPER_COMMIT,
        size: 574_041_195,
        sha256: "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2",
    },
    ModelSpec {
        id: "large-v3",
        label: "Whisper large-v3",
        description: "Most accurate; meetings use it first. About twice as slow as turbo.",
        file: "ggml-large-v3.bin",
        repo: WHISPER_REPO,
        commit: WHISPER_COMMIT,
        size: 3_095_033_483,
        sha256: "64d182b440b98d5203c4f9bd541544d84c605196c4f7b845dfa11fb23594d1e2",
    },
];

pub fn spec(id: &str) -> Result<&'static ModelSpec> {
    Ok(CATALOGUE
        .iter()
        .find(|m| m.id == id)
        .ok_or_else(|| AppError::not_found(format!("Unknown model '{id}'")))?)
}

/// What a Whisper model is used for; each has its own model (Settings → Models, D-047).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SpeechJob {
    /// A meeting's transcript after Stop or an import: accuracy first.
    Meetings,
    /// A Live session: speed first.
    Live,
}

impl SpeechJob {
    /// Registry setting with the owner's choice (absent or empty = automatic).
    fn setting(self) -> &'static str {
        match self {
            Self::Meetings => "speech_model_meetings",
            Self::Live => "speech_model_live",
        }
    }

    /// The automatic choice, best first. large-v3 beat turbo 17–7 judged blind on typical
    /// stretches of three real meetings, at twice the time; Live needs the speed.
    fn preference(self) -> [&'static str; 3] {
        match self {
            Self::Meetings => ["large-v3", "large-v3-turbo", "large-v3-turbo-q5_0"],
            Self::Live => ["large-v3-turbo", "large-v3-turbo-q5_0", "large-v3"],
        }
    }
}

/// One job's model in Settings → Models: the owner's choice and what runs now.
#[derive(Debug, Clone, Serialize, Type)]
pub struct SpeechChoice {
    pub job: SpeechJob,
    /// None = automatic.
    pub chosen: Option<String>,
    /// What automatic picks now; None when no model is downloaded.
    pub automatic: Option<String>,
}

/// The model `job` uses now: the owner's choice when it's downloaded, else the job's automatic one.
pub async fn speech_model(
    app: &AppHandle,
    job: SpeechJob,
) -> Result<(&'static ModelSpec, PathBuf)> {
    let chosen = chosen_model(&app.state::<WorkspaceHub>(), job).await?;
    app.state::<ModelStore>()
        .speech_model(job, chosen.as_deref())
}

async fn chosen_model(hub: &WorkspaceHub, job: SpeechJob) -> Result<Option<String>> {
    Ok(hub
        .app_setting(job.setting())
        .await?
        .filter(|id| !id.is_empty()))
}

pub async fn speech_choices(hub: &WorkspaceHub, store: &ModelStore) -> Result<Vec<SpeechChoice>> {
    let mut out = Vec::new();
    for job in [SpeechJob::Meetings, SpeechJob::Live] {
        let automatic = store
            .speech_model(job, None)
            .ok()
            .map(|(spec, _)| spec.id.to_owned());
        out.push(SpeechChoice {
            job,
            chosen: chosen_model(hub, job).await?,
            automatic,
        });
    }
    Ok(out)
}

/// Saves the model for `job` (None = automatic).
pub async fn choose_speech_model(
    hub: &WorkspaceHub,
    job: SpeechJob,
    id: Option<&str>,
) -> Result<()> {
    if let Some(id) = id {
        spec(id)?;
    }
    hub.set_app_setting(job.setting(), id.unwrap_or_default())
        .await
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ModelState {
    Missing,
    /// A `.part` file from an interrupted or cancelled download; Download resumes it.
    Partial {
        bytes: u32,
    },
    Downloading,
    Ready,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct ModelInfo {
    pub id: String,
    pub label: String,
    pub description: String,
    // ponytail: u32 byte counts (max 4 GiB) keep the TS type a plain number; widen if a model outgrows it.
    pub size_bytes: u32,
    pub status: ModelState,
}

pub struct Progress {
    pub downloaded: u64,
    pub total: u64,
    pub bytes_per_sec: f64,
}

/// `<app_data_dir>/models/` plus the downloads in flight (id → cancel flag).
pub struct ModelStore {
    dir: PathBuf,
    active: Mutex<HashMap<&'static str, Arc<AtomicBool>>>,
}

impl ModelStore {
    pub fn new(dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        Ok(Self {
            dir,
            active: Mutex::default(),
        })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// `chosen` when it's downloaded, else the first downloaded model in the job's order.
    pub fn speech_model(
        &self,
        job: SpeechJob,
        chosen: Option<&str>,
    ) -> Result<(&'static ModelSpec, PathBuf)> {
        chosen
            .into_iter()
            .chain(job.preference())
            .filter_map(|id| spec(id).ok())
            .map(|spec| (spec, self.dir.join(spec.file)))
            .find(|(_, path)| path.exists())
            .ok_or_else(|| {
                AppError::new(
                    "model_missing",
                    "No Whisper model downloaded yet (Settings → Models)",
                )
                .into()
            })
    }

    pub fn list(&self) -> Vec<ModelInfo> {
        let active = self.active.lock().expect("model lock poisoned");
        CATALOGUE
            .iter()
            .map(|spec| {
                let status = if active.contains_key(spec.id) {
                    ModelState::Downloading
                } else if self.dir.join(spec.file).exists() {
                    ModelState::Ready
                } else {
                    match std::fs::metadata(self.part_path(spec)) {
                        Ok(meta) => ModelState::Partial {
                            bytes: meta.len() as u32,
                        },
                        Err(_) => ModelState::Missing,
                    }
                };
                ModelInfo {
                    id: spec.id.to_owned(),
                    label: spec.label.to_owned(),
                    description: spec.description.to_owned(),
                    size_bytes: spec.size as u32,
                    status,
                }
            })
            .collect()
    }

    /// Registers a download; errors if this model is already downloading. Returns its cancel flag.
    pub fn begin(&self, id: &str) -> Result<(&'static ModelSpec, Arc<AtomicBool>)> {
        let spec = spec(id)?;
        let mut active = self.active.lock().expect("model lock poisoned");
        if active.contains_key(spec.id) {
            bail!(AppError::invalid(format!(
                "{} is already downloading",
                spec.label
            )));
        }
        let cancel = Arc::new(AtomicBool::new(false));
        active.insert(spec.id, cancel.clone());
        Ok((spec, cancel))
    }

    pub fn finish(&self, id: &str) {
        self.active.lock().expect("model lock poisoned").remove(id);
    }

    pub fn cancel(&self, id: &str) {
        if let Some(flag) = self.active.lock().expect("model lock poisoned").get(id) {
            flag.store(true, Ordering::Relaxed);
        }
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        let spec = spec(id)?;
        if self
            .active
            .lock()
            .expect("model lock poisoned")
            .contains_key(spec.id)
        {
            bail!(AppError::invalid("Cancel the download before deleting"));
        }
        for path in [self.dir.join(spec.file), self.part_path(spec)] {
            if path.exists() {
                std::fs::remove_file(&path)
                    .with_context(|| format!("deleting {}", path.display()))?;
            }
        }
        Ok(())
    }

    fn part_path(&self, spec: &ModelSpec) -> PathBuf {
        self.dir.join(format!("{}.part", spec.file))
    }

    /// Downloads into `<file>.part` (resuming it), verifies size + SHA-256, then renames into place.
    /// Cancel and stalls keep the partial file; a hash mismatch deletes it.
    pub async fn download(
        &self,
        spec: &'static ModelSpec,
        cancel: &AtomicBool,
        mut on_progress: impl FnMut(Progress),
    ) -> Result<()> {
        let part = self.part_path(spec);
        let mut have = tokio::fs::metadata(&part)
            .await
            .map(|m| m.len())
            .unwrap_or(0);

        if have < spec.size {
            let client = reqwest::Client::builder()
                .connect_timeout(STALL_TIMEOUT)
                .read_timeout(STALL_TIMEOUT)
                .redirect(reqwest::redirect::Policy::custom(|attempt| {
                    let host = attempt.url().host_str().unwrap_or_default();
                    let allowed = ["huggingface.co", "hf.co"]
                        .iter()
                        .any(|h| host == *h || host.ends_with(&format!(".{h}")));
                    if allowed && attempt.previous().len() < 10 {
                        attempt.follow()
                    } else {
                        attempt.stop()
                    }
                }))
                .build()?;
            let mut request = client.get(spec.url());
            if have > 0 {
                request = request.header(reqwest::header::RANGE, format!("bytes={have}-"));
            }
            let response = request.send().await.context("contacting huggingface.co")?;
            let status = response.status();
            let append = match status.as_u16() {
                206 => true,
                // stop() on a disallowed redirect leaves us holding the 3xx itself
                300..=399 => bail!(AppError::new(
                    "download_failed",
                    "Download was redirected outside Hugging Face"
                )),
                200 => false, // server ignored the range: start over
                416 => true,  // nothing left to fetch; fall through to verification
                _ => bail!(AppError::new(
                    "download_failed",
                    format!("Download failed: HTTP {status}")
                )),
            };
            if !append {
                have = 0;
            }

            let mut file = tokio::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .append(append)
                .truncate(!append)
                .open(&part)
                .await
                .with_context(|| format!("opening {}", part.display()))?;

            if status.as_u16() != 416 {
                let mut stream = response.bytes_stream();
                let started = Instant::now();
                let start_bytes = have;
                let mut last_report = Instant::now() - PROGRESS_EVERY;
                let mut idle = Duration::ZERO;
                loop {
                    if cancel.load(Ordering::Relaxed) {
                        file.flush().await?;
                        bail!(AppError::new(
                            "cancelled",
                            "Download cancelled; it will resume next time"
                        ));
                    }
                    let chunk = match tokio::time::timeout(CANCEL_POLL, stream.next()).await {
                        Err(_) => {
                            idle += CANCEL_POLL;
                            if idle >= STALL_TIMEOUT {
                                file.flush().await?;
                                bail!(AppError::new(
                                    "download_stalled",
                                    "No data for 30 s; try again to resume"
                                ));
                            }
                            continue;
                        }
                        Ok(None) => break,
                        Ok(Some(chunk)) => chunk.context("reading the download")?,
                    };
                    idle = Duration::ZERO;
                    file.write_all(&chunk).await?;
                    have += chunk.len() as u64;
                    if last_report.elapsed() >= PROGRESS_EVERY {
                        let secs = started.elapsed().as_secs_f64().max(0.001);
                        on_progress(Progress {
                            downloaded: have,
                            total: spec.size,
                            bytes_per_sec: (have - start_bytes) as f64 / secs,
                        });
                        last_report = Instant::now();
                    }
                }
            }
            file.flush().await?;
            if have < spec.size {
                bail!(AppError::new(
                    "download_incomplete",
                    "The connection ended early; try again to resume"
                ));
            }
        }

        let path = part.clone();
        let expected = spec.sha256;
        let size = spec.size;
        let verified = tokio::task::spawn_blocking(move || verify(&path, size, expected)).await??;
        if !verified {
            tokio::fs::remove_file(&part).await.ok();
            bail!(AppError::new(
                "checksum_mismatch",
                "The downloaded file failed its SHA-256 check and was deleted"
            ));
        }
        tokio::fs::rename(&part, self.dir.join(spec.file)).await?;
        Ok(())
    }
}

/// Starts (or resumes) a download as a background task; the UI follows the progress events.
pub fn start_download(app: &AppHandle, id: &str) -> Result<()> {
    let (spec, cancel) = app.state::<ModelStore>().begin(id)?;
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let store = app.state::<ModelStore>();
        let result = store
            .download(spec, &cancel, |p| {
                let _ = ModelDownloadProgress {
                    id: spec.id.to_owned(),
                    downloaded: p.downloaded as u32,
                    total: p.total as u32,
                    bytes_per_sec: p.bytes_per_sec as u32,
                }
                .emit(&app);
            })
            .await;
        store.finish(spec.id);
        let error = result.err().map(AppError::from);
        let _ = ModelDownloadFinished {
            id: spec.id.to_owned(),
            error,
        }
        .emit(&app);
    });
    Ok(())
}

/// True when the file has exactly `size` bytes and the expected SHA-256.
fn verify(path: &Path, size: u64, sha256_hex: &str) -> Result<bool> {
    if std::fs::metadata(path)?.len() != size {
        return Ok(false);
    }
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let digest: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Ok(digest == sha256_hex)
}

#[cfg(test)]
mod tests {
    use super::{CATALOGUE, ModelStore, SpeechJob, spec, verify};

    /// Meetings take the most accurate downloaded model, Live the fastest; a choice wins only
    /// while its file is there.
    #[test]
    fn each_job_takes_its_own_model() {
        let dir = std::env::temp_dir().join(format!("newdawn-models-{}", uuid::Uuid::now_v7()));
        let store = ModelStore::new(dir.clone()).unwrap();
        let pick = |job, chosen| {
            store
                .speech_model(job, chosen)
                .map(|(spec, _)| spec.id)
                .ok()
        };
        assert_eq!(pick(SpeechJob::Meetings, None), None);

        std::fs::write(dir.join(spec("large-v3-turbo").unwrap().file), b"").unwrap();
        assert_eq!(pick(SpeechJob::Meetings, None), Some("large-v3-turbo"));
        assert_eq!(pick(SpeechJob::Live, None), Some("large-v3-turbo"));

        std::fs::write(dir.join(spec("large-v3").unwrap().file), b"").unwrap();
        assert_eq!(pick(SpeechJob::Meetings, None), Some("large-v3"));
        assert_eq!(pick(SpeechJob::Live, None), Some("large-v3-turbo"));
        assert_eq!(pick(SpeechJob::Live, Some("large-v3")), Some("large-v3"));
        // Chosen but not downloaded: the job's automatic model runs.
        assert_eq!(
            pick(SpeechJob::Live, Some("large-v3-turbo-q5_0")),
            Some("large-v3-turbo")
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// Byte counts cross the bridge as u32 (see ModelInfo); every catalogue file must fit.
    #[test]
    fn catalogue_sizes_fit_u32() {
        for model in CATALOGUE {
            assert!(
                model.size <= u64::from(u32::MAX),
                "{} is too large for u32",
                model.id
            );
        }
    }

    #[test]
    fn verify_checks_size_and_hash() {
        let path = std::env::temp_dir().join(format!("newdawn-verify-{}", uuid::Uuid::now_v7()));
        std::fs::write(&path, b"abc").unwrap();
        let abc = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert!(verify(&path, 3, abc).unwrap());
        assert!(!verify(&path, 4, abc).unwrap());
        assert!(!verify(&path, 3, &abc.replace('b', "c")).unwrap());
        std::fs::remove_file(path).unwrap();
    }

    /// The bundled VAD model is the pinned file, byte for byte.
    #[test]
    fn bundled_vad_matches_pin() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join(super::BUNDLED_VAD);
        assert!(
            verify(
                &path,
                885_098,
                "29940d98d42b91fbd05ce489f3ecf7c72f0a42f027e4875919a28fb4c04ea2cf"
            )
            .unwrap()
        );
    }

    /// Network test (run with `cargo test -- --ignored`): download a small pinned file (the VAD
    /// model), cut it short to simulate an interrupted download, then resume it with a Range request.
    #[test]
    #[ignore]
    fn download_resume_and_verify() {
        use super::{ModelSpec, ModelStore};
        use std::sync::atomic::AtomicBool;

        static VAD: ModelSpec = ModelSpec {
            id: "test-vad",
            label: "Silero VAD (test)",
            description: "",
            file: "ggml-silero-v5.1.2.bin",
            repo: "ggml-org/whisper-vad",
            commit: "9ffd54a1e1ee413ddf265af9913beaf518d1639b",
            size: 885_098,
            sha256: "29940d98d42b91fbd05ce489f3ecf7c72f0a42f027e4875919a28fb4c04ea2cf",
        };
        let dir = std::env::temp_dir().join(format!("newdawn-models-{}", uuid::Uuid::now_v7()));
        let store = ModelStore::new(dir.clone()).unwrap();
        let vad = &VAD;
        let cancel = AtomicBool::new(false);
        tauri::async_runtime::block_on(async {
            store.download(vad, &cancel, |_| {}).await.unwrap();
            let done = dir.join(vad.file);
            let part = dir.join(format!("{}.part", vad.file));
            std::fs::rename(&done, &part).unwrap();
            std::fs::OpenOptions::new()
                .write(true)
                .open(&part)
                .unwrap()
                .set_len(400_000)
                .unwrap();

            store.download(vad, &cancel, |_| {}).await.unwrap();
            assert_eq!(std::fs::metadata(&done).unwrap().len(), vad.size);
            assert!(!part.exists());
        });
        std::fs::remove_dir_all(dir).unwrap();
    }
}
