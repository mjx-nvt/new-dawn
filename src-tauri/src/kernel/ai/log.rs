use anyhow::Result;
use sqlx::SqlitePool;
use uuid::Uuid;

/// One finished AI call, written to the workspace's `ai_log`.
pub struct AiLogEntry<'a> {
    pub kind: &'a str, // transcribe | complete
    /// summary | naming | email | minutes — which agent asked (None for transcription).
    pub agent: Option<&'a str>,
    pub model: &'a str,
    pub meeting_id: Option<&'a str>,
    pub duration_ms: u32,
    pub status: &'a str, // ok | error | cancelled
    pub error: Option<String>,
    pub prompt_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}

pub async fn record(pool: &SqlitePool, entry: AiLogEntry<'_>) -> Result<()> {
    sqlx::query(
        "INSERT INTO ai_log (id, kind, model, meeting_id, duration_ms, status, error, prompt_tokens, output_tokens, agent) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::now_v7().to_string())
    .bind(entry.kind)
    .bind(entry.model)
    .bind(entry.meeting_id)
    .bind(entry.duration_ms)
    .bind(entry.status)
    .bind(entry.error)
    .bind(entry.prompt_tokens)
    .bind(entry.output_tokens)
    .bind(entry.agent)
    .execute(pool)
    .await?;
    Ok(())
}
