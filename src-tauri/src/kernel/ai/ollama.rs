//! Text → text with Qwen through Ollama on localhost (CLAUDE.md rule 2/3). Features open a
//! `TextSession` (their turn in the AI queue, Whisper unloaded first) and call `complete` on it;
//! every call streams, sets `num_ctx` explicitly and writes one `ai_log` row.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::{AppHandle, Manager};
use tokio::sync::MutexGuard;

use crate::error::AppError;
use crate::kernel::ai::log::{self as ai_log, AiLogEntry};
use crate::kernel::ai::queue::AiQueue;
use crate::kernel::ai::whisper::WhisperEngine;

const BASE: &str = "http://localhost:11434";
pub const MODEL: &str = "qwen3.5:9b";
/// Context window asked of Ollama for every call (fits the 9B model on 16 GB with Whisper unloaded).
pub const NUM_CTX: u32 = 32_768;
/// How long Ollama keeps Qwen loaded after a call (several calls per summary reuse it).
const KEEP_ALIVE: &str = "5m";

#[derive(Debug, Clone, Serialize, Type)]
pub struct OllamaStatus {
    pub running: bool,
    pub model_installed: bool,
    pub model: String,
}

/// A client that never uses HTTP(S)_PROXY / ALL_PROXY: meeting text must not leave the Mac
/// through an environment proxy (rule 3).
fn client(read_timeout: Duration) -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder().no_proxy().connect_timeout(Duration::from_secs(5)).read_timeout(read_timeout).build()?)
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
    }
    let down = || OllamaStatus { running: false, model_installed: false, model: MODEL.to_owned() };
    let Ok(client) = client(Duration::from_secs(2)) else { return down() };
    let tags = client.get(format!("{BASE}/api/tags")).timeout(Duration::from_secs(2)).send().await;
    match tags {
        Ok(response) => {
            let installed = response.json::<Tags>().await.map(|t| t.models.iter().any(|m| m.name == MODEL)).unwrap_or(false);
            OllamaStatus { running: true, model_installed: installed, model: MODEL.to_owned() }
        }
        Err(_) => down(),
    }
}

/// Asks Ollama to drop Qwen from memory now (before Whisper loads). Best effort.
pub async fn unload() {
    let body = serde_json::json!({ "model": MODEL, "keep_alive": 0 });
    if let Ok(client) = client(Duration::from_secs(10)) {
        let _ = client.post(format!("{BASE}/api/generate")).timeout(Duration::from_secs(10)).json(&body).send().await;
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
}

/// Waits for the AI queue, checks Ollama, and unloads Whisper (never two models at once).
pub async fn session<'a>(app: &'a AppHandle, pool: SqlitePool, meeting_id: Option<&str>) -> Result<TextSession<'a>> {
    let turn = app.state::<AiQueue>().inner().acquire().await;
    let ollama = status().await;
    if !ollama.running {
        bail!(AppError::new("ollama_unavailable", "Ollama isn't running — open the Ollama app and try again"));
    }
    if !ollama.model_installed {
        bail!(AppError::new("ollama_model_missing", format!("Ollama doesn't have {MODEL} yet (run: ollama pull {MODEL})")));
    }
    app.state::<WhisperEngine>().unload_now();
    Ok(TextSession { _turn: turn, pool, meeting_id: meeting_id.map(str::to_owned) })
}

#[cfg(test)]
impl<'a> TextSession<'a> {
    /// A session for tests: the caller supplies the queue turn (no AppHandle needed).
    pub fn for_test(turn: MutexGuard<'a, ()>, pool: SqlitePool) -> Self {
        Self { _turn: turn, pool, meeting_id: None }
    }
}

impl TextSession<'_> {
    /// One streamed `/api/chat` call. `on_token` gets the running count of generated tokens.
    /// Errors with `context_overflow` if the prompt filled the context (Ollama truncates silently).
    pub async fn complete(&self, request: Request, cancel: &AtomicBool, mut on_token: impl FnMut(u32)) -> Result<Completion> {
        if cancel.load(Ordering::Relaxed) {
            bail!(AppError::new("cancelled", "Summary cancelled"));
        }
        let started = Instant::now();
        let result = self.stream(request, cancel, &mut on_token).await;
        // Ollama truncates an over-long prompt silently; treat a full context as an overflow,
        // but keep the token counts in the log row that explains it.
        let overflow = matches!(&result, Ok(c) if c.prompt_tokens + 16 >= NUM_CTX);
        let (status, error, prompt_tokens, output_tokens) = match &result {
            Ok(c) if overflow => ("error", Some("context_overflow".to_owned()), Some(c.prompt_tokens), Some(c.output_tokens)),
            Ok(c) => ("ok", None, Some(c.prompt_tokens), Some(c.output_tokens)),
            Err(err) if err.downcast_ref::<AppError>().is_some_and(|e| e.code == "cancelled") => ("cancelled", None, None, None),
            Err(err) => ("error", Some(format!("{err:#}")), None, None),
        };
        let entry = AiLogEntry {
            kind: "complete",
            model: MODEL,
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
            bail!(AppError::new("context_overflow", "The prompt filled the model's context"));
        }
        result
    }

    async fn stream(&self, request: Request, cancel: &AtomicBool, on_token: &mut impl FnMut(u32)) -> Result<Completion> {
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
            "model": MODEL,
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
            bail!(AppError::new("ollama_error", format!("Ollama returned {status}: {text}")));
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
                let chunk: Chunk = serde_json::from_slice(&line).context("reading Ollama's reply")?;
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
            bail!(AppError::new("output_truncated", "The answer hit its length limit before finishing"));
        }
        Ok(Completion { content: strip_think(&content), prompt_tokens, output_tokens })
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
