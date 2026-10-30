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
    Projects,
}

impl Agent {
    /// The agents that talk to the model. The minutes are filled from the summary by rules
    /// and the Word template (D-024), so `Minutes` has no guide to edit.
    pub const ALL: [Agent; 8] = [
        Agent::Summary,
        Agent::Naming,
        Agent::Email,
        Agent::Tasks,
        Agent::Corrections,
        Agent::Translate,
        Agent::Live,
        Agent::Projects,
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
            Agent::Projects => "projects",
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
            Agent::Projects => "Projects",
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
                "Translates each finished line of a live session between Vietnamese and English."
            }
            Agent::Projects => {
                "After each summary, finds the projects the meeting touched and proposes their updates for you to apply."
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
            Agent::Projects => include_str!("../../agents/projects.md"),
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
    /// The owner's edits and decisions about this agent's work, in the feedback ledger (D-049).
    pub feedback: u32,
    /// How the owner judged its work, per model (D-052).
    pub numbers: Vec<crate::modules::feedback::Judged>,
    /// It learns from the owner's edits ("What I learned").
    pub learns: bool,
    /// Lines proposed for its guide, waiting for the owner.
    pub lessons: Vec<crate::modules::lessons::Lesson>,
    /// Lines the owner approved, which every prompt gets after the guide.
    pub learned: Vec<String>,
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

/// The heading the approved lines go under in every prompt (D-052).
pub const LEARNED_HEADING: &str =
    "## Learned from your edits (these win over the rest of this guide)";

/// The guide in force: the workspace's file, else the default — then the lines the owner approved
/// from "What I learned", which win over the rest.
pub fn guide(folder: &Path, agent: Agent) -> String {
    let own = std::fs::read_to_string(file(folder, agent))
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| agent.default_guide().trim().to_owned());
    let learned = learned(folder, agent);
    if learned.is_empty() {
        return own;
    }
    let lines: Vec<String> = learned.iter().map(|l| format!("- {l}")).collect();
    format!("{own}\n\n{LEARNED_HEADING}\n{}", lines.join("\n"))
}

/// Approved lines live beside the guide, not in it: Reset to default keeps them, and the bundled
/// default keeps reaching a workspace that approved some.
fn learned_file(folder: &Path, agent: Agent) -> PathBuf {
    folder
        .join("brand")
        .join("agents")
        .join(format!("{}.learned.md", agent.id()))
}

/// The lines the owner approved for `agent`, in the order approved.
pub fn learned(folder: &Path, agent: Agent) -> Vec<String> {
    std::fs::read_to_string(learned_file(folder, agent))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.strip_prefix("- "))
        .map(|l| l.trim().to_owned())
        .filter(|l| !l.is_empty())
        .collect()
}

/// Saves the approved lines (temp file, fsync, rename); none = no file.
pub fn set_learned(folder: &Path, agent: Agent, lines: &[String]) -> Result<()> {
    let path = learned_file(folder, agent);
    if lines.is_empty() {
        return match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
            _ => Ok(()),
        };
    }
    std::fs::create_dir_all(path.parent().context("guide has no folder")?)?;
    let body: String = lines.iter().map(|l| format!("- {l}\n")).collect();
    let tmp = path.with_extension("md.tmp");
    std::fs::write(&tmp, body)?;
    std::fs::File::open(&tmp)?.sync_all()?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

pub async fn list(pool: &SqlitePool, folder: &Path) -> Result<Vec<AgentGuide>> {
    let mut out = Vec::with_capacity(Agent::ALL.len());
    for agent in Agent::ALL {
        let learns = crate::modules::lessons::LEARNERS.contains(&agent);
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
        let (feedback,): (u32,) = sqlx::query_as("SELECT count(*) FROM feedback WHERE agent = ?")
            .bind(agent.id())
            .fetch_one(pool)
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
            feedback,
            numbers: crate::modules::feedback::numbers(pool, agent).await?,
            learns,
            lessons: if learns {
                crate::modules::lessons::proposed(pool, agent).await?
            } else {
                Vec::new()
            },
            learned: learned(folder, agent),
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
        // Approved lines come after the guide, under their heading, and survive a reset.
        set_learned(
            dir.path(),
            Agent::Summary,
            &["Keep the overview short.".into()],
        )
        .unwrap();
        reset(dir.path(), Agent::Summary).unwrap();
        let told = guide(dir.path(), Agent::Summary);
        assert!(told.starts_with("# Summary agent"));
        assert!(told.ends_with(&format!("{LEARNED_HEADING}\n- Keep the overview short.")));
        assert_eq!(
            learned(dir.path(), Agent::Summary),
            ["Keep the overview short."]
        );
        set_learned(dir.path(), Agent::Summary, &[]).unwrap();
        assert!(!guide(dir.path(), Agent::Summary).contains(LEARNED_HEADING));
        for agent in Agent::ALL {
            assert!(!agent.default_guide().trim().is_empty());
        }
    }
}
