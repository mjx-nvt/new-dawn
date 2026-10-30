//! Try a model on your meetings (D-050): a candidate speech or text model against the one in use,
//! on the meetings the owner marked as references, in the background. The figures say how fast and
//! how close each comes to the owner's corrected transcript or edited summary; blind pairs — the
//! same stretch or section from both models, unlabelled — let the owner judge, and the tally says
//! which won. Nothing a check writes reaches the meetings themselves.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::{AppHandle, Manager};

use crate::error::AppError;
use crate::kernel::ai::models::{self, SpeechJob};
use crate::kernel::ai::ollama;
use crate::kernel::ai::queue::{AiQueue, Task};
use crate::kernel::ai::whisper::{self, Language};
use crate::kernel::hub::WorkspaceHub;
use crate::kernel::profile;
use crate::modules::agents::{self, Agent};
use crate::modules::meetings::summarise::{self, SectionKind, SummaryDoc};
use crate::modules::meetings::{self, audio};
use crate::modules::{people, vocabulary};

/// Reference meetings a check reads, at most.
const SPEECH_MEETINGS: i64 = 5;
const TEXT_MEETINGS: i64 = 3;
/// Each speech window: 90 s, two per meeting (a third and two thirds in).
const WINDOW_MS: u32 = 90_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum CheckKind {
    /// A Whisper model for meetings.
    Speech,
    /// A text model for summaries.
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum CheckStatus {
    Waiting,
    Running,
    Done,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Pick {
    A,
    B,
    Same,
}

/// One line of figures: what it measures, for the model in use and the candidate.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Figure {
    pub label: String,
    pub baseline: String,
    pub candidate: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MeetingFigures {
    /// So deleting the meeting finds the check holding its words (migration 0013).
    #[serde(default)]
    pub meeting_id: String,
    pub meeting: String,
    pub figures: Vec<Figure>,
}

/// A blind pair as stored: both texts and which model wrote which (never sent to the page).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredPair {
    id: String,
    meeting: String,
    label: String,
    baseline: String,
    candidate: String,
    candidate_is_a: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Stored {
    meetings: Vec<MeetingFigures>,
    pairs: Vec<StoredPair>,
    /// What to keep in mind reading the figures.
    note: String,
}

/// A blind pair as the owner sees it.
#[derive(Debug, Clone, Serialize, Type)]
pub struct Pair {
    pub id: String,
    pub meeting: String,
    pub label: String,
    pub a: String,
    pub b: String,
    pub pick: Option<Pick>,
}

#[derive(Debug, Clone, Default, Serialize, Type)]
pub struct Tally {
    pub candidate: u32,
    pub baseline: u32,
    pub same: u32,
    /// Pairs not judged yet.
    pub left: u32,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct ModelCheck {
    pub id: String,
    pub kind: CheckKind,
    pub baseline: String,
    pub candidate: String,
    pub status: CheckStatus,
    pub progress: u32,
    pub error: Option<String>,
    pub created_at: String,
    pub meetings: Vec<MeetingFigures>,
    pub note: String,
    pub pairs: Vec<Pair>,
    pub tally: Tally,
}

/// The reference meetings, for Settings → Models.
#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct Reference {
    pub id: String,
    pub title: String,
    pub held_on: String,
}

/// Running checks' cancel flags, by check id.
#[derive(Default)]
pub struct ModelChecks(Mutex<HashMap<String, Arc<AtomicBool>>>);

impl ModelChecks {
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Arc<AtomicBool>>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

pub async fn set_reference(pool: &SqlitePool, meeting_id: &str, on: bool) -> Result<()> {
    let done = sqlx::query(
        "UPDATE meeting SET reference = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE id = ?",
    )
    .bind(on)
    .bind(meeting_id)
    .execute(pool)
    .await?;
    if done.rows_affected() == 0 {
        bail!(AppError::not_found("No such meeting"));
    }
    Ok(())
}

pub async fn references(pool: &SqlitePool) -> Result<Vec<Reference>> {
    Ok(sqlx::query_as(
        "SELECT id, title, held_on FROM meeting WHERE reference = 1 ORDER BY held_on DESC",
    )
    .fetch_all(pool)
    .await?)
}

/// The model each kind of check compares with: the Meetings speech model, or the Summary agent's.
async fn in_use(app: &AppHandle, pool: &SqlitePool, kind: CheckKind) -> Result<String> {
    Ok(match kind {
        CheckKind::Speech => models::speech_model(app, SpeechJob::Meetings)
            .await?
            .0
            .id
            .to_owned(),
        CheckKind::Text => agents::model_for(pool, Agent::Summary)
            .await
            .unwrap_or_else(ollama::model),
    })
}

/// References a speech check can use: with their recording and a transcript.
macro_rules! speech_references {
    () => {
        "FROM meeting m WHERE m.reference = 1 AND m.audio_path IS NOT NULL \
         AND EXISTS (SELECT 1 FROM transcript_segment t WHERE t.meeting_id = m.id)"
    };
}
/// References a text check can use: summarised since 0.8.1, so the summary the model in use
/// wrote, before the owner's edits, is kept (D-049).
macro_rules! text_references {
    () => {
        "FROM meeting m JOIN summary s ON s.meeting_id = m.id \
         WHERE m.reference = 1 AND s.ai_sections_json IS NOT NULL"
    };
}

/// Starts a check of `candidate` against the model in use, in the background (it waits for an
/// idle, cool, charged Mac unless the owner says Run now).
pub async fn start(
    app: &AppHandle,
    slug: &str,
    kind: CheckKind,
    candidate: &str,
) -> Result<ModelCheck> {
    let pool = app.state::<WorkspaceHub>().pool(slug)?;
    let baseline = in_use(app, &pool, kind).await?;
    if candidate == baseline {
        bail!(AppError::invalid(
            "That model is the one in use: choose another to compare"
        ));
    }
    match kind {
        CheckKind::Speech => {
            let spec = models::spec(candidate)?;
            if !app
                .state::<models::ModelStore>()
                .dir()
                .join(spec.file)
                .exists()
            {
                bail!(AppError::invalid(format!("Download {} first", spec.label)));
            }
        }
        CheckKind::Text => {
            if !ollama::status()
                .await
                .installed
                .iter()
                .any(|m| m.name == candidate)
            {
                bail!(AppError::invalid(format!(
                    "Ollama doesn't have {candidate} yet"
                )));
            }
        }
    }
    let usable = match kind {
        CheckKind::Speech => concat!("SELECT count(*) ", speech_references!()),
        CheckKind::Text => concat!("SELECT count(*) ", text_references!()),
    };
    let (count,): (i64,) = sqlx::query_as(usable).fetch_one(&pool).await?;
    if count == 0 {
        bail!(AppError::invalid(match kind {
            CheckKind::Speech =>
                "Mark a transcribed meeting with its recording as a reference first (its ⋯ menu)",
            CheckKind::Text =>
                "Mark a meeting summarised with New Dawn 0.8.1 or later as a reference first (its ⋯ menu); for an older one, re-run its summary keeping your edits",
        }));
    }
    // The ten newest are kept: older checks (and the meeting words they hold) go.
    sqlx::query(
        "DELETE FROM model_check WHERE id NOT IN \
         (SELECT id FROM model_check ORDER BY created_at DESC LIMIT 9)",
    )
    .execute(&pool)
    .await?;
    let id = meetings::new_id();
    sqlx::query("INSERT INTO model_check (id, kind, baseline, candidate) VALUES (?, ?, ?, ?)")
        .bind(&id)
        .bind(kind)
        .bind(&baseline)
        .bind(candidate)
        .execute(&pool)
        .await?;
    let cancel = Arc::new(AtomicBool::new(false));
    app.state::<ModelChecks>()
        .lock()
        .insert(id.clone(), cancel.clone());
    let (app2, slug2, id2, candidate2) = (
        app.clone(),
        slug.to_owned(),
        id.clone(),
        candidate.to_owned(),
    );
    tauri::async_runtime::spawn(async move {
        run(&app2, &slug2, &id2, kind, &baseline, &candidate2, cancel).await;
    });
    get(&pool, &id).await
}

/// Stops a check: a held one stops waiting for the Mac (Run now lifts the wait) and ends at once;
/// one this run of the app doesn't know (a launch since) is marked cancelled.
pub async fn cancel(app: &AppHandle, slug: &str, id: &str) -> Result<()> {
    let flag = app.state::<ModelChecks>().lock().get(id).cloned();
    match flag {
        Some(flag) => {
            flag.store(true, Ordering::Relaxed);
            app.state::<AiQueue>().run_now(id);
        }
        None => {
            let pool = app.state::<WorkspaceHub>().pool(slug)?;
            sqlx::query(
                "UPDATE model_check SET status = 'cancelled', \
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
                 WHERE id = ? AND status IN ('waiting', 'running')",
            )
            .bind(id)
            .execute(&pool)
            .await?;
        }
    }
    Ok(())
}

async fn run(
    app: &AppHandle,
    slug: &str,
    id: &str,
    kind: CheckKind,
    baseline: &str,
    candidate: &str,
    cancel: Arc<AtomicBool>,
) {
    let result = async {
        let pool = app.state::<WorkspaceHub>().pool(slug)?;
        set_status(&pool, id, CheckStatus::Running, None).await?;
        let mut stored = match kind {
            CheckKind::Speech => speech(app, slug, &pool, id, baseline, candidate, &cancel).await?,
            CheckKind::Text => text(app, slug, &pool, id, baseline, candidate, &cancel).await?,
        };
        // A meeting deleted while the check ran takes its words with it (migration 0013).
        let mut gone = Vec::new();
        for m in &stored.meetings {
            if meetings::get(&pool, &m.meeting_id).await.is_err() {
                gone.push(m.meeting_id.clone());
            }
        }
        stored.meetings.retain(|m| !gone.contains(&m.meeting_id));
        stored
            .pairs
            .retain(|p| !gone.iter().any(|g| p.id.starts_with(g.as_str())));
        sqlx::query(
            "UPDATE model_check SET status = 'done', progress = 100, result_json = ?, \
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?",
        )
        .bind(serde_json::to_string(&stored)?)
        .bind(id)
        .execute(&pool)
        .await?;
        anyhow::Ok(())
    }
    .await;
    app.state::<ModelChecks>().lock().remove(id);
    app.state::<AiQueue>().forget_now(id);
    if let Err(err) = result {
        let (status, message) = if whisper::is_cancelled(&err) || cancel.load(Ordering::Relaxed) {
            (CheckStatus::Cancelled, None)
        } else {
            tracing::warn!("model check {id} failed: {err:#}");
            (CheckStatus::Failed, Some(AppError::from(err).message))
        };
        if let Ok(pool) = app.state::<WorkspaceHub>().pool(slug) {
            let _ = set_status(&pool, id, status, message.as_deref()).await;
        }
    }
}

async fn set_status(
    pool: &SqlitePool,
    id: &str,
    status: CheckStatus,
    error: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "UPDATE model_check SET status = ?, error = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE id = ?",
    )
    .bind(status)
    .bind(error)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn set_progress(pool: &SqlitePool, id: &str, done: usize, of: usize) {
    let percent = (done * 100 / of.max(1)) as u32;
    let _ = sqlx::query(
        "UPDATE model_check SET progress = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE id = ?",
    )
    .bind(percent)
    .bind(id)
    .execute(pool)
    .await;
}

fn task(id: &str, slug: &str, label: String) -> Task {
    Task {
        id: Some(id.to_owned()),
        workspace_slug: Some(slug.to_owned()),
        ..Task::new(label)
    }
}

/// Windows of a meeting `duration` ms long: the whole of a short one, else two 90-s windows a third
/// and two thirds in.
fn windows(duration: u32) -> Vec<(u32, u32)> {
    if duration <= 3 * WINDOW_MS {
        vec![(0, duration)]
    } else {
        [duration / 3, duration * 2 / 3]
            .iter()
            .map(|&mid| (mid - WINDOW_MS / 2, mid + WINDOW_MS / 2))
            .collect()
    }
}

/// Two 90-s windows of each transcribed reference meeting with its recording, both models, decoded
/// and polished exactly as a meeting is; the gold is the owner's transcript of the speech covered.
async fn speech(
    app: &AppHandle,
    slug: &str,
    pool: &SqlitePool,
    id: &str,
    baseline: &str,
    candidate: &str,
    cancel: &Arc<AtomicBool>,
) -> Result<Stored> {
    #[derive(sqlx::FromRow)]
    struct Row {
        id: String,
        title: String,
        audio_path: String,
        language: Language,
        presenter: Option<String>,
        duration_ms: Option<i64>,
    }
    let rows: Vec<Row> = sqlx::query_as(concat!(
        "SELECT m.id, m.title, m.audio_path, m.language, m.presenter, m.duration_ms ",
        speech_references!(),
        " ORDER BY m.held_on DESC LIMIT ?"
    ))
    .bind(SPEECH_MEETINGS)
    .fetch_all(pool)
    .await?;
    let owner = profile::owner(&app.state::<WorkspaceHub>(), slug).await?;
    let people = people::with_owner(people::list(pool).await?, owner.as_ref());
    let terms = vocabulary::list(pool).await?;
    let mut stored = Stored::default();
    let mut made_by = Vec::new();
    let mut apart = false;
    for (i, m) in rows.iter().enumerate() {
        let segments = meetings::transcript(pool, &m.id).await?;
        apart |= segments.iter().any(|s| s.speaker.is_some());
        let duration = m
            .duration_ms
            .map(|d| d as u32)
            .or_else(|| segments.last().map(|s| s.end_ms))
            .unwrap_or_default();
        let windows = windows(duration);
        let prompt = vocabulary::whisper_prompt(&m.title, m.presenter.as_deref(), &people, &terms);
        let source = std::path::PathBuf::from(&m.audio_path);
        let mut trials = whisper::trial(
            app,
            pool,
            &m.id,
            &windows,
            &[baseline, candidate],
            m.language,
            Some(prompt.clone()),
            task(id, slug, format!("Trying {candidate} on “{}”", m.title)),
            cancel.clone(),
            move || audio::load_16k_mono(&source),
        )
        .await?;
        for trial in trials.iter_mut().flatten() {
            meetings::transcribe::polish(&mut trial.lines, &prompt, &people, &terms);
        }
        let (base, cand) = (&trials[0], &trials[1]);
        // One paragraph either way: how a model splits lines mustn't tell which wrote which.
        let text = |lines: &[whisper::Segment]| -> String {
            lines
                .iter()
                .map(|l| l.text.trim())
                .collect::<Vec<_>>()
                .join(" ")
        };
        // The owner's lines over the speech the models heard (both heard the same stretches).
        let gold: Vec<String> = base
            .iter()
            .map(|t| {
                let (from, to) = t.covered;
                segments
                    .iter()
                    .filter(|s| s.start_ms < to && s.end_ms > from)
                    .map(|s| s.text.trim())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();
        let gold_text = gold.join(" ");
        let heard_ms: u32 = base.iter().map(|t| t.covered.1 - t.covered.0).sum();
        // All windows together, so a long window weighs more than a short one.
        let score = |trials: &[whisper::Trial]| -> String {
            if gold_text.trim().is_empty() {
                return "—".into();
            }
            let theirs: Vec<String> = trials.iter().map(|t| text(&t.lines)).collect();
            percent(agreement(&theirs.join(" "), &gold_text))
        };
        let speed = |trials: &[whisper::Trial]| -> String {
            let ms: u32 = trials.iter().map(|t| t.ms).sum();
            if heard_ms == 0 {
                return "—".into();
            }
            format!("{:.1}× real time", heard_ms as f32 / ms.max(1) as f32)
        };
        let repeats = |trials: &[whisper::Trial]| -> u32 {
            trials
                .iter()
                .map(|t| {
                    t.lines
                        .windows(2)
                        .filter(|w| w[0].text.trim() == w[1].text.trim())
                        .count() as u32
                })
                .sum()
        };
        stored.meetings.push(MeetingFigures {
            meeting_id: m.id.clone(),
            meeting: m.title.clone(),
            figures: vec![
                Figure {
                    label: "Words as in your transcript".into(),
                    baseline: score(base),
                    candidate: score(cand),
                },
                Figure {
                    label: "Speed".into(),
                    baseline: speed(base),
                    candidate: speed(cand),
                },
                Figure {
                    label: "Lines repeated".into(),
                    baseline: repeats(base).to_string(),
                    candidate: repeats(cand).to_string(),
                },
            ],
        });
        for (w, t) in base.iter().enumerate() {
            let (from, to) = t.covered;
            if to <= from {
                continue; // no speech in this window
            }
            stored.pairs.push(pair(
                id,
                &format!("{}-{w}", m.id),
                &m.title,
                &format!("{} – {}", clock(from), clock(to)),
                text(&base[w].lines),
                text(&cand[w].lines),
            ));
        }
        if let Some(model) = transcript_model(pool, &m.id).await {
            made_by.push(model);
        }
        set_progress(pool, id, i + 1, rows.len()).await;
    }
    made_by.sort();
    made_by.dedup();
    stored.note = match made_by.as_slice() {
        [] => String::new(),
        [one] => format!(
            "Your transcripts were written by {one} and then corrected, so {one} starts ahead on “words as in your transcript”."
        ),
        _ => "Your transcripts were written by several models and then corrected.".into(),
    };
    if apart {
        // The meeting heard each voice on its own (D-051); a trial hears the mix.
        stored
            .note
            .push_str(if stored.note.is_empty() { "" } else { " " });
        stored.note.push_str("Some of these meetings were transcribed as your microphone and the call apart; here both models hear them mixed, so both score a little lower on overlapping speech.");
    }
    Ok(stored)
}

/// A saved summary, read without quoting its words in an error (D-026).
fn saved_doc(json: &str) -> Result<SummaryDoc> {
    serde_json::from_str(json)
        .map_err(|e| anyhow::anyhow!("a saved summary couldn't be read ({:?})", e.classify()))
}

/// Each reference meeting summarised since 0.8.1 summarised again with the candidate; the model in
/// use is represented by the summary it wrote before the owner's edits, the gold by the summary as
/// the owner left it.
async fn text(
    app: &AppHandle,
    slug: &str,
    pool: &SqlitePool,
    id: &str,
    baseline: &str,
    candidate: &str,
    cancel: &Arc<AtomicBool>,
) -> Result<Stored> {
    let rows: Vec<(String, String, String, String, Option<String>)> = sqlx::query_as(concat!(
        "SELECT m.id, m.title, s.sections_json, s.ai_sections_json, s.model ",
        text_references!(),
        " ORDER BY m.held_on DESC LIMIT ?"
    ))
    .bind(TEXT_MEETINGS)
    .fetch_all(pool)
    .await?;
    let typical = agents::typical(pool, "summary", baseline).await;
    let mut stored = Stored::default();
    let mut written_by = Vec::new();
    for (i, (meeting_id, title, saved, ai, model)) in rows.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            bail!(AppError::new("cancelled", "Model check cancelled"));
        }
        let gold = saved_doc(saved)?;
        let base = saved_doc(ai)?;
        if let Some(model) = model.as_deref().filter(|m| *m != baseline) {
            written_by.push(model.to_owned());
        }
        let trial = summarise::trial(
            app,
            slug,
            meeting_id,
            candidate,
            task(id, slug, format!("Trying {candidate} on “{title}”")),
            cancel,
        )
        .await;
        // A summary the candidate can't write is a finding, not a failed check; Ollama gone, a
        // model missing or a cancel stop the check.
        let (cand, ms) = match trial {
            Ok(written) => written,
            Err(err) if stops(&err) || cancel.load(Ordering::Relaxed) => return Err(err),
            Err(err) => {
                tracing::warn!("model check {id}: {candidate} couldn't write a summary: {err:#}");
                stored.meetings.push(MeetingFigures {
                    meeting_id: meeting_id.clone(),
                    meeting: title.clone(),
                    figures: vec![Figure {
                        label: "Summary written".into(),
                        baseline: "Yes".into(),
                        candidate: "No: its answer couldn't be read".into(),
                    }],
                });
                set_progress(pool, id, i + 1, rows.len()).await;
                continue;
            }
        };
        let names = |doc: &SummaryDoc| -> Vec<String> {
            doc.attendees
                .iter()
                .map(|a| people::fold(&a.name))
                .collect()
        };
        let gold_names = names(&gold);
        let found = |doc: &SummaryDoc| {
            let theirs = names(doc);
            format!(
                "{} of {}",
                gold_names.iter().filter(|n| theirs.contains(n)).count(),
                gold_names.len()
            )
        };
        let matched = |doc: &SummaryDoc| {
            let hit = gold
                .action_items
                .iter()
                .filter(|g| {
                    doc.action_items.iter().any(|a| {
                        people::fold(&a.owner) == people::fold(&g.owner)
                            && agreement(&a.item, &g.item) >= 0.5
                    })
                })
                .count();
            format!("{hit} of {}", gold.action_items.len())
        };
        stored.meetings.push(MeetingFigures {
            meeting_id: meeting_id.clone(),
            meeting: title.clone(),
            figures: vec![
                Figure {
                    label: "Attendees as in your summary".into(),
                    baseline: found(&base),
                    candidate: found(&cand),
                },
                Figure {
                    label: "Action items as in your summary".into(),
                    baseline: matched(&base),
                    candidate: matched(&cand),
                },
                Figure {
                    label: "Decisions".into(),
                    baseline: base.decisions.len().to_string(),
                    candidate: cand.decisions.len().to_string(),
                },
                Figure {
                    label: "Time".into(),
                    baseline: if typical.0 > 0 {
                        format!("about {} s", typical.0 / 1000)
                    } else {
                        "—".into()
                    },
                    candidate: format!("{} s", ms / 1000),
                },
            ],
        });
        for kind in [SectionKind::Overview, SectionKind::ActionItems] {
            let section = |doc: &SummaryDoc| {
                doc.sections
                    .iter()
                    .find(|s| s.kind == kind)
                    .map(|s| (s.title.clone(), s.markdown.clone()))
                    .unwrap_or_default()
            };
            let ((label, b), (_, c)) = (section(&base), section(&cand));
            stored.pairs.push(pair(
                id,
                &format!("{meeting_id}-{kind:?}"),
                title,
                &label,
                b,
                c,
            ));
        }
        set_progress(pool, id, i + 1, rows.len()).await;
    }
    written_by.sort();
    written_by.dedup();
    stored.note = if written_by.is_empty() {
        "The model in use is shown by the summary it wrote for each meeting, before your edits; its time is its usual time for a summary.".into()
    } else {
        format!(
            "The model in use is shown by the summary it wrote for each meeting, before your edits — for some meetings that was {}, the model in use then. Its time is its usual time for a summary.",
            written_by.join(", ")
        )
    };
    Ok(stored)
}

/// Errors that end a check rather than count against the candidate.
fn stops(err: &anyhow::Error) -> bool {
    err.downcast_ref::<AppError>().is_some_and(|e| {
        e.code == "cancelled"
            || e.code == "invalid_input"
            || e.code == "not_found"
            || e.code == "model_missing"
            || e.code.starts_with("ollama_")
    })
}

/// The model that wrote a meeting's transcript, from the AI log.
async fn transcript_model(pool: &SqlitePool, meeting_id: &str) -> Option<String> {
    sqlx::query_as::<_, (String,)>(
        "SELECT model FROM ai_log WHERE kind = 'transcribe' AND status = 'ok' AND meeting_id = ? \
         AND (agent IS NULL OR agent != 'check') ORDER BY created_at DESC LIMIT 1",
    )
    .bind(meeting_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(|(model,)| model)
}

/// A blind pair: which model is A comes from the check's and the pair's ids (stable, unguessable
/// from the page). `key` must start with the meeting's id: deleting the meeting finds the checks
/// holding its words by it (migration 0013's trigger).
fn pair(
    check: &str,
    key: &str,
    meeting: &str,
    label: &str,
    baseline: String,
    candidate: String,
) -> StoredPair {
    use sha2::{Digest, Sha256};
    let candidate_is_a = Sha256::digest(format!("{check}/{key}").as_bytes())[0] % 2 == 0;
    StoredPair {
        id: key.to_owned(),
        meeting: meeting.to_owned(),
        label: label.to_owned(),
        baseline,
        candidate,
        candidate_is_a,
    }
}

fn percent(share: f32) -> String {
    format!("{} %", (share * 100.0).round())
}

fn clock(ms: u32) -> String {
    format!("{}:{:02}", ms / 60_000, ms / 1000 % 60)
}

fn words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

/// How much of two texts is the same words in the same order: 2 × the longest common subsequence
/// of words over both lengths (difflib's ratio, by words). 1.0 for two empty texts.
fn agreement(a: &str, b: &str) -> f32 {
    let (a, b) = (words(a), words(b));
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let mut prev = vec![0u32; b.len() + 1];
    for x in &a {
        let mut cur = vec![0u32; b.len() + 1];
        for (j, y) in b.iter().enumerate() {
            cur[j + 1] = if x == y {
                prev[j] + 1
            } else {
                cur[j].max(prev[j + 1])
            };
        }
        prev = cur;
    }
    2.0 * prev[b.len()] as f32 / (a.len() + b.len()) as f32
}

/// The checks, newest first (the last ten).
pub async fn list(pool: &SqlitePool) -> Result<Vec<ModelCheck>> {
    let ids: Vec<(String,)> =
        sqlx::query_as("SELECT id FROM model_check ORDER BY created_at DESC LIMIT 10")
            .fetch_all(pool)
            .await?;
    let mut out = Vec::with_capacity(ids.len());
    for (id,) in ids {
        out.push(get(pool, &id).await?);
    }
    Ok(out)
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<ModelCheck> {
    #[derive(sqlx::FromRow)]
    struct Row {
        kind: CheckKind,
        baseline: String,
        candidate: String,
        status: CheckStatus,
        progress: u32,
        error: Option<String>,
        created_at: String,
        result_json: Option<String>,
        picks_json: String,
    }
    let row: Row = sqlx::query_as(
        "SELECT kind, baseline, candidate, status, progress, error, created_at, result_json, picks_json \
         FROM model_check WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::not_found("No such model check"))?;
    let stored: Stored = row
        .result_json
        .as_deref()
        .and_then(|json| serde_json::from_str(json).ok())
        .unwrap_or_default();
    let picks: HashMap<String, Pick> = serde_json::from_str(&row.picks_json).unwrap_or_default();
    let mut tally = Tally::default();
    let pairs = stored
        .pairs
        .iter()
        .map(|p| {
            let pick = picks.get(&p.id).copied();
            match pick {
                None => tally.left += 1,
                Some(Pick::Same) => tally.same += 1,
                Some(Pick::A) if p.candidate_is_a => tally.candidate += 1,
                Some(Pick::B) if !p.candidate_is_a => tally.candidate += 1,
                Some(_) => tally.baseline += 1,
            }
            let (a, b) = if p.candidate_is_a {
                (p.candidate.clone(), p.baseline.clone())
            } else {
                (p.baseline.clone(), p.candidate.clone())
            };
            Pair {
                id: p.id.clone(),
                meeting: p.meeting.clone(),
                label: p.label.clone(),
                a,
                b,
                pick,
            }
        })
        .collect();
    Ok(ModelCheck {
        id: id.to_owned(),
        kind: row.kind,
        baseline: row.baseline,
        candidate: row.candidate,
        status: row.status,
        progress: row.progress,
        error: row.error,
        created_at: row.created_at,
        meetings: stored.meetings,
        note: stored.note,
        pairs,
        tally,
    })
}

/// The owner's choice on one blind pair (one statement: two quick picks can't lose one).
pub async fn pick(pool: &SqlitePool, id: &str, pair_id: &str, choice: Pick) -> Result<ModelCheck> {
    let done = sqlx::query(
        "UPDATE model_check SET picks_json = json_patch(picks_json, json_object(?, ?)), \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?",
    )
    .bind(pair_id)
    .bind(serde_json::to_value(choice)?.as_str().unwrap_or_default())
    .bind(id)
    .execute(pool)
    .await?;
    if done.rows_affected() == 0 {
        bail!(AppError::not_found("No such model check"));
    }
    get(pool, id).await
}

/// At launch: a check a quit interrupted can't go on (it lived in memory), so it says so.
pub async fn recover(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "UPDATE model_check SET status = 'failed', error = 'Stopped when New Dawn quit', \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE status IN ('waiting', 'running')",
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{agreement, clock, pair};

    #[test]
    fn agreement_is_difflibs_ratio_by_words() {
        assert_eq!(agreement("", ""), 1.0);
        assert_eq!(agreement("Drill two cores.", "drill two cores"), 1.0);
        let r = agreement("the core drilling is done", "the drilling is done today");
        assert!((r - 0.8).abs() < 1e-6, "{r}"); // 4 common of 5 + 5
        assert_eq!(agreement("a b c", "x y z"), 0.0);
    }

    #[test]
    fn short_meetings_are_heard_whole_long_ones_twice() {
        use super::{WINDOW_MS, windows};
        assert_eq!(windows(0), [(0, 0)]);
        assert_eq!(
            windows(3 * WINDOW_MS),
            [(0, 3 * WINDOW_MS)],
            "no overlapping windows"
        );
        let long = windows(60 * 60_000);
        assert_eq!(long, [(1_155_000, 1_245_000), (2_355_000, 2_445_000)]);
    }

    #[test]
    fn only_a_bad_answer_counts_against_the_candidate() {
        use crate::error::AppError;
        let stops = |e: AppError| super::stops(&anyhow::Error::from(e));
        assert!(stops(AppError::new(
            "ollama_unavailable",
            "Ollama isn't running"
        )));
        assert!(stops(AppError::new("cancelled", "Model check cancelled")));
        assert!(stops(AppError::invalid("The transcript is empty")));
        assert!(!super::stops(&anyhow::anyhow!(
            "the model's answer was not valid JSON"
        )));
    }

    #[test]
    fn pairs_are_blind_but_stable() {
        let a = pair(
            "check-1",
            "m1-0",
            "Weekly",
            "1:00 – 2:30",
            "base".into(),
            "cand".into(),
        );
        let b = pair(
            "check-1",
            "m1-0",
            "Weekly",
            "1:00 – 2:30",
            "base".into(),
            "cand".into(),
        );
        assert_eq!(a.candidate_is_a, b.candidate_is_a);
        let sides: std::collections::HashSet<bool> = (0..16)
            .map(|i| {
                pair(
                    "check-1",
                    &format!("m1-{i}"),
                    "",
                    "",
                    String::new(),
                    String::new(),
                )
                .candidate_is_a
            })
            .collect();
        assert_eq!(sides.len(), 2, "both orders occur");
        assert_eq!(clock(5_415_000), "90:15");
    }
}
