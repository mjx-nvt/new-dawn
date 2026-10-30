//! The feedback ledger (D-049): every decision the owner makes about what the AI wrote, kept as
//! plain text, append-only — what later prompts learn from (examples), and what will show whether
//! an agent and its model are getting better (0.8.6). Writing to it never fails the owner's action.

use anyhow::Result;
use serde::Serialize;
use serde_json::Value;
use specta::Type;
use sqlx::SqlitePool;

use crate::modules::agents::Agent;

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
    /// The Projects expert (D-054).
    Projects,
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
    /// A project update the expert proposed: applied, or dismissed.
    UpdateApplied,
    UpdateDismissed,
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

/// How the owner judged an agent's work on one model (D-052): of `judged` pieces of work —
/// summaries, emails, names the check doubted, tasks it added — `changed` were rewritten by hand
/// (an email: written again on request; a name: changed from what was heard, so the doubt was
/// right) and `dropped` deleted (tasks); the rest stood. `model` is "" when it isn't known. A
/// summary, its email or a task counts once it is three days old (time to be read), and only the
/// edits made since the AI last wrote it; the ledger exists since 0.8.1.
#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct Judged {
    pub model: String,
    pub judged: u32,
    pub changed: u32,
    pub dropped: u32,
}

/// The summaries the AI wrote, by model, with when it last wrote each (its newest successful run,
/// else when the summary was made) — for the summary and email numbers.
macro_rules! written {
    () => {
        "WITH written AS (SELECT s.meeting_id, s.model, s.email_md, coalesce((SELECT max(a.created_at) \
         FROM ai_log a WHERE a.meeting_id = s.meeting_id AND a.agent = 'summary' AND a.status = 'ok'), \
         s.created_at) AS at FROM summary s WHERE s.ai_sections_json IS NOT NULL AND s.model IS NOT NULL) "
    };
}

pub async fn numbers(pool: &SqlitePool, agent: Agent) -> Result<Vec<Judged>> {
    let sql = match agent {
        Agent::Summary => concat!(
            written!(),
            "SELECT w.model AS model, count(*) AS judged, coalesce(sum(EXISTS (SELECT 1 FROM feedback f \
             WHERE f.agent = 'summary' AND f.kind = 'section_edited' AND f.meeting_id = w.meeting_id \
             AND f.created_at >= w.at)), 0) AS changed, 0 AS dropped FROM written w \
             WHERE w.at < strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-3 days') \
             GROUP BY w.model ORDER BY judged DESC"
        ),
        Agent::Email => concat!(
            written!(),
            "SELECT w.model AS model, count(*) AS judged, coalesce(sum(EXISTS (SELECT 1 FROM feedback f \
             WHERE f.agent = 'email' AND f.kind = 'email_rewritten' AND f.meeting_id = w.meeting_id \
             AND f.created_at >= w.at)), 0) AS changed, 0 AS dropped FROM written w \
             WHERE w.email_md IS NOT NULL AND w.at < strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-3 days') \
             GROUP BY w.model ORDER BY judged DESC"
        ),
        Agent::Naming => {
            "SELECT coalesce(s.model, '') AS model, count(*) AS judged, \
             coalesce(sum(f.kind = 'name_confirmed'), 0) AS changed, 0 AS dropped \
             FROM feedback f LEFT JOIN summary s ON s.meeting_id = f.meeting_id \
             WHERE f.agent = 'naming' GROUP BY 1 ORDER BY judged DESC"
        }
        // A reworded task is found by its id (0.8.4 on) or, before, by its meeting and new title.
        Agent::Tasks => {
            "SELECT coalesce(s.model, '') AS model, count(*) AS judged, \
             coalesce(sum(t.deleted_at IS NULL AND EXISTS (SELECT 1 FROM feedback f WHERE f.agent = 'tasks' \
               AND f.kind = 'task_reworded' AND (json_extract(f.context_json, '$.task') = t.id \
                    OR (f.meeting_id IS t.meeting_id AND f.after_text = t.title)))), 0) AS changed, \
             coalesce(sum(t.deleted_at IS NOT NULL), 0) AS dropped \
             FROM task t LEFT JOIN summary s ON s.meeting_id = t.meeting_id \
             WHERE t.origin = 'auto' AND t.created_at < strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-3 days') \
             GROUP BY 1 ORDER BY judged DESC"
        }
        // Its proposals the owner decided: applied stood, dismissed dropped.
        Agent::Projects => {
            "SELECT coalesce(model, '') AS model, count(*) AS judged, 0 AS changed, \
             coalesce(sum(status = 'dismissed'), 0) AS dropped FROM project_update \
             WHERE status IN ('applied', 'dismissed') GROUP BY 1 ORDER BY judged DESC"
        }
        _ => return Ok(Vec::new()),
    };
    Ok(sqlx::query_as(sql).fetch_all(pool).await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::hub::WorkspaceHub;

    #[test]
    fn a_task_reworded_then_deleted_counts_once() {
        let root = std::env::temp_dir().join(format!("newdawn-feedback-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            for (id, deleted) in [("t1", false), ("t2", true), ("t3", false)] {
                sqlx::query(
                    "INSERT INTO task (id, title, origin, deleted_at, created_at) VALUES (?, ?, 'auto', \
                     CASE WHEN ? THEN strftime('%Y-%m-%dT%H:%M:%fZ', 'now') END, \
                     strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-5 days'))",
                )
                .bind(id)
                .bind(format!("Task {id}"))
                .bind(deleted)
                .execute(&pool)
                .await
                .unwrap();
            }
            for (kind, task) in [
                ("task_reworded", "t2"),
                ("task_deleted", "t2"),
                ("task_reworded", "t3"),
            ] {
                record(
                    &pool,
                    Feedback {
                        meeting_id: None,
                        source: Source::Tasks,
                        kind: if kind == "task_deleted" {
                            Kind::TaskDeleted
                        } else {
                            Kind::TaskReworded
                        },
                        before: "Task",
                        after: "",
                        context: serde_json::json!({ "task": task }),
                    },
                )
                .await;
            }
            let got = numbers(&pool, Agent::Tasks).await.unwrap();
            assert_eq!(got.len(), 1);
            assert_eq!(
                (
                    got[0].model.as_str(),
                    got[0].judged,
                    got[0].changed,
                    got[0].dropped
                ),
                ("", 3, 1, 1),
                "t2 counts as deleted only; t3 as reworded"
            );
            hub.close_all().await;
        });
        let _ = std::fs::remove_dir_all(root);
    }
}
