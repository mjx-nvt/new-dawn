//! Shared setup for the model comparison (`cargo test eval_ -- --ignored --nocapture --test-threads 1`):
//! a temporary VIFC workspace with the owner, the people and vocabulary of
//! `NEWDAWN_EVAL_PEOPLE_SQL`, and a meeting holding the transcript in `NEWDAWN_EVAL_SEGMENTS`
//! (written by `eval_transcribe`). Each test runs the app's own code with the text model in
//! `NEWDAWN_EVAL_MODEL` and writes what it got, with the AI log's timings and tokens, to
//! `NEWDAWN_EVAL_OUT`. Meeting content stays in those files: point them outside the repo.

use std::path::PathBuf;

use serde_json::{Value, json};
use sqlx::SqlitePool;

use super::{Meeting, MeetingStatus, NewMeeting, TranscriptSegment};
use crate::kernel::ai::ollama;
use crate::kernel::ai::whisper::{Language, Segment};
use crate::kernel::hub::WorkspaceHub;
use crate::kernel::profile::Owner;

pub fn var(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
}

/// The model under test; it becomes the app's model, which every test session uses.
pub fn use_model() -> String {
    let model = var("NEWDAWN_EVAL_MODEL").expect("set NEWDAWN_EVAL_MODEL (e.g. qwen3.5:4b)");
    ollama::use_model(&model);
    model
}

/// The owner as the user's VIFC workspace had them.
pub fn owner() -> Owner {
    Owner {
        name: "Tien Nguyen".into(),
        title: "Growth Manager".into(),
        organisation: "VIFC".into(),
    }
}

pub struct Setup {
    pub root: PathBuf,
    pub hub: WorkspaceHub,
    pub pool: SqlitePool,
    pub meeting: Meeting,
    pub segments: Vec<TranscriptSegment>,
}

/// The workspace, its people and terms, and a meeting ("m1", language auto, a discussion)
/// holding the transcript at `segments` (none: an empty meeting).
pub async fn setup(segments: Option<&str>) -> Setup {
    // The app's warnings (a summary that didn't parse, a retry) go to the test's output.
    let _ = tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_test_writer()
        .try_init();
    let root = std::env::temp_dir().join(format!("newdawn-eval-{}", uuid::Uuid::now_v7()));
    let hub = WorkspaceHub::open(root.clone()).await.unwrap();
    hub.create_workspace("VIFC", "vifc").await.unwrap();
    let pool = hub.pool("vifc").unwrap();
    if let Some(sql) = var("NEWDAWN_EVAL_PEOPLE_SQL") {
        let sql = std::fs::read_to_string(sql).unwrap();
        // The restore script written for this comparison, not user input.
        sqlx::raw_sql(sqlx::AssertSqlSafe(sql))
            .execute(&pool)
            .await
            .unwrap();
    }
    let title = var("NEWDAWN_EVAL_TITLE").unwrap_or_else(|| "Meeting".into());
    super::create(
        &pool,
        NewMeeting {
            id: "m1",
            meeting_type_id: None,
            title: &title,
            held_at_unix: 1_793_160_000, // Wed 28 Oct 2026, 03:20 UTC
            language: Language::Auto,
            audio_path: "/nowhere.m4a",
            status: MeetingStatus::Transcribed,
        },
    )
    .await
    .unwrap();
    if let Some(path) = segments {
        let segments: Vec<Segment> =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let duration = segments.last().map_or(0, |s| s.end_ms);
        super::save_transcript(&pool, "m1", &segments, &[], duration)
            .await
            .unwrap();
    }
    let meeting = super::get(&pool, "m1").await.unwrap();
    let segments = super::transcript(&pool, "m1").await.unwrap();
    Setup {
        root,
        hub,
        pool,
        meeting,
        segments,
    }
}

impl Setup {
    pub async fn finish(self) {
        self.hub.close_all().await;
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// One `ai_log` row: agent, model, status, milliseconds, tokens in and out.
type Call = (
    Option<String>,
    String,
    String,
    Option<i64>,
    Option<i64>,
    Option<i64>,
);

/// Every model call the run made: agent, status, seconds, tokens in and out.
pub async fn usage(pool: &SqlitePool) -> Value {
    let rows: Vec<Call> = sqlx::query_as(
        "SELECT agent, model, status, duration_ms, prompt_tokens, output_tokens FROM ai_log \
             WHERE kind = 'complete' ORDER BY created_at",
    )
    .fetch_all(pool)
    .await
    .unwrap();
    Value::Array(
        rows.into_iter()
            .map(|(agent, model, status, ms, input, output)| {
                json!({ "agent": agent, "model": model, "status": status,
                        "seconds": ms.map(|m| m as f64 / 1000.0), "prompt_tokens": input,
                        "output_tokens": output })
            })
            .collect(),
    )
}

/// Writes `value` to `NEWDAWN_EVAL_OUT` (pretty JSON) and says where.
pub fn write(value: &Value) {
    let out = var("NEWDAWN_EVAL_OUT").expect("set NEWDAWN_EVAL_OUT (a .json path)");
    std::fs::write(&out, serde_json::to_string_pretty(value).unwrap()).unwrap();
    eprintln!("wrote {out}");
}
