//! Text → text with Qwen through Ollama on localhost (CLAUDE.md rule 2/3). Features open a
//! `TextSession` (their turn in the AI queue, Whisper unloaded first) and call `complete` on it;
//! every call streams, sets `num_ctx` explicitly and writes one `ai_log` row.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use tokio::sync::MutexGuard;

use crate::error::AppError;
use crate::events::{OllamaPullFinished, OllamaPullProgress};
use crate::kernel::ai::log::{self as ai_log, AiLogEntry};
use crate::kernel::ai::queue::AiQueue;
use crate::kernel::ai::whisper::WhisperEngine;

const BASE: &str = "http://localhost:11434";
/// The text model until the user picks another in Settings → Models (D-027).
pub const DEFAULT_MODEL: &str = "qwen3.5:9b";
/// App setting holding the picked model.
pub const MODEL_SETTING: &str = "ollama_model";
/// The picked model ("" = default); loaded at launch, changed by `select`.
static SELECTED: std::sync::RwLock<String> = std::sync::RwLock::new(String::new());

/// The model the last text session ran on — the one Ollama may still hold in memory (D-038).
static LOADED: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

/// The text model every call uses.
pub fn model() -> String {
    let picked = SELECTED
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if picked.is_empty() {
        DEFAULT_MODEL.to_owned()
    } else {
        picked.clone()
    }
}

/// Uses `name` from now on (the caller stores it as `MODEL_SETTING`).
pub fn use_model(name: &str) {
    *SELECTED
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = name.to_owned();
}

/// Ollama model names: letters, digits and `._:/-` (e.g. `qwen3.5:9b`, `library/qwen3:4b`).
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 200
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._:/-".contains(c))
}
/// Context window asked of Ollama for every call (fits the 9B model on 16 GB with Whisper unloaded).
pub const NUM_CTX: u32 = 32_768;
/// How long Ollama keeps Qwen loaded after a call (several calls per summary reuse it).
const KEEP_ALIVE: &str = "5m";

#[derive(Debug, Clone, Serialize, Type)]
pub struct OllamaStatus {
    pub running: bool,
    /// The selected model is pulled.
    pub model_installed: bool,
    /// The selected model.
    pub model: String,
    /// Every model Ollama has, for the picker.
    pub installed: Vec<InstalledModel>,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct InstalledModel {
    pub name: String,
    pub size_mb: u32,
}

/// A client that never uses HTTP(S)_PROXY / ALL_PROXY: meeting text must not leave the Mac
/// through an environment proxy (rule 3).
fn client(read_timeout: Duration) -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .no_proxy()
        .connect_timeout(Duration::from_secs(5))
        .read_timeout(read_timeout)
        .build()?)
}

/// Is Ollama reachable, and is the model pulled?
pub async fn status() -> OllamaStatus {
    #[derive(Deserialize)]
    struct Tags {
        models: Vec<Tag>,
    }
    #[derive(Deserialize)]
    struct Tag {
        name: String,
        #[serde(default)]
        size: u64,
    }
    let model = model();
    let down = || OllamaStatus {
        running: false,
        model_installed: false,
        model: model.clone(),
        installed: Vec::new(),
    };
    let Ok(client) = client(Duration::from_secs(2)) else {
        return down();
    };
    let tags = client
        .get(format!("{BASE}/api/tags"))
        .timeout(Duration::from_secs(2))
        .send()
        .await;
    match tags {
        Ok(response) => {
            let mut installed: Vec<InstalledModel> = response
                .json::<Tags>()
                .await
                .map(|t| t.models)
                .unwrap_or_default()
                .into_iter()
                .map(|m| InstalledModel {
                    name: m.name,
                    size_mb: (m.size / 1_000_000) as u32,
                })
                .collect();
            installed.sort_by(|a, b| a.name.cmp(&b.name));
            OllamaStatus {
                running: true,
                model_installed: installed.iter().any(|m| m.name == model),
                model,
                installed,
            }
        }
        Err(_) => down(),
    }
}

/// One pull at a time (the onboarding button can't start two), and which model it is.
static PULLING: AtomicBool = AtomicBool::new(false);
static PULLING_NAME: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

/// Asks Ollama to download Qwen in the background (Ollama fetches it; New Dawn only talks to
/// localhost). Progress: `OllamaPullProgress` in MB; the end: `OllamaPullFinished`.
pub fn start_pull(app: &AppHandle, name: &str) -> Result<()> {
    if !valid_name(name) {
        bail!(AppError::invalid(
            "That isn't an Ollama model name (e.g. qwen3.5:9b)"
        ));
    }
    {
        let mut current = PULLING_NAME
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if PULLING.load(Ordering::Acquire) {
            if *current == name {
                return Ok(()); // already running: the caller just listens to its progress
            }
            bail!(AppError::new(
                "busy",
                format!("{current} is downloading — try again when it's done")
            ));
        }
        PULLING.store(true, Ordering::Release);
        name.clone_into(&mut current);
    }
    let (app, name) = (app.clone(), name.to_owned());
    tauri::async_runtime::spawn(async move {
        /// Clears the flag however the task ends (a panic included).
        struct Done;
        impl Drop for Done {
            fn drop(&mut self) {
                PULLING.store(false, Ordering::Release);
            }
        }
        let done = Done;
        let result = pull(&app, &name).await;
        drop(done);
        if let Err(err) = &result {
            tracing::warn!("model pull ended: {err:#}");
        }
        let _ = OllamaPullFinished {
            model: name,
            error: result.err().map(AppError::from),
        }
        .emit(&app);
    });
    Ok(())
}

async fn pull(app: &AppHandle, name: &str) -> Result<()> {
    #[derive(Deserialize)]
    struct Line {
        #[serde(default)]
        status: String,
        total: Option<u64>,
        completed: Option<u64>,
        error: Option<String>,
    }
    // A long read timeout: Ollama can go quiet while it verifies a multi-GB layer.
    let response = client(Duration::from_secs(300))?
        .post(format!("{BASE}/api/pull"))
        .json(&serde_json::json!({ "model": name, "stream": true }))
        .send()
        .await
        .map_err(|_| AppError::new("ollama_unavailable", "Ollama isn't running"))?;
    if !response.status().is_success() {
        bail!(AppError::new(
            "ollama_error",
            format!("Ollama returned {}", response.status())
        ));
    }
    let mut stream = response.bytes_stream();
    let mut buffer = Vec::new();
    let mut last = Instant::now() - Duration::from_secs(1);
    let mut last_status = String::new();
    while let Some(bytes) = stream.next().await {
        buffer.extend_from_slice(&bytes.context("reading from Ollama")?);
        while let Some(newline) = buffer.iter().position(|b| *b == b'\n') {
            let raw: Vec<u8> = buffer.drain(..=newline).collect();
            if raw.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            let line: Line = serde_json::from_slice(&raw).context("reading Ollama's reply")?;
            if let Some(error) = line.error {
                bail!(AppError::new("ollama_error", error));
            }
            if line.status == "success" {
                return Ok(());
            }
            if last.elapsed() >= Duration::from_millis(250) || line.status != last_status {
                last = Instant::now();
                last_status.clone_from(&line.status);
                let mb = |b: Option<u64>| (b.unwrap_or(0) / 1_000_000) as u32;
                let _ = OllamaPullProgress {
                    model: name.to_owned(),
                    status: line.status,
                    completed_mb: mb(line.completed),
                    total_mb: mb(line.total),
                }
                .emit(app);
            }
        }
    }
    bail!(AppError::new(
        "ollama_error",
        "Ollama stopped before the download finished"
    ))
}

/// Picks the text model (must already be in Ollama) and remembers it.
/// Waits for the AI queue, so a running summary finishes with the model it started with.
pub async fn select(app: &AppHandle, name: &str) -> Result<OllamaStatus> {
    if !valid_name(name) {
        bail!(AppError::invalid("That isn't an Ollama model name"));
    }
    let _turn = app.state::<AiQueue>().inner().acquire().await;
    if !status().await.installed.iter().any(|m| m.name == name) {
        bail!(AppError::invalid(format!(
            "Download {name} before choosing it"
        )));
    }
    unload().await; // the old one leaves memory
    app.state::<crate::kernel::hub::WorkspaceHub>()
        .set_app_setting(MODEL_SETTING, name)
        .await?;
    use_model(name);
    Ok(status().await)
}

/// Deletes a model from Ollama. Waits for the AI queue, so it never pulls the model out from
/// under a running summary.
pub async fn remove(app: &AppHandle, name: &str) -> Result<()> {
    if !valid_name(name) {
        bail!(AppError::invalid("That isn't an Ollama model name"));
    }
    if name == model() {
        bail!(AppError::invalid(
            "This model writes the summaries — choose another one first"
        ));
    }
    let _turn = app.state::<AiQueue>().inner().acquire().await;
    let response = client(Duration::from_secs(30))?
        .delete(format!("{BASE}/api/delete"))
        .json(&serde_json::json!({ "model": name }))
        .send()
        .await
        .map_err(|_| AppError::new("ollama_unavailable", "Ollama isn't running"))?;
    if !response.status().is_success() {
        bail!(AppError::new(
            "ollama_error",
            format!("Ollama couldn't remove {name} ({})", response.status())
        ));
    }
    Ok(())
}

/// Asks Ollama to drop the text models from memory now (before Whisper loads): the app's and the
/// one the last session used, which may be an agent's own. Best effort.
pub async fn unload() {
    let last = std::mem::take(
        &mut *LOADED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    );
    let app = model();
    if !last.is_empty() && last != app {
        unload_one(&last).await;
    }
    unload_one(&app).await;
}

async fn unload_one(name: &str) {
    let body = serde_json::json!({ "model": name, "keep_alive": 0 });
    if let Ok(client) = client(Duration::from_secs(10)) {
        let _ = client
            .post(format!("{BASE}/api/generate"))
            .timeout(Duration::from_secs(10))
            .json(&body)
            .send()
            .await;
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Message {
    pub role: &'static str, // system | user | assistant
    pub content: String,
}

pub struct Request {
    pub messages: Vec<Message>,
    pub think: bool,
    /// Cap on generated tokens (thinking included) so a runaway answer can't hold the queue.
    pub max_tokens: u32,
}

pub struct Completion {
    pub content: String,
    pub prompt_tokens: u32,
    pub output_tokens: u32,
}

/// A feature's turn with the text model: holds the AI queue so nothing else runs meanwhile.
pub struct TextSession<'a> {
    _turn: MutexGuard<'a, ()>,
    pool: SqlitePool,
    meeting_id: Option<String>,
    /// Which agent is talking, for `ai_log` (D-035).
    agent: Option<&'static str>,
    /// Fixed for the whole session (a pick in Settings waits for the queue anyway).
    model: String,
}

/// Waits for the AI queue, checks Ollama, and unloads Whisper (never two models at once).
/// `model`: an agent's own model (D-038); None = the app's selected one.
pub async fn session<'a>(
    app: &'a AppHandle,
    pool: SqlitePool,
    meeting_id: Option<&str>,
    model: Option<String>,
) -> Result<TextSession<'a>> {
    let turn = app.state::<AiQueue>().inner().acquire().await;
    let ollama = status().await;
    if !ollama.running {
        bail!(AppError::new(
            "ollama_unavailable",
            "Ollama isn't running — open the Ollama app and try again"
        ));
    }
    let has = |name: &str| ollama.installed.iter().any(|m| m.name == name);
    // An agent's model that was removed falls back to the app's, rather than failing the run.
    let chosen = match model.filter(|m| !m.is_empty()) {
        Some(m) if has(&m) => m,
        Some(m) => {
            tracing::warn!("agent model {m} isn't installed; using {}", ollama.model);
            ollama.model.clone()
        }
        None => ollama.model.clone(),
    };
    if !has(&chosen) {
        bail!(AppError::new(
            "ollama_model_missing",
            format!("Ollama doesn't have {chosen} yet — download it in Settings → Models")
        ));
    }
    app.state::<WhisperEngine>().unload_now();
    // Never two text models at once: drop the previous one when an agent uses another.
    let previous = std::mem::replace(
        &mut *LOADED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
        chosen.clone(),
    );
    if !previous.is_empty() && previous != chosen {
        unload_one(&previous).await;
    }
    Ok(TextSession {
        _turn: turn,
        pool,
        meeting_id: meeting_id.map(str::to_owned),
        agent: None,
        model: chosen,
    })
}

#[cfg(test)]
mod name_tests {
    #[test]
    fn model_names_are_checked() {
        assert!(super::valid_name("qwen3.5:9b"));
        assert!(super::valid_name("library/qwen3:4b-q4_K_M"));
        assert!(!super::valid_name(""));
        assert!(!super::valid_name("qwen; rm -rf"));
        assert!(!super::valid_name("a b"));
    }
}

#[cfg(test)]
impl<'a> TextSession<'a> {
    /// A session for tests: the caller supplies the queue turn (no AppHandle needed).
    pub fn for_test(turn: MutexGuard<'a, ()>, pool: SqlitePool) -> Self {
        Self {
            _turn: turn,
            pool,
            meeting_id: None,
            agent: None,
            model: model(),
        }
    }
}

impl TextSession<'_> {
    /// The model this session talks to.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Tags the calls that follow with the agent making them (one session can serve several
    /// agents in a row, so the queue is held once and Whisper isn't reloaded in between).
    pub fn for_agent(&mut self, agent: &'static str) -> &mut Self {
        self.agent = Some(agent);
        self
    }

    /// One streamed `/api/chat` call. `on_token` gets the running count of generated tokens.
    /// Errors with `context_overflow` if the prompt filled the context (Ollama truncates silently).
    pub async fn complete(
        &self,
        request: Request,
        cancel: &AtomicBool,
        mut on_token: impl FnMut(u32),
    ) -> Result<Completion> {
        if cancel.load(Ordering::Relaxed) {
            bail!(AppError::new("cancelled", "Summary cancelled"));
        }
        let started = Instant::now();
        let result = self.stream(request, cancel, &mut on_token).await;
        // Ollama truncates an over-long prompt silently; treat a full context as an overflow,
        // but keep the token counts in the log row that explains it.
        let overflow = matches!(&result, Ok(c) if c.prompt_tokens + 16 >= NUM_CTX);
        let (status, error, prompt_tokens, output_tokens) = match &result {
            Ok(c) if overflow => (
                "error",
                Some("context_overflow".to_owned()),
                Some(c.prompt_tokens),
                Some(c.output_tokens),
            ),
            Ok(c) => ("ok", None, Some(c.prompt_tokens), Some(c.output_tokens)),
            Err(err)
                if err
                    .downcast_ref::<AppError>()
                    .is_some_and(|e| e.code == "cancelled") =>
            {
                ("cancelled", None, None, None)
            }
            Err(err) => ("error", Some(format!("{err:#}")), None, None),
        };
        let entry = AiLogEntry {
            kind: "complete",
            agent: self.agent,
            model: &self.model,
            meeting_id: self.meeting_id.as_deref(),
            duration_ms: started.elapsed().as_millis() as u32,
            status,
            error,
            prompt_tokens,
            output_tokens,
        };
        if let Err(err) = ai_log::record(&self.pool, entry).await {
            tracing::warn!("could not write ai_log: {err:#}");
        }
        if overflow {
            bail!(AppError::new(
                "context_overflow",
                "The prompt filled the model's context"
            ));
        }
        result
    }

    async fn stream(
        &self,
        request: Request,
        cancel: &AtomicBool,
        on_token: &mut impl FnMut(u32),
    ) -> Result<Completion> {
        #[derive(Deserialize)]
        struct Chunk {
            #[serde(default)]
            message: Option<ChunkMessage>,
            #[serde(default)]
            done: bool,
            prompt_eval_count: Option<u32>,
            eval_count: Option<u32>,
            done_reason: Option<String>,
            error: Option<String>,
        }
        #[derive(Deserialize)]
        struct ChunkMessage {
            #[serde(default)]
            content: String,
        }

        let body = serde_json::json!({
            "model": self.model,
            "messages": request.messages,
            "stream": true,
            "think": request.think,
            "keep_alive": KEEP_ALIVE,
            "options": { "num_ctx": NUM_CTX, "num_predict": request.max_tokens, "temperature": 0.3 },
        });
        let response = client(Duration::from_secs(180))? // the first token can wait for the model to load
            .post(format!("{BASE}/api/chat"))
            .json(&body)
            .send()
            .await
            .context("calling Ollama")?;
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            bail!(AppError::new(
                "ollama_error",
                format!("Ollama returned {status}: {text}")
            ));
        }

        let mut stream = response.bytes_stream();
        let mut buffer = Vec::new();
        let mut content = String::new();
        let mut tokens = 0u32;
        let (mut prompt_tokens, mut output_tokens) = (0, 0);
        let mut truncated = false;
        while let Some(bytes) = stream.next().await {
            if cancel.load(Ordering::Relaxed) {
                bail!(AppError::new("cancelled", "Summary cancelled"));
            }
            buffer.extend_from_slice(&bytes.context("reading from Ollama")?);
            while let Some(newline) = buffer.iter().position(|b| *b == b'\n') {
                let line: Vec<u8> = buffer.drain(..=newline).collect();
                if line.iter().all(u8::is_ascii_whitespace) {
                    continue;
                }
                let chunk: Chunk =
                    serde_json::from_slice(&line).context("reading Ollama's reply")?;
                if let Some(error) = chunk.error {
                    bail!(AppError::new("ollama_error", error));
                }
                if let Some(message) = chunk.message {
                    content.push_str(&message.content);
                }
                tokens += 1; // one streamed chunk ≈ one token (thinking included)
                on_token(tokens);
                if chunk.done {
                    prompt_tokens = chunk.prompt_eval_count.unwrap_or(0);
                    output_tokens = chunk.eval_count.unwrap_or(tokens);
                    truncated = chunk.done_reason.as_deref() == Some("length");
                }
            }
        }
        if truncated {
            bail!(AppError::new(
                "output_truncated",
                "The answer hit its length limit before finishing"
            ));
        }
        Ok(Completion {
            content: strip_think(&content),
            prompt_tokens,
            output_tokens,
        })
    }
}

/// Removes `<think>…</think>` blocks a model may leave inline in its answer.
pub fn strip_think(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("<think>") {
        out.push_str(&rest[..start]);
        match rest[start..].find("</think>") {
            Some(end) => rest = &rest[start + end + "</think>".len()..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out.trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::strip_think;

    #[test]
    fn strips_think_blocks() {
        assert_eq!(strip_think("<think>plan…</think>\n{\"a\":1}"), "{\"a\":1}");
        assert_eq!(strip_think("a<think>x</think>b<think>unclosed"), "ab");
        assert_eq!(strip_think("plain"), "plain");
    }
}
