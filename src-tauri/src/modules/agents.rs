//! The AI agents and their guides: one Markdown file per agent in the workspace's
//! `brand/agents/` folder (the bundled default until the user changes it). The guide is the
//! human-tunable part of a prompt; the code keeps the data fence and the JSON shape (D-035).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;

use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Agent {
    Summary,
    Naming,
    Email,
    Tasks,
    Minutes,
    Corrections,
    Translate,
    Live,
}

impl Agent {
    /// The agents that talk to the model. The minutes are filled from the summary by rules
    /// and the Word template (D-024), so `Minutes` has no guide to edit.
    pub const ALL: [Agent; 7] = [
        Agent::Summary,
        Agent::Naming,
        Agent::Email,
        Agent::Tasks,
        Agent::Corrections,
        Agent::Translate,
        Agent::Live,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Agent::Summary => "summary",
            Agent::Naming => "naming",
            Agent::Email => "email",
            Agent::Tasks => "tasks",
            Agent::Minutes => "minutes",
            Agent::Corrections => "corrections",
            Agent::Translate => "translate",
            Agent::Live => "live",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Agent::Summary => "Summary",
            Agent::Naming => "Naming",
            Agent::Email => "Email",
            Agent::Tasks => "Tasks",
            Agent::Minutes => "Minutes",
            Agent::Corrections => "Corrections",
            Agent::Translate => "Translation",
            Agent::Live => "Live translation",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Agent::Summary => {
                "Writes the executive summary from the transcript and your notes, right after transcription."
            }
            Agent::Naming => {
                "Checks the people in a new summary against the people memory and asks you to confirm the doubtful ones."
            }
            Agent::Email => {
                "Writes the cover note that goes with the minutes: key points, decisions, actions."
            }
            Agent::Tasks => {
                "Finds the action items that are yours after each summary and adds them to Tasks."
            }
            Agent::Minutes => "Fills the Word template from the summary.",
            Agent::Corrections => {
                "Applies what you tell it is wrong to the summary, email and transcript, and remembers the names and terms."
            }
            Agent::Translate => "Translates your slogan into Vietnamese for your documents.",
            Agent::Live => {
                "Translates each finished line of a live session from Vietnamese to English."
            }
        }
    }

    /// The guide shipped with the app.
    pub fn default_guide(self) -> &'static str {
        match self {
            Agent::Summary => include_str!("../../agents/summary.md"),
            Agent::Naming => include_str!("../../agents/naming.md"),
            Agent::Email => include_str!("../../agents/email.md"),
            Agent::Tasks => include_str!("../../agents/tasks.md"),
            Agent::Minutes => include_str!("../../agents/minutes.md"),
            Agent::Corrections => include_str!("../../agents/corrections.md"),
            Agent::Translate => include_str!("../../agents/translate.md"),
            Agent::Live => include_str!("../../agents/live.md"),
        }
    }
}

/// The last finished call an agent made, from `ai_log`.
#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct AgentRun {
    pub model: String,
    pub status: String,
    pub duration_ms: Option<u32>,
    pub prompt_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub created_at: String,
}

/// How one model has done for an agent (successful runs).
#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct ModelStats {
    pub model: String,
    pub runs: u32,
    pub avg_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct AgentGuide {
    pub agent: Agent,
    pub label: String,
    pub description: String,
    pub guide: String,
    /// The workspace has its own text (else the bundled default is shown).
    pub customised: bool,
    pub last_run: Option<AgentRun>,
    /// The model this agent uses (None = the app's selected model).
    pub model: Option<String>,
    pub by_model: Vec<ModelStats>,
}

fn file(folder: &Path, agent: Agent) -> PathBuf {
    folder
        .join("brand")
        .join("agents")
        .join(format!("{}.md", agent.id()))
}

/// How long a call of `agent` on `model` usually takes and how many tokens it writes (mean of the
/// last ten successful runs on that model, or on any model before its first; 0 with no history).
/// Feeds the progress estimate (D-036).
pub async fn typical(pool: &SqlitePool, agent: &str, model: &str) -> (u32, u32) {
    let row: Result<(Option<f64>, Option<f64>), _> = sqlx::query_as(
        "SELECT avg(duration_ms), avg(output_tokens) FROM (SELECT duration_ms, output_tokens FROM ai_log \
         WHERE agent = ?1 AND status = 'ok' AND (model = ?2 OR NOT EXISTS \
           (SELECT 1 FROM ai_log WHERE agent = ?1 AND status = 'ok' AND model = ?2)) \
         ORDER BY created_at DESC LIMIT 10)",
    )
    .bind(agent)
    .bind(model)
    .fetch_one(pool)
    .await;
    match row {
        Ok((ms, tokens)) => (ms.unwrap_or(0.0) as u32, tokens.unwrap_or(0.0) as u32),
        Err(err) => {
            tracing::warn!("could not read typical {agent} run: {err:#}");
            (0, 0)
        }
    }
}

fn model_key(agent: Agent) -> String {
    format!("agent_model.{}", agent.id())
}

/// The model `agent` runs on in this workspace (None = the app's selected model).
pub async fn model_for(pool: &SqlitePool, agent: Agent) -> Option<String> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM setting WHERE key = ?")
        .bind(model_key(agent))
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();
    row.map(|(v,)| v).filter(|v| !v.is_empty())
}

/// Chooses the model for `agent` (None = follow the app's selected model).
pub async fn set_model(pool: &SqlitePool, agent: Agent, model: Option<&str>) -> Result<()> {
    match model.map(str::trim).filter(|m| !m.is_empty()) {
        Some(m) => {
            if !crate::kernel::ai::ollama::valid_name(m) {
                anyhow::bail!(AppError::invalid("That isn't a model name"));
            }
            sqlx::query(
                "INSERT INTO setting (key, value) VALUES (?, ?) ON CONFLICT (key) DO UPDATE SET value = excluded.value, \
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
            )
            .bind(model_key(agent))
            .bind(m)
            .execute(pool)
            .await?;
        }
        None => {
            sqlx::query("DELETE FROM setting WHERE key = ?")
                .bind(model_key(agent))
                .execute(pool)
                .await?;
        }
    }
    Ok(())
}

/// The guide in force: the workspace's file, else the default.
pub fn guide(folder: &Path, agent: Agent) -> String {
    std::fs::read_to_string(file(folder, agent))
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| agent.default_guide().trim().to_owned())
}

pub async fn list(pool: &SqlitePool, folder: &Path) -> Result<Vec<AgentGuide>> {
    let mut out = Vec::with_capacity(Agent::ALL.len());
    for agent in Agent::ALL {
        let custom = std::fs::read_to_string(file(folder, agent)).ok();
        let last: Option<AgentRun> = sqlx::query_as(
            "SELECT model, status, duration_ms, prompt_tokens, output_tokens, created_at FROM ai_log \
             WHERE agent = ? ORDER BY created_at DESC LIMIT 1",
        )
        .bind(agent.id())
        .fetch_optional(pool)
        .await?;
        let by_model: Vec<ModelStats> = sqlx::query_as(
            "SELECT model, count(*) AS runs, avg(duration_ms) AS avg_ms FROM ai_log \
             WHERE agent = ? AND status = 'ok' GROUP BY model ORDER BY runs DESC",
        )
        .bind(agent.id())
        .fetch_all(pool)
        .await?;
        out.push(AgentGuide {
            agent,
            label: agent.label().to_owned(),
            description: agent.description().to_owned(),
            guide: custom
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| agent.default_guide().trim())
                .to_owned(),
            customised: custom.is_some(),
            last_run: last,
            model: model_for(pool, agent).await,
            by_model,
        });
    }
    Ok(out)
}

/// Saves the workspace's own guide (temp file, fsync, rename). Empty text = back to the default.
pub fn set_guide(folder: &Path, agent: Agent, text: &str) -> Result<()> {
    let text = text.trim();
    if text.chars().count() > 20_000 {
        anyhow::bail!(AppError::invalid("A guide is at most 20,000 characters"));
    }
    let path = file(folder, agent);
    if text.is_empty() {
        return reset(folder, agent);
    }
    std::fs::create_dir_all(path.parent().context("guide has no folder")?)?;
    let tmp = path.with_extension("md.tmp");
    std::fs::write(&tmp, format!("{text}\n"))?;
    std::fs::File::open(&tmp)?.sync_all()?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

/// Removes the workspace's text so the bundled default applies again.
pub fn reset(folder: &Path, agent: Agent) -> Result<()> {
    match std::fs::remove_file(file(folder, agent)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);
    impl TempDir {
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn guide_falls_back_to_the_default_and_round_trips() {
        let dir = std::env::temp_dir().join(format!("newdawn-agents-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&dir).unwrap();
        let dir = TempDir(dir);
        assert!(guide(dir.path(), Agent::Summary).starts_with("# Summary agent"));
        set_guide(dir.path(), Agent::Summary, "  Be brief.  ").unwrap();
        assert_eq!(guide(dir.path(), Agent::Summary), "Be brief.");
        set_guide(dir.path(), Agent::Summary, "").unwrap();
        assert!(guide(dir.path(), Agent::Summary).starts_with("# Summary agent"));
        reset(dir.path(), Agent::Summary).unwrap(); // twice is fine
        for agent in Agent::ALL {
            assert!(!agent.default_guide().trim().is_empty());
        }
    }
}
