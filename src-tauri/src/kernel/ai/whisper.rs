//! Speech → text with whisper.cpp (whisper-rs, Metal). One model in memory, loaded on first use
//! and dropped after an idle period; one `WhisperState` per job (per utterance in a live session,
//! `live.rs`); whisper.cpp's built-in Silero VAD.

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager};
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
    WhisperVadContext, WhisperVadContextParams, WhisperVadParams,
};

use crate::error::AppError;
use crate::kernel::ai::log::{self as ai_log, AiLogEntry};
use crate::kernel::ai::models::{BUNDLED_VAD, ModelStore};
use crate::kernel::ai::queue::AiQueue;

pub const SAMPLE_RATE: usize = 16_000;
/// The model is dropped after this long without a job; adjustable in Settings → Models.
pub const DEFAULT_IDLE_UNLOAD_SECS: u64 = 120;
/// Registry setting key for the idle period (seconds).
pub const IDLE_UNLOAD_SETTING: &str = "whisper_idle_unload_secs";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum Language {
    En,
    Vi,
    Auto,
}

impl Language {
    fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Vi => "vi",
            Self::Auto => "auto", // whisper.cpp detects the language
        }
    }
}

/// Where a transcription job is; sent to the UI with a 0–100 percent.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Queued,
    Decoding,
    DetectingSpeech,
    Transcribing,
    Saving,
}

pub type Progress = Arc<dyn Fn(Stage, u32) + Send + Sync>;

/// What a feature asks the gateway to transcribe.
pub struct TranscribeJob<'a> {
    pub meeting_id: &'a str,
    pub language: Language,
    pub cancel: Arc<AtomicBool>,
    /// Names and terms that bias Whisper's spelling (the workspace vocabulary, D-036).
    pub prompt: Option<String>,
}

pub struct Transcript {
    pub model: &'static str,
    pub segments: Vec<Segment>,
    pub duration_ms: u32,
}

/// The speech gateway (CLAUDE.md rule 2): waits for its turn in the AI queue, runs `prepare`
/// (audio → 16 kHz mono) and Whisper off the async runtime, and writes one `ai_log` row.
pub async fn transcribe(
    app: &AppHandle,
    pool: &SqlitePool,
    job: TranscribeJob<'_>,
    prepare: impl FnOnce() -> Result<Vec<f32>> + Send + 'static,
    progress: Progress,
) -> Result<Transcript> {
    progress(Stage::Queued, 0);
    let queue = app.state::<AiQueue>();
    let _turn = queue.acquire().await;
    if job.cancel.load(Ordering::Relaxed) {
        bail!(AppError::new("cancelled", "Transcription cancelled"));
    }
    // Never two models in memory: drop Qwen from Ollama before Whisper loads (best effort).
    super::ollama::unload().await;
    let (model, model_path) = app.state::<ModelStore>().preferred_speech_model()?;
    let vad_path = app.path().resolve(BUNDLED_VAD, BaseDirectory::Resource)?;
    let started = Instant::now();

    let (app_for_job, cancel, language) = (app.clone(), job.cancel.clone(), job.language);
    let prompt = job.prompt.clone().filter(|p| !p.trim().is_empty());
    let result = tokio::task::spawn_blocking(move || -> Result<Transcript> {
        progress(Stage::Decoding, 5);
        let samples = prepare()?;
        if cancel.load(Ordering::Relaxed) {
            bail!(AppError::new("cancelled", "Transcription cancelled"));
        }
        progress(Stage::DetectingSpeech, 15);
        let engine = app_for_job.state::<WhisperEngine>();
        let speech_model = SpeechModel {
            id: model.id,
            path: &model_path,
        };
        let segments = engine.transcribe(
            speech_model,
            &vad_path,
            &samples,
            language,
            prompt.as_deref(),
            &cancel,
            |done| progress(Stage::Transcribing, 20 + (done * 75.0) as u32),
        )?;
        Ok(Transcript {
            model: model.id,
            segments,
            duration_ms: (samples.len() * 1000 / SAMPLE_RATE) as u32,
        })
    })
    .await
    .map_err(anyhow::Error::from)
    .and_then(|r| r);

    let (status, error) = match &result {
        Ok(_) => ("ok", None),
        Err(err) if is_cancelled(err) => ("cancelled", None),
        Err(err) => ("error", Some(format!("{err:#}"))),
    };
    let entry = AiLogEntry {
        kind: "transcribe",
        agent: None,
        model: model.id,
        meeting_id: Some(job.meeting_id),
        duration_ms: started.elapsed().as_millis() as u32,
        status,
        error,
        prompt_tokens: None,
        output_tokens: None,
    };
    if let Err(err) = ai_log::record(pool, entry).await {
        tracing::warn!("could not write ai_log: {err:#}");
    }
    result
}

/// Saves the idle period in `registry.db` and applies it to the running engine.
pub async fn set_idle_unload(
    hub: &crate::kernel::hub::WorkspaceHub,
    engine: &WhisperEngine,
    secs: u32,
) -> Result<()> {
    if !(30..=3600).contains(&secs) {
        bail!(AppError::invalid("Choose between 30 seconds and 1 hour"));
    }
    hub.set_app_setting(IDLE_UNLOAD_SETTING, &secs.to_string())
        .await?;
    engine.set_idle_unload_secs(secs.into());
    Ok(())
}

pub fn is_cancelled(err: &anyhow::Error) -> bool {
    err.downcast_ref::<AppError>()
        .is_some_and(|e| e.code == "cancelled")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub start_ms: u32,
    pub end_ms: u32,
    pub text: String,
    /// The language it was written in (Vietnamese or English); None when not known.
    #[serde(default)]
    pub lang: Option<Language>,
}

/// Whisper's odds for one of the two languages at least this share of their sum: written in
/// that language only; below, written both ways (D-046).
const SURE: f32 = 0.85;
/// Written both ways, the other language replaces the likelier one only when Whisper is clearly
/// surer of it (mean log-probability): a translation can score within a hair of what was said.
const CLEARLY: f32 = 0.03;

/// Which languages to write a stretch of speech in, from Whisper's odds that it is Vietnamese and
/// English: the likelier one, and the other too when it's a close call (D-046).
fn candidates(vi: f32, en: f32) -> Vec<Language> {
    let (first, second) = if vi >= en {
        (Language::Vi, Language::En)
    } else {
        (Language::En, Language::Vi)
    };
    if vi.max(en) >= SURE * (vi + en) {
        vec![first]
    } else {
        vec![first, second]
    }
}

/// How likely `samples` are Vietnamese and English, by Whisper's language detection (one encoder
/// pass over the first 30 s).
fn vi_en_odds(state: &mut WhisperState, samples: &[f32]) -> Result<(f32, f32)> {
    let threads = threads() as usize;
    state
        .pcm_to_mel(samples, threads)
        .context("reading the speech")?;
    let (_, probs) = state
        .lang_detect(0, threads)
        .context("detecting the language")?;
    let odds = |language: Language| {
        whisper_rs::get_lang_id(language.code())
            .and_then(|id| probs.get(id as usize).copied())
            .unwrap_or(0.0)
    };
    Ok((odds(Language::Vi), odds(Language::En)))
}

/// Which downloaded Whisper model to use.
#[derive(Clone, Copy)]
pub(in crate::kernel::ai) struct SpeechModel<'a> {
    pub id: &'a str,
    pub path: &'a Path,
}

struct Loaded {
    model_id: String,
    ctx: WhisperContext,
    last_used: Instant,
}

pub struct WhisperEngine {
    loaded: Mutex<Option<Loaded>>,
    idle_unload_secs: AtomicU64,
}

impl WhisperEngine {
    pub fn new(idle_unload_secs: u64) -> Self {
        whisper_rs::install_logging_hooks(); // whisper.cpp logs → tracing
        Self {
            loaded: Mutex::new(None),
            idle_unload_secs: AtomicU64::new(idle_unload_secs),
        }
    }

    /// Drops the model now if no job is using it (before the text model runs).
    pub fn unload_now(&self) {
        if let Ok(mut loaded) = self.loaded.try_lock() {
            if let Some(model) = loaded.take() {
                tracing::info!(
                    "whisper model {} unloaded for the text model",
                    model.model_id
                );
            }
        }
    }

    pub fn idle_unload_secs(&self) -> u64 {
        self.idle_unload_secs.load(Ordering::Relaxed)
    }

    pub fn set_idle_unload_secs(&self, secs: u64) {
        self.idle_unload_secs.store(secs, Ordering::Relaxed);
    }

    /// Drops the model if no job has used it for the idle period. Never waits on a running job.
    pub fn unload_if_idle(&self) {
        let mut loaded = match self.loaded.try_lock() {
            Ok(guard) => guard,
            Err(std::sync::TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => return,
        };
        let idle = Duration::from_secs(self.idle_unload_secs());
        if loaded
            .as_ref()
            .is_some_and(|l| l.last_used.elapsed() >= idle)
        {
            let model = loaded.take().map(|l| l.model_id).unwrap_or_default();
            tracing::info!("whisper model {model} unloaded after {idle:?} idle");
        }
    }

    /// The engine, locked, with `model` loaded (a different one is freed first). Blocking.
    fn with_model(&self, model: SpeechModel<'_>) -> Result<MutexGuard<'_, Option<Loaded>>> {
        let mut guard = self.loaded.lock().unwrap_or_else(PoisonError::into_inner);
        if guard.as_ref().is_none_or(|l| l.model_id != model.id) {
            *guard = None; // free the old model before loading the new one
            let params = WhisperContextParameters {
                use_gpu: true,
                flash_attn: true,
                ..Default::default()
            };
            let path = model.path.to_str().context("model path is not UTF-8")?;
            let ctx = WhisperContext::new_with_params(path, params)
                .context("loading the Whisper model")?;
            tracing::info!("whisper model {} loaded", model.id);
            *guard = Some(Loaded {
                model_id: model.id.to_owned(),
                ctx,
                last_used: Instant::now(),
            });
        }
        Ok(guard)
    }

    /// Loads `model` now, so a live session's first line doesn't wait for it (D-041). Blocking;
    /// takes no queue turn (the live worker holds it).
    pub(in crate::kernel::ai) fn load(&self, model: SpeechModel<'_>) -> Result<()> {
        if let Some(loaded) = self.with_model(model)?.as_mut() {
            loaded.last_used = Instant::now();
        }
        Ok(())
    }

    /// One utterance of a live session (D-041) in `language` (Vietnamese unless English): no VAD
    /// (the live segmenter cut it), the session's `LiveDecoder` settings, a fresh state per call,
    /// language-safe cleanup, and how sure Whisper was. Blocking; takes no queue turn. None when
    /// nothing real was said.
    pub(in crate::kernel::ai) fn transcribe_utterance(
        &self,
        model: SpeechModel<'_>,
        decoder: &LiveDecoder,
        samples: &[f32],
        language: Language,
    ) -> Result<Option<Heard>> {
        let mut guard = self.with_model(model)?;
        let loaded = guard.as_mut().expect("model loaded above");
        let eot = loaded.ctx.token_eot();
        let mut state = loaded
            .ctx
            .create_state()
            .context("creating a Whisper state")?;
        let params = match language {
            Language::En => &decoder.en,
            _ => &decoder.vi,
        };
        if let Err(err) = state.full(params.clone(), samples) {
            if decoder.cancel.load(Ordering::Relaxed) {
                bail!(AppError::new("cancelled", "Live transcription stopped"));
            }
            return Err(err).context("running Whisper");
        }
        let (mut raw, mut tokens) = (String::new(), Vec::new());
        for segment in state.as_iter() {
            raw.push_str(&segment.to_str_lossy()?);
            for i in 0..segment.n_tokens() {
                let token = segment.get_token(i).context("reading a token")?;
                let data = token.token_data();
                let bytes = (data.id < eot)
                    .then(|| token.to_bytes().map(<[u8]>::to_vec))
                    .transpose()?;
                tokens.push(Tok {
                    bytes,
                    p: data.p,
                    plog: data.plog,
                });
            }
        }
        loaded.last_used = Instant::now();
        let Some(text) = clean_text(&raw) else {
            return Ok(None);
        };
        let (confidence, avg_logprob, ended) = score(&tokens);
        // Shorter than what Whisper wrote: `collapse_repeats` cut a loop.
        let runaway = !ended || text.len() < raw.trim().len();
        Ok(Some(Heard {
            text,
            confidence,
            avg_logprob,
            runaway,
        }))
    }

    /// How likely an utterance is Vietnamese and English by Whisper's language detection — an extra
    /// encoder pass (about a second), so only for a line in doubt. Blocking; takes no queue turn.
    pub(in crate::kernel::ai) fn language_odds(
        &self,
        model: SpeechModel<'_>,
        samples: &[f32],
    ) -> Result<(f32, f32)> {
        let mut guard = self.with_model(model)?;
        let loaded = guard.as_mut().expect("model loaded above");
        let mut state = loaded
            .ctx
            .create_state()
            .context("creating a Whisper state")?;
        let odds = vi_en_odds(&mut state, samples)?;
        loaded.last_used = Instant::now();
        Ok(odds)
    }

    /// VAD → Whisper on each speech segment → language-safe cleanup. Blocking. Features call the
    /// `transcribe()` gateway above instead, which adds the queue and `ai_log`.
    /// `progress` gets 0.0–1.0; `cancel` is checked between and inside segments.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::kernel::ai) fn transcribe(
        &self,
        model: SpeechModel<'_>,
        vad_model: &Path,
        samples: &[f32],
        language: Language,
        prompt: Option<&str>,
        cancel: &Arc<AtomicBool>,
        mut progress: impl FnMut(f32),
    ) -> Result<Vec<Segment>> {
        let prompt = prompt.map(|p| p.replace('\0', " "));
        let threads = threads();
        let speech = detect_speech(vad_model, samples)?;

        let mut guard = self.with_model(model)?;
        let loaded = guard.as_mut().expect("model loaded above");
        let mut state = loaded
            .ctx
            .create_state()
            .context("creating a Whisper state")?;

        let eot = loaded.ctx.token_eot();
        // One stretch of speech written in `lang`: its lines, and how sure Whisper was (the mean
        // log-probability of its words; a translation scores lower than what was said).
        let decode = |state: &mut WhisperState, lang: Language, start: usize, end: usize| {
            let mut params = FullParams::new(SamplingStrategy::BeamSearch {
                beam_size: 3,
                patience: -1.0,
            });
            params.set_n_threads(threads);
            params.set_language(Some(lang.code()));
            if let Some(prompt) = &prompt {
                // ponytail: whisper-rs never frees this CString (~700 bytes per speech segment, a
                // few hundred KB for a long meeting); upstream fix or one FullParams per job if it matters.
                params.set_initial_prompt(prompt);
            }
            params.set_translate(false);
            params.set_no_timestamps(false);
            params.set_suppress_blank(true);
            params.set_suppress_nst(true);
            params.set_temperature(0.2);
            params.set_entropy_thold(2.4);
            params.set_logprob_thold(-1.0);
            params.set_no_speech_thold(0.55);
            params.set_print_progress(false);
            params.set_print_realtime(false);
            params.set_print_special(false);
            params.set_print_timestamps(false);
            // whisper-rs 0.16's `set_abort_callback_safe` is broken (its trampoline casts the boxed
            // `dyn FnMut` back to the concrete closure type, so whisper.cpp reads garbage and aborts
            // the encoder). Use the raw API with a pointer to the cancel flag instead.
            // SAFETY: `cancel` is borrowed for this whole fn, so it outlives `params` and `state`
            // (locals here), the only holders of the pointer; `AtomicBool` is Sync, so reading it
            // from ggml worker threads is fine, and the callback cannot panic.
            unsafe {
                params.set_abort_callback(Some(abort_when_cancelled));
                params.set_abort_callback_user_data(Arc::as_ptr(cancel) as *mut std::ffi::c_void);
            }

            if let Err(err) = state.full(params, &samples[start..end]) {
                if cancel.load(Ordering::Relaxed) {
                    bail!(AppError::new("cancelled", "Transcription cancelled"));
                }
                return Err(err).context("running Whisper");
            }

            let offset_ms = (start * 1000 / SAMPLE_RATE) as u32;
            let limit_ms = (end * 1000 / SAMPLE_RATE) as u32;
            let (mut lines, mut plogs) = (Vec::new(), Vec::new());
            for segment in state.as_iter() {
                for i in 0..segment.n_tokens() {
                    let data = segment
                        .get_token(i)
                        .context("reading a token")?
                        .token_data();
                    if data.id < eot {
                        plogs.push(data.plog);
                    }
                }
                let Some(text) = clean_text(&segment.to_str_lossy()?) else {
                    continue;
                };
                // whisper timestamps are centiseconds from the start of this speech segment
                let start_ms = offset_ms + (segment.start_timestamp().max(0) as u32) * 10;
                let end_ms =
                    (offset_ms + (segment.end_timestamp().max(0) as u32) * 10).min(limit_ms);
                lines.push(Segment {
                    start_ms,
                    end_ms: end_ms.max(start_ms),
                    text,
                    lang: Some(lang),
                });
            }
            // Nothing written in this language can't beat what the other one wrote.
            let sure = if lines.is_empty() {
                f32::NEG_INFINITY
            } else {
                plogs.iter().sum::<f32>() / plogs.len().max(1) as f32
            };
            Ok((sure, lines))
        };

        let mut segments = Vec::new();
        for (index, &(start, end)) in speech.iter().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                bail!(AppError::new("cancelled", "Transcription cancelled"));
            }
            // "Auto" is Vietnamese or English, chosen for each stretch of speech (D-046): left to
            // pick among its 99 languages, Whisper wrote English it took for Vietnamese as a
            // Vietnamese translation, and once as Malay.
            let tries = match language {
                Language::Auto => {
                    let (vi, en) = vi_en_odds(&mut state, &samples[start..end])?;
                    candidates(vi, en)
                }
                fixed => vec![fixed],
            };
            let mut best: Option<(f32, Vec<Segment>)> = None;
            for lang in tries {
                let (sure, lines) = decode(&mut state, lang, start, end)?;
                if best.as_ref().is_none_or(|(most, _)| sure > most + CLEARLY) {
                    best = Some((sure, lines));
                }
            }
            segments.extend(best.map(|(_, lines)| lines).unwrap_or_default());
            progress((index + 1) as f32 / speech.len().max(1) as f32);
        }

        loaded.last_used = Instant::now();
        Ok(segments)
    }
}

/// Decoding settings for a live session's utterances (D-041), built once per session and cloned
/// per call: the language forced (Vietnamese or English; no detection pass), greedy with one
/// candidate at temperature 0 and no fallback (so every line's token probabilities mean the same
/// thing, and a garbled line costs one pass, not six), one segment with no timestamps (the times
/// come from the segmenter), up to whisper.cpp's own 220-token ceiling (a 15 s presenter line),
/// the vocabulary prompt, and whisper.cpp's default `no_context` (true: nothing carries over
/// between calls). It owns the cancel flag its abort callback points to, so the pointer outlives
/// every use of the settings.
pub(in crate::kernel::ai) struct LiveDecoder {
    vi: FullParams<'static, 'static>,
    en: FullParams<'static, 'static>,
    cancel: Arc<AtomicBool>,
}

impl LiveDecoder {
    pub fn new(prompt: &str, cancel: Arc<AtomicBool>) -> Self {
        let prompt = prompt.replace('\0', " ");
        let params = |language: Language| {
            let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
            params.set_n_threads(threads());
            params.set_language(Some(language.code()));
            if !prompt.trim().is_empty() {
                params.set_initial_prompt(&prompt); // leaked once per session (see `transcribe`)
            }
            params.set_translate(false);
            params.set_single_segment(true);
            params.set_no_timestamps(true);
            params.set_max_tokens(220);
            params.set_suppress_blank(true);
            params.set_suppress_nst(true);
            params.set_temperature(0.0);
            params.set_temperature_inc(0.0);
            params.set_entropy_thold(2.4);
            params.set_logprob_thold(-1.0);
            params.set_no_speech_thold(0.55);
            params.set_print_progress(false);
            params.set_print_realtime(false);
            params.set_print_special(false);
            params.set_print_timestamps(false);
            // SAFETY: the pointer is to the flag `Self` owns, and the settings (and their clones)
            // are only used through `&self`, so the flag outlives every whisper call that reads it.
            unsafe {
                params.set_abort_callback(Some(abort_when_cancelled));
                params.set_abort_callback_user_data(Arc::as_ptr(&cancel) as *mut std::ffi::c_void);
            }
            params
        };
        Self {
            vi: params(Language::Vi),
            en: params(Language::En),
            cancel,
        }
    }
}

/// One live utterance as Whisper heard it.
pub(in crate::kernel::ai) struct Heard {
    pub text: String,
    /// Per word, the probability of its least likely token, averaged over the words (punctuation
    /// skipped): 0–1.
    pub confidence: f32,
    /// The mean log-probability of the decoded tokens, end-of-text included (whisper.cpp's own).
    pub avg_logprob: f32,
    /// No end-of-text within the token cap, or a loop the cleanup collapsed.
    pub runaway: bool,
}

/// A decoded token as `score` sees it: its bytes (None: end-of-text, the only special token a
/// live decode writes), probability and log-probability.
pub(in crate::kernel::ai) struct Tok {
    pub bytes: Option<Vec<u8>>,
    pub p: f32,
    pub plog: f32,
}

/// (confidence, avg_logprob, ended) of a decoded line — see `Heard`. A word is as sure as its least
/// sure token: a Vietnamese syllable takes ~1.8 byte tokens and the continuation bytes are almost
/// always certain, which would flatter a plain token average (r3 §4.2).
pub(in crate::kernel::ai) fn score(tokens: &[Tok]) -> (f32, f32, bool) {
    let (mut words, mut sum, mut word, mut ended) = (0u32, 0f32, None::<f32>, false);
    for token in tokens {
        let Some(bytes) = &token.bytes else {
            ended = true;
            continue;
        };
        if bytes.iter().all(|b| b.is_ascii_punctuation() || *b == b' ') {
            continue; // Whisper is often unsure of punctuation
        }
        if bytes.first() == Some(&b' ') {
            if let Some(p) = word.take() {
                (words, sum) = (words + 1, sum + p);
            }
        }
        word = Some(word.map_or(token.p, |least| least.min(token.p)));
    }
    if let Some(p) = word {
        (words, sum) = (words + 1, sum + p);
    }
    let confidence = if words == 0 { 0.0 } else { sum / words as f32 };
    let avg_logprob = tokens.iter().map(|t| t.plog).sum::<f32>() / tokens.len().max(1) as f32;
    (confidence, avg_logprob, ended)
}

/// whisper.cpp abort callback: `user_data` is the job's `AtomicBool` cancel flag.
unsafe extern "C" fn abort_when_cancelled(user_data: *mut std::ffi::c_void) -> bool {
    // SAFETY: set from `Arc::as_ptr` on a flag that outlives the whisper call (see `transcribe`).
    unsafe { (*(user_data as *const AtomicBool)).load(Ordering::Relaxed) }
}

fn threads() -> i32 {
    std::thread::available_parallelism().map_or(8, |n| n.get().min(8)) as i32
}

/// The bundled Silero VAD, on the CPU. Batch transcription loads one per job; a live session keeps
/// one for the whole session. One thread: its steps are tiny (one 32 ms frame each), so more threads
/// only wait for each other — measured on an M1 Pro, 10 s of audio takes 20 ms on 1 thread, 114 ms
/// on 8, and live runs it on every window for each voice while Whisper and Qwen use the cores.
pub(in crate::kernel::ai) fn vad_context(vad_model: &Path) -> Result<WhisperVadContext> {
    let mut ctx_params = WhisperVadContextParams::new();
    ctx_params.set_n_threads(1);
    let path = vad_model.to_str().context("VAD model path is not UTF-8")?;
    WhisperVadContext::new(path, ctx_params).context("loading the VAD model")
}

/// Speech ranges as sample indices: threshold 0.5, min speech 250 ms, 2000 ms of silence ends a
/// segment (file-import "redemption"), segments capped at 25 s, 300 ms padding (D-014). Live
/// sessions cut their own utterances from the VAD's probabilities instead (`live::cut`).
fn detect_speech(vad_model: &Path, samples: &[f32]) -> Result<Vec<(usize, usize)>> {
    let mut vad = vad_context(vad_model)?;
    vad.detect_speech(samples).context("detecting speech")?;

    let mut params = WhisperVadParams::new();
    params.set_threshold(0.5);
    params.set_min_speech_duration(250);
    params.set_min_silence_duration(2000);
    params.set_max_speech_duration(25.0);
    params.set_speech_pad(300);
    let found = vad
        .segments_from_probabilities(params)
        .context("finding speech segments")?;

    let per_cs = SAMPLE_RATE / 100; // VAD timestamps are centiseconds
    Ok((0..found.num_segments())
        .filter_map(|i| found.get_segment(i))
        .map(|s| {
            (
                (s.start.max(0.0) as usize * per_cs).min(samples.len()),
                (s.end.max(0.0) as usize * per_cs).min(samples.len()),
            )
        })
        .filter(|(start, end)| end > start)
        .collect())
}

/// Whole-segment hallucinations Whisper produces on silence or music.
const DROP_IF_EQUALS: &[&str] = &[
    "thank you for watching",
    "thanks for watching",
    "please subscribe",
    "you",
];
/// Distinctive fragments that only ever appear as hallucinations (EN and VI).
const DROP_IF_CONTAINS: &[&str] = &[
    "amara.org",
    "subscribe to my channel",
    "ghiền mì gõ",
    "subscribe cho kênh",
];
/// A chunk repeated this many times in a row is collapsed to one copy.
const MIN_REPEATS: usize = 4;

/// Language-safe cleanup: drops annotation-only and known hallucination segments and collapses
/// runaway repetition by character n-grams, never by splitting on spaces (so Vietnamese
/// reduplication like "từ từ" survives). Returns None when nothing real is left.
pub fn clean_text(raw: &str) -> Option<String> {
    let text = raw.trim();
    let is_annotation = (text.starts_with('[') && text.ends_with(']'))
        || (text.starts_with('(') && text.ends_with(')'));
    let core =
        text.trim_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace() || c == '♪');
    if is_annotation || core.is_empty() {
        return None;
    }
    let lower = core.to_lowercase();
    if DROP_IF_EQUALS.contains(&lower.as_str())
        || DROP_IF_CONTAINS.iter().any(|p| lower.contains(p))
    {
        return None;
    }
    Some(collapse_repeats(text))
}

fn collapse_repeats(text: &str) -> String {
    // A trailing space lets the last word of "ha ha ha ha" count as a full repeat.
    let chars: Vec<char> = text.chars().chain(std::iter::once(' ')).collect();
    let mut out: Vec<char> = Vec::with_capacity(chars.len());
    let mut i = 0;
    'scan: while i < chars.len() {
        for period in 1..=(chars.len() - i) / MIN_REPEATS {
            let unit = &chars[i..i + period];
            if !unit.iter().any(|c| c.is_alphabetic()) {
                continue; // never collapse digits or punctuation ("10000", "....")
            }
            let mut count = 1;
            while i + (count + 1) * period <= chars.len()
                && chars[i + count * period..i + (count + 1) * period] == *unit
            {
                count += 1;
            }
            if count >= MIN_REPEATS {
                out.extend_from_slice(unit);
                i += count * period;
                continue 'scan;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out.into_iter().collect::<String>().trim().to_owned()
}

#[cfg(test)]
mod tests {

    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    use super::{Language, SpeechModel, WhisperEngine, clean_text};
    use crate::kernel::ai::models::{BUNDLED_VAD, CATALOGUE, ModelStore};
    use crate::kernel::hub::WorkspaceHub;
    use crate::modules::meetings::{self, NewMeeting, audio};

    /// Encoder benchmark (D-017): `NEWDAWN_BENCH_MODEL=<ggml .bin> NEWDAWN_BENCH_AUDIO=<file>
    /// cargo test bench_transcribe -- --ignored --nocapture [--features coreml]`. Reports model load
    /// (with a 2 s warm-up) and a full transcription with the model already loaded.
    #[test]
    #[ignore]
    fn bench_transcribe() {
        let (Ok(model), Ok(audio_path)) = (
            std::env::var("NEWDAWN_BENCH_MODEL"),
            std::env::var("NEWDAWN_BENCH_AUDIO"),
        ) else {
            eprintln!("set NEWDAWN_BENCH_MODEL and NEWDAWN_BENCH_AUDIO");
            return;
        };
        let _ = tracing_subscriber::fmt()
            .with_env_filter("info")
            .with_test_writer()
            .try_init();
        let vad = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join(BUNDLED_VAD);
        let samples = audio::load_16k_mono(std::path::Path::new(&audio_path)).unwrap();
        let engine = WhisperEngine::new(super::DEFAULT_IDLE_UNLOAD_SECS);
        let model_path = PathBuf::from(&model);
        let speech = || SpeechModel {
            id: "bench",
            path: &model_path,
        };
        let cancel = Arc::new(AtomicBool::new(false));

        let t = std::time::Instant::now();
        engine
            .transcribe(
                speech(),
                &vad,
                &samples[..2 * super::SAMPLE_RATE],
                Language::En,
                None,
                &cancel,
                |_| {},
            )
            .unwrap();
        let load = t.elapsed();
        let t = std::time::Instant::now();
        let segments = engine
            .transcribe(
                speech(),
                &vad,
                &samples,
                Language::En,
                None,
                &cancel,
                |_| {},
            )
            .unwrap();
        let run = t.elapsed();
        let audio_s = samples.len() as f64 / super::SAMPLE_RATE as f64;
        eprintln!(
            "BENCH coreml={} audio={audio_s:.0}s load+warmup={load:.1?} transcribe={run:.1?} ({:.1}x realtime) segments={}",
            cfg!(feature = "coreml"),
            audio_s / run.as_secs_f64(),
            segments.len()
        );
    }

    /// Model comparison, step 1 (see `modules::meetings::eval`): a recording transcribed the way
    /// a meeting is — language auto, the workspace's people and terms as Whisper's prompt, echoes
    /// of the prompt dropped, the vocabulary's corrections — with the debug build's preferred
    /// Whisper model. `NEWDAWN_EVAL_AUDIO=<file> NEWDAWN_EVAL_SEGMENTS=<out .json>
    /// [NEWDAWN_EVAL_PEOPLE_SQL=<.sql>] cargo test eval_transcribe -- --ignored --nocapture`.
    #[test]
    fn writes_a_close_call_both_ways_likelier_first() {
        use super::candidates;
        assert_eq!(candidates(0.95, 0.03), vec![Language::Vi]);
        assert_eq!(candidates(0.0, 0.98), vec![Language::En]);
        assert_eq!(candidates(0.55, 0.40), vec![Language::Vi, Language::En]);
        assert_eq!(candidates(0.15, 0.27), vec![Language::En, Language::Vi]);
    }

    #[test]
    #[ignore]
    fn eval_transcribe() {
        use crate::modules::meetings::eval;
        use crate::modules::{people, vocabulary};
        let audio_path = eval::var("NEWDAWN_EVAL_AUDIO").expect("set NEWDAWN_EVAL_AUDIO");
        let out = eval::var("NEWDAWN_EVAL_SEGMENTS").expect("set NEWDAWN_EVAL_SEGMENTS");
        let app_data = PathBuf::from(std::env::var("HOME").unwrap())
            .join("Library/Application Support/co.majorisdigital.newdawn.dev"); // the debug build's own (D-044)
        let models = ModelStore::new(app_data.join("models")).unwrap();
        let (spec, model_path) = models
            .preferred_speech_model()
            .expect("download a Whisper model first");
        // NEWDAWN_EVAL_WHISPER: another downloaded model's file (e.g. the full large-v3).
        let (spec, model_path) = match eval::var("NEWDAWN_EVAL_WHISPER") {
            Some(file) => (
                CATALOGUE
                    .iter()
                    .find(|s| s.file == file)
                    .expect("a catalogue file name"),
                app_data.join("models").join(&file),
            ),
            None => (spec, model_path),
        };
        let vad = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join(BUNDLED_VAD);
        tauri::async_runtime::block_on(async {
            let setup = eval::setup(None).await;
            let owner = eval::owner();
            let people = people::with_owner(people::list(&setup.pool).await.unwrap(), Some(&owner));
            let terms = vocabulary::list(&setup.pool).await.unwrap();
            let prompt = vocabulary::whisper_prompt(&setup.meeting.title, None, &people, &terms);
            let samples = audio::load_16k_mono(std::path::Path::new(&audio_path)).unwrap();
            let engine = WhisperEngine::new(super::DEFAULT_IDLE_UNLOAD_SECS);
            let cancel = Arc::new(AtomicBool::new(false));
            let started = std::time::Instant::now();
            let mut segments = engine
                .transcribe(
                    SpeechModel {
                        id: spec.id,
                        path: &model_path,
                    },
                    &vad,
                    &samples,
                    Language::Auto,
                    Some(&prompt),
                    &cancel,
                    |_| {},
                )
                .unwrap();
            segments.retain(|s| !vocabulary::echoes_prompt(&s.text, &prompt));
            let mut lines: Vec<String> = segments.iter().map(|s| s.text.clone()).collect();
            let fixed =
                vocabulary::correct_lines(&mut lines, &vocabulary::replacements(&people, &terms));
            for (segment, line) in segments.iter_mut().zip(lines) {
                segment.text = line;
            }
            std::fs::write(&out, serde_json::to_string_pretty(&segments).unwrap()).unwrap();
            eprintln!(
                "EVAL transcribe: {} s of audio, {} lines ({} corrected by the vocabulary) in {:.1?} with {}",
                samples.len() / super::SAMPLE_RATE,
                segments.len(),
                fixed.iter().filter(|f| f.is_some()).count(),
                started.elapsed(),
                spec.id
            );
            setup.finish().await;
        });
    }

    /// End to end on real speech, minus the queue/event glue: `say` → m4a → decode → VAD →
    /// Whisper → mid-job cancel → save → FTS. Needs a downloaded model: `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn transcribes_generated_speech() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter("info")
            .with_test_writer()
            .try_init();
        let app_data = PathBuf::from(std::env::var("HOME").unwrap())
            .join("Library/Application Support/co.majorisdigital.newdawn.dev"); // the debug build's own (D-044)
        let models = ModelStore::new(app_data.join("models")).unwrap();
        let (spec, model_path) = models
            .preferred_speech_model()
            .expect("download a Whisper model first");
        let vad = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join(BUNDLED_VAD);

        let tmp = std::env::temp_dir().join(format!("newdawn-e2e-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&tmp).unwrap();
        let (aiff, m4a) = (tmp.join("speech.aiff"), tmp.join("speech.m4a"));
        let script = "Good morning everyone. Today we review the partner agreement with Acme Capital. \
                      [[slnc 3000]] The first decision is to sign the term sheet by Friday.";
        assert!(
            std::process::Command::new("say")
                .args(["-o", aiff.to_str().unwrap(), script])
                .status()
                .unwrap()
                .success()
        );
        assert!(
            std::process::Command::new("afconvert")
                .args([
                    "-f",
                    "m4af",
                    "-d",
                    "aac",
                    aiff.to_str().unwrap(),
                    m4a.to_str().unwrap()
                ])
                .status()
                .unwrap()
                .success()
        );

        let samples = audio::load_16k_mono(&m4a).unwrap();
        let engine = WhisperEngine::new(super::DEFAULT_IDLE_UNLOAD_SECS);
        let model = || SpeechModel {
            id: spec.id,
            path: &model_path,
        };
        let started = std::time::Instant::now();
        let segments = engine
            .transcribe(
                model(),
                &vad,
                &samples,
                Language::En,
                None,
                &Arc::new(AtomicBool::new(false)),
                |_| {},
            )
            .unwrap();
        eprintln!("{} in {:?}:", spec.id, started.elapsed());
        for s in &segments {
            eprintln!("  [{:>6} – {:>6}] {}", s.start_ms, s.end_ms, s.text);
        }
        let text = segments
            .iter()
            .map(|s| s.text.to_lowercase())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(text.contains("acme") && text.contains("friday"), "{text}");
        assert!(
            segments.last().unwrap().start_ms > 3_000,
            "timestamps should be offset past the 3 s pause"
        );

        // Cancel mid-job: the abort callback must stop Whisper and surface code "cancelled".
        let flag = Arc::new(AtomicBool::new(false));
        let setter = flag.clone();
        let stopper = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            setter.store(true, std::sync::atomic::Ordering::Relaxed);
        });
        let err = engine
            .transcribe(model(), &vad, &samples, Language::En, None, &flag, |_| {})
            .unwrap_err();
        stopper.join().unwrap();
        assert!(super::is_cancelled(&err), "{err:#}");

        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(tmp.join("data")).await.unwrap();
            hub.create_workspace("Test", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let id = meetings::new_id();
            let new = NewMeeting {
                id: &id,
                meeting_type_id: None,
                title: "E2E",
                held_at_unix: 0,
                language: Language::En,
                audio_path: m4a.to_str().unwrap(),
                status: meetings::MeetingStatus::Recorded,
            };
            meetings::create(&pool, new).await.unwrap();
            meetings::save_transcript(&pool, &id, &segments, &[], (samples.len() / 16) as u32)
                .await
                .unwrap();
            assert_eq!(
                meetings::transcript(&pool, &id).await.unwrap().len(),
                segments.len()
            );
            assert_eq!(
                meetings::get(&pool, &id).await.unwrap().status,
                meetings::MeetingStatus::Transcribed
            );
            let (hits,): (i64,) = sqlx::query_as(
                "SELECT count(*) FROM transcript_fts WHERE transcript_fts MATCH 'friday'",
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            assert!(hits >= 1, "full-text search should find the transcript");

            // A meeting left `transcribing` by a quit is settled on the next launch.
            meetings::set_status(&pool, &id, meetings::MeetingStatus::Transcribing)
                .await
                .unwrap();
            meetings::recover_interrupted(&pool).await.unwrap();
            assert_eq!(
                meetings::get(&pool, &id).await.unwrap().status,
                meetings::MeetingStatus::Transcribed
            );
            hub.close_all().await;
        });
        std::fs::remove_dir_all(tmp).unwrap();
    }

    #[test]
    fn cleanup_is_language_safe() {
        // Kept as-is
        assert_eq!(
            clean_text(" Từ từ đã, chúng ta bàn tiếp. ").as_deref(),
            Some("Từ từ đã, chúng ta bàn tiếp.")
        );
        assert_eq!(clean_text("Thank you.").as_deref(), Some("Thank you."));
        assert_eq!(
            clean_text("Revenue grew 10000 units....").as_deref(),
            Some("Revenue grew 10000 units....")
        );
        // Dropped
        for junk in [
            "[Music]",
            "(applause)",
            "♪ ♪",
            "Thanks for watching!",
            "  ",
            "You",
            "Hãy subscribe cho kênh Ghiền Mì Gõ để không bỏ lỡ",
        ] {
            assert_eq!(clean_text(junk), None, "{junk}");
        }
        // Runaway repetition collapsed, short reduplication untouched
        assert_eq!(clean_text("ha ha ha ha ha").as_deref(), Some("ha"));
        assert_eq!(
            clean_text("OK. OK. OK. OK. OK. Let's start").as_deref(),
            Some("OK. Let's start")
        );
        assert_eq!(clean_text("từ từ từ từ từ").as_deref(), Some("từ"));
    }
}
