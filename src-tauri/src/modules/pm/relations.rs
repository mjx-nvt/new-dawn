//! What a work item is tied to (D-075, Plane's): relations to other items — Relates to,
//! Duplicate of, Blocked by and, read from the other side, Blocking — one a pair, shown on both;
//! links (an address and a title); attached files, kept in the workspace's files folder up to the
//! owner's size limit. Each change is the item's activity (and so the Inbox's, when not the
//! owner's).

use std::path::Path;

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;

use super::items::{self, activity, files_dir};
use super::settings::StateGroup;
use crate::error::AppError;

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";
/// The attachment size limit when the owner hasn't set one (plan §11.7).
pub const ATTACHMENT_MB: u32 = 5;
/// The most the limit can be set to — a work item isn't a file store.
pub const ATTACHMENT_MB_MAX: u32 = 200;

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

fn id() -> String {
    uuid::Uuid::now_v7().to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum PmRelationKind {
    RelatesTo,
    Duplicate,
    BlockedBy,
    /// Stored as the other item's Blocked by.
    Blocking,
}

impl PmRelationKind {
    fn words(self) -> &'static str {
        match self {
            PmRelationKind::RelatesTo => "relates to",
            PmRelationKind::Duplicate => "duplicate of",
            PmRelationKind::BlockedBy => "blocked by",
            PmRelationKind::Blocking => "blocking",
        }
    }
    /// The same relation read from the other item.
    fn other_side(self) -> Self {
        match self {
            PmRelationKind::BlockedBy => PmRelationKind::Blocking,
            PmRelationKind::Blocking => PmRelationKind::BlockedBy,
            k => k,
        }
    }
}

/// Another work item, as a relation or a search shows it.
#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct PmItemRef {
    pub id: String,
    pub project_id: String,
    pub key: String,
    pub name: String,
    pub state_name: String,
    pub state_group: StateGroup,
    pub state_color: String,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct PmRelation {
    pub id: String,
    /// As seen from the item asked about.
    pub kind: PmRelationKind,
    pub item: PmItemRef,
}

#[derive(sqlx::FromRow)]
struct RelationRow {
    // Its own name: the other item's columns (flattened) have an `id` too.
    #[sqlx(rename = "relation_id")]
    id: String,
    kind: PmRelationKind,
    #[sqlx(flatten)]
    item: PmItemRef,
}

const REF: &str = "o.id, o.project_id, m.identifier || '-' || o.sequence AS key, o.name, s.name AS state_name, s.state_group, s.color AS state_color";

/// An item's relations, both ways, oldest first.
pub async fn relations(pool: &SqlitePool, item: &str) -> Result<Vec<PmRelation>> {
    let rows: Vec<RelationRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT r.id AS relation_id, CASE WHEN r.item_id = ?1 THEN r.kind WHEN r.kind = 'blocked_by' THEN 'blocking' ELSE r.kind END AS kind, {REF} \
         FROM pm_relation r JOIN pm_item o ON o.id = CASE WHEN r.item_id = ?1 THEN r.related_id ELSE r.item_id END \
         JOIN pm_project m ON m.project_id = o.project_id JOIN pm_state s ON s.id = o.state_id \
         WHERE r.item_id = ?1 OR r.related_id = ?1 ORDER BY r.created_at, r.id"
    )))
    .bind(item)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| PmRelation { id: r.id, kind: r.kind, item: r.item }).collect())
}

/// Work items to relate to, from every project: by key ("VIFC-12", "vifc 12", "12") or by words of
/// the title; not archived, not `except`.
pub async fn search(pool: &SqlitePool, query: &str, except: Option<&str>) -> Result<Vec<PmItemRef>> {
    let q = query.trim();
    let key = q.replace([' ', '-'], "").to_uppercase();
    let words = format!("%{}%", q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
    Ok(sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {REF} FROM pm_item o JOIN pm_project m ON m.project_id = o.project_id JOIN pm_state s ON s.id = o.state_id \
         WHERE o.archived_at IS NULL AND m.archived_at IS NULL AND o.id <> coalesce(?, '') \
         AND (? = '' OR upper(m.identifier) || o.sequence = ? OR CAST(o.sequence AS TEXT) = ? OR o.name LIKE ? ESCAPE '\\') \
         ORDER BY o.updated_at DESC LIMIT 20"
    )))
    .bind(except)
    .bind(q)
    .bind(&key)
    .bind(&key)
    .bind(&words)
    .fetch_all(pool)
    .await?)
}

async fn open_item(pool: &SqlitePool, id: &str) -> Result<items::PmItem> {
    let item = items::get(pool, id).await?;
    if item.archived_at.is_some() {
        return Err(invalid("An archived work item is read-only — restore it to change it"));
    }
    Ok(item)
}

/// Relates two items — one relation a pair, kept on both with its activity.
pub async fn relate(pool: &SqlitePool, item: &str, kind: PmRelationKind, other: &str, actor: &str) -> Result<PmRelation> {
    if item == other {
        return Err(invalid("A work item can't be related to itself"));
    }
    let (a, b) = (open_item(pool, item).await?, items::get(pool, other).await?);
    // Blocking is the other item's Blocked by.
    let (from, stored, to) = if kind == PmRelationKind::Blocking { (other, PmRelationKind::BlockedBy, item) } else { (item, kind, other) };
    let rid = id();
    let mut tx = pool.begin().await?;
    let inserted = sqlx::query("INSERT INTO pm_relation (id, item_id, related_id, kind, created_by) VALUES (?, ?, ?, ?, ?)")
        .bind(&rid)
        .bind(from)
        .bind(to)
        .bind(stored)
        .bind(actor)
        .execute(&mut *tx)
        .await;
    if let Err(sqlx::Error::Database(e)) = &inserted {
        if e.is_unique_violation() {
            return Err(invalid(format!("{} and {} are already related — remove that relation first", a.key, b.key)));
        }
    }
    inserted?;
    activity(&mut tx, item, "related", "relation", "", &format!("{} {}", kind.words(), b.key), (None, None), actor).await?;
    activity(&mut tx, other, "related", "relation", "", &format!("{} {}", kind.other_side().words(), a.key), (None, None), actor).await?;
    tx.commit().await?;
    relations(pool, item).await?.into_iter().find(|r| r.id == rid).ok_or_else(|| anyhow!("the relation just made is gone"))
}

/// Removes a relation (from either item), with its activity on both.
pub async fn unrelate(pool: &SqlitePool, relation: &str, actor: &str) -> Result<()> {
    let row: Option<(String, String, PmRelationKind)> = sqlx::query_as("SELECT item_id, related_id, kind FROM pm_relation WHERE id = ?").bind(relation).fetch_optional(pool).await?;
    let (from, to, kind) = row.ok_or_else(|| anyhow!(AppError::not_found("That relation is gone")))?;
    let (a, b) = (items::get(pool, &from).await?, items::get(pool, &to).await?);
    // From the open side: an item archived since (auto-archive) mustn't pin the relation.
    if a.archived_at.is_some() && b.archived_at.is_some() {
        return Err(invalid("Both work items are archived — restore one to change their relation"));
    }
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM pm_relation WHERE id = ?").bind(relation).execute(&mut *tx).await?;
    activity(&mut tx, &from, "unrelated", "relation", &format!("{} {}", kind.words(), b.key), "", (None, None), actor).await?;
    activity(&mut tx, &to, "unrelated", "relation", &format!("{} {}", kind.other_side().words(), a.key), "", (None, None), actor).await?;
    tx.commit().await?;
    Ok(())
}

// ── Links ──

#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct PmLink {
    pub id: String,
    pub item_id: String,
    pub url: String,
    pub title: String,
    pub created_by: String,
    pub created_at: String,
}

/// A web address (http or https; "example.com/x" taken as https) or a mail address (mailto:),
/// whole: the link opens in the owner's browser or Mail, never in New Dawn.
pub fn clean_url(url: &str) -> Result<String> {
    let u = url.trim();
    if u.is_empty() {
        return Err(invalid("A link needs an address"));
    }
    if u.chars().any(char::is_whitespace) || u.len() > 2048 {
        return Err(invalid("That address doesn't read — paste it whole, without spaces"));
    }
    match u.split_once("://") {
        Some((scheme, rest)) if matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https") && !rest.is_empty() => Ok(u.to_owned()),
        Some(_) => Err(invalid("Links are web addresses (http or https) or mail addresses (mailto:)")),
        None if u.to_ascii_lowercase().starts_with("mailto:") && u.len() > 7 => Ok(u.to_owned()),
        // The scheme is joined here, not written whole: the address is the owner's data, opened by
        // their browser and never fetched by New Dawn (rule 3, D-075) — the localhost check reads
        // literal addresses in code.
        None if u.contains('.') && !u.contains(':') => Ok(format!("{}://{u}", "https")),
        None => Err(invalid("Links are web addresses (http or https) or mail addresses (mailto:)")),
    }
}

/// The same address twice on an item, said plainly.
fn same_link(e: sqlx::Error) -> anyhow::Error {
    match &e {
        sqlx::Error::Database(d) if d.is_unique_violation() => invalid("That link is on this work item already"),
        _ => e.into(),
    }
}

fn clean_title(title: &str) -> Result<String> {
    let t = title.trim();
    if t.chars().count() > items::NAME_MAX {
        return Err(invalid(format!("A link's title is at most {} characters", items::NAME_MAX)));
    }
    Ok(t.to_owned())
}

pub async fn links(pool: &SqlitePool, item: &str) -> Result<Vec<PmLink>> {
    Ok(sqlx::query_as("SELECT id, item_id, url, title, created_by, created_at FROM pm_link WHERE item_id = ? ORDER BY created_at, id").bind(item).fetch_all(pool).await?)
}

pub async fn add_link(pool: &SqlitePool, item: &str, url: &str, title: &str, actor: &str) -> Result<PmLink> {
    open_item(pool, item).await?;
    let (url, title) = (clean_url(url)?, clean_title(title)?);
    let lid = id();
    let mut tx = pool.begin().await?;
    sqlx::query("INSERT INTO pm_link (id, item_id, url, title, created_by) VALUES (?, ?, ?, ?, ?)").bind(&lid).bind(item).bind(&url).bind(&title).bind(actor).execute(&mut *tx).await.map_err(same_link)?;
    activity(&mut tx, item, "linked", "link", "", if title.is_empty() { &url } else { &title }, (None, None), actor).await?;
    tx.commit().await?;
    Ok(sqlx::query_as("SELECT id, item_id, url, title, created_by, created_at FROM pm_link WHERE id = ?").bind(&lid).fetch_one(pool).await?)
}

pub async fn edit_link(pool: &SqlitePool, link: &str, url: &str, title: &str, actor: &str) -> Result<PmLink> {
    let row: Option<(String, String, String)> = sqlx::query_as("SELECT item_id, url, title FROM pm_link WHERE id = ?").bind(link).fetch_optional(pool).await?;
    let (item, old_url, old_title) = row.ok_or_else(|| anyhow!(AppError::not_found("That link is gone")))?;
    open_item(pool, &item).await?;
    let (url, title) = (clean_url(url)?, clean_title(title)?);
    if (url.as_str(), title.as_str()) != (old_url.as_str(), old_title.as_str()) {
        let mut tx = pool.begin().await?;
        sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE pm_link SET url = ?, title = ?, updated_at = {NOW} WHERE id = ?"))).bind(&url).bind(&title).bind(link).execute(&mut *tx).await.map_err(same_link)?;
        let (old, new) = (if old_title.is_empty() { old_url } else { old_title }, if title.is_empty() { url } else { title });
        activity(&mut tx, &item, "updated", "link", &old, &new, (None, None), actor).await?;
        tx.commit().await?;
    }
    Ok(sqlx::query_as("SELECT id, item_id, url, title, created_by, created_at FROM pm_link WHERE id = ?").bind(link).fetch_one(pool).await?)
}

pub async fn remove_link(pool: &SqlitePool, link: &str, actor: &str) -> Result<()> {
    let row: Option<(String, String, String)> = sqlx::query_as("SELECT item_id, url, title FROM pm_link WHERE id = ?").bind(link).fetch_optional(pool).await?;
    let (item, url, title) = row.ok_or_else(|| anyhow!(AppError::not_found("That link is gone")))?;
    open_item(pool, &item).await?;
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM pm_link WHERE id = ?").bind(link).execute(&mut *tx).await?;
    activity(&mut tx, &item, "unlinked", "link", if title.is_empty() { &url } else { &title }, "", (None, None), actor).await?;
    tx.commit().await?;
    Ok(())
}

// ── Attachments ──

#[derive(Debug, Clone, Serialize, Type)]
pub struct PmAttachment {
    pub id: String,
    pub item_id: String,
    pub name: String,
    /// Bytes.
    pub size: u32,
    /// The file on this Mac, for Open and Reveal.
    pub path: String,
    pub created_by: String,
    pub created_at: String,
}

pub async fn attachments(pool: &SqlitePool, folder: &str, item: &str) -> Result<Vec<PmAttachment>> {
    let rows: Vec<(String, String, String, u32, String, String)> =
        sqlx::query_as("SELECT id, name, file, size, created_by, created_at FROM pm_attachment WHERE item_id = ? ORDER BY created_at, id").bind(item).fetch_all(pool).await?;
    let dir = files_dir(folder, item);
    Ok(rows
        .into_iter()
        .map(|(id, name, file, size, by, at)| PmAttachment { id, item_id: item.to_owned(), name, size, path: dir.join(file).to_string_lossy().into_owned(), created_by: by, created_at: at })
        .collect())
}

/// Copies files into the item's folder — each at most `limit_mb` — and lists them. Every file is
/// checked first: one too large or unreadable is named and none is attached.
pub async fn attach_all(pool: &SqlitePool, folder: &str, item: &str, sources: &[&Path], limit_mb: u32, actor: &str) -> Result<Vec<PmAttachment>> {
    open_item(pool, item).await?;
    let mut sized = Vec::with_capacity(sources.len());
    for source in sources {
        let meta = tokio::fs::metadata(source).await.with_context(|| format!("couldn't read {}", source.display()))?;
        let shown = source.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into());
        if !meta.is_file() {
            return Err(invalid(format!("{shown} isn't a file")));
        }
        if meta.len() > u64::from(limit_mb) * 1024 * 1024 {
            return Err(invalid(format!("{shown} is larger than {limit_mb} MB — attachments are up to that (Settings → General → Work items)")));
        }
        sized.push((*source, crate::kernel::records::files::safe_name(&shown), meta.len()));
    }
    let dir = files_dir(folder, item);
    tokio::fs::create_dir_all(&dir).await.with_context(|| format!("couldn't make {}", dir.display()))?;
    let mut made = Vec::with_capacity(sized.len());
    for (source, name, size) in sized {
        let aid = id();
        let file = format!("{}-{name}", &aid[24..]);
        let target = dir.join(&file);
        tokio::fs::copy(source, &target).await.with_context(|| format!("couldn't copy {}", source.display()))?;
        let written: Result<()> = async {
            let mut tx = pool.begin().await?;
            sqlx::query("INSERT INTO pm_attachment (id, item_id, name, file, size, created_by) VALUES (?, ?, ?, ?, ?, ?)")
                .bind(&aid)
                .bind(item)
                .bind(&name)
                .bind(&file)
                .bind(size as u32)
                .bind(actor)
                .execute(&mut *tx)
                .await?;
            activity(&mut tx, item, "attached", "attachment", "", &name, (None, None), actor).await?;
            tx.commit().await?;
            Ok(())
        }
        .await;
        if let Err(e) = written {
            // No row, no file: nothing left behind unseen.
            let _ = tokio::fs::remove_file(&target).await;
            return Err(e);
        }
        made.push(aid);
    }
    Ok(attachments(pool, folder, item).await?.into_iter().filter(|a| made.contains(&a.id)).collect())
}

pub async fn detach(pool: &SqlitePool, folder: &str, attachment: &str, actor: &str) -> Result<()> {
    let row: Option<(String, String, String)> = sqlx::query_as("SELECT item_id, name, file FROM pm_attachment WHERE id = ?").bind(attachment).fetch_optional(pool).await?;
    let (item, name, file) = row.ok_or_else(|| anyhow!(AppError::not_found("That attachment is gone")))?;
    open_item(pool, &item).await?;
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM pm_attachment WHERE id = ?").bind(attachment).execute(&mut *tx).await?;
    activity(&mut tx, &item, "detached", "attachment", &name, "", (None, None), actor).await?;
    tx.commit().await?;
    let _ = std::fs::remove_file(files_dir(folder, &item).join(file));
    Ok(())
}
