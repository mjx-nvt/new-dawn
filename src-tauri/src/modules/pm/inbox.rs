//! The Inbox (D-075, Plane's): what someone else — an agent, an automation — did on a work item
//! the owner follows, the owner's mentions, due-date reminders; read or unread, snoozed, archived.
//! In the app only: nothing is mailed or pushed. Which kinds show is the owner's choice (Settings
//! → General → Inbox), applied when the Inbox is read.

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::error::AppError;

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum NoticeKind {
    /// A property changed (and an item made, copied, archived, restored, related, linked…).
    Property,
    State,
    /// Moved into a completed state.
    Completed,
    Comment,
    Mention,
    Reminder,
}

/// Which notices a list shows — Plane's display options, one at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum InboxShow {
    /// Not archived, not snoozed (or snoozed until a time now past).
    Inbox,
    Unread,
    /// Snoozed, its time past or not.
    Snoozed,
    Archived,
}

#[derive(Debug, Clone, Deserialize, Type)]
pub struct InboxQuery {
    /// The Mentions tab; otherwise All (everything but mentions).
    pub mentions: bool,
    pub show: InboxShow,
    /// Plane's type filters — items assigned to the owner, made by them, followed by them; any
    /// of those chosen (none: every one).
    pub assigned: bool,
    pub created: bool,
    pub subscribed: bool,
}

#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct Notice {
    pub id: String,
    pub kind: NoticeKind,
    pub item_id: String,
    pub project_id: String,
    /// "VIFC-12".
    pub item_key: String,
    pub item_name: String,
    /// Who did it ("" for a reminder).
    pub actor: String,
    /// The activity it tells of, as the item's activity has it ("" for a reminder).
    pub activity_kind: String,
    pub field: String,
    pub old_value: String,
    pub new_value: String,
    pub due_on: Option<String>,
    pub read_at: Option<String>,
    pub snoozed_until: Option<String>,
    pub archived_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct InboxCounts {
    /// Unread in All.
    pub all: u32,
    /// Unread in Mentions.
    pub mentions: u32,
}

/// The owner's notice of an activity row someone else wrote, when the owner follows the item.
pub(crate) async fn notify(tx: &mut Transaction<'_, Sqlite>, item: &str, activity: &str, kind: &str, field: &str, new_state: Option<&str>, actor: &str) -> Result<()> {
    let follows: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM pm_item_subscriber WHERE item_id = ? AND who = 'me'").bind(item).fetch_optional(&mut **tx).await?;
    if follows.is_none() {
        return Ok(());
    }
    let notice = match (kind, field) {
        ("commented", _) => NoticeKind::Comment,
        (_, "state") => {
            let group: Option<(String,)> = sqlx::query_as("SELECT state_group FROM pm_state WHERE id = ?").bind(new_state).fetch_optional(&mut **tx).await?;
            if group.is_some_and(|(g,)| g == "completed") { NoticeKind::Completed } else { NoticeKind::State }
        }
        _ => NoticeKind::Property,
    };
    sqlx::query("INSERT INTO notification (id, kind, item_id, activity_id, actor) VALUES (?, ?, ?, ?, ?)")
        .bind(uuid::Uuid::now_v7().to_string())
        .bind(notice)
        .bind(item)
        .bind(activity)
        .bind(actor)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// A comment by someone else mentions the owner: its notice is a mention (made when the owner
/// doesn't follow the item, turned into one when they do — never both).
pub(crate) async fn mention(tx: &mut Transaction<'_, Sqlite>, item: &str, activity: &str, actor: &str) -> Result<()> {
    let turned = sqlx::query("UPDATE notification SET kind = 'mention' WHERE activity_id = ?").bind(activity).execute(&mut **tx).await?;
    if turned.rows_affected() == 0 {
        sqlx::query("INSERT INTO notification (id, kind, item_id, activity_id, actor) VALUES (?, 'mention', ?, ?, ?)")
            .bind(uuid::Uuid::now_v7().to_string())
            .bind(item)
            .bind(activity)
            .bind(actor)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

fn muted_json(muted: &[NoticeKind]) -> Result<String> {
    Ok(serde_json::to_string(muted)?)
}

/// The notices a tab, a display option and the type filters show, newest first (200 at most);
/// the kinds the owner turned off left out.
pub async fn list(pool: &SqlitePool, query: &InboxQuery, muted: &[NoticeKind]) -> Result<Vec<Notice>> {
    let show = match query.show {
        InboxShow::Inbox => format!("n.archived_at IS NULL AND (n.snoozed_until IS NULL OR n.snoozed_until <= {NOW})"),
        InboxShow::Unread => format!("n.archived_at IS NULL AND (n.snoozed_until IS NULL OR n.snoozed_until <= {NOW}) AND n.read_at IS NULL"),
        InboxShow::Snoozed => "n.archived_at IS NULL AND n.snoozed_until IS NOT NULL".to_owned(),
        InboxShow::Archived => "n.archived_at IS NOT NULL".to_owned(),
    };
    let mut types = Vec::new();
    if query.assigned {
        types.push("EXISTS (SELECT 1 FROM pm_item_assignee a WHERE a.item_id = i.id AND a.who = 'me')");
    }
    if query.created {
        types.push("i.created_by = 'me'");
    }
    if query.subscribed {
        types.push("EXISTS (SELECT 1 FROM pm_item_subscriber b WHERE b.item_id = i.id AND b.who = 'me')");
    }
    let types = if types.is_empty() { String::new() } else { format!(" AND ({})", types.join(" OR ")) };
    Ok(sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT n.id, n.kind, n.item_id, i.project_id, m.identifier || '-' || i.sequence AS item_key, i.name AS item_name, \
         coalesce(a.actor, n.actor) AS actor, coalesce(a.kind, '') AS activity_kind, coalesce(a.field, '') AS field, \
         coalesce(a.old_value, '') AS old_value, coalesce(a.new_value, '') AS new_value, \
         n.due_on, n.read_at, n.snoozed_until, n.archived_at, n.created_at \
         FROM notification n JOIN pm_item i ON i.id = n.item_id JOIN pm_project m ON m.project_id = i.project_id \
         LEFT JOIN pm_activity a ON a.id = n.activity_id \
         WHERE {show} AND (n.kind = 'mention') = ? AND n.kind NOT IN (SELECT value FROM json_each(?)){types} \
         ORDER BY n.created_at DESC, n.id DESC LIMIT 200"
    )))
    .bind(query.mentions)
    .bind(muted_json(muted)?)
    .fetch_all(pool)
    .await?)
}

/// Unread notices in each tab (not archived, not snoozed, not muted).
pub async fn counts(pool: &SqlitePool, muted: &[NoticeKind]) -> Result<InboxCounts> {
    let (all, mentions): (u32, u32) = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT coalesce(sum(kind <> 'mention'), 0), coalesce(sum(kind = 'mention'), 0) FROM notification \
         WHERE read_at IS NULL AND archived_at IS NULL AND (snoozed_until IS NULL OR snoozed_until <= {NOW}) \
         AND kind NOT IN (SELECT value FROM json_each(?))"
    )))
    .bind(muted_json(muted)?)
    .fetch_one(pool)
    .await?;
    Ok(InboxCounts { all, mentions })
}

async fn found(pool: &SqlitePool, id: &str) -> Result<()> {
    let row: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM notification WHERE id = ?").bind(id).fetch_optional(pool).await?;
    row.map(|_| ()).ok_or_else(|| anyhow!(AppError::not_found("That notice is gone")))
}

/// Read or unread.
pub async fn mark(pool: &SqlitePool, id: &str, read: bool) -> Result<()> {
    found(pool, id).await?;
    let sql = if read { format!("UPDATE notification SET read_at = coalesce(read_at, {NOW}), updated_at = {NOW} WHERE id = ?") } else { format!("UPDATE notification SET read_at = NULL, updated_at = {NOW} WHERE id = ?") };
    sqlx::query(sqlx::AssertSqlSafe(sql)).bind(id).execute(pool).await?;
    Ok(())
}

/// Every unread notice the tab shows, read — not the snoozed ones (they come back unread) nor the
/// kinds turned off.
pub async fn mark_all(pool: &SqlitePool, mentions: bool, muted: &[NoticeKind]) -> Result<()> {
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE notification SET read_at = {NOW}, updated_at = {NOW} WHERE read_at IS NULL AND archived_at IS NULL \
         AND (snoozed_until IS NULL OR snoozed_until <= {NOW}) AND (kind = 'mention') = ? AND kind NOT IN (SELECT value FROM json_each(?))"
    )))
    .bind(mentions)
    .bind(muted_json(muted)?)
    .execute(pool)
    .await?;
    Ok(())
}

/// Hidden until a time (ISO-8601, UTC, in the future), or back now (none).
pub async fn snooze(pool: &SqlitePool, id: &str, until: Option<&str>) -> Result<()> {
    found(pool, id).await?;
    if let Some(u) = until {
        let ok: Option<(i64,)> = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT 1 WHERE julianday(?1) IS NOT NULL AND julianday(?1) > julianday({NOW})"))).bind(u).fetch_optional(pool).await?;
        if ok.is_none() {
            return Err(anyhow!(AppError::invalid("Snooze until a time to come")));
        }
    }
    // Kept in the app's own form (UTC, milliseconds), so it compares as text with "now".
    sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE notification SET snoozed_until = strftime('%Y-%m-%dT%H:%M:%fZ', ?), updated_at = {NOW} WHERE id = ?"))).bind(until).bind(id).execute(pool).await?;
    Ok(())
}

/// Archived (out of the Inbox, under InboxShow archived) or back.
pub async fn archive(pool: &SqlitePool, id: &str, archived: bool) -> Result<()> {
    found(pool, id).await?;
    let sql = if archived { format!("UPDATE notification SET archived_at = {NOW}, updated_at = {NOW} WHERE id = ?") } else { format!("UPDATE notification SET archived_at = NULL, updated_at = {NOW} WHERE id = ?") };
    sqlx::query(sqlx::AssertSqlSafe(sql)).bind(id).execute(pool).await?;
    Ok(())
}
