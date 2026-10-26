//! Meetings in one workspace database. Pure data: callers get the pool from the hub.

pub mod audio;
pub mod corrections;
pub mod email;
pub mod mail;
pub mod minutes;
pub mod naming;
pub mod review;
pub mod summarise;
pub mod template;
pub mod transcribe;
pub mod waveform;

use anyhow::{Result, bail};
use serde::Serialize;
use specta::Type;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::AppError;
use crate::kernel::ai::whisper::{Language, Segment};

/// A discussion among several people, or a presentation by one main presenter (D-038).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize, Type, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum MeetingFormat {
    Discussion,
    Presentation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum MeetingStatus {
    Recording,
    Recorded,
    Transcribing,
    Transcribed,
    Summarising,
    Summarised,
    Failed,
}

#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct Meeting {
    pub id: String,
    pub meeting_type_id: Option<String>,
    pub title: String,
    pub held_on: String,
    pub language: Language,
    pub audio_path: Option<String>,
    pub duration_ms: Option<u32>,
    pub status: MeetingStatus,
    /// The owner's notes; passed to the next summary run (P3.3).
    pub notes_md: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub format: MeetingFormat,
    /// The main presenter's name, when the audio may never say it.
    pub presenter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct TranscriptSegment {
    pub seq: u32,
    pub start_ms: u32,
    pub end_ms: u32,
    pub text: String,
    /// What Whisper wrote, when the vocabulary corrected this line (D-036).
    pub raw_text: Option<String>,
    /// The English of a line written during a live session (D-041); None until translated.
    pub translation: Option<String>,
}

/// Just enough of a meeting type for pickers (full editing arrives with tenant setup, P1.4).
#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct MeetingTypeSummary {
    pub id: String,
    pub name: String,
    pub language: Language,
    pub is_default: bool,
}

pub async fn meeting_types(pool: &SqlitePool) -> Result<Vec<MeetingTypeSummary>> {
    Ok(sqlx::query_as(
        "SELECT id, name, language, is_default FROM meeting_type ORDER BY is_default DESC, name",
    )
    .fetch_all(pool)
    .await?)
}

/// Changes (or clears) a meeting's type; the next minutes export uses its template.
pub async fn set_type(
    pool: &SqlitePool,
    meeting_id: &str,
    type_id: Option<&str>,
) -> Result<Meeting> {
    if let Some(type_id) = type_id {
        let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM meeting_type WHERE id = ?")
            .bind(type_id)
            .fetch_optional(pool)
            .await?;
        if exists.is_none() {
            bail!(AppError::not_found("No such meeting type"));
        }
    }
    sqlx::query(
        "UPDATE meeting SET meeting_type_id = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?",
    )
    .bind(type_id)
    .bind(meeting_id)
    .execute(pool)
    .await?;
    get(pool, meeting_id).await
}

/// One search result: the meeting, plus where the words were found in its transcript or summary.
#[derive(Debug, Clone, Serialize, Type)]
pub struct MeetingHit {
    pub meeting: Meeting,
    /// Matched words are wrapped in U+0002 … U+0003 (rendered as highlights, never as HTML).
    pub snippet: Option<String>,
    /// Start of the best-matching transcript segment, for jumping straight to it.
    pub at_ms: Option<u32>,
}

/// User text → a safe FTS5 query: every word becomes a quoted phrase (so punctuation and FTS
/// operators are just text) and the last word matches as a prefix while typing.
pub fn fts_query(input: &str) -> Option<String> {
    let words: Vec<String> = input
        .split_whitespace()
        .map(|w| w.replace('"', ""))
        .filter(|w| w.chars().any(char::is_alphanumeric)) // "-" or "…" alone is not a word
        .map(|w| format!("\"{w}\""))
        .collect();
    let last = words.len().checked_sub(1)?;
    Some(
        words
            .iter()
            .enumerate()
            .map(|(i, w)| {
                if i == last {
                    format!("{w}*")
                } else {
                    w.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// Title matches first, then transcript matches by FTS rank, then summary matches.
pub async fn search(pool: &SqlitePool, input: &str) -> Result<Vec<MeetingHit>> {
    let Some(query) = fts_query(input) else {
        return Ok(Vec::new());
    };
    // Titles: every word must appear, like the FTS side (LIKE ignores case for ASCII only).
    let patterns: Vec<String> = input
        .split_whitespace()
        .map(|w| {
            format!(
                "%{}%",
                w.replace('\\', "\\\\")
                    .replace('%', "\\%")
                    .replace('_', "\\_")
            )
        })
        .collect();
    let title_sql = format!(
        "SELECT id FROM meeting WHERE {} ORDER BY held_on DESC",
        vec!["title LIKE ? ESCAPE '\\'"; patterns.len()].join(" AND ")
    );
    // Only the count of a constant `title LIKE ?` fragment varies; user text is always bound.
    let mut titles_query = sqlx::query_as::<_, (String,)>(sqlx::AssertSqlSafe(title_sql));
    for pattern in &patterns {
        titles_query = titles_query.bind(pattern);
    }
    let titles = titles_query.fetch_all(pool).await?;

    // Best transcript lines for snippets (joined on the segment's primary key), plus every
    // matching meeting, so a common word can't crowd meetings out of the snippet LIMIT.
    let transcript: Vec<(String, String, u32)> = sqlx::query_as(
        "SELECT f.meeting_id, snippet(transcript_fts, 0, char(2), char(3), '…', 12), s.start_ms \
         FROM transcript_fts f JOIN transcript_segment s ON s.id = f.segment_id \
         WHERE transcript_fts MATCH ? ORDER BY rank LIMIT 500",
    )
    .bind(&query)
    .fetch_all(pool)
    .await?;
    let transcript_meetings: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT meeting_id FROM transcript_fts WHERE transcript_fts MATCH ?",
    )
    .bind(&query)
    .fetch_all(pool)
    .await?;
    let summary: Vec<(String, String)> = sqlx::query_as(
        "SELECT meeting_id, snippet(summary_fts, 1, char(2), char(3), '…', 12) FROM summary_fts \
         WHERE summary_fts MATCH ? ORDER BY rank LIMIT 200",
    )
    .bind(&query)
    .fetch_all(pool)
    .await?;

    let meetings: std::collections::HashMap<String, Meeting> = list(pool)
        .await?
        .into_iter()
        .map(|m| (m.id.clone(), m))
        .collect();
    let mut seen = std::collections::HashSet::new();
    let mut hits = Vec::new();
    let mut push = |id: String, snippet: Option<String>, at_ms: Option<u32>| {
        if seen.insert(id.clone()) {
            if let Some(meeting) = meetings.get(&id) {
                hits.push(MeetingHit {
                    meeting: meeting.clone(),
                    snippet,
                    at_ms,
                });
            }
        }
    };
    // A title hit still shows its best transcript snippet when there is one.
    for (id,) in titles {
        let best = transcript.iter().find(|(m, _, _)| *m == id);
        push(
            id,
            best.map(|(_, s, _)| s.clone()),
            best.map(|(_, _, t)| *t),
        );
    }
    for (id, snippet, at) in transcript.iter().cloned() {
        push(id, Some(snippet), Some(at));
    }
    for (id,) in transcript_meetings {
        push(id, None, None);
    }
    for (id, snippet) in summary {
        push(id, Some(snippet), None);
    }
    Ok(hits)
}

pub struct NewMeeting<'a> {
    pub id: &'a str,
    pub meeting_type_id: Option<&'a str>,
    pub title: &'a str,
    /// Unix seconds; stored as ISO-8601 UTC by SQLite.
    pub held_at_unix: i64,
    pub language: Language,
    pub audio_path: &'a str,
    pub status: MeetingStatus,
}

pub fn new_id() -> String {
    Uuid::now_v7().to_string()
}

pub async fn create(pool: &SqlitePool, new: NewMeeting<'_>) -> Result<Meeting> {
    sqlx::query(
        "INSERT INTO meeting (id, meeting_type_id, title, held_on, language, audio_path, status) \
         VALUES (?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', ?, 'unixepoch'), ?, ?, ?)",
    )
    .bind(new.id)
    .bind(new.meeting_type_id)
    .bind(new.title)
    .bind(new.held_at_unix)
    .bind(new.language)
    .bind(new.audio_path)
    .bind(new.status)
    .execute(pool)
    .await?;
    get(pool, new.id).await
}

/// Sets the meeting's format and presenter (blank = none; a discussion has none).
pub async fn set_format(
    pool: &SqlitePool,
    id: &str,
    format: MeetingFormat,
    presenter: Option<&str>,
) -> Result<Meeting> {
    let presenter = presenter
        .filter(|_| format == MeetingFormat::Presentation)
        .map(|p| crate::modules::people::clean(p, 120))
        .filter(|p| !p.is_empty());
    sqlx::query("UPDATE meeting SET format = ?, presenter = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?")
        .bind(format)
        .bind(presenter)
        .bind(id)
        .execute(pool)
        .await?;
    get(pool, id).await
}

/// The summary's view of the format: who to attribute statements to.
pub fn format_note(meeting: &Meeting) -> Option<String> {
    match (meeting.format, meeting.presenter.as_deref()) {
        (MeetingFormat::Presentation, Some(p)) => Some(format!(
            "Meeting format: a presentation by {p}. Statements not clearly made by someone else are {p}'s; other people's contributions are questions or comments to the presenter. List {p} as an attendee."
        )),
        (MeetingFormat::Presentation, None) => Some("Meeting format: a presentation by one main presenter; other people's contributions are questions or comments.".to_owned()),
        // The default: the summary rules already attribute positions, and a note here would
        // count as owner notes (highlights).
        (MeetingFormat::Discussion, _) => None,
    }
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<Meeting> {
    let found: Option<Meeting> = sqlx::query_as("SELECT * FROM meeting WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(found.ok_or_else(|| AppError::not_found(format!("No meeting '{id}'")))?)
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<Meeting>> {
    Ok(
        sqlx::query_as("SELECT * FROM meeting ORDER BY held_on DESC")
            .fetch_all(pool)
            .await?,
    )
}

pub async fn set_status(pool: &SqlitePool, id: &str, status: MeetingStatus) -> Result<()> {
    sqlx::query("UPDATE meeting SET status = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?")
        .bind(status)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// A recording that never captured anything (start failed).
pub async fn delete(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM meeting WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// The recording ended: its length, and `recorded` (or `transcribed` after a live session, D-041).
pub async fn finish_recording(
    pool: &SqlitePool,
    id: &str,
    duration_ms: u32,
    status: MeetingStatus,
) -> Result<()> {
    sqlx::query(
        "UPDATE meeting SET status = ?, duration_ms = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?",
    )
    .bind(status)
    .bind(duration_ms)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SummaryRow {
    pub sections_json: String,
    pub email_subject: Option<String>,
    pub email_md: Option<String>,
    pub model: Option<String>,
    pub edited_at: Option<String>,
    pub updated_at: String,
    /// The naming agent's pending suspects (JSON), until confirmed or dismissed.
    pub naming_json: Option<String>,
}

pub async fn summary(pool: &SqlitePool, meeting_id: &str) -> Result<Option<SummaryRow>> {
    Ok(sqlx::query_as(
        "SELECT sections_json, email_subject, email_md, model, edited_at, updated_at, naming_json FROM summary WHERE meeting_id = ?",
    )
    .bind(meeting_id)
    .fetch_optional(pool)
    .await?)
}

/// Upsert (never REPLACE — keeps the FTS triggers right, D-012). `edited` stamps `edited_at`; an
/// AI run clears it.
pub async fn save_summary(
    pool: &SqlitePool,
    meeting_id: &str,
    sections_json: &str,
    email_subject: &str,
    email_md: &str,
    model: Option<&str>,
    edited: bool,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO summary (id, meeting_id, sections_json, email_subject, email_md, model, edited_at) \
         VALUES (?, ?, ?, ?, ?, ?, CASE WHEN ? THEN strftime('%Y-%m-%dT%H:%M:%fZ', 'now') END) \
         ON CONFLICT (meeting_id) DO UPDATE SET sections_json = excluded.sections_json, \
           email_subject = excluded.email_subject, email_md = excluded.email_md, \
           model = coalesce(excluded.model, summary.model), edited_at = excluded.edited_at, \
           updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')",
    )
    .bind(new_id())
    .bind(meeting_id)
    .bind(sections_json)
    .bind(email_subject)
    .bind(email_md)
    .bind(model)
    .bind(edited)
    .execute(pool)
    .await?;
    Ok(())
}

/// Saves the summary document only; the stored email stays as it is.
pub async fn save_sections(
    pool: &SqlitePool,
    meeting_id: &str,
    sections_json: &str,
    edited: bool,
) -> Result<()> {
    sqlx::query(
        "UPDATE summary SET sections_json = ?, \
           edited_at = CASE WHEN ? THEN strftime('%Y-%m-%dT%H:%M:%fZ', 'now') ELSE edited_at END, \
           updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE meeting_id = ?",
    )
    .bind(sections_json)
    .bind(edited)
    .bind(meeting_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Saves the email (subject + Markdown body) of a summary.
pub async fn save_email(
    pool: &SqlitePool,
    meeting_id: &str,
    subject: &str,
    body: &str,
) -> Result<()> {
    sqlx::query("UPDATE summary SET email_subject = ?, email_md = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE meeting_id = ?")
        .bind(subject)
        .bind(body)
        .bind(meeting_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Stores (or clears, with None) the naming agent's pending suspects for a summary. Deliberately
/// leaves `updated_at` alone: this is the agent's working state, not a change to the summary.
pub async fn set_naming(
    pool: &SqlitePool,
    meeting_id: &str,
    naming_json: Option<&str>,
) -> Result<()> {
    sqlx::query("UPDATE summary SET naming_json = ? WHERE meeting_id = ?")
        .bind(naming_json)
        .bind(meeting_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Meetings whose recording was cut short (a quit or crash left them `recording`).
pub async fn interrupted_recordings(pool: &SqlitePool) -> Result<Vec<(String, Option<String>)>> {
    Ok(
        sqlx::query_as("SELECT id, audio_path FROM meeting WHERE status = 'recording'")
            .fetch_all(pool)
            .await?,
    )
}

/// A job can't survive a quit or crash: settle meetings left `transcribing` at launch.
pub async fn recover_interrupted(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "UPDATE meeting SET status = CASE WHEN EXISTS (SELECT 1 FROM transcript_segment s WHERE s.meeting_id = meeting.id) \
         THEN 'transcribed' ELSE 'failed' END, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE status = 'transcribing'",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "UPDATE meeting SET status = CASE WHEN EXISTS (SELECT 1 FROM summary s WHERE s.meeting_id = meeting.id) \
         THEN 'summarised' ELSE 'transcribed' END, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE status = 'summarising'",
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn transcript(pool: &SqlitePool, meeting_id: &str) -> Result<Vec<TranscriptSegment>> {
    Ok(sqlx::query_as("SELECT seq, start_ms, end_ms, text, raw_text, translation FROM transcript_segment WHERE meeting_id = ? ORDER BY seq")
        .bind(meeting_id)
        .fetch_all(pool)
        .await?)
}

/// Sets a line's text from a correction, keeping what Whisper wrote (for Undo) unless a
/// previous correction already kept it.
pub async fn correct_segment(
    pool: &SqlitePool,
    meeting_id: &str,
    seq: u32,
    text: &str,
    whisper_text: &str,
) -> Result<()> {
    sqlx::query(
        "UPDATE transcript_segment SET text = ?, raw_text = coalesce(raw_text, ?), \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE meeting_id = ? AND seq = ?",
    )
    .bind(text)
    .bind(whisper_text)
    .bind(meeting_id)
    .bind(seq)
    .execute(pool)
    .await?;
    Ok(())
}

/// Replaces the meeting's transcript and marks it transcribed, in one transaction.
/// Delete + insert (never REPLACE) so the FTS triggers stay correct (D-012).
/// `raw` holds, per line, what Whisper wrote when the vocabulary corrected it (may be empty).
pub async fn save_transcript(
    pool: &SqlitePool,
    meeting_id: &str,
    segments: &[Segment],
    raw: &[Option<String>],
    duration_ms: u32,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM transcript_segment WHERE meeting_id = ?")
        .bind(meeting_id)
        .execute(&mut *tx)
        .await?;
    for (seq, segment) in segments.iter().enumerate() {
        sqlx::query("INSERT INTO transcript_segment (id, meeting_id, seq, start_ms, end_ms, text, raw_text) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(new_id())
            .bind(meeting_id)
            .bind(seq as u32)
            .bind(segment.start_ms)
            .bind(segment.end_ms)
            .bind(&segment.text)
            .bind(raw.get(seq).cloned().flatten())
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query(
        "UPDATE meeting SET status = 'transcribed', duration_ms = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?",
    )
    .bind(duration_ms)
    .bind(meeting_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Appends one line of a live session (D-041) as it arrives; the FTS trigger indexes it like any
/// other. `raw` is what Whisper wrote when the vocabulary corrected the line.
pub async fn append_segment(
    pool: &SqlitePool,
    meeting_id: &str,
    seq: u32,
    segment: &Segment,
    raw: Option<&str>,
) -> Result<()> {
    sqlx::query("INSERT INTO transcript_segment (id, meeting_id, seq, start_ms, end_ms, text, raw_text) VALUES (?, ?, ?, ?, ?, ?, ?)")
        .bind(new_id())
        .bind(meeting_id)
        .bind(seq)
        .bind(segment.start_ms)
        .bind(segment.end_ms)
        .bind(&segment.text)
        .bind(raw)
        .execute(pool)
        .await?;
    Ok(())
}

/// Stores the English of a live line (D-041).
pub async fn set_translation(
    pool: &SqlitePool,
    meeting_id: &str,
    seq: u32,
    translation: &str,
) -> Result<()> {
    sqlx::query(
        "UPDATE transcript_segment SET translation = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE meeting_id = ? AND seq = ?",
    )
    .bind(translation)
    .bind(meeting_id)
    .bind(seq)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        MeetingStatus, NewMeeting, append_segment, create, fts_query, save_transcript, search,
        set_translation, transcript,
    };
    use crate::kernel::ai::whisper::{Language, Segment};
    use crate::kernel::hub::WorkspaceHub;

    #[test]
    fn live_lines_append_and_take_their_translation() {
        let root = std::env::temp_dir().join(format!("newdawn-live-db-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("Test", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let new = NewMeeting {
                id: "m1",
                meeting_type_id: None,
                title: "Live",
                held_at_unix: 0,
                language: Language::Vi,
                audio_path: "/tmp/x.wav",
                status: MeetingStatus::Recording,
            };
            create(&pool, new).await.unwrap();
            let line = |start_ms, text: &str| Segment {
                start_ms,
                end_ms: start_ms + 2_000,
                text: text.into(),
            };
            append_segment(&pool, "m1", 0, &line(0, "Chào mọi người"), None)
                .await
                .unwrap();
            append_segment(
                &pool,
                "m1",
                1,
                &line(3_000, "Đồng ý với WiredScore"),
                Some("Đồng ý với Wired Score"),
            )
            .await
            .unwrap();
            set_translation(&pool, "m1", 1, "Agreed with WiredScore")
                .await
                .unwrap();

            let lines = transcript(&pool, "m1").await.unwrap();
            assert_eq!(lines.len(), 2);
            assert_eq!((lines[0].seq, lines[0].translation.as_deref()), (0, None));
            assert_eq!(lines[1].start_ms, 3_000);
            assert_eq!(lines[1].raw_text.as_deref(), Some("Đồng ý với Wired Score"));
            assert_eq!(
                lines[1].translation.as_deref(),
                Some("Agreed with WiredScore")
            );
            assert_eq!(
                search(&pool, "dong y").await.unwrap().len(),
                1,
                "live lines are searchable as they arrive"
            );
            assert!(
                append_segment(&pool, "m1", 1, &line(5_000, "again"), None)
                    .await
                    .is_err(),
                "a line number is used once"
            );
            hub.close_all().await;
        });
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn fts_query_is_safe() {
        assert_eq!(fts_query("acme cap").as_deref(), Some("\"acme\" \"cap\"*"));
        assert_eq!(
            fts_query(r#"  "OR" NOT( a*b) "#).as_deref(),
            Some("\"OR\" \"NOT(\" \"a*b)\"*")
        );
        assert_eq!(fts_query("   "), None);
        assert_eq!(
            fts_query("Q3 - results").as_deref(),
            Some(r#""Q3" "results"*"#)
        );
    }

    #[test]
    fn searches_titles_and_transcripts() {
        let root = std::env::temp_dir().join(format!("newdawn-search-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("Test", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let new = |id: &'static str, title: &'static str| NewMeeting {
                id,
                meeting_type_id: None,
                title,
                held_at_unix: 0,
                language: Language::En,
                audio_path: "/tmp/x.wav",
                status: MeetingStatus::Recorded,
            };
            create(&pool, new("m1", "Partner call with Acme Capital"))
                .await
                .unwrap();
            create(&pool, new("m2", "Internal sync")).await.unwrap();
            let seg = |start_ms, text: &str| Segment {
                start_ms,
                end_ms: start_ms + 1000,
                text: text.into(),
            };
            save_transcript(
                &pool,
                "m2",
                &[
                    seg(0, "Welcome everyone"),
                    seg(65_000, "Tiếng Việt: the sandbox framework is published"),
                    seg(70_000, "Đồng ý với đối tác"),
                ],
                &[],
                75_000,
            )
            .await
            .unwrap();

            let hits = search(&pool, "acme").await.unwrap();
            assert_eq!(hits.len(), 1);
            assert_eq!(hits[0].meeting.id, "m1");

            let hits = search(&pool, "sandbox frame").await.unwrap(); // prefix on the last word
            assert_eq!(hits.len(), 1);
            assert_eq!(hits[0].at_ms, Some(65_000));
            assert!(hits[0].snippet.as_deref().unwrap().contains('\u{2}'));

            assert_eq!(
                search(&pool, "tiếng").await.unwrap().len(),
                1,
                "Vietnamese matches with its accents"
            );
            assert_eq!(
                search(&pool, "tieng viet").await.unwrap().len(),
                1,
                "and without them (0002: remove_diacritics 2)"
            );
            assert!(
                search(&pool, r#"" OR ) NEAR("#).await.is_ok(),
                "operators are plain text"
            );
            assert_eq!(
                search(&pool, "dong y doi tac").await.unwrap().len(),
                1,
                "đ folds to d (0002: folded column)"
            );
            assert_eq!(
                search(&pool, "sandbox - framework").await.unwrap().len(),
                1,
                "lone punctuation is ignored"
            );
            assert_eq!(
                search(&pool, "acme partner").await.unwrap().len(),
                1,
                "title words match in any order"
            );
            hub.close_all().await;
        });
        std::fs::remove_dir_all(root).unwrap();
    }
}
