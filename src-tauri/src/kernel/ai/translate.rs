//! Translation on this Mac (D-046): Apple's Translation framework, through the bundled
//! `newdawn-translate` helper (`helpers/translate.swift`). On-device — the app makes no network
//! call; macOS keeps the language models (Apple Intelligence's where there is one). The third AI
//! capability beside `transcribe()` and `complete()`; it uses neither Ollama nor Whisper, so it
//! holds no AI-queue turn.

use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tokio::io::AsyncWriteExt;

use crate::error::AppError;
use crate::kernel::ai::log::{self as ai_log, AiLogEntry};

/// The helper's name beside the app's binary (Tauri `externalBin`).
const HELPER: &str = "newdawn-translate";
/// What `ai_log` calls this engine (`kind` stays `complete`: the table allows only two kinds).
pub const MODEL: &str = "apple-translation";

/// The languages a summary can be read in besides English (0.7.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SummaryLang {
    Vi,
    Fr,
    De,
    Ja,
    Ko,
    Zh,
    Es,
}

impl SummaryLang {
    /// How it is stored ("vi", "zh", …): its serde name.
    pub fn key(self) -> &'static str {
        match self {
            Self::Vi => "vi",
            Self::Fr => "fr",
            Self::De => "de",
            Self::Ja => "ja",
            Self::Ko => "ko",
            Self::Zh => "zh",
            Self::Es => "es",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        [
            Self::Vi,
            Self::Fr,
            Self::De,
            Self::Ja,
            Self::Ko,
            Self::Zh,
            Self::Es,
        ]
        .into_iter()
        .find(|l| l.key() == key)
    }

    /// The language as Apple's Translation framework names it (BCP-47).
    pub fn code(self) -> &'static str {
        match self {
            Self::Zh => "zh-Hans",
            other => other.key(),
        }
    }
}

fn unavailable(why: &str) -> anyhow::Error {
    AppError::new(
        "translation_unavailable",
        format!(
            "Translation isn't available on this Mac ({why}). It needs macOS 26 with Apple \
             Intelligence on, or the language downloaded in System Settings → General → Language \
             & Region → Translation Languages."
        ),
    )
    .into()
}

/// The helper: in the app bundle beside the binary; a test runs from `target/<profile>/deps`.
fn helper() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("finding the app")?;
    exe.ancestors()
        .skip(1)
        .take(2)
        .map(|dir| dir.join(HELPER))
        .find(|path| path.exists())
        .ok_or_else(|| unavailable("Translation isn't part of this build"))
}

/// Runs the helper with `args` and `input` (JSON) on stdin; its JSON answer. It gets 90 s (Apple's
/// model loading for a language pair it hasn't used lately) plus 4 s for each of `items` (about a
/// second a line once loaded), then it is stopped. Its stderr
/// is a short reason, never the text (D-026).
async fn run(args: &[&str], input: Vec<u8>, items: usize) -> Result<Vec<u8>> {
    let mut child = tokio::process::Command::new(helper()?)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .context("starting the translation helper")?;
    let mut stdin = child.stdin.take().context("the helper's input")?;
    let limit = Duration::from_secs(90 + 4 * items as u64);
    // Written while its answer is read; a helper that stops early gives its reason, not a
    // broken pipe.
    tauri::async_runtime::spawn(async move {
        let _ = stdin.write_all(&input).await;
    });
    let out = tokio::time::timeout(limit, child.wait_with_output())
        .await
        .map_err(|_| unavailable("Translation took too long"))??;
    if !out.status.success() {
        let reason = String::from_utf8_lossy(&out.stderr);
        bail!(unavailable(reason.trim()));
    }
    Ok(out.stdout)
}

/// `texts` from `from` into `to`, one answer per text (an empty text stays empty), and one
/// `ai_log` row for `agent`.
pub async fn translate(
    pool: &SqlitePool,
    meeting_id: Option<&str>,
    agent: &'static str,
    from: &str,
    to: &str,
    texts: &[String],
) -> Result<Vec<String>> {
    let started = Instant::now();
    let result: Result<Vec<String>> = async {
        let out = run(
            &["translate", from, to],
            serde_json::to_vec(texts)?,
            texts.len(),
        )
        .await?;
        let translated: Vec<String> =
            serde_json::from_slice(&out).context("reading the translation")?;
        if translated.len() != texts.len() {
            bail!(unavailable("The translation came back incomplete"));
        }
        Ok(translated)
    }
    .await;
    let entry = AiLogEntry {
        kind: "complete",
        agent: Some(agent),
        model: MODEL,
        meeting_id,
        duration_ms: started.elapsed().as_millis() as u32,
        status: if result.is_ok() { "ok" } else { "error" },
        error: result.as_ref().err().map(|e| format!("{e:#}")),
        prompt_tokens: None,
        output_tokens: None,
    };
    if let Err(err) = ai_log::record(pool, entry).await {
        tracing::warn!("couldn't log the translation: {err:#}");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_languages_round_trip() {
        for lang in [SummaryLang::Vi, SummaryLang::Zh, SummaryLang::Es] {
            assert_eq!(SummaryLang::from_key(lang.key()), Some(lang));
        }
        assert_eq!(SummaryLang::Zh.code(), "zh-Hans");
        assert_eq!(SummaryLang::from_key("en"), None);
    }

    /// Through the bundled helper with Apple's models (macOS 26, the languages on this Mac):
    /// `cargo test translates_on_this_mac -- --ignored`.
    #[test]
    #[ignore]
    fn translates_on_this_mac() {
        tauri::async_runtime::block_on(async {
            let pool = SqlitePool::connect("sqlite::memory:").await.unwrap(); // no ai_log: only warns
            let texts = ["Ngày mai mình họp lúc 9 giờ.".to_owned(), String::new()];
            let out = translate(&pool, None, "test", "vi", "en", &texts)
                .await
                .unwrap();
            assert!(out[0].to_lowercase().contains("tomorrow"), "{}", out[0]);
            assert_eq!(out[1], "");
            let fr = translate(
                &pool,
                None,
                "test",
                "en",
                SummaryLang::Fr.code(),
                &["Good morning".to_owned()],
            )
            .await
            .unwrap();
            assert!(fr[0].to_lowercase().contains("bonjour"), "{}", fr[0]);
        });
    }
}
