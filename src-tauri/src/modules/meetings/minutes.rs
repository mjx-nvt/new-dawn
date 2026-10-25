//! Meeting minutes: summary → the workspace's DOCX template → optional PDF (D-024). Every export
//! is a `document_export` row; files go to `workspaces/<slug>/exports/`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Local, NaiveTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;
use tauri::{AppHandle, Manager};

use super::summarise;
use crate::error::AppError;
use crate::kernel::export::docx;
use crate::kernel::export::model::{self, MinutesInput};
use crate::kernel::export::pdf::{self, Converter};
use crate::kernel::hub::WorkspaceHub;

#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct MinutesExport {
    pub id: String,
    pub meeting_id: String,
    pub docx_path: Option<String>,
    pub pdf_path: Option<String>,
    pub converter: Option<Converter>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Type)]
pub struct ExportOptions {
    /// Fills a `{{#transcript}}` block when the template has one.
    pub include_transcript: bool,
    /// Without extension; default "<yyyy-mm-dd> <title>".
    pub file_name: Option<String>,
}

pub async fn list(pool: &SqlitePool, meeting_id: &str) -> Result<Vec<MinutesExport>> {
    Ok(sqlx::query_as(
        "SELECT id, meeting_id, docx_path, pdf_path, converter, created_at, updated_at \
         FROM document_export WHERE meeting_id = ? ORDER BY created_at DESC",
    )
    .bind(meeting_id)
    .fetch_all(pool)
    .await?)
}

async fn get(pool: &SqlitePool, id: &str) -> Result<MinutesExport> {
    let found: Option<MinutesExport> = sqlx::query_as(
        "SELECT id, meeting_id, docx_path, pdf_path, converter, created_at, updated_at \
         FROM document_export WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(found.ok_or_else(|| AppError::not_found(format!("No export '{id}'")))?)
}

/// Fills the workspace's minutes template from the meeting's summary.
pub async fn export_docx(
    app: &AppHandle,
    slug: &str,
    meeting_id: &str,
    options: ExportOptions,
) -> Result<MinutesExport> {
    let hub = app.state::<WorkspaceHub>();
    let pool = hub.pool(slug)?;
    let workspace = hub.workspace(slug).await?;
    let meeting = super::get(&pool, meeting_id).await?;
    let summary = summarise::view(&pool, meeting_id)
        .await?
        .ok_or_else(|| AppError::invalid("Write the summary before exporting the minutes"))?;
    let start = DateTime::parse_from_rfc3339(&meeting.held_on)
        .with_context(|| format!("reading the meeting time {}", meeting.held_on))?
        .with_timezone(&Local)
        .fixed_offset();
    let meeting_type: Option<(String,)> =
        sqlx::query_as("SELECT name FROM meeting_type WHERE id = ?")
            .bind(&meeting.meeting_type_id)
            .fetch_optional(&pool)
            .await?;
    let transcript = if options.include_transcript {
        Some(summarise::transcript_text(
            &super::transcript(&pool, meeting_id).await?,
        ))
    } else {
        None
    };
    let owner = crate::kernel::profile::owner(&hub, slug)
        .await?
        .unwrap_or_default();
    let slogan = crate::kernel::brand::tokens::get(app, slug).await?.slogan;
    let data = model::minutes(&MinutesInput {
        meeting: &meeting,
        doc: &summary.doc,
        start,
        meeting_type: meeting_type.as_ref().map(|(name,)| name.as_str()),
        day_seq: day_seq(&pool, &meeting.id, &meeting.held_on, start).await?,
        transcript: transcript.as_deref(),
        sender: &model::Sender {
            workspace: &workspace.name,
            slogan_en: &slogan.en,
            slogan_vi: &slogan.vi,
            name: &owner.name,
            title: &owner.title,
        },
    });

    let folder = Path::new(&workspace.folder_path);
    let template = super::template::path_for(app, slug, meeting.meeting_type_id.as_deref()).await?;
    let stem = options
        .file_name
        .as_deref()
        .map(file_stem)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| file_stem(&format!("{} {}", start.format("%Y-%m-%d"), meeting.title)));
    let out = reserve_path(&folder.join("exports"), &stem)?;
    let target = out.clone();
    let saved: Result<String> = async {
        tauri::async_runtime::spawn_blocking(move || docx::render(&template, &data, &target))
            .await??;
        let id = super::new_id();
        sqlx::query("INSERT INTO document_export (id, meeting_id, docx_path) VALUES (?, ?, ?)")
            .bind(&id)
            .bind(meeting_id)
            .bind(out.display().to_string())
            .execute(&pool)
            .await?;
        Ok(id)
    }
    .await;
    match saved {
        Ok(id) => get(&pool, &id).await,
        Err(err) => {
            let _ = std::fs::remove_file(&out); // the reserved name, or a DOCX with no row
            Err(err)
        }
    }
}

/// Converts an export's DOCX (including any edits made in Word since) to a PDF beside it;
/// run again, it replaces that PDF.
pub async fn export_pdf(hub: &WorkspaceHub, slug: &str, export_id: &str) -> Result<MinutesExport> {
    let pool = &hub.pool(slug)?;
    let docx = file_path(hub, slug, export_id, MinutesFile::Docx).await?;
    let pdf_path = docx.with_extension("pdf");
    let converter = pdf::convert(&docx, &pdf_path).await?;
    sqlx::query(
        "UPDATE document_export SET pdf_path = ?, converter = ?, \
         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?",
    )
    .bind(pdf_path.display().to_string())
    .bind(converter)
    .bind(export_id)
    .execute(pool)
    .await?;
    get(pool, export_id).await
}

#[derive(Debug, Clone, Copy, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum MinutesFile {
    Docx,
    Pdf,
}

/// Where an export's file is: it must still exist and be a .docx/.pdf inside the workspace's
/// exports folder (a stored path is never trusted to point anywhere else).
async fn file_path(
    hub: &WorkspaceHub,
    slug: &str,
    export_id: &str,
    file: MinutesFile,
) -> Result<PathBuf> {
    let export = get(&hub.pool(slug)?, export_id).await?;
    let (stored, extension) = match file {
        MinutesFile::Docx => (export.docx_path, "docx"),
        MinutesFile::Pdf => (export.pdf_path, "pdf"),
    };
    let exports = Path::new(&hub.workspace(slug).await?.folder_path)
        .join("exports")
        .canonicalize()?;
    stored
        .and_then(|p| PathBuf::from(p).canonicalize().ok())
        .filter(|p| {
            p.is_file() && p.starts_with(&exports) && p.extension().is_some_and(|e| e == extension)
        })
        .ok_or_else(|| {
            AppError::not_found("The file was moved or deleted — export it again").into()
        })
}

/// Moves an export's files to the Trash and removes it from the list.
pub async fn delete_export(hub: &WorkspaceHub, slug: &str, export_id: &str) -> Result<()> {
    for file in [MinutesFile::Docx, MinutesFile::Pdf] {
        // Files already gone (or never made) are fine; only existing exports-folder files move.
        if let Ok(path) = file_path(hub, slug, export_id, file).await {
            super::review::to_trash(path).await?;
        }
    }
    let done = sqlx::query("DELETE FROM document_export WHERE id = ?")
        .bind(export_id)
        .execute(&hub.pool(slug)?)
        .await?;
    if done.rows_affected() == 0 {
        bail!(AppError::not_found(format!("No export '{export_id}'")));
    }
    Ok(())
}

/// Opens an export's file in its default app, or shows it in Finder.
pub async fn open_file(
    hub: &WorkspaceHub,
    slug: &str,
    export_id: &str,
    file: MinutesFile,
    reveal: bool,
) -> Result<()> {
    let path = file_path(hub, slug, export_id, file).await?;
    if reveal {
        tauri_plugin_opener::reveal_item_in_dir(path)?;
    } else {
        tauri_plugin_opener::open_path(path, None::<&str>)?;
    }
    Ok(())
}

/// 1-based position of the meeting among those held on the same local day (for the reference).
async fn day_seq(
    pool: &SqlitePool,
    id: &str,
    held_on: &str,
    start: DateTime<chrono::FixedOffset>,
) -> Result<u32> {
    let midnight = start
        .with_time(NaiveTime::MIN)
        .single()
        .unwrap_or(start)
        .with_timezone(&Utc)
        .to_rfc3339_opts(SecondsFormat::Millis, true);
    let (n,): (u32,) = sqlx::query_as(
        "SELECT count(*) FROM meeting WHERE held_on >= ?1 AND (held_on < ?2 OR (held_on = ?2 AND id <= ?3))",
    )
    .bind(midnight)
    .bind(held_on)
    .bind(id)
    .fetch_one(pool)
    .await?;
    Ok(n.max(1))
}

/// A safe file name: no path separators, colons or control characters, no typed extension, at
/// most 200 bytes (macOS allows 255; room for " 12.docx.tmp").
fn file_stem(name: &str) -> String {
    let name = name.trim();
    let name = [".docx", ".pdf"]
        .iter()
        .find_map(|ext| name.strip_suffix(ext))
        .unwrap_or(name);
    let mut cleaned = String::new();
    for c in name.chars() {
        let c = if c.is_control() || matches!(c, '/' | '\\' | ':') {
            '-'
        } else {
            c
        };
        if cleaned.len() + c.len_utf8() > 200 {
            break;
        }
        cleaned.push(c);
    }
    cleaned.trim().trim_start_matches('.').trim().to_owned()
}

/// Claims `<stem>.docx`, or `<stem> 2.docx`, … by creating it empty, so two exports at once can't
/// pick the same name; never overwrites a file the user may have edited.
fn reserve_path(dir: &Path, stem: &str) -> Result<PathBuf> {
    for n in 1.. {
        let name = if n == 1 {
            format!("{stem}.docx")
        } else {
            format!("{stem} {n}.docx")
        };
        let path = dir.join(name);
        if path.with_extension("pdf").exists() {
            continue;
        }
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(_) => return Ok(path),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(err).with_context(|| format!("creating {}", path.display())),
        }
    }
    unreachable!("names run out after usize::MAX tries")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_are_safe_and_unique() {
        assert_eq!(
            file_stem("2026-10-24 Q3 review: Acme/VIFC\n"),
            "2026-10-24 Q3 review- Acme-VIFC"
        );
        assert_eq!(file_stem("../../x"), "-..-x");
        assert_eq!(file_stem("Biên bản.docx "), "Biên bản");
        assert!(file_stem(&"ệ".repeat(300)).len() <= 200);
        let dir = std::env::temp_dir().join(format!("newdawn-exp-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(reserve_path(&dir, "M").unwrap(), dir.join("M.docx"));
        std::fs::write(dir.join("M 2.pdf"), "").unwrap();
        assert_eq!(reserve_path(&dir, "M").unwrap(), dir.join("M 3.docx"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn numbers_meetings_within_the_local_day() {
        let root = std::env::temp_dir().join(format!("newdawn-min-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("Test", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            // 23 Oct 23:30, 24 Oct 08:00 and 24 Oct 14:00 in Vietnam (UTC+7).
            for (id, at) in [
                ("a", "2026-10-23T16:30:00.000Z"),
                ("b", "2026-10-24T01:00:00.000Z"),
                ("c", "2026-10-24T07:00:00.000Z"),
            ] {
                sqlx::query("INSERT INTO meeting (id, title, held_on) VALUES (?, 'm', ?)")
                    .bind(id)
                    .bind(at)
                    .execute(&pool)
                    .await
                    .unwrap();
            }
            let local = |at: &str| {
                DateTime::parse_from_rfc3339(at)
                    .unwrap()
                    .with_timezone(&chrono::FixedOffset::east_opt(7 * 3600).unwrap())
            };
            let seq = |id: &'static str, at: &'static str| day_seq(&pool, id, at, local(at));
            assert_eq!(seq("a", "2026-10-23T16:30:00.000Z").await.unwrap(), 1);
            assert_eq!(seq("b", "2026-10-24T01:00:00.000Z").await.unwrap(), 1);
            assert_eq!(seq("c", "2026-10-24T07:00:00.000Z").await.unwrap(), 2);
            hub.close_all().await;
        });
        std::fs::remove_dir_all(root).unwrap();
    }
}
