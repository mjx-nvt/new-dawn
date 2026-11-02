//! Plane's pages (D-080): a project's documents beside its work — a name (empty reads Untitled),
//! an emoji, the editor's document, private or public, locked, labels, sub-pages. A locked page
//! takes no edit of its words; an archived one (with its sub-pages) none at all, and only an
//! archived page is deleted. Every saved change of the name or the document keeps a version (the
//! same hand's within ten minutes are one), and a version restored is the page's again. Exported
//! as Markdown.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::Type;
use sqlx::{Sqlite, SqlitePool, Transaction};

use super::items::{self, ItemFile, NAME_MAX};
use super::views::{self, Pin};
use crate::error::AppError;
use crate::kernel::records::types::Json;

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";
const EMOJI_MAX: usize = 8;

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

fn id() -> String {
    uuid::Uuid::now_v7().to_string()
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct PmPage {
    pub id: String,
    pub project_id: String,
    /// Its page, when it is a sub-page.
    pub parent_id: Option<String>,
    pub name: String,
    pub emoji: String,
    pub is_private: bool,
    pub is_locked: bool,
    pub labels: Vec<String>,
    pub archived_at: Option<String>,
    /// In the sidebar's Favorites.
    pub is_favorite: bool,
    pub sub_pages: u32,
    pub sort_order: f64,
    pub created_by: String,
    pub updated_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(sqlx::FromRow)]
struct Row {
    id: String,
    project_id: String,
    parent_id: Option<String>,
    name: String,
    emoji: String,
    is_private: bool,
    is_locked: bool,
    labels: Option<String>,
    archived_at: Option<String>,
    is_favorite: bool,
    sub_pages: u32,
    sort_order: f64,
    created_by: String,
    updated_by: String,
    created_at: String,
    updated_at: String,
}

impl From<Row> for PmPage {
    fn from(r: Row) -> Self {
        PmPage {
            id: r.id,
            project_id: r.project_id,
            parent_id: r.parent_id,
            name: r.name,
            emoji: r.emoji,
            is_private: r.is_private,
            is_locked: r.is_locked,
            labels: r.labels.map(|l| l.split('\u{1f}').filter(|s| !s.is_empty()).map(str::to_owned).collect()).unwrap_or_default(),
            archived_at: r.archived_at,
            is_favorite: r.is_favorite,
            sub_pages: r.sub_pages,
            sort_order: r.sort_order,
            created_by: r.created_by,
            updated_by: r.updated_by,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

const SELECT: &str = "SELECT p.id, p.project_id, p.parent_id, p.name, p.emoji, p.is_private, p.is_locked, \
     (SELECT group_concat(label_id, char(31)) FROM pm_page_label l WHERE l.page_id = p.id) AS labels, p.archived_at, \
     EXISTS (SELECT 1 FROM favorite f WHERE f.pm_page_id = p.id AND f.deleted_at IS NULL) AS is_favorite, \
     (SELECT count(*) FROM pm_page c WHERE c.parent_id = p.id AND c.archived_at IS NULL) AS sub_pages, \
     p.sort_order, p.created_by, p.updated_by, p.created_at, p.updated_at FROM pm_page p";

/// A page with its document.
#[derive(Debug, Clone, Serialize, Type)]
pub struct PmPageDoc {
    pub page: PmPage,
    pub description: Option<Json>,
}

/// A project's pages, the archived ones too (the page filters them), favorites first.
pub async fn list(pool: &SqlitePool, project_id: &str) -> Result<Vec<PmPage>> {
    let rows: Vec<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} WHERE p.project_id = ? ORDER BY is_favorite DESC, p.sort_order, p.created_at"))).bind(project_id).fetch_all(pool).await?;
    Ok(rows.into_iter().map(PmPage::from).collect())
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<PmPage> {
    let row: Option<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} WHERE p.id = ?"))).bind(id).fetch_optional(pool).await?;
    row.map(PmPage::from).ok_or_else(|| anyhow!(AppError::not_found("That page is gone")))
}

pub async fn get_doc(pool: &SqlitePool, id: &str) -> Result<PmPageDoc> {
    let page = get(pool, id).await?;
    let (json,): (Option<String>,) = sqlx::query_as("SELECT description_json FROM pm_page WHERE id = ?").bind(id).fetch_one(pool).await?;
    Ok(PmPageDoc { page, description: json.and_then(|j| serde_json::from_str(&j).ok()).map(Json) })
}

/// One line, at most 255 characters; empty reads Untitled.
fn clean_name(name: &str) -> Result<String> {
    let n = name.split_whitespace().collect::<Vec<_>>().join(" ");
    if n.chars().count() > NAME_MAX {
        return Err(invalid(format!("A page's name is at most {NAME_MAX} characters")));
    }
    Ok(n)
}

fn clean_emoji(emoji: &str) -> String {
    emoji.trim().chars().take(EMOJI_MAX).collect()
}

/// A page and every page under it, the page first.
async fn with_sub_pages(pool: &SqlitePool, id: &str) -> Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as("WITH RECURSIVE t(id) AS (SELECT ? UNION SELECT p.id FROM pm_page p JOIN t ON p.parent_id = t.id) SELECT id FROM t").bind(id).fetch_all(pool).await?;
    Ok(rows.into_iter().map(|(i,)| i).collect())
}

#[derive(Debug, Clone, Deserialize, Type)]
pub struct PageInput {
    pub project_id: String,
    /// Made inside a page (a sub-page).
    pub parent_id: Option<String>,
    pub name: String,
    pub emoji: String,
    pub is_private: bool,
    pub labels: Vec<String>,
}

pub async fn create(pool: &SqlitePool, input: PageInput, actor: &str) -> Result<PmPage> {
    let project = super::get(pool, &input.project_id).await?;
    if project.archived_at.is_some() {
        return Err(invalid("The project is archived — restore it to add pages"));
    }
    let parent = match input.parent_id.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
        Some(p) => {
            let parent = get(pool, p).await?;
            if parent.project_id != project.id {
                return Err(invalid("A sub-page is in its page's project"));
            }
            if parent.archived_at.is_some() {
                return Err(invalid("That page is archived — restore it first"));
            }
            Some(parent.id)
        }
        None => None,
    };
    let name = clean_name(&input.name)?;
    let labels = items::check_labels(pool, &project.id, &input.labels).await?;
    let (last,): (Option<f64>,) = sqlx::query_as("SELECT max(sort_order) FROM pm_page WHERE project_id = ? AND parent_id IS ?").bind(&project.id).bind(&parent).fetch_one(pool).await?;
    let page = id();
    let mut tx = pool.begin().await?;
    sqlx::query("INSERT INTO pm_page (id, project_id, parent_id, name, emoji, is_private, sort_order, created_by, updated_by) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(&page)
        .bind(&project.id)
        .bind(&parent)
        .bind(&name)
        .bind(clean_emoji(&input.emoji))
        .bind(input.is_private)
        .bind(last.unwrap_or(0.0) + 1000.0)
        .bind(actor)
        .bind(actor)
        .execute(&mut *tx)
        .await?;
    for l in &labels {
        sqlx::query("INSERT INTO pm_page_label (page_id, label_id) VALUES (?, ?)").bind(&page).bind(l).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    get(pool, &page).await
}

/// The edits; a field left out stays. The document is the editor's JSON text.
#[derive(Debug, Clone, Default, Deserialize, Type)]
pub struct PagePatch {
    pub name: Option<String>,
    pub emoji: Option<String>,
    pub description: Option<String>,
    pub is_private: Option<bool>,
    pub is_locked: Option<bool>,
    pub labels: Option<Vec<String>>,
    pub sort_order: Option<f64>,
}

/// Changes a page: an archived one takes nothing; a locked one only its unlocking, its access
/// and its order. A change of the name or the document is kept as a version.
pub async fn update(pool: &SqlitePool, id: &str, patch: PagePatch, actor: &str) -> Result<PmPage> {
    let p = get(pool, id).await?;
    if p.archived_at.is_some() {
        return Err(invalid("An archived page is read-only — restore it to change it"));
    }
    let words = patch.name.is_some() || patch.emoji.is_some() || patch.description.is_some() || patch.labels.is_some();
    if words && p.is_locked && patch.is_locked != Some(false) {
        return Err(invalid("The page is locked — unlock it to edit it"));
    }
    let name = patch.name.as_deref().map(clean_name).transpose()?;
    let description = patch.description.as_deref().map(|d| items::clean_description(Some(d))).transpose()?;
    let labels = match &patch.labels {
        Some(l) => Some(items::check_labels(pool, &p.project_id, l).await?),
        None => None,
    };
    // Nothing changed (a click in and out of the editor): nothing written, the page not "updated".
    let (doc_now,): (Option<String>,) = sqlx::query_as("SELECT description_json FROM pm_page WHERE id = ?").bind(id).fetch_one(pool).await?;
    let changed = name.as_ref().is_some_and(|n| *n != p.name)
        || patch.emoji.as_deref().map(clean_emoji).is_some_and(|e| e != p.emoji)
        || description.as_ref().is_some_and(|(j, _)| *j != doc_now)
        || patch.is_private.is_some_and(|v| v != p.is_private)
        || patch.is_locked.is_some_and(|v| v != p.is_locked)
        || patch.sort_order.filter(|o| o.is_finite()).is_some_and(|o| o != p.sort_order)
        || labels.as_ref().is_some_and(|l| l.len() != p.labels.len() || l.iter().any(|x| !p.labels.contains(x)));
    if !changed {
        return Ok(p);
    }
    let mut tx = pool.begin().await?;
    let before: (String, Option<String>) = sqlx::query_as("SELECT name, description_json FROM pm_page WHERE id = ?").bind(id).fetch_one(&mut *tx).await?;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE pm_page SET name = coalesce(?, name), emoji = coalesce(?, emoji), \
         description_json = CASE WHEN ? THEN ? ELSE description_json END, description_text = CASE WHEN ? THEN ? ELSE description_text END, \
         is_private = coalesce(?, is_private), is_locked = coalesce(?, is_locked), sort_order = coalesce(?, sort_order), \
         updated_by = ?, updated_at = {NOW} WHERE id = ?"
    )))
    .bind(&name)
    .bind(patch.emoji.as_deref().map(clean_emoji))
    .bind(description.is_some())
    .bind(description.as_ref().and_then(|(j, _)| j.clone()))
    .bind(description.is_some())
    .bind(description.as_ref().map(|(_, t)| t.clone()))
    .bind(patch.is_private)
    .bind(patch.is_locked)
    .bind(patch.sort_order.filter(|o| o.is_finite()))
    .bind(actor)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    if let Some(l) = &labels {
        sqlx::query("DELETE FROM pm_page_label WHERE page_id = ?").bind(id).execute(&mut *tx).await?;
        for label in l {
            sqlx::query("INSERT INTO pm_page_label (page_id, label_id) VALUES (?, ?)").bind(id).bind(label).execute(&mut *tx).await?;
        }
    }
    let after: (String, Option<String>, String) = sqlx::query_as("SELECT name, description_json, description_text FROM pm_page WHERE id = ?").bind(id).fetch_one(&mut *tx).await?;
    if (&after.0, &after.1) != (&before.0, &before.1) {
        version(&mut tx, id, &after, actor, true).await?;
    }
    tx.commit().await?;
    get(pool, id).await
}

/// Keeps the page as it now is — or, `merge`, changes the newest version when it is the same
/// hand's within ten minutes (autosaves are one change). A restore never merges.
async fn version(tx: &mut Transaction<'_, Sqlite>, page: &str, now: &(String, Option<String>, String), actor: &str, merge: bool) -> Result<()> {
    let (name, json, text) = now;
    let recent: Option<(String,)> = if merge {
        sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT id FROM pm_page_version WHERE page_id = ? AND created_by = ? AND created_at > strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-{} minutes') ORDER BY created_at DESC, id DESC LIMIT 1",
            items::VERSION_MINUTES
        )))
        .bind(page)
        .bind(actor)
        .fetch_optional(&mut **tx)
        .await?
    } else {
        None
    };
    match recent {
        Some((v,)) => {
            sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE pm_page_version SET name = ?, description_json = ?, description_text = ?, updated_at = {NOW} WHERE id = ?")))
                .bind(name)
                .bind(json)
                .bind(text)
                .bind(v)
                .execute(&mut **tx)
                .await?;
        }
        None => {
            sqlx::query("INSERT INTO pm_page_version (id, page_id, name, description_json, description_text, created_by) VALUES (?, ?, ?, ?, ?, ?)")
                .bind(id())
                .bind(page)
                .bind(name)
                .bind(json)
                .bind(text)
                .bind(actor)
                .execute(&mut **tx)
                .await?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct PmPageVersion {
    pub id: String,
    pub name: String,
    pub description: Option<Json>,
    pub description_text: String,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(sqlx::FromRow)]
struct VersionRow {
    id: String,
    name: String,
    description_json: Option<String>,
    description_text: String,
    created_by: String,
    created_at: String,
    updated_at: String,
}

/// A page's versions, newest first.
pub async fn versions(pool: &SqlitePool, id: &str) -> Result<Vec<PmPageVersion>> {
    get(pool, id).await?;
    let rows: Vec<VersionRow> =
        sqlx::query_as("SELECT id, name, description_json, description_text, created_by, created_at, updated_at FROM pm_page_version WHERE page_id = ? ORDER BY updated_at DESC, id DESC").bind(id).fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|r| PmPageVersion { id: r.id, name: r.name, description: r.description_json.and_then(|j| serde_json::from_str(&j).ok()).map(Json), description_text: r.description_text, created_by: r.created_by, created_at: r.created_at, updated_at: r.updated_at })
        .collect())
}

/// A version's name and document become the page's again — a version of its own. Not while the
/// page is locked or archived.
pub async fn restore(pool: &SqlitePool, version_id: &str, actor: &str) -> Result<PmPage> {
    let row: Option<(String, String, Option<String>, String)> = sqlx::query_as("SELECT page_id, name, description_json, description_text FROM pm_page_version WHERE id = ?").bind(version_id).fetch_optional(pool).await?;
    let (page, name, json, text) = row.ok_or_else(|| anyhow!(AppError::not_found("That version is gone")))?;
    let p = get(pool, &page).await?;
    if p.archived_at.is_some() {
        return Err(invalid("An archived page is read-only — restore it to change it"));
    }
    if p.is_locked {
        return Err(invalid("The page is locked — unlock it to edit it"));
    }
    let mut tx = pool.begin().await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE pm_page SET name = ?, description_json = ?, description_text = ?, updated_by = ?, updated_at = {NOW} WHERE id = ?")))
        .bind(&name)
        .bind(&json)
        .bind(&text)
        .bind(actor)
        .bind(&page)
        .execute(&mut *tx)
        .await?;
    version(&mut tx, &page, &(name, json, text), actor, false).await?;
    tx.commit().await?;
    get(pool, &page).await
}

/// Archived with its sub-pages (unstarred, as Plane does), or back. A page restored under a page
/// still archived comes back at the top.
pub async fn archive(pool: &SqlitePool, id: &str, on: bool) -> Result<PmPage> {
    let p = get(pool, id).await?;
    let all = with_sub_pages(pool, id).await?;
    let mut tx = pool.begin().await?;
    for page in &all {
        let sql = if on { format!("UPDATE pm_page SET archived_at = coalesce(archived_at, {NOW}), updated_at = {NOW} WHERE id = ?") } else { format!("UPDATE pm_page SET archived_at = NULL, updated_at = {NOW} WHERE id = ?") };
        sqlx::query(sqlx::AssertSqlSafe(sql)).bind(page).execute(&mut *tx).await?;
        if on {
            sqlx::query("DELETE FROM favorite WHERE pm_page_id = ?").bind(page).execute(&mut *tx).await?;
        }
    }
    if !on {
        if let Some(parent) = &p.parent_id {
            let (archived,): (Option<String>,) = sqlx::query_as("SELECT archived_at FROM pm_page WHERE id = ?").bind(parent).fetch_one(&mut *tx).await?;
            if archived.is_some() {
                sqlx::query("UPDATE pm_page SET parent_id = NULL WHERE id = ?").bind(id).execute(&mut *tx).await?;
            }
        }
    }
    tx.commit().await?;
    get(pool, id).await
}

/// Deleted with its sub-pages and their files — an archived page only (Plane's).
pub async fn delete(pool: &SqlitePool, folder: &str, id: &str) -> Result<()> {
    if get(pool, id).await?.archived_at.is_none() {
        return Err(invalid("Archive a page before deleting it"));
    }
    let all = with_sub_pages(pool, id).await?;
    let mut tx = pool.begin().await?;
    for page in all.iter().rev() {
        sqlx::query("DELETE FROM pm_page WHERE id = ?").bind(page).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    for page in &all {
        let _ = std::fs::remove_dir_all(items::files_dir(folder, page));
    }
    Ok(())
}

/// Into the sidebar's Favorites, or out (an archived page takes no star).
pub async fn favorite(pool: &SqlitePool, id: &str, on: bool) -> Result<PmPage> {
    if on && get(pool, id).await?.archived_at.is_some() {
        return Err(invalid("An archived page is read-only — restore it to star it"));
    }
    views::pin(pool, Pin::Page, id, on).await?;
    get(pool, id).await
}

/// "Copy of …" beside it: the same document (its pictures copied to the copy's folder), emoji,
/// access and labels; not locked; no sub-pages.
pub async fn duplicate(pool: &SqlitePool, folder: &str, id: &str, actor: &str) -> Result<PmPage> {
    let doc = get_doc(pool, id).await?;
    let p = doc.page;
    if p.archived_at.is_some() {
        return Err(invalid("An archived page is read-only — restore it first"));
    }
    let base = if p.name.is_empty() { "Untitled" } else { p.name.as_str() };
    let name: String = format!("Copy of {base}").chars().take(NAME_MAX).collect();
    let made = create(pool, PageInput { project_id: p.project_id, parent_id: p.parent_id, name, emoji: p.emoji, is_private: p.is_private, labels: p.labels }, actor).await?;
    if let Some(d) = doc.description {
        let mut text = d.0.to_string();
        if items::copy_files(folder, &p.id, &made.id, &[]).await? {
            let (a, b) = items::picture_addresses(&p.id, &made.id);
            text = text.replace(&a, &b);
        }
        update(pool, &made.id, PagePatch { description: Some(text), ..PagePatch::default() }, actor).await?;
    }
    get(pool, &made.id).await
}

/// A picture for a page, kept in its folder — not while it's locked or archived.
pub async fn add_file_bytes(pool: &SqlitePool, folder: &str, id: &str, name: &str, bytes: &[u8]) -> Result<ItemFile> {
    let p = get(pool, id).await?;
    if p.archived_at.is_some() || p.is_locked {
        return Err(invalid("The page takes no edit now — unlock or restore it first"));
    }
    items::keep_bytes(folder, id, name, bytes).await
}

pub async fn add_file(pool: &SqlitePool, folder: &str, id: &str, source: &Path) -> Result<ItemFile> {
    let (name, bytes) = items::read_file(source).await?;
    add_file_bytes(pool, folder, id, &name, &bytes).await
}

// ── Markdown ──

/// A page as Markdown: its name as the title, then its blocks.
pub(crate) fn markdown(name: &str, doc: Option<&Value>) -> String {
    let mut out = format!("# {}\n\n", if name.is_empty() { "Untitled" } else { name });
    if let Some(blocks) = doc.and_then(|d| d.get("content")).and_then(Value::as_array) {
        write_blocks(blocks, "", &mut out);
    }
    format!("{}\n", out.trim_end())
}

fn children(v: &Value) -> &[Value] {
    v.get("content").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn attr<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.get("attrs").and_then(|a| a.get(key))
}

/// Inline content: text with its marks, mentions, line breaks, pictures.
fn inline(content: &[Value]) -> String {
    let mut out = String::new();
    for v in content {
        match v.get("type").and_then(Value::as_str) {
            Some("text") => {
                // The marks around the words, the spaces outside them ("** now**" isn't bold).
                let raw = v.get("text").and_then(Value::as_str).unwrap_or("");
                let core = raw.trim();
                let (lead, trail) = if core.is_empty() { (raw, "") } else { (&raw[..raw.len() - raw.trim_start().len()], &raw[raw.trim_end().len()..]) };
                out.push_str(lead);
                let mut t = core.to_owned();
                let marks: Vec<&str> = if core.is_empty() { Vec::new() } else { v.get("marks").and_then(Value::as_array).map(|m| m.iter().filter_map(|m| m.get("type").and_then(Value::as_str)).collect()).unwrap_or_default() };
                if marks.contains(&"code") {
                    t = format!("`{t}`");
                }
                if marks.contains(&"bold") {
                    t = format!("**{t}**");
                }
                if marks.contains(&"italic") {
                    t = format!("*{t}*");
                }
                if marks.contains(&"strike") {
                    t = format!("~~{t}~~");
                }
                let link = v.get("marks").and_then(Value::as_array).and_then(|m| m.iter().find(|m| m.get("type").and_then(Value::as_str) == Some("link"))).and_then(|m| attr(m, "href")).and_then(Value::as_str);
                out.push_str(&match link {
                    Some(href) if !core.is_empty() => format!("[{t}]({href})"),
                    _ => t,
                });
                out.push_str(trail);
            }
            Some("mention") => out.push_str(&format!("@{}", attr(v, "label").and_then(Value::as_str).unwrap_or(""))),
            Some("hardBreak") => out.push_str("  \n"),
            Some("image") => out.push_str(&format!("![{}]({})", attr(v, "alt").and_then(Value::as_str).unwrap_or(""), attr(v, "src").and_then(Value::as_str).unwrap_or(""))),
            _ => out.push_str(&inline(children(v))),
        }
    }
    out
}

/// Blocks, each line after the first indented by `pad` (a list item's body).
fn write_blocks(blocks: &[Value], pad: &str, out: &mut String) {
    for b in blocks {
        let kind = b.get("type").and_then(Value::as_str).unwrap_or("");
        match kind {
            "heading" => {
                let level = attr(b, "level").and_then(Value::as_u64).unwrap_or(1).clamp(1, 6) as usize;
                out.push_str(&format!("{pad}{} {}\n\n", "#".repeat(level), inline(children(b))));
            }
            "blockquote" | "callout" => {
                let mut inner = String::new();
                if kind == "callout" {
                    let emoji = attr(b, "emoji").and_then(Value::as_str).unwrap_or("💡");
                    inner.push_str(&format!("{emoji} "));
                }
                write_blocks(children(b), "", &mut inner);
                for line in inner.trim_end().lines() {
                    out.push_str(format!("{pad}> {line}").trim_end());
                    out.push('\n');
                }
                out.push('\n');
            }
            "codeBlock" => {
                let lang = attr(b, "language").and_then(Value::as_str).unwrap_or("");
                out.push_str(&format!("{pad}```{lang}\n"));
                for line in inline(children(b)).lines() {
                    out.push_str(&format!("{pad}{line}\n"));
                }
                out.push_str(&format!("{pad}```\n\n"));
            }
            "bulletList" | "orderedList" | "taskList" => {
                let start = attr(b, "start").and_then(Value::as_u64).unwrap_or(1);
                for (n, item) in children(b).iter().enumerate() {
                    let marker = match kind {
                        "orderedList" => format!("{}. ", start + n as u64),
                        "taskList" => format!("- [{}] ", if attr(item, "checked").and_then(Value::as_bool) == Some(true) { "x" } else { " " }),
                        _ => "- ".into(),
                    };
                    let mut body = String::new();
                    write_blocks(children(item), "", &mut body);
                    let inner_pad = format!("{pad}{}", " ".repeat(marker.len()));
                    for (i, line) in body.trim_end().lines().enumerate() {
                        if i == 0 {
                            out.push_str(&format!("{pad}{marker}{line}\n"));
                        } else if line.is_empty() {
                            out.push('\n');
                        } else {
                            out.push_str(&format!("{inner_pad}{line}\n"));
                        }
                    }
                }
                out.push('\n');
            }
            "horizontalRule" => out.push_str(&format!("{pad}---\n\n")),
            "table" => {
                let rows: Vec<Vec<String>> = children(b).iter().map(|r| children(r).iter().map(|c| inline_cell(children(c))).collect()).collect();
                if let Some(first) = rows.first() {
                    out.push_str(&format!("{pad}| {} |\n", first.join(" | ")));
                    out.push_str(&format!("{pad}|{}\n", " --- |".repeat(first.len())));
                    for r in &rows[1..] {
                        out.push_str(&format!("{pad}| {} |\n", r.join(" | ")));
                    }
                    out.push('\n');
                }
            }
            "image" => out.push_str(&format!("{pad}{}\n\n", inline(std::slice::from_ref(b)))),
            "pageLink" => out.push_str(&format!("{pad}📄 {}\n\n", attr(b, "title").and_then(Value::as_str).filter(|t| !t.is_empty()).unwrap_or("Untitled"))),
            _ => {
                let text = inline(children(b));
                out.push_str(&format!("{pad}{text}\n\n"));
            }
        }
    }
}

/// A table cell's paragraphs on one line, its pipes escaped.
fn inline_cell(blocks: &[Value]) -> String {
    blocks.iter().map(|p| inline(children(p))).collect::<Vec<_>>().join(" ").replace('|', "\\|")
}

/// Sub-page links named as their pages are now (a link keeps the name it was made with).
fn name_links(v: &mut Value, names: &HashMap<String, String>) {
    if v.get("type").and_then(Value::as_str) == Some("pageLink") {
        let id = attr(v, "id").and_then(Value::as_str).unwrap_or("").to_owned();
        if let (Some(name), Some(attrs)) = (names.get(&id), v.get_mut("attrs").and_then(Value::as_object_mut)) {
            attrs.insert("title".into(), Value::String(name.clone()));
        }
    }
    if let Some(children) = v.get_mut("content").and_then(Value::as_array_mut) {
        for c in children {
            name_links(c, names);
        }
    }
}

/// Writes a page as Markdown to `path` (the save dialog's: an absolute path to a .md file).
pub async fn export_markdown(pool: &SqlitePool, id: &str, path: &Path) -> Result<()> {
    if !path.is_absolute() || !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("md")) {
        return Err(invalid("A page is exported to a Markdown (.md) file"));
    }
    let doc = get_doc(pool, id).await?;
    let names: HashMap<String, String> = sqlx::query_as::<_, (String, String)>("SELECT id, name FROM pm_page WHERE project_id = ?").bind(&doc.page.project_id).fetch_all(pool).await?.into_iter().collect();
    let mut body = doc.description.map(|d| d.0);
    if let Some(b) = body.as_mut() {
        name_links(b, &names);
    }
    let text = markdown(&doc.page.name, body.as_ref());
    tokio::fs::write(path, text).await.with_context(|| format!("couldn't write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::hub::WorkspaceHub;
    use crate::modules::pm::{self, Features, NewPmProject, Network};

    #[test]
    fn a_page_keeps_versions_refuses_edits_locked_and_reads_as_markdown() {
        let root = std::env::temp_dir().join(format!("newdawn-pm-pages-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let folder = hub.workspace("test").await.unwrap().folder_path;
            let p = pm::create(&pool, NewPmProject { name: "VIFC Fintech Hub".into(), identifier: "VIFC".into(), description: String::new(), emoji: String::new(), cover: String::new(), network: Network::Private, lead: None, features: Features { pages: true, ..Features::default() } })
                .await
                .unwrap();
            let doc = |text: &str| serde_json::json!({ "type": "doc", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": text }] }] }).to_string();
            let input = PageInput { project_id: p.id.clone(), parent_id: None, name: "Sandbox rules".into(), emoji: "📘".into(), is_private: false, labels: vec![] };
            let page = create(&pool, input.clone(), "me").await.unwrap();
            assert_eq!((page.name.as_str(), page.is_locked, page.sub_pages), ("Sandbox rules", false, 0));

            // Two saves of mine are one version; an agent's is another.
            update(&pool, &page.id, PagePatch { description: Some(doc("First draft")), ..PagePatch::default() }, "me").await.unwrap();
            update(&pool, &page.id, PagePatch { description: Some(doc("Second draft")), ..PagePatch::default() }, "me").await.unwrap();
            assert_eq!(versions(&pool, &page.id).await.unwrap().len(), 1);
            update(&pool, &page.id, PagePatch { description: Some(doc("The agent's draft")), ..PagePatch::default() }, "Projects agent").await.unwrap();
            let all = versions(&pool, &page.id).await.unwrap();
            assert_eq!(all.iter().map(|v| v.description_text.as_str()).collect::<Vec<_>>(), ["The agent's draft", "Second draft"]);

            // Locked: no edit of its words, nor a restore; unlocked, the restore is the text.
            update(&pool, &page.id, PagePatch { is_locked: Some(true), ..PagePatch::default() }, "me").await.unwrap();
            assert!(update(&pool, &page.id, PagePatch { description: Some(doc("Sneaky")), ..PagePatch::default() }, "me").await.unwrap_err().to_string().contains("locked"));
            assert!(update(&pool, &page.id, PagePatch { name: Some("Renamed".into()), ..PagePatch::default() }, "me").await.is_err());
            assert!(restore(&pool, &all[1].id, "me").await.is_err());
            assert!(update(&pool, &page.id, PagePatch { is_private: Some(true), ..PagePatch::default() }, "me").await.unwrap().is_private, "access still changes");
            update(&pool, &page.id, PagePatch { is_locked: Some(false), ..PagePatch::default() }, "me").await.unwrap();
            restore(&pool, &all[1].id, "me").await.unwrap();
            let back = get_doc(&pool, &page.id).await.unwrap();
            assert_eq!(items::words(&back.description.unwrap().0), "Second draft");
            assert_eq!(versions(&pool, &page.id).await.unwrap().len(), 3, "the restore is a version of its own");

            // A sub-page; archived with its page, unstarred; deleted only archived, with it.
            let sub = create(&pool, PageInput { parent_id: Some(page.id.clone()), name: String::new(), ..input.clone() }, "me").await.unwrap();
            assert_eq!(get(&pool, &page.id).await.unwrap().sub_pages, 1);
            // An index page — nothing but its sub-page's link — keeps it (no words is not nothing).
            let index = create(&pool, PageInput { name: "Index".into(), ..input.clone() }, "me").await.unwrap();
            let link = serde_json::json!({ "type": "doc", "content": [{ "type": "pageLink", "attrs": { "id": sub.id, "title": "" } }] }).to_string();
            update(&pool, &index.id, PagePatch { description: Some(link.clone()), ..PagePatch::default() }, "me").await.unwrap();
            assert!(get_doc(&pool, &index.id).await.unwrap().description.is_some());
            let before = get(&pool, &index.id).await.unwrap().updated_at;
            assert_eq!(update(&pool, &index.id, PagePatch { description: Some(link), ..PagePatch::default() }, "me").await.unwrap().updated_at, before, "the same document is no change");
            assert!(favorite(&pool, &page.id, true).await.unwrap().is_favorite);
            assert!(delete(&pool, &folder, &page.id).await.is_err(), "archive first");
            let archived = archive(&pool, &page.id, true).await.unwrap();
            assert!(archived.archived_at.is_some() && !archived.is_favorite);
            assert!(get(&pool, &sub.id).await.unwrap().archived_at.is_some());
            assert!(update(&pool, &sub.id, PagePatch { name: Some("x".into()), ..PagePatch::default() }, "me").await.is_err());
            archive(&pool, &page.id, false).await.unwrap();
            let copy = duplicate(&pool, &folder, &page.id, "me").await.unwrap();
            assert_eq!(copy.name, "Copy of Sandbox rules");
            assert_eq!(items::words(&get_doc(&pool, &copy.id).await.unwrap().description.unwrap().0), "Second draft");
            archive(&pool, &page.id, true).await.unwrap();
            delete(&pool, &folder, &page.id).await.unwrap();
            assert!(get(&pool, &sub.id).await.is_err(), "its sub-page went with it");
            assert_eq!(list(&pool, &p.id).await.unwrap().len(), 2, "the copy and the index stay");
            hub.close_all().await;
        });
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn markdown_reads_every_block() {
        let doc = serde_json::json!({ "type": "doc", "content": [
            { "type": "heading", "attrs": { "level": 2 }, "content": [{ "type": "text", "text": "Rules" }] },
            { "type": "paragraph", "content": [{ "type": "text", "text": "Ask " }, { "type": "mention", "attrs": { "id": "x", "label": "VIFC-1 Draft the rules" } }, { "type": "text", "text": " now", "marks": [{ "type": "bold" }] }] },
            { "type": "callout", "attrs": { "emoji": "⚠️", "color": "yellow" }, "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "Mind the dates" }] }] },
            { "type": "codeBlock", "attrs": { "language": "mermaid" }, "content": [{ "type": "text", "text": "graph TD\nA-->B" }] },
            { "type": "taskList", "content": [{ "type": "taskItem", "attrs": { "checked": true }, "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "Done" }] }] }] },
            { "type": "table", "content": [
                { "type": "tableRow", "content": [{ "type": "tableHeader", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "Who" }] }] }, { "type": "tableHeader", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "When" }] }] }] },
                { "type": "tableRow", "content": [{ "type": "tableCell", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "Minh" }] }] }, { "type": "tableCell", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "a|b" }] }] }] }
            ] },
            { "type": "horizontalRule" },
            { "type": "pageLink", "attrs": { "id": "p2", "title": "Annex" } }
        ] });
        let md = markdown("Sandbox", Some(&doc));
        let want = "# Sandbox\n\n## Rules\n\nAsk @VIFC-1 Draft the rules **now**\n\n> ⚠️ Mind the dates\n\n```mermaid\ngraph TD\nA-->B\n```\n\n- [x] Done\n\n| Who | When |\n| --- | --- |\n| Minh | a\\|b |\n\n---\n\n📄 Annex\n";
        assert_eq!(md, want);
    }
}
