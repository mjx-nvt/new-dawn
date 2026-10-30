//! The feedback ledger (D-049): every decision the owner makes about what the AI wrote, kept as
//! plain text, append-only — what later prompts learn from (examples), and what will show whether
//! an agent and its model are getting better (0.8.6). Writing to it never fails the owner's action.

use serde_json::Value;
use sqlx::SqlitePool;

/// Which AI wrote what the owner judged (the same ids as `agents::Agent`, so Workspace → Agents
/// counts them).
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub enum Source {
    /// Whisper (and the vocabulary's automatic corrections).
    Transcript,
    Summary,
    Naming,
    Email,
    Tasks,
}

/// The owner's decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub enum Kind {
    /// A transcript line rewritten by hand.
    LineFixed,
    /// A vocabulary correction undone: Whisper's text was right.
    CorrectionUndone,
    /// The Corrections card: what the owner said was wrong.
    Correction,
    /// A summary section rewritten by hand (a tick is not an edit).
    SectionEdited,
    /// A name the naming check doubted: the spelling the owner chose, or kept as heard.
    NameConfirmed,
    NameKept,
    /// Who attended, renamed or moved in or out of the minutes.
    AttendeesChanged,
    /// An action item the owner sent to Tasks themselves.
    TaskAdded,
    /// A task the AI added: reworded, or deleted.
    TaskReworded,
    TaskDeleted,
    /// The email written again.
    EmailRewritten,
}

pub struct Feedback<'a> {
    pub meeting_id: Option<&'a str>,
    pub source: Source,
    pub kind: Kind,
    pub before: &'a str,
    pub after: &'a str,
    /// The section, the model, the reason… (`{}` when none).
    pub context: Value,
}

/// Appends one decision. Best effort: a ledger that can't be written costs the learning, never
/// the owner's edit.
pub async fn record(pool: &SqlitePool, entry: Feedback<'_>) {
    let result = sqlx::query(
        "INSERT INTO feedback (id, meeting_id, agent, kind, before_text, after_text, context_json) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(uuid::Uuid::now_v7().to_string())
    .bind(entry.meeting_id)
    .bind(entry.source)
    .bind(entry.kind)
    .bind(entry.before)
    .bind(entry.after)
    .bind(entry.context.to_string())
    .execute(pool)
    .await;
    if let Err(err) = result {
        tracing::warn!("could not record feedback: {err}");
    }
}
