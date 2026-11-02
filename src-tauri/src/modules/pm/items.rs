//! Plane's work items (D-074, plan v4 §4.3): numbered in their project ("VIFC-12" composed when
//! read), each with its state, priority, assignees, labels, dates, estimate and parent; every
//! change kept as activity (Activity, History, Transition); the description's versions; archive
//! (completed or cancelled only), restore, delete with sub-items, copy; files of their own.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::Type;
use sqlx::{Sqlite, SqlitePool, Transaction};

use super::settings::StateGroup;
use crate::error::AppError;
use crate::kernel::records::types::Json;

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";
pub const NAME_MAX: usize = 255;
/// A description saved again within this many minutes changes its newest version.
const VERSION_MINUTES: i64 = 10;

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

fn id() -> String {
    uuid::Uuid::now_v7().to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum Priority {
    Urgent,
    High,
    Medium,
    Low,
    None,
}

impl Priority {
    pub fn label(self) -> &'static str {
        match self {
            Priority::Urgent => "Urgent",
            Priority::High => "High",
            Priority::Medium => "Medium",
            Priority::Low => "Low",
            Priority::None => "None",
        }
    }
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct PmItem {
    pub id: String,
    pub project_id: String,
    pub sequence: u32,
    /// "VIFC-12".
    pub key: String,
    pub name: String,
    /// The editor's document, as stored.
    pub description: Option<Json>,
    pub description_text: String,
    pub state_id: String,
    pub state_name: String,
    pub state_group: StateGroup,
    pub state_color: String,
    pub priority: Priority,
    pub start_date: Option<String>,
    pub target_date: Option<String>,
    pub estimate_point_id: Option<String>,
    pub estimate_value: Option<String>,
    pub parent_id: Option<String>,
    pub parent_key: Option<String>,
    pub parent_name: Option<String>,
    /// "me" or people's ids.
    pub assignees: Vec<String>,
    pub labels: Vec<String>,
    pub subscribers: Vec<String>,
    pub sub_items: u32,
    pub sub_done: u32,
    pub sub_cancelled: u32,
    pub comments: u32,
    pub sort_order: f64,
    pub completed_at: Option<String>,
    pub archived_at: Option<String>,
    pub created_by: String,
    pub updated_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(sqlx::FromRow)]
struct Row {
    id: String,
    project_id: String,
    sequence: u32,
    key: String,
    name: String,
    description_json: Option<String>,
    description_text: String,
    state_id: String,
    state_name: String,
    state_group: StateGroup,
    state_color: String,
    priority: Priority,
    start_date: Option<String>,
    target_date: Option<String>,
    estimate_point_id: Option<String>,
    estimate_value: Option<String>,
    parent_id: Option<String>,
    parent_key: Option<String>,
    parent_name: Option<String>,
    assignees: Option<String>,
    labels: Option<String>,
    subscribers: Option<String>,
    sub_items: u32,
    sub_done: u32,
    sub_cancelled: u32,
    comments: u32,
    sort_order: f64,
    completed_at: Option<String>,
    archived_at: Option<String>,
    created_by: String,
    updated_by: String,
    created_at: String,
    updated_at: String,
}

/// Assignees and labels: the same ones in any order.
fn same_set(a: &[String], b: &[String]) -> bool {
    a.len() == b.len() && a.iter().all(|x| b.contains(x))
}

fn split(list: Option<String>) -> Vec<String> {
    list.map(|l| l.split('\u{1f}').filter(|s| !s.is_empty()).map(str::to_owned).collect()).unwrap_or_default()
}

impl From<Row> for PmItem {
    fn from(r: Row) -> Self {
        PmItem {
            id: r.id,
            project_id: r.project_id,
            sequence: r.sequence,
            key: r.key,
            name: r.name,
            description: r.description_json.and_then(|d| serde_json::from_str(&d).ok()).map(Json),
            description_text: r.description_text,
            state_id: r.state_id,
            state_name: r.state_name,
            state_group: r.state_group,
            state_color: r.state_color,
            priority: r.priority,
            start_date: r.start_date,
            target_date: r.target_date,
            estimate_point_id: r.estimate_point_id,
            estimate_value: r.estimate_value,
            parent_id: r.parent_id,
            parent_key: r.parent_key,
            parent_name: r.parent_name,
            assignees: split(r.assignees),
            labels: split(r.labels),
            subscribers: split(r.subscribers),
            sub_items: r.sub_items,
            sub_done: r.sub_done,
            sub_cancelled: r.sub_cancelled,
            comments: r.comments,
            sort_order: r.sort_order,
            completed_at: r.completed_at,
            archived_at: r.archived_at,
            created_by: r.created_by,
            updated_by: r.updated_by,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

macro_rules! select {
    ($rest:literal) => {
        concat!(
            "SELECT i.id, i.project_id, i.sequence, m.identifier || '-' || i.sequence AS key, i.name, i.description_json, i.description_text, \
             i.state_id, s.name AS state_name, s.state_group, s.color AS state_color, i.priority, i.start_date, i.target_date, \
             i.estimate_point_id, ep.value AS estimate_value, i.parent_id, pm.identifier || '-' || p.sequence AS parent_key, p.name AS parent_name, \
             (SELECT group_concat(who, char(31)) FROM pm_item_assignee a WHERE a.item_id = i.id) AS assignees, \
             (SELECT group_concat(label_id, char(31)) FROM pm_item_label l WHERE l.item_id = i.id) AS labels, \
             (SELECT group_concat(who, char(31)) FROM pm_item_subscriber b WHERE b.item_id = i.id) AS subscribers, \
             (SELECT count(*) FROM pm_item c WHERE c.parent_id = i.id) AS sub_items, \
             (SELECT count(*) FROM pm_item c JOIN pm_state cs ON cs.id = c.state_id WHERE c.parent_id = i.id AND cs.state_group = 'completed') AS sub_done, \
             (SELECT count(*) FROM pm_item c JOIN pm_state cs ON cs.id = c.state_id WHERE c.parent_id = i.id AND cs.state_group = 'cancelled') AS sub_cancelled, \
             (SELECT count(*) FROM pm_comment k WHERE k.item_id = i.id) AS comments, \
             i.sort_order, i.completed_at, i.archived_at, i.created_by, i.updated_by, i.created_at, i.updated_at \
             FROM pm_item i JOIN pm_project m ON m.project_id = i.project_id JOIN pm_state s ON s.id = i.state_id \
             LEFT JOIN pm_estimate_point ep ON ep.id = i.estimate_point_id \
             LEFT JOIN pm_item p ON p.id = i.parent_id LEFT JOIN pm_project pm ON pm.project_id = p.project_id ",
            $rest
        )
    };
}

/// A project's work items — the archived ones, or the others — in their order.
pub async fn list(pool: &SqlitePool, project_id: &str, archived: bool) -> Result<Vec<PmItem>> {
    let rows: Vec<Row> = if archived {
        sqlx::query_as(select!("WHERE i.project_id = ? AND i.archived_at IS NOT NULL ORDER BY i.archived_at DESC")).bind(project_id).fetch_all(pool).await?
    } else {
        sqlx::query_as(select!("WHERE i.project_id = ? AND i.archived_at IS NULL ORDER BY i.sort_order, i.sequence")).bind(project_id).fetch_all(pool).await?
    };
    Ok(rows.into_iter().map(PmItem::from).collect())
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<PmItem> {
    let row: Option<Row> = sqlx::query_as(select!("WHERE i.id = ?")).bind(id).fetch_optional(pool).await?;
    row.map(PmItem::from).ok_or_else(|| anyhow!(AppError::not_found("That work item is gone")))
}

/// An item's sub-items, in their order.
pub async fn children(pool: &SqlitePool, id: &str) -> Result<Vec<PmItem>> {
    let rows: Vec<Row> = sqlx::query_as(select!("WHERE i.parent_id = ? ORDER BY i.sort_order, i.sequence")).bind(id).fetch_all(pool).await?;
    Ok(rows.into_iter().map(PmItem::from).collect())
}

/// What the create form and a draft hold.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct ItemInput {
    pub project_id: String,
    pub name: String,
    /// The editor's document as JSON text.
    pub description: Option<String>,
    pub state_id: Option<String>,
    pub priority: Option<Priority>,
    pub assignees: Vec<String>,
    pub labels: Vec<String>,
    pub start_date: Option<String>,
    pub target_date: Option<String>,
    pub estimate_point_id: Option<String>,
    pub parent_id: Option<String>,
}

fn clean_item_name(text: &str) -> Result<String> {
    let name = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        return Err(invalid("A work item needs a title"));
    }
    if name.chars().count() > NAME_MAX {
        return Err(invalid(format!("A title is at most {NAME_MAX} characters")));
    }
    Ok(name)
}

fn clean_day(text: Option<&str>) -> Result<Option<String>> {
    match text.map(str::trim).filter(|t| !t.is_empty()) {
        None => Ok(None),
        Some(t) => chrono::NaiveDate::parse_from_str(t, "%Y-%m-%d").map(|d| Some(d.format("%Y-%m-%d").to_string())).map_err(|_| invalid("A date is written YYYY-MM-DD")),
    }
}

/// The editor's document and its words; an empty document is none.
fn clean_description(json: Option<&str>) -> Result<(Option<String>, String)> {
    let Some(json) = json.map(str::trim).filter(|j| !j.is_empty()) else { return Ok((None, String::new())) };
    let doc: Value = serde_json::from_str(json).map_err(|_| invalid("The description doesn't read"))?;
    let text = words(&doc);
    Ok(if text.trim().is_empty() && !json.contains("\"image\"") { (None, String::new()) } else { (Some(doc.to_string()), text) })
}

/// The words of an editor document, a line a block.
pub(crate) fn words(doc: &Value) -> String {
    fn walk(v: &Value, out: &mut String) {
        match v.get("type").and_then(Value::as_str) {
            Some("text") => out.push_str(v.get("text").and_then(Value::as_str).unwrap_or("")),
            Some("mention") => {
                out.push('@');
                out.push_str(v.get("attrs").and_then(|a| a.get("label")).and_then(Value::as_str).unwrap_or(""));
            }
            Some("hardBreak") => out.push('\n'),
            _ => {}
        }
        if let Some(children) = v.get("content").and_then(Value::as_array) {
            for c in children {
                walk(c, out);
            }
            if matches!(v.get("type").and_then(Value::as_str), Some("paragraph" | "heading" | "listItem" | "taskItem" | "codeBlock" | "blockquote")) {
                out.push('\n');
            }
        }
    }
    let mut out = String::new();
    walk(doc, &mut out);
    out.trim().to_owned()
}

/// A state of this project items can be in (not Intake's triage).
async fn check_state(pool: &SqlitePool, project_id: &str, state_id: &str) -> Result<(String, StateGroup)> {
    let row: Option<(String, StateGroup)> = sqlx::query_as("SELECT name, state_group FROM pm_state WHERE id = ? AND project_id = ?").bind(state_id).bind(project_id).fetch_optional(pool).await?;
    match row {
        Some((_, StateGroup::Triage)) | None => Err(invalid("That state isn't one of the project's")),
        Some(r) => Ok(r),
    }
}

async fn default_state(pool: &SqlitePool, project_id: &str) -> Result<String> {
    let row: Option<(String,)> = sqlx::query_as("SELECT id FROM pm_state WHERE project_id = ? AND is_default = 1").bind(project_id).fetch_optional(pool).await?;
    match row {
        Some((id,)) => Ok(id),
        None => {
            let any: Option<(String,)> = sqlx::query_as("SELECT id FROM pm_state WHERE project_id = ? AND state_group = 'backlog' ORDER BY sequence LIMIT 1").bind(project_id).fetch_optional(pool).await?;
            any.map(|(id,)| id).ok_or_else(|| invalid("The project has no state to start in"))
        }
    }
}

async fn check_labels(pool: &SqlitePool, project_id: &str, labels: &[String]) -> Result<Vec<String>> {
    let known: Vec<(String,)> = sqlx::query_as("SELECT id FROM pm_label WHERE project_id = ?").bind(project_id).fetch_all(pool).await?;
    let mut kept = Vec::new();
    for l in labels {
        if !known.iter().any(|(k,)| k == l) {
            return Err(invalid("That label isn't one of the project's"));
        }
        if !kept.contains(l) {
            kept.push(l.clone());
        }
    }
    Ok(kept)
}

async fn check_people(pool: &SqlitePool, who: &[String], already: &[String]) -> Result<Vec<String>> {
    let mut kept: Vec<String> = Vec::new();
    for w in who {
        let r = if already.iter().any(|a| a == w.trim()) { Some(w.trim().to_owned()) } else { super::check_ref(pool, Some(w)).await? };
        if let Some(r) = r {
            if !kept.contains(&r) {
                kept.push(r);
            }
        }
    }
    Ok(kept)
}

/// A point of the project's active estimate.
async fn check_point(pool: &SqlitePool, project_id: &str, point: Option<&str>) -> Result<Option<String>> {
    let Some(point) = point.filter(|p| !p.is_empty()) else { return Ok(None) };
    let ok: Option<(String,)> = sqlx::query_as("SELECT ep.id FROM pm_estimate_point ep JOIN pm_project m ON m.estimate_id = ep.estimate_id WHERE ep.id = ? AND m.project_id = ?").bind(point).bind(project_id).fetch_optional(pool).await?;
    ok.map(|(id,)| Some(id)).ok_or_else(|| invalid("That estimate isn't one of the project's active ones"))
}

/// A parent in the same project, not archived, and not the item or one of its sub-items.
async fn check_parent(pool: &SqlitePool, project_id: &str, parent: Option<&str>, me: Option<&str>) -> Result<Option<String>> {
    let Some(parent) = parent.filter(|p| !p.is_empty()) else { return Ok(None) };
    let row: Option<(String, Option<String>)> = sqlx::query_as("SELECT project_id, archived_at FROM pm_item WHERE id = ?").bind(parent).fetch_optional(pool).await?;
    match row {
        Some((p, None)) if p == project_id => {}
        _ => return Err(invalid("A parent is a work item of the same project, not archived")),
    }
    if let Some(me) = me {
        if parent == me || descendants(pool, me).await?.iter().any(|d| d == parent) {
            return Err(invalid("An item can't be its own parent, nor one of its sub-items'"));
        }
    }
    Ok(Some(parent.to_owned()))
}

/// Every item under this one, at any depth.
async fn descendants(pool: &SqlitePool, id: &str) -> Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as("WITH RECURSIVE d(id) AS (SELECT id FROM pm_item WHERE parent_id = ?1 UNION SELECT i.id FROM pm_item i JOIN d ON i.parent_id = d.id) SELECT id FROM d").bind(id).fetch_all(pool).await?;
    Ok(rows.into_iter().map(|(i,)| i).collect())
}

/// How "me" and people read in the activity.
async fn names(pool: &SqlitePool, who: &[String]) -> Result<String> {
    let mut out = Vec::new();
    for w in who {
        if w == "me" {
            out.push("You".to_owned());
        } else {
            let n: Option<(String,)> = sqlx::query_as("SELECT trim(name_first || ' ' || name_last) FROM person WHERE id = ?").bind(w).fetch_optional(pool).await?;
            out.push(n.map_or_else(|| "Someone".to_owned(), |(n,)| n));
        }
    }
    Ok(out.join(", "))
}

async fn label_names(pool: &SqlitePool, labels: &[String]) -> Result<String> {
    let mut out = Vec::new();
    for l in labels {
        let n: Option<(String,)> = sqlx::query_as("SELECT name FROM pm_label WHERE id = ?").bind(l).fetch_optional(pool).await?;
        out.push(n.map_or_else(|| "a label".to_owned(), |(n,)| n));
    }
    Ok(out.join(", "))
}

#[allow(clippy::too_many_arguments)]
async fn activity(tx: &mut Transaction<'_, Sqlite>, item: &str, kind: &str, field: &str, old: &str, new: &str, refs: (Option<&str>, Option<&str>), actor: &str) -> Result<()> {
    sqlx::query("INSERT INTO pm_activity (id, item_id, kind, field, old_value, new_value, old_ref, new_ref, actor) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(id())
        .bind(item)
        .bind(kind)
        .bind(field)
        .bind(old)
        .bind(new)
        .bind(refs.0)
        .bind(refs.1)
        .bind(actor)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// A description saved: a new version, or — when `merge` — the newest one changed when it is under
/// ten minutes old. Whether a version was added.
async fn version(tx: &mut Transaction<'_, Sqlite>, item: &str, json: Option<&str>, text: &str, actor: &str, merge: bool) -> Result<bool> {
    if !merge {
        sqlx::query("INSERT INTO pm_item_version (id, item_id, description_json, description_text, created_by) VALUES (?, ?, ?, ?, ?)").bind(id()).bind(item).bind(json).bind(text).bind(actor).execute(&mut **tx).await?;
        return Ok(true);
    }
    let recent: Option<(String,)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT id FROM pm_item_version WHERE item_id = ? AND created_by = ? AND created_at > strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-{VERSION_MINUTES} minutes') ORDER BY created_at DESC LIMIT 1"
    )))
    .bind(item)
    .bind(actor)
    .fetch_optional(&mut **tx)
    .await?;
    match recent {
        Some((v,)) => {
            sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE pm_item_version SET description_json = ?, description_text = ?, updated_at = {NOW} WHERE id = ?"))).bind(json).bind(text).bind(v).execute(&mut **tx).await?;
            Ok(false)
        }
        None => {
            sqlx::query("INSERT INTO pm_item_version (id, item_id, description_json, description_text, created_by) VALUES (?, ?, ?, ?, ?)").bind(id()).bind(item).bind(json).bind(text).bind(actor).execute(&mut **tx).await?;
            Ok(true)
        }
    }
}

/// Makes a work item: numbered in its project in the same transaction; the default state when
/// none is given; the project's default assignee when none; the maker, the assignees and the
/// project's default subscribers subscribed.
pub async fn create(pool: &SqlitePool, input: ItemInput, actor: &str) -> Result<PmItem> {
    let project = super::get(pool, &input.project_id).await?;
    if project.archived_at.is_some() {
        return Err(invalid("The project is archived — restore it to add work items"));
    }
    let name = clean_item_name(&input.name)?;
    let (description, text) = clean_description(input.description.as_deref())?;
    let state_id = match input.state_id.as_deref().filter(|s| !s.is_empty()) {
        Some(s) => s.to_owned(),
        None => default_state(pool, &project.id).await?,
    };
    let (_, group) = check_state(pool, &project.id, &state_id).await?;
    let mut assignees = check_people(pool, &input.assignees, &[]).await?;
    if assignees.is_empty() {
        assignees.extend(project.default_assignee.clone());
    }
    let labels = check_labels(pool, &project.id, &input.labels).await?;
    let (start, target) = (clean_day(input.start_date.as_deref())?, clean_day(input.target_date.as_deref())?);
    if let (Some(s), Some(t)) = (&start, &target) {
        if s > t {
            return Err(invalid("The start date comes before the due date"));
        }
    }
    let point = check_point(pool, &project.id, input.estimate_point_id.as_deref()).await?;
    let parent = check_parent(pool, &project.id, input.parent_id.as_deref(), None).await?;
    // The owner follows what they make (an agent or an automation doesn't subscribe).
    let mut subscribers: Vec<String> = if actor == "me" { vec!["me".to_owned()] } else { Vec::new() };
    for s in assignees.iter().chain(project.subscribers.iter()) {
        if !subscribers.contains(s) {
            subscribers.push(s.clone());
        }
    }

    let item = id();
    let mut tx = pool.begin().await?;
    let (sequence,): (i64,) = sqlx::query_as(sqlx::AssertSqlSafe(format!("UPDATE pm_project SET last_sequence = last_sequence + 1, updated_at = {NOW} WHERE project_id = ? RETURNING last_sequence")))
        .bind(&project.id)
        .fetch_one(&mut *tx)
        .await?;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO pm_item (id, project_id, sequence, name, description_json, description_text, state_id, priority, start_date, target_date, estimate_point_id, parent_id, sort_order, completed_at, created_by, updated_by) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, (SELECT coalesce(max(sort_order), 0) + 1000 FROM pm_item WHERE project_id = ?), {}, ?, ?)",
        if group == StateGroup::Completed { NOW } else { "NULL" }
    )))
    .bind(&item)
    .bind(&project.id)
    .bind(sequence)
    .bind(&name)
    .bind(&description)
    .bind(&text)
    .bind(&state_id)
    .bind(input.priority.unwrap_or(Priority::None))
    .bind(&start)
    .bind(&target)
    .bind(&point)
    .bind(&parent)
    .bind(&project.id)
    .bind(actor)
    .bind(actor)
    .execute(&mut *tx)
    .await?;
    for a in &assignees {
        sqlx::query("INSERT INTO pm_item_assignee (item_id, who) VALUES (?, ?)").bind(&item).bind(a).execute(&mut *tx).await?;
    }
    for l in &labels {
        sqlx::query("INSERT INTO pm_item_label (item_id, label_id) VALUES (?, ?)").bind(&item).bind(l).execute(&mut *tx).await?;
    }
    for s in &subscribers {
        sqlx::query("INSERT INTO pm_item_subscriber (item_id, who) VALUES (?, ?)").bind(&item).bind(s).execute(&mut *tx).await?;
    }
    if description.is_some() {
        version(&mut tx, &item, description.as_deref(), &text, actor, true).await?;
    }
    activity(&mut tx, &item, "created", "", "", &name, (None, Some(&state_id)), actor).await?;
    tx.commit().await?;
    get(pool, &item).await
}

/// One field changed: its name, its readable before and after, and the states' ids (a state's).
type Change<'a> = (&'a str, String, String, (Option<String>, Option<String>));

/// The edits: a field left out stays; an empty date, estimate or parent clears it.
#[derive(Debug, Clone, Default, Deserialize, Type)]
pub struct ItemPatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub state_id: Option<String>,
    pub priority: Option<Priority>,
    pub assignees: Option<Vec<String>>,
    pub labels: Option<Vec<String>>,
    pub start_date: Option<String>,
    pub target_date: Option<String>,
    pub estimate_point_id: Option<String>,
    pub parent_id: Option<String>,
    pub sort_order: Option<f64>,
}

/// Changes an item: everything checked first, then one transaction — the item, its assignees
/// and labels, one activity row a field changed (readable before and after), the description's
/// version; entering a completed state sets when, leaving it clears it.
pub async fn update(pool: &SqlitePool, id: &str, patch: ItemPatch, actor: &str) -> Result<PmItem> {
    update_with(pool, id, patch, actor, true).await
}

/// `merge`: whether a description saved within ten minutes changes the newest version (a restore
/// never does — it is a version of its own).
async fn update_with(pool: &SqlitePool, id: &str, patch: ItemPatch, actor: &str, merge: bool) -> Result<PmItem> {
    let item = get(pool, id).await?;
    if item.archived_at.is_some() {
        return Err(invalid("An archived work item is read-only — restore it to change it"));
    }
    let mut changes: Vec<Change> = Vec::new();
    let name = match patch.name.as_deref() {
        Some(n) => {
            let n = clean_item_name(n)?;
            (n != item.name).then(|| {
                changes.push(("title", item.name.clone(), n.clone(), (None, None)));
                n
            })
        }
        None => None,
    };
    let description = match patch.description.as_deref() {
        Some(d) => {
            let (json, text) = clean_description(Some(d))?;
            let before = item.description.as_ref().map(|j| j.0.to_string());
            (json != before).then(|| {
                changes.push(("description", String::new(), String::new(), (None, None)));
                (json, text)
            })
        }
        None => None,
    };
    let state = match patch.state_id.as_deref().filter(|s| *s != item.state_id) {
        Some(s) => {
            let (state_name, group) = check_state(pool, &item.project_id, s).await?;
            changes.push(("state", item.state_name.clone(), state_name, (Some(item.state_id.clone()), Some(s.to_owned()))));
            Some((s.to_owned(), group))
        }
        None => None,
    };
    let priority = patch.priority.filter(|p| *p != item.priority).inspect(|p| changes.push(("priority", item.priority.label().into(), p.label().into(), (None, None))));
    let assignees = match patch.assignees {
        Some(a) => {
            let kept = check_people(pool, &a, &item.assignees).await?;
            if !same_set(&kept, &item.assignees) {
                changes.push(("assignees", names(pool, &item.assignees).await?, names(pool, &kept).await?, (None, None)));
                Some(kept)
            } else {
                None
            }
        }
        None => None,
    };
    let labels = match patch.labels {
        Some(l) => {
            let kept = check_labels(pool, &item.project_id, &l).await?;
            if !same_set(&kept, &item.labels) {
                changes.push(("labels", label_names(pool, &item.labels).await?, label_names(pool, &kept).await?, (None, None)));
                Some(kept)
            } else {
                None
            }
        }
        None => None,
    };
    let start = match patch.start_date.as_deref() {
        Some(d) => {
            let d = clean_day(Some(d))?;
            (d != item.start_date).then(|| {
                changes.push(("start date", item.start_date.clone().unwrap_or_default(), d.clone().unwrap_or_default(), (None, None)));
                d
            })
        }
        None => None,
    };
    let target = match patch.target_date.as_deref() {
        Some(d) => {
            let d = clean_day(Some(d))?;
            (d != item.target_date).then(|| {
                changes.push(("due date", item.target_date.clone().unwrap_or_default(), d.clone().unwrap_or_default(), (None, None)));
                d
            })
        }
        None => None,
    };
    let (s, t) = (start.clone().unwrap_or_else(|| item.start_date.clone()), target.clone().unwrap_or_else(|| item.target_date.clone()));
    if let (Some(s), Some(t)) = (&s, &t) {
        if s > t {
            return Err(invalid("The start date comes before the due date"));
        }
    }
    let point = match patch.estimate_point_id.as_deref() {
        Some(p) => {
            let p = check_point(pool, &item.project_id, Some(p)).await?;
            if p != item.estimate_point_id {
                let value: Option<(String,)> = match &p {
                    Some(p) => sqlx::query_as("SELECT value FROM pm_estimate_point WHERE id = ?").bind(p).fetch_optional(pool).await?,
                    None => None,
                };
                changes.push(("estimate", item.estimate_value.clone().unwrap_or_default(), value.map(|(v,)| v).unwrap_or_default(), (None, None)));
                Some(p)
            } else {
                None
            }
        }
        None => None,
    };
    let parent = match patch.parent_id.as_deref() {
        Some(p) => {
            let p = check_parent(pool, &item.project_id, Some(p), Some(id)).await?;
            if p != item.parent_id {
                let key: Option<(String,)> = match &p {
                    Some(p) => sqlx::query_as("SELECT m.identifier || '-' || i.sequence FROM pm_item i JOIN pm_project m ON m.project_id = i.project_id WHERE i.id = ?").bind(p).fetch_optional(pool).await?,
                    None => None,
                };
                changes.push(("parent", item.parent_key.clone().unwrap_or_default(), key.map(|(k,)| k).unwrap_or_default(), (None, None)));
                Some(p)
            } else {
                None
            }
        }
        None => None,
    };
    let order = patch.sort_order.filter(|o| o.is_finite());
    if changes.is_empty() && order.is_none() {
        return Ok(item);
    }

    let mut tx = pool.begin().await?;
    let completed = match &state {
        Some((_, StateGroup::Completed)) => Some(true),
        Some(_) => Some(false),
        None => None,
    };
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE pm_item SET name = coalesce(?, name), \
         description_json = CASE WHEN ? THEN ? ELSE description_json END, description_text = CASE WHEN ? THEN ? ELSE description_text END, \
         state_id = coalesce(?, state_id), priority = coalesce(?, priority), \
         start_date = CASE WHEN ? THEN ? ELSE start_date END, target_date = CASE WHEN ? THEN ? ELSE target_date END, \
         estimate_point_id = CASE WHEN ? THEN ? ELSE estimate_point_id END, parent_id = CASE WHEN ? THEN ? ELSE parent_id END, \
         sort_order = coalesce(?, sort_order), \
         completed_at = CASE WHEN ? IS NULL THEN completed_at WHEN ? THEN coalesce(completed_at, {NOW}) ELSE NULL END, \
         updated_by = ?, updated_at = {NOW} WHERE id = ?"
    )))
    .bind(&name)
    .bind(description.is_some())
    .bind(description.as_ref().and_then(|(j, _)| j.clone()))
    .bind(description.is_some())
    .bind(description.as_ref().map(|(_, t)| t.clone()))
    .bind(state.as_ref().map(|(s, _)| s.clone()))
    .bind(priority)
    .bind(start.is_some())
    .bind(start.clone().flatten())
    .bind(target.is_some())
    .bind(target.clone().flatten())
    .bind(point.is_some())
    .bind(point.clone().flatten())
    .bind(parent.is_some())
    .bind(parent.clone().flatten())
    .bind(order)
    .bind(completed)
    .bind(completed.unwrap_or(false))
    .bind(actor)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    if let Some(a) = &assignees {
        sqlx::query("DELETE FROM pm_item_assignee WHERE item_id = ?").bind(id).execute(&mut *tx).await?;
        for w in a {
            sqlx::query("INSERT INTO pm_item_assignee (item_id, who) VALUES (?, ?)").bind(id).bind(w).execute(&mut *tx).await?;
            // An assignee follows the item.
            sqlx::query("INSERT INTO pm_item_subscriber (item_id, who) VALUES (?, ?) ON CONFLICT DO NOTHING").bind(id).bind(w).execute(&mut *tx).await?;
        }
    }
    if let Some(l) = &labels {
        sqlx::query("DELETE FROM pm_item_label WHERE item_id = ?").bind(id).execute(&mut *tx).await?;
        for label in l {
            sqlx::query("INSERT INTO pm_item_label (item_id, label_id) VALUES (?, ?)").bind(id).bind(label).execute(&mut *tx).await?;
        }
    }
    // The description's autosaves are one change: no second row while the newest activity is this
    // actor's description change of the last ten minutes (as versions merge).
    let quiet = if let Some((json, text)) = &description {
        version(&mut tx, id, json.as_deref(), text, actor, merge).await?;
        merge
            && sqlx::query_as::<_, (i64,)>(sqlx::AssertSqlSafe(format!(
                "SELECT 1 FROM pm_activity WHERE id = (SELECT id FROM pm_activity WHERE item_id = ? ORDER BY created_at DESC, id DESC LIMIT 1) \
                 AND field = 'description' AND actor = ? AND created_at > strftime('%Y-%m-%dT%H:%M:%fZ', 'now', '-{VERSION_MINUTES} minutes')"
            )))
            .bind(id)
            .bind(actor)
            .fetch_optional(&mut *tx)
            .await?
            .is_some()
    } else {
        false
    };
    for (field, old, new, refs) in &changes {
        if quiet && *field == "description" {
            continue;
        }
        activity(&mut tx, id, "updated", field, old, new, (refs.0.as_deref(), refs.1.as_deref()), actor).await?;
    }
    tx.commit().await?;
    get(pool, id).await
}

/// Archives an item — only a completed or cancelled one (read-only after, still found) — or
/// restores it.
pub async fn archive(pool: &SqlitePool, id: &str, archived: bool, actor: &str) -> Result<PmItem> {
    let item = get(pool, id).await?;
    if archived && !matches!(item.state_group, StateGroup::Completed | StateGroup::Cancelled) {
        return Err(invalid("Only completed or cancelled work items can be archived"));
    }
    if archived == item.archived_at.is_some() {
        return Ok(item);
    }
    let mut tx = pool.begin().await?;
    let sql = if archived { format!("UPDATE pm_item SET archived_at = {NOW}, updated_at = {NOW} WHERE id = ?") } else { format!("UPDATE pm_item SET archived_at = NULL, updated_at = {NOW} WHERE id = ?") };
    sqlx::query(sqlx::AssertSqlSafe(sql)).bind(id).execute(&mut *tx).await?;
    activity(&mut tx, id, if archived { "archived" } else { "restored" }, "", "", "", (None, None), actor).await?;
    tx.commit().await?;
    get(pool, id).await
}

/// A project's items' folders (their files) — gone with the project.
pub async fn delete_project_files(pool: &SqlitePool, folder: &str, project_id: &str) -> Result<Vec<PathBuf>> {
    let ids: Vec<(String,)> = sqlx::query_as("SELECT id FROM pm_item WHERE project_id = ?").bind(project_id).fetch_all(pool).await?;
    Ok(ids.into_iter().map(|(i,)| files_dir(folder, &i)).collect())
}

/// Deletes an item with its sub-items (and their comments, activity and files).
pub async fn delete(pool: &SqlitePool, folder: &str, id: &str) -> Result<()> {
    get(pool, id).await?;
    let mut all = descendants(pool, id).await?;
    all.push(id.to_owned());
    let mut tx = pool.begin().await?;
    for i in &all {
        sqlx::query("DELETE FROM pm_item WHERE id = ?").bind(i).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    for i in &all {
        let _ = std::fs::remove_dir_all(files_dir(folder, i));
    }
    Ok(())
}

/// A copy of an item, in its project or another: the same title, description (its pictures copied
/// into the copy's own folder, so deleting the original keeps them), priority, dates and
/// assignees; in another project its default state, no labels, estimate or parent.
pub async fn copy(pool: &SqlitePool, folder: &str, id: &str, project_id: Option<&str>, actor: &str) -> Result<PmItem> {
    let item = get(pool, id).await?;
    let target = project_id.filter(|p| !p.is_empty()).unwrap_or(&item.project_id).to_owned();
    let same = target == item.project_id;
    let made = create(
        pool,
        ItemInput {
            project_id: target,
            name: item.name.clone(),
            description: item.description.as_ref().map(|d| d.0.to_string()),
            state_id: same.then(|| item.state_id.clone()),
            priority: Some(item.priority),
            assignees: item.assignees.clone(),
            labels: if same { item.labels.clone() } else { Vec::new() },
            start_date: item.start_date.clone(),
            target_date: item.target_date.clone(),
            estimate_point_id: if same { item.estimate_point_id.clone() } else { None },
            parent_id: if same { item.parent_id.clone() } else { None },
        },
        actor,
    )
    .await?;
    let from = files_dir(folder, &item.id);
    let mut tx = pool.begin().await?;
    if from.is_dir() {
        let to = files_dir(folder, &made.id);
        tokio::fs::create_dir_all(&to).await.with_context(|| format!("couldn't make {}", to.display()))?;
        let mut entries = tokio::fs::read_dir(&from).await?;
        while let Some(e) = entries.next_entry().await? {
            if e.file_type().await?.is_file() {
                tokio::fs::copy(e.path(), to.join(e.file_name())).await.with_context(|| format!("couldn't copy {}", e.path().display()))?;
            }
        }
        // The editor keeps a picture as an asset address with its path's slashes as %2F.
        let (a, b) = (format!("pm%2F{}%2F", item.id), format!("pm%2F{}%2F", made.id));
        sqlx::query("UPDATE pm_item SET description_json = replace(description_json, ?, ?) WHERE id = ?").bind(&a).bind(&b).bind(&made.id).execute(&mut *tx).await?;
        sqlx::query("UPDATE pm_item_version SET description_json = replace(description_json, ?, ?) WHERE item_id = ?").bind(&a).bind(&b).bind(&made.id).execute(&mut *tx).await?;
    }
    activity(&mut tx, &made.id, "copied", "", "", &item.key, (None, None), actor).await?;
    tx.commit().await?;
    get(pool, &made.id).await
}

/// Follows or stops following an item.
pub async fn subscribe(pool: &SqlitePool, id: &str, who: &str, on: bool) -> Result<PmItem> {
    get(pool, id).await?;
    if on {
        sqlx::query("INSERT INTO pm_item_subscriber (item_id, who) VALUES (?, ?) ON CONFLICT DO NOTHING").bind(id).bind(who).execute(pool).await?;
    } else {
        sqlx::query("DELETE FROM pm_item_subscriber WHERE item_id = ? AND who = ?").bind(id).bind(who).execute(pool).await?;
    }
    get(pool, id).await
}

#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct PmActivity {
    pub id: String,
    pub kind: String,
    pub field: String,
    pub old_value: String,
    pub new_value: String,
    pub old_ref: Option<String>,
    pub new_ref: Option<String>,
    pub actor: String,
    pub created_at: String,
}

/// An item's activity, oldest first.
pub async fn activities(pool: &SqlitePool, id: &str) -> Result<Vec<PmActivity>> {
    Ok(sqlx::query_as("SELECT id, kind, field, old_value, new_value, old_ref, new_ref, actor, created_at FROM pm_activity WHERE item_id = ? ORDER BY created_at, id").bind(id).fetch_all(pool).await?)
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct PmVersion {
    pub id: String,
    pub description: Option<Json>,
    pub description_text: String,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

/// The description's versions, newest first.
pub async fn versions(pool: &SqlitePool, id: &str) -> Result<Vec<PmVersion>> {
    let rows: Vec<(String, Option<String>, String, String, String, String)> =
        sqlx::query_as("SELECT id, description_json, description_text, created_by, created_at, updated_at FROM pm_item_version WHERE item_id = ? ORDER BY updated_at DESC, id DESC").bind(id).fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|(id, json, text, by, created, updated)| PmVersion { id, description: json.and_then(|j| serde_json::from_str(&j).ok()).map(Json), description_text: text, created_by: by, created_at: created, updated_at: updated })
        .collect())
}

/// Brings a version's description back (itself kept as a new version).
pub async fn restore_version(pool: &SqlitePool, version_id: &str, actor: &str) -> Result<PmItem> {
    let row: Option<(String, Option<String>)> = sqlx::query_as("SELECT item_id, description_json FROM pm_item_version WHERE id = ?").bind(version_id).fetch_optional(pool).await?;
    let (item, json) = row.ok_or_else(|| anyhow!(AppError::not_found("That version is gone")))?;
    update_with(pool, &item, ItemPatch { description: Some(json.unwrap_or_default()), ..ItemPatch::default() }, actor, false).await
}

// ── Files ──

/// Under the workspace's `files/`, where the window may show and open files (the asset and opener
/// scopes).
fn files_dir(folder: &str, item: &str) -> PathBuf {
    Path::new(folder).join("files").join("pm").join(item)
}

/// A file kept with an item (a picture in its description or a comment).
#[derive(Debug, Clone, Serialize, Type)]
pub struct ItemFile {
    pub path: String,
    pub name: String,
}

const FILE_MAX: usize = 50 * 1024 * 1024;

pub async fn add_file_bytes(pool: &SqlitePool, folder: &str, item: &str, name: &str, bytes: &[u8]) -> Result<ItemFile> {
    get(pool, item).await?;
    if bytes.is_empty() {
        return Err(invalid("An empty file"));
    }
    if bytes.len() > FILE_MAX {
        return Err(invalid("A file kept with a work item is at most 50 MB"));
    }
    let dir = files_dir(folder, item);
    tokio::fs::create_dir_all(&dir).await.with_context(|| format!("couldn't make {}", dir.display()))?;
    let name = crate::kernel::records::files::safe_name(name);
    let path = dir.join(format!("{}-{name}", &id()[24..]));
    tokio::fs::write(&path, bytes).await.with_context(|| format!("couldn't write {}", path.display()))?;
    Ok(ItemFile { path: path.to_string_lossy().into_owned(), name })
}

pub async fn add_file(pool: &SqlitePool, folder: &str, item: &str, source: &Path) -> Result<ItemFile> {
    let meta = tokio::fs::metadata(source).await.with_context(|| format!("couldn't read {}", source.display()))?;
    if !meta.is_file() || meta.len() > FILE_MAX as u64 {
        return Err(invalid("A file kept with a work item is a file of at most 50 MB"));
    }
    let bytes = tokio::fs::read(source).await.with_context(|| format!("couldn't read {}", source.display()))?;
    let name = source.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into());
    add_file_bytes(pool, folder, item, &name, &bytes).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::hub::WorkspaceHub;
    use crate::modules::pm::{self, comments, drafts, settings, Features, NewPmProject, Network};

    fn project(name: &str, identifier: &str) -> NewPmProject {
        NewPmProject { name: name.into(), identifier: identifier.into(), description: String::new(), emoji: String::new(), cover: String::new(), network: Network::Private, lead: None, features: Features::default() }
    }

    #[test]
    fn a_work_item_lives_as_plane_says() {
        let root = std::env::temp_dir().join(format!("newdawn-pm-items-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let folder = hub.workspace("test").await.unwrap().folder_path;
            let p = pm::create(&pool, project("VIFC Fintech Hub", "VIFC")).await.unwrap();
            let states = settings::states(&pool, &p.id).await.unwrap();
            let state = |name: &str| states.iter().find(|s| s.name == name).unwrap().id.clone();

            // From a draft: VIFC-1, in the default state, the maker subscribed.
            let draft = drafts::save(&pool, None, ItemInput { project_id: p.id.clone(), name: "Sandbox launch plan".into(), ..ItemInput::default() }).await.unwrap();
            assert_eq!(drafts::list(&pool).await.unwrap().len(), 1);
            let mut input = draft.input.clone();
            input.description = Some(r#"{"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Three phases."}]}]}"#.into());
            let parent = drafts::publish(&pool, &draft.id, input, "me").await.unwrap();
            assert_eq!((parent.key.as_str(), parent.state_name.as_str(), parent.subscribers.clone(), parent.description_text.as_str()), ("VIFC-1", "Backlog", vec!["me".to_string()], "Three phases."));
            assert!(drafts::list(&pool).await.unwrap().is_empty(), "the draft is gone once made");

            // Two sub-items; one Done → 1 of 2 (50 %).
            let sub = |name: &str| ItemInput { project_id: p.id.clone(), name: name.into(), parent_id: Some(parent.id.clone()), ..ItemInput::default() };
            let a = create(&pool, sub("Write the rules"), "me").await.unwrap();
            let b = create(&pool, sub("Pick the first fintechs"), "me").await.unwrap();
            assert_eq!((a.key.as_str(), b.key.as_str(), b.parent_key.as_deref()), ("VIFC-2", "VIFC-3", Some("VIFC-1")));
            let done = update(&pool, &a.id, ItemPatch { state_id: Some(state("Done")), ..ItemPatch::default() }, "me").await.unwrap();
            assert!(done.completed_at.is_some());
            let p1 = get(&pool, &parent.id).await.unwrap();
            assert_eq!((p1.sub_items, p1.sub_done), (2, 1), "progress 50 %");
            assert_eq!(update(&pool, &a.id, ItemPatch { state_id: Some(state("Todo")), ..ItemPatch::default() }, "me").await.unwrap().completed_at, None, "leaving Done clears when");

            // The parent through In Progress: its state changes are kept with their states (Transition).
            update(&pool, &parent.id, ItemPatch { state_id: Some(state("In Progress")), priority: Some(Priority::High), ..ItemPatch::default() }, "me").await.unwrap();
            let acts = activities(&pool, &parent.id).await.unwrap();
            let moves: Vec<(&str, &str, Option<&str>)> = acts.iter().filter(|x| x.field == "state").map(|x| (x.old_value.as_str(), x.new_value.as_str(), x.new_ref.as_deref())).collect();
            assert_eq!(moves, [("Backlog", "In Progress", Some(state("In Progress").as_str()))]);
            assert!(acts.iter().any(|x| x.field == "priority" && x.new_value == "High") && acts[0].kind == "created");
            // Archive: In Progress refused; Done allowed, read-only, restored.
            assert!(archive(&pool, &parent.id, true, "me").await.unwrap_err().to_string().contains("Only completed or cancelled"));
            update(&pool, &b.id, ItemPatch { state_id: Some(state("Cancelled")), ..ItemPatch::default() }, "me").await.unwrap();
            assert!(archive(&pool, &b.id, true, "me").await.unwrap().archived_at.is_some());
            assert!(update(&pool, &b.id, ItemPatch { name: Some("x".into()), ..ItemPatch::default() }, "me").await.is_err(), "archived is read-only");
            assert_eq!(list(&pool, &p.id, true).await.unwrap().len(), 1);
            archive(&pool, &b.id, false, "me").await.unwrap();

            // A comment with a mention and a reaction; a reply to a reply answers its comment.
            sqlx::query("INSERT INTO person (id, name, name_first) VALUES ('p1', 'Lan Pham', 'Lan')").execute(&pool).await.unwrap();
            let body = r#"{"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Ask "},{"type":"mention","attrs":{"id":"p1","label":"Lan Pham","object":"people"}}]}]}"#;
            let cs = comments::add(&pool, &parent.id, body, None, "me").await.unwrap();
            let first = cs[0].id.clone();
            assert_eq!(cs[0].body_text, "Ask @Lan Pham");
            let (mentions,): (i64,) = sqlx::query_as("SELECT count(*) FROM pm_mention WHERE who = 'p1'").fetch_one(&pool).await.unwrap();
            assert_eq!(mentions, 1);
            let cs = comments::add(&pool, &parent.id, r#"{"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Done."}]}]}"#, Some(&first), "me").await.unwrap();
            let cs = comments::add(&pool, &parent.id, r#"{"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Thanks."}]}]}"#, Some(&cs[1].id), "me").await.unwrap();
            assert_eq!(cs[2].parent_id.as_deref(), Some(first.as_str()), "one level of replies");
            comments::toggle_reaction(&pool, &parent.id, Some(&first), "🎉", "me").await.unwrap();
            assert_eq!(comments::list(&pool, &parent.id).await.unwrap()[0].reactions[0].emoji, "🎉");
            comments::toggle_reaction(&pool, &parent.id, Some(&first), "🎉", "me").await.unwrap();
            assert!(comments::list(&pool, &parent.id).await.unwrap()[0].reactions.is_empty(), "a second press takes it back");
            comments::toggle_reaction(&pool, &parent.id, None, "👍", "me").await.unwrap();
            assert_eq!(comments::item_reactions(&pool, &parent.id).await.unwrap().len(), 1, "on the description");
            assert!(comments::toggle_reaction(&pool, &parent.id, None, "🦄", "me").await.is_err());
            assert!(comments::add(&pool, &parent.id, r#"{"type":"doc","content":[]}"#, None, "me").await.is_err(), "a comment needs words");
            assert_eq!(get(&pool, &parent.id).await.unwrap().comments, 3);

            // The description's versions: two saves within ten minutes are one version.
            let doc = |t: &str| format!(r#"{{"type":"doc","content":[{{"type":"paragraph","content":[{{"type":"text","text":"{t}"}}]}}]}}"#);
            update(&pool, &parent.id, ItemPatch { description: Some(doc("Four phases.")), ..ItemPatch::default() }, "me").await.unwrap();
            update(&pool, &parent.id, ItemPatch { description: Some(doc("Five phases.")), ..ItemPatch::default() }, "me").await.unwrap();
            let vs = versions(&pool, &parent.id).await.unwrap();
            assert_eq!((vs.len(), vs[0].description_text.as_str()), (1, "Five phases."));
            let described = || async { activities(&pool, &parent.id).await.unwrap().iter().filter(|a| a.field == "description").count() };
            assert_eq!(described().await, 1, "autosaves are one change");
            // An agent's save is a version of its own; restoring the owner's makes another, the
            // agent's kept (never merged away).
            update(&pool, &parent.id, ItemPatch { description: Some(doc("Six phases.")), ..ItemPatch::default() }, "Projects agent").await.unwrap();
            let vs = versions(&pool, &parent.id).await.unwrap();
            assert_eq!(vs.len(), 2);
            restore_version(&pool, &vs[1].id, "me").await.unwrap();
            let vs = versions(&pool, &parent.id).await.unwrap();
            assert_eq!((vs.len(), vs[0].description_text.as_str(), vs[1].description_text.as_str()), (3, "Five phases.", "Six phases."));
            assert_eq!(described().await, 3);

            // A state items are in can't go; numbers aren't given twice; delete takes sub-items.
            assert!(settings::delete_state(&pool, &state("In Progress")).await.unwrap_err().to_string().contains("work item"));
            delete(&pool, &folder, &parent.id).await.unwrap();
            assert!(list(&pool, &p.id, false).await.unwrap().is_empty(), "sub-items went with it");
            let next = create(&pool, ItemInput { project_id: p.id.clone(), name: "After".into(), ..ItemInput::default() }, "me").await.unwrap();
            assert_eq!(next.key, "VIFC-4");
            // A copy elsewhere: its default state, no labels.
            let other = pm::create(&pool, project("Other", "OT")).await.unwrap();
            let label = settings::create_label(&pool, &p.id, settings::LabelInput { name: "Area".into(), description: String::new(), color: "purple".into(), parent_id: None }).await.unwrap()[0].id.clone();
            update(&pool, &next.id, ItemPatch { labels: Some(vec![label]), state_id: Some(state("Todo")), ..ItemPatch::default() }, "me").await.unwrap();
            // Its picture copied with it: the original's deletion leaves the copy's.
            let pic = add_file_bytes(&pool, &folder, &next.id, "map.png", b"png").await.unwrap();
            let src = format!("asset://localhost/{}", pic.path.replace('/', "%2F"));
            let pictured = serde_json::json!({"type": "doc", "content": [{"type": "paragraph", "content": [{"type": "text", "text": "Map"}]}, {"type": "image", "attrs": {"src": src}}]});
            update(&pool, &next.id, ItemPatch { description: Some(pictured.to_string()), ..ItemPatch::default() }, "me").await.unwrap();
            let copied = copy(&pool, &folder, &next.id, Some(&other.id), "me").await.unwrap();
            assert_eq!((copied.key.as_str(), copied.state_name.as_str(), copied.labels.len()), ("OT-1", "Backlog", 0));
            let doc = copied.description.as_ref().unwrap().0.to_string();
            assert!(doc.contains(&format!("pm%2F{}%2F", copied.id)) && !doc.contains(&next.id));
            assert!(std::path::Path::new(&pic.path.replace(&next.id, &copied.id)).exists());
            // Checks: a date order, a title, a parent loop.
            assert!(create(&pool, ItemInput { project_id: p.id.clone(), name: "x".into(), start_date: Some("2026-12-01".into()), target_date: Some("2026-11-01".into()), ..ItemInput::default() }, "me").await.is_err());
            assert!(create(&pool, ItemInput { project_id: p.id.clone(), name: "  ".into(), ..ItemInput::default() }, "me").await.is_err());
            let child = create(&pool, ItemInput { project_id: p.id.clone(), name: "Child".into(), parent_id: Some(next.id.clone()), ..ItemInput::default() }, "me").await.unwrap();
            assert!(update(&pool, &next.id, ItemPatch { parent_id: Some(child.id.clone()), ..ItemPatch::default() }, "me").await.is_err(), "no loops");
            // Files: kept under the workspace, gone with the item.
            let f = add_file_bytes(&pool, &folder, &child.id, "plan.png", b"png").await.unwrap();
            assert!(std::path::Path::new(&f.path).exists());
            delete(&pool, &folder, &child.id).await.unwrap();
            assert!(!std::path::Path::new(&f.path).exists());
            hub.close_all().await;
        });
        let _ = std::fs::remove_dir_all(root);
    }
}
