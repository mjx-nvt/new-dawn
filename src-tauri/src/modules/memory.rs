//! Search by meaning (D-053): each meeting's transcript, in pieces of a few lines, and its summary,
//! by section, become vectors from the embedding model (bge-m3 through Ollama) in the background;
//! a search compares the owner's words with all of them, so "partner onboarding" finds the meeting
//! that never used those words — in Vietnamese too.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::Result;
use futures_util::TryStreamExt;
use serde::Serialize;
use sha2::{Digest, Sha256};
use specta::Type;
use sqlx::SqlitePool;
use tauri::{AppHandle, Manager};

use crate::error::AppError;
use crate::kernel::ai::log::{self as ai_log, AiLogEntry};
use crate::kernel::ai::ollama::{self, EMBED_MODEL};
use crate::kernel::ai::queue::{AiQueue, Lane, Task, Turn};
use crate::kernel::ai::resources;
use crate::kernel::hub::WorkspaceHub;
use crate::modules::meetings::{self, MeetingHit, summarise};

/// A piece: consecutive transcript lines, or a summary section, up to about this many characters.
const PIECE_CHARS: usize = 800;
/// Pieces per embedding call; the indexer lets the owner's work go first between calls.
const BATCH: usize = 16;
/// Meetings a search shows, and how close the best piece must be: bge-m3 puts unrelated text
/// around 0.3 and a match from 0.45 up, in either language.
const RESULTS: usize = 8;
const MIN_SCORE: f32 = 0.42;
/// How much of a piece the result shows.
const SNIPPET_CHARS: usize = 220;
/// Vectors read at a time by a search.
const PAGE: i64 = 2_000;

/// What a search by meaning found, and how much of the workspace it could look at.
#[derive(Debug, Clone, Serialize, Type)]
pub struct MeaningSearch {
    pub hits: Vec<MeetingHit>,
    /// Meetings indexed with the current model, of those with a transcript.
    pub indexed: u32,
    pub meetings: u32,
}

struct Piece {
    source: &'static str,
    start_ms: Option<u32>,
    text: String,
}

impl Piece {
    fn hash(&self) -> String {
        let digest = Sha256::digest(format!(
            "{}\0{:?}\0{}",
            self.source, self.start_ms, self.text
        ));
        digest.iter().map(|b| format!("{b:02x}")).collect()
    }
}

/// Lines joined into pieces of up to `PIECE_CHARS`; a longer line is cut into pieces of its own.
fn join_lines<'a>(
    lines: impl Iterator<Item = (Option<u32>, &'a str)>,
    source: &'static str,
) -> Vec<Piece> {
    let mut out: Vec<Piece> = Vec::new();
    for (start_ms, line) in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let chars: Vec<char> = line.chars().collect();
        for (i, part) in chars.chunks(PIECE_CHARS).enumerate() {
            let part: String = part.iter().collect();
            match out.last_mut() {
                Some(last)
                    if i == 0 && last.text.chars().count() + part.chars().count() < PIECE_CHARS =>
                {
                    last.text.push('\n');
                    last.text.push_str(&part);
                }
                _ => out.push(Piece {
                    source,
                    start_ms,
                    text: part,
                }),
            }
        }
    }
    out
}

async fn pieces(pool: &SqlitePool, meeting_id: &str) -> Result<Vec<Piece>> {
    let segments = meetings::transcript(pool, meeting_id).await?;
    let mut out = join_lines(
        segments.iter().map(|s| (Some(s.start_ms), s.text.as_str())),
        "transcript",
    );
    if let Some(view) = summarise::view(pool, meeting_id).await? {
        for section in &view.doc.sections {
            // "Overview: …" — the title reads as one with its text in a snippet.
            let title = format!("{}:", section.title.trim());
            let lines = std::iter::once(title.as_str())
                .chain(section.markdown.lines())
                .map(|l| (None, l));
            out.extend(join_lines(lines, "summary"));
        }
    }
    Ok(out)
}

/// Brings a meeting's vectors up to date: new pieces are embedded (a batch at a time, letting the
/// owner's work go first in between), pieces that are gone (and any other model's) are deleted,
/// and the meeting is marked checked. Returns how many pieces were embedded.
async fn index_meeting(pool: &SqlitePool, meeting_id: &str, turn: &mut Turn<'_>) -> Result<usize> {
    // Taken before reading: an edit made while the pieces are embedded is newer, so the next
    // sweep comes back for it.
    let checked_at: String = sqlx::query_scalar("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')")
        .fetch_one(pool)
        .await?;
    let pieces = pieces(pool, meeting_id).await?;
    let wanted: Vec<String> = pieces.iter().map(Piece::hash).collect();
    let have: Vec<(String,)> =
        sqlx::query_as("SELECT text_hash FROM embedding WHERE meeting_id = ? AND model = ?")
            .bind(meeting_id)
            .bind(EMBED_MODEL)
            .fetch_all(pool)
            .await?;
    let have: std::collections::HashSet<String> = have.into_iter().map(|(h,)| h).collect();
    let missing: Vec<(&Piece, &String)> = pieces
        .iter()
        .zip(&wanted)
        .filter(|(_, hash)| !have.contains(*hash))
        .collect();
    let mut vectors = Vec::with_capacity(missing.len());
    for batch in missing.chunks(BATCH) {
        if turn.wanted() {
            turn.make_way().await;
        }
        let texts: Vec<String> = batch.iter().map(|(p, _)| p.text.clone()).collect();
        vectors.extend(ollama::embed(&texts, Duration::from_secs(120)).await?);
    }
    let mut tx = pool.begin().await?;
    sqlx::query(
        "DELETE FROM embedding WHERE meeting_id = ?1 AND (model <> ?2 \
         OR text_hash NOT IN (SELECT value FROM json_each(?3)))",
    )
    .bind(meeting_id)
    .bind(EMBED_MODEL)
    .bind(serde_json::to_string(&wanted)?)
    .execute(&mut *tx)
    .await?;
    for ((piece, hash), vector) in missing.iter().zip(&vectors) {
        let bytes: Vec<u8> = vector.iter().flat_map(|f| f.to_le_bytes()).collect();
        sqlx::query(
            "INSERT INTO embedding (id, meeting_id, source, start_ms, text, text_hash, model, vector) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(uuid::Uuid::now_v7().to_string())
        .bind(meeting_id)
        .bind(piece.source)
        .bind(piece.start_ms)
        .bind(&piece.text)
        .bind(*hash)
        .bind(EMBED_MODEL)
        .bind(bytes)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query(
        "INSERT INTO embedding_check (meeting_id, model, checked_at) VALUES (?1, ?2, ?3) \
         ON CONFLICT (meeting_id) DO UPDATE SET model = excluded.model, checked_at = excluded.checked_at, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
    )
    .bind(meeting_id)
    .bind(EMBED_MODEL)
    .bind(&checked_at)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(missing.len())
}

/// Meetings whose transcript or summary changed since they were last indexed with the current
/// model (or never were), newest first.
async fn stale(pool: &SqlitePool) -> Result<Vec<(String, String)>> {
    Ok(sqlx::query_as(
        "SELECT m.id, m.title FROM meeting m WHERE m.status IN ('transcribed', 'summarised') AND \
         coalesce((SELECT c.checked_at FROM embedding_check c WHERE c.meeting_id = m.id AND c.model = ?1), '') < \
         max(m.updated_at, \
             coalesce((SELECT s.updated_at FROM summary s WHERE s.meeting_id = m.id), ''), \
             coalesce((SELECT max(t.updated_at) FROM transcript_segment t WHERE t.meeting_id = m.id), '')) \
         ORDER BY m.held_on DESC",
    )
    .bind(EMBED_MODEL)
    .fetch_all(pool)
    .await?)
}

/// The sweep's Run now (`index-<slug>`), let go however the sweep ends.
struct RunNow<'a> {
    queue: &'a AiQueue,
    id: &'a str,
}

impl Drop for RunNow<'_> {
    fn drop(&mut self) {
        self.queue.forget_now(self.id);
    }
}

/// Indexes a workspace's changed meetings, each in its own catch-up turn (waits for a free Mac,
/// lets the owner's work go first). A meeting that fails is skipped until it changes or the next
/// sweep; Ollama gone or the model removed ends the sweep. Returns how many pieces were embedded.
async fn index_workspace(app: &AppHandle, slug: &str) -> Result<usize> {
    let pool = app.state::<WorkspaceHub>().pool(slug)?;
    let id = format!("index-{slug}");
    let queue = app.state::<AiQueue>();
    let _run_now = RunNow {
        queue: queue.inner(),
        id: &id,
    };
    let mut embedded = 0;
    for (meeting_id, title) in stale(&pool).await? {
        let task = Task {
            id: Some(id.clone()),
            workspace_slug: Some(slug.to_owned()),
            ..Task::new(format!("Indexing “{title}” for search"))
        };
        let mut turn = resources::acquire(app, Lane::Background, task).await;
        // The text model, kept loaded after a summary, makes room for this one.
        ollama::unload_text().await;
        let started = Instant::now();
        let result = index_meeting(&pool, &meeting_id, &mut turn).await;
        drop(turn);
        let entry = AiLogEntry {
            kind: "complete",
            agent: Some("memory"),
            model: EMBED_MODEL,
            meeting_id: Some(&meeting_id),
            duration_ms: started.elapsed().as_millis() as u32,
            status: if result.is_ok() { "ok" } else { "error" },
            error: result.as_ref().err().map(|e| format!("{e:#}")),
            prompt_tokens: None,
            output_tokens: None,
        };
        if let Err(err) = ai_log::record(&pool, entry).await {
            tracing::warn!("couldn't log the indexing: {err:#}");
        }
        match result {
            Ok(n) => embedded += n,
            Err(err) => {
                let code = err.downcast_ref::<AppError>().map(|e| e.code.as_str());
                if matches!(code, Some("ollama_unavailable" | "model_missing")) {
                    return Err(err);
                }
                tracing::warn!("indexing meeting {meeting_id} for search: {err:#}");
            }
        }
    }
    Ok(embedded)
}

/// Every 30 minutes (first after three), each workspace's changed meetings are indexed — when the
/// embedding model is there.
pub fn spawn(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(180)).await;
        loop {
            if ollama::status().await.embed_installed {
                let hub = app.state::<WorkspaceHub>();
                for w in hub
                    .list()
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|w| !w.archived)
                {
                    match index_workspace(&app, &w.slug).await {
                        Ok(0) => {}
                        Ok(n) => tracing::info!("indexed {n} piece(s) of {} for search", w.slug),
                        Err(err) => tracing::warn!("indexing {} for search: {err:#}", w.slug),
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(1800)).await;
        }
    });
}

/// Search by meaning: the meetings whose transcript or summary is closest to `query`, best first,
/// each with its closest piece; empty when nothing is close enough. The query is embedded without
/// a queue turn (one short text, D-053); the vectors are read a page at a time.
// ponytail: compares every vector of the workspace per search (≈ 4 KB × 150 pieces an hour of
// meetings); keep them in memory, or an ANN index, if a workspace grows past ~100k pieces.
pub async fn search(pool: &SqlitePool, query: &str) -> Result<MeaningSearch> {
    let (indexed, meetings): (u32, u32) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM embedding_check WHERE model = ?1), \
         (SELECT count(*) FROM meeting WHERE status IN ('transcribed', 'summarised'))",
    )
    .bind(EMBED_MODEL)
    .fetch_one(pool)
    .await?;
    let query = query.trim();
    if query.chars().count() < 3 {
        return Ok(MeaningSearch {
            hits: Vec::new(),
            indexed,
            meetings,
        });
    }
    let wanted = ollama::embed(&[query.to_owned()], Duration::from_secs(20))
        .await?
        .pop()
        .unwrap_or_default();
    // One snapshot for the pages and the winners' text: the indexer may delete and insert rows
    // meanwhile, and a freed rowid is reused.
    let mut tx = pool.begin().await?;
    // Each meeting's closest piece, by its row.
    let mut best: HashMap<String, (f32, i64, Option<u32>)> = HashMap::new();
    let mut after = 0i64;
    loop {
        let mut rows = sqlx::query_as::<_, (i64, String, Option<u32>, Vec<u8>)>(
            "SELECT rowid, meeting_id, start_ms, vector FROM embedding \
             WHERE model = ? AND rowid > ? ORDER BY rowid LIMIT ?",
        )
        .bind(EMBED_MODEL)
        .bind(after)
        .bind(PAGE)
        .fetch(&mut *tx);
        let mut read = 0;
        while let Some((rowid, meeting_id, start_ms, bytes)) = rows.try_next().await? {
            read += 1;
            after = rowid;
            let vector: Vec<f32> = bytes
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .collect();
            let score = cosine(&wanted, &vector);
            if score < MIN_SCORE || best.get(&meeting_id).is_some_and(|b| b.0 >= score) {
                continue;
            }
            best.insert(meeting_id, (score, rowid, start_ms));
        }
        if read < PAGE {
            break;
        }
    }
    let mut ranked: Vec<_> = best.into_iter().collect();
    ranked.sort_by(|a, b| b.1.0.total_cmp(&a.1.0));
    let mut hits = Vec::new();
    for (meeting_id, (_, rowid, at_ms)) in ranked.into_iter().take(RESULTS) {
        let Ok(meeting) = meetings::get(pool, &meeting_id).await else {
            continue;
        };
        let text: String = sqlx::query_scalar("SELECT text FROM embedding WHERE rowid = ?")
            .bind(rowid)
            .fetch_one(&mut *tx)
            .await?;
        let snippet: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let snippet = match snippet.char_indices().nth(SNIPPET_CHARS) {
            Some((at, _)) => format!("{}…", &snippet[..at]),
            None => snippet,
        };
        hits.push(MeetingHit {
            meeting,
            snippet: Some(snippet),
            at_ms,
        });
    }
    tx.commit().await?;
    Ok(MeaningSearch {
        hits,
        indexed,
        meetings,
    })
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let (mut dot, mut na, mut nb) = (0.0f32, 0.0f32, 0.0f32);
    for (x, y) in a.iter().zip(b) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na.sqrt() * nb.sqrt())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_become_pieces_of_a_few_lines() {
        let long = "x".repeat(PIECE_CHARS * 2 + 10);
        let lines = [
            (Some(0), "Chào anh"),
            (Some(1_000), "  "),
            (Some(2_000), "Hôm nay"),
            (Some(3_000), long.as_str()),
            (Some(9_000), "Cảm ơn"),
        ];
        let got = join_lines(lines.into_iter(), "transcript");
        let shape: Vec<(Option<u32>, usize)> = got
            .iter()
            .map(|p| (p.start_ms, p.text.chars().count()))
            .collect();
        assert_eq!(
            shape,
            [
                (Some(0), 16),
                (Some(3_000), PIECE_CHARS),
                (Some(3_000), PIECE_CHARS),
                (Some(3_000), 10 + 1 + 6),
            ],
            "a long line is cut; the next line joins its last part"
        );
        assert_ne!(got[0].hash(), got[1].hash());
    }

    #[test]
    fn cosine_is_safe_on_odd_vectors() {
        assert!((cosine(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-6);
        assert_eq!(cosine(&[1.0, 0.0], &[0.0, 1.0]), 0.0);
        assert_eq!(cosine(&[1.0], &[1.0, 2.0]), 0.0, "another model's size");
        assert_eq!(cosine(&[0.0, 0.0], &[1.0, 2.0]), 0.0);
    }
}
