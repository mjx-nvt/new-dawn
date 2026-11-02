//! A work item's comments (D-074): the editor's documents, one level of replies, edited ("edited"
//! shown), deleted (with their replies); the people a comment mentions kept for the Inbox; and
//! reactions — one of each emoji a person — on the description or a comment.

use anyhow::{Result, anyhow};
use serde::Serialize;
use serde_json::Value;
use specta::Type;
use sqlx::SqlitePool;

use super::items::{self, PmItem};
use crate::error::AppError;
use crate::kernel::records::types::Json;

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";
/// The reactions offered (Plane's set).
pub const EMOJI: [&str; 8] = ["👍", "👎", "😄", "🎉", "😕", "❤️", "🚀", "👀"];

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct Reaction {
    pub emoji: String,
    /// "me" or people's ids.
    pub by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct PmComment {
    pub id: String,
    pub parent_id: Option<String>,
    pub body: Json,
    pub body_text: String,
    pub created_by: String,
    pub edited_at: Option<String>,
    pub created_at: String,
    pub reactions: Vec<Reaction>,
}

/// Reactions on the description (no comment) or on each comment.
async fn reactions(pool: &SqlitePool, item: &str) -> Result<Vec<(Option<String>, String, String)>> {
    Ok(sqlx::query_as("SELECT comment_id, emoji, created_by FROM pm_reaction WHERE item_id = ? ORDER BY created_at").bind(item).fetch_all(pool).await?)
}

fn grouped(all: &[(Option<String>, String, String)], comment: Option<&str>) -> Vec<Reaction> {
    let mut out: Vec<Reaction> = Vec::new();
    for (c, emoji, by) in all.iter().filter(|(c, _, _)| c.as_deref() == comment) {
        let _ = c;
        match out.iter_mut().find(|r| &r.emoji == emoji) {
            Some(r) => r.by.push(by.clone()),
            None => out.push(Reaction { emoji: emoji.clone(), by: vec![by.clone()] }),
        }
    }
    out
}

/// The reactions on an item's description.
pub async fn item_reactions(pool: &SqlitePool, item: &str) -> Result<Vec<Reaction>> {
    Ok(grouped(&reactions(pool, item).await?, None))
}

#[derive(sqlx::FromRow)]
struct CommentRow {
    id: String,
    parent_id: Option<String>,
    body_json: String,
    body_text: String,
    created_by: String,
    edited_at: Option<String>,
    created_at: String,
}

/// An item's comments, oldest first (a reply after its comment's).
pub async fn list(pool: &SqlitePool, item: &str) -> Result<Vec<PmComment>> {
    let rows: Vec<CommentRow> = sqlx::query_as("SELECT id, parent_id, body_json, body_text, created_by, edited_at, created_at FROM pm_comment WHERE item_id = ? ORDER BY created_at, id").bind(item).fetch_all(pool).await?;
    let all = reactions(pool, item).await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let reactions = grouped(&all, Some(&r.id));
            PmComment { id: r.id, parent_id: r.parent_id, body: Json(serde_json::from_str(&r.body_json).unwrap_or(Value::Null)), body_text: r.body_text, created_by: r.created_by, edited_at: r.edited_at, created_at: r.created_at, reactions }
        })
        .collect())
}

/// The people a comment mentions: the editor's mentions of the CRM's people.
fn mentioned(doc: &Value) -> Vec<String> {
    fn walk(v: &Value, out: &mut Vec<String>) {
        if v.get("type").and_then(Value::as_str) == Some("mention") {
            let attrs = v.get("attrs");
            let object = attrs.and_then(|a| a.get("object")).and_then(Value::as_str).unwrap_or("");
            if let (Some(id), "people") = (attrs.and_then(|a| a.get("id")).and_then(Value::as_str), object) {
                if !out.iter().any(|o| o == id) {
                    out.push(id.to_owned());
                }
            }
        }
        for c in v.get("content").and_then(Value::as_array).into_iter().flatten() {
            walk(c, out);
        }
    }
    let mut out = Vec::new();
    walk(doc, &mut out);
    out
}

fn body(json: &str) -> Result<(Value, String)> {
    let doc: Value = serde_json::from_str(json).map_err(|_| invalid("The comment doesn't read"))?;
    let text = items::words(&doc);
    if text.trim().is_empty() && !json.contains("\"image\"") {
        return Err(invalid("A comment needs some words"));
    }
    Ok((doc, text))
}

async fn keep_mentions(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, comment: &str, doc: &Value) -> Result<()> {
    sqlx::query("DELETE FROM pm_mention WHERE comment_id = ?").bind(comment).execute(&mut **tx).await?;
    for who in mentioned(doc) {
        sqlx::query("INSERT INTO pm_mention (id, comment_id, who) VALUES (?, ?, ?)").bind(uuid::Uuid::now_v7().to_string()).bind(comment).bind(who).execute(&mut **tx).await?;
    }
    Ok(())
}

/// An item that takes comments and reactions: not archived (read-only).
async fn open_item(pool: &SqlitePool, item: &str) -> Result<PmItem> {
    let it = items::get(pool, item).await?;
    if it.archived_at.is_some() {
        return Err(invalid("An archived work item is read-only — restore it to change it"));
    }
    Ok(it)
}

/// Adds a comment, or a reply to one (one level: a reply to a reply answers its comment).
pub async fn add(pool: &SqlitePool, item: &str, json: &str, parent: Option<&str>, actor: &str) -> Result<Vec<PmComment>> {
    open_item(pool, item).await?;
    let (doc, text) = body(json)?;
    let parent = match parent.filter(|p| !p.is_empty()) {
        Some(p) => {
            let row: Option<(String, Option<String>)> = sqlx::query_as("SELECT item_id, parent_id FROM pm_comment WHERE id = ?").bind(p).fetch_optional(pool).await?;
            match row {
                Some((i, up)) if i == item => Some(up.unwrap_or_else(|| p.to_owned())),
                _ => return Err(invalid("That comment isn't on this work item")),
            }
        }
        None => None,
    };
    let id = uuid::Uuid::now_v7().to_string();
    let mut tx = pool.begin().await?;
    sqlx::query("INSERT INTO pm_comment (id, item_id, parent_id, body_json, body_text, created_by) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(item)
        .bind(&parent)
        .bind(doc.to_string())
        .bind(&text)
        .bind(actor)
        .execute(&mut *tx)
        .await?;
    keep_mentions(&mut tx, &id, &doc).await?;
    sqlx::query("INSERT INTO pm_activity (id, item_id, kind, field, new_value, actor) VALUES (?, ?, 'commented', '', ?, ?)").bind(uuid::Uuid::now_v7().to_string()).bind(item).bind(&id).bind(actor).execute(&mut *tx).await?;
    // The owner follows what they comment on (an agent doesn't subscribe).
    if actor == "me" {
        sqlx::query("INSERT INTO pm_item_subscriber (item_id, who) VALUES (?, ?) ON CONFLICT DO NOTHING").bind(item).bind(actor).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    list(pool, item).await
}

pub async fn edit(pool: &SqlitePool, id: &str, json: &str) -> Result<Vec<PmComment>> {
    let row: Option<(String,)> = sqlx::query_as("SELECT item_id FROM pm_comment WHERE id = ?").bind(id).fetch_optional(pool).await?;
    let (item,) = row.ok_or_else(|| anyhow!(AppError::not_found("That comment is gone")))?;
    open_item(pool, &item).await?;
    let (doc, text) = body(json)?;
    let mut tx = pool.begin().await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE pm_comment SET body_json = ?, body_text = ?, edited_at = {NOW}, updated_at = {NOW} WHERE id = ?"))).bind(doc.to_string()).bind(&text).bind(id).execute(&mut *tx).await?;
    keep_mentions(&mut tx, id, &doc).await?;
    tx.commit().await?;
    list(pool, &item).await
}

/// Deletes a comment with its replies.
pub async fn delete(pool: &SqlitePool, id: &str) -> Result<Vec<PmComment>> {
    let row: Option<(String,)> = sqlx::query_as("SELECT item_id FROM pm_comment WHERE id = ?").bind(id).fetch_optional(pool).await?;
    let (item,) = row.ok_or_else(|| anyhow!(AppError::not_found("That comment is gone")))?;
    open_item(pool, &item).await?;
    sqlx::query("DELETE FROM pm_comment WHERE id = ?").bind(id).execute(pool).await?;
    list(pool, &item).await
}

/// A reaction on, or off: on the description (no comment) or a comment of the item.
pub async fn toggle_reaction(pool: &SqlitePool, item: &str, comment: Option<&str>, emoji: &str, who: &str) -> Result<()> {
    if !EMOJI.contains(&emoji) {
        return Err(invalid("That reaction isn't one of the set"));
    }
    open_item(pool, item).await?;
    let comment = comment.filter(|c| !c.is_empty());
    if let Some(c) = comment {
        let on: Option<(String,)> = sqlx::query_as("SELECT id FROM pm_comment WHERE id = ? AND item_id = ?").bind(c).bind(item).fetch_optional(pool).await?;
        if on.is_none() {
            return Err(invalid("That comment isn't on this work item"));
        }
    }
    let gone = sqlx::query("DELETE FROM pm_reaction WHERE item_id = ? AND comment_id IS ? AND emoji = ? AND created_by = ?").bind(item).bind(comment).bind(emoji).bind(who).execute(pool).await?;
    if gone.rows_affected() == 0 {
        sqlx::query("INSERT INTO pm_reaction (id, item_id, comment_id, emoji, created_by) VALUES (?, ?, ?, ?, ?)")
            .bind(uuid::Uuid::now_v7().to_string())
            .bind(item)
            .bind(comment)
            .bind(emoji)
            .bind(who)
            .execute(pool)
            .await?;
    }
    Ok(())
}
