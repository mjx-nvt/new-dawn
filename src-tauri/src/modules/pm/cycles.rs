//! Plane's cycles (D-078): a project's time-boxed runs of work — a name, a description, both
//! dates or none (a draft), never two at once in a project; Draft, Upcoming, Active, Completed
//! from the dates (or completed early); one cycle an item; the open items of a completed cycle
//! moved on once; archived when completed; deleted with their items left. The burn-down reads the
//! items' completion days; the breakdowns count them by assignee, state and label.

use anyhow::{Result, anyhow};
use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::{Sqlite, SqlitePool, Transaction};

use super::items;
use crate::error::AppError;

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";
const NAME_MAX: usize = 255;

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum CycleStatus {
    Draft,
    Upcoming,
    Active,
    Completed,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct PmCycle {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub status: CycleStatus,
    pub completed_at: Option<String>,
    pub transferred_at: Option<String>,
    pub archived_at: Option<String>,
    pub is_favorite: bool,
    /// Its work items, those completed, those cancelled.
    pub items: u32,
    pub done: u32,
    pub cancelled: u32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(sqlx::FromRow)]
struct Row {
    id: String,
    project_id: String,
    name: String,
    description: String,
    start_date: Option<String>,
    end_date: Option<String>,
    completed_at: Option<String>,
    transferred_at: Option<String>,
    progress_json: Option<String>,
    archived_at: Option<String>,
    is_favorite: bool,
    items: u32,
    done: u32,
    cancelled: u32,
    created_at: String,
    updated_at: String,
}

/// This Mac's day — what a cycle's status is read against.
pub fn today() -> NaiveDate {
    chrono::Local::now().date_naive()
}

/// The status a cycle's dates give on `today`.
pub fn status(start: Option<&str>, end: Option<&str>, completed: bool, today: NaiveDate) -> CycleStatus {
    let day = |d: Option<&str>| d.and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());
    match (day(start), day(end)) {
        _ if completed => CycleStatus::Completed,
        (Some(s), Some(_)) if today < s => CycleStatus::Upcoming,
        (Some(_), Some(e)) if today > e => CycleStatus::Completed,
        (Some(_), Some(_)) => CycleStatus::Active,
        _ => CycleStatus::Draft,
    }
}

const SELECT: &str = "SELECT c.id, c.project_id, c.name, c.description, c.start_date, c.end_date, c.completed_at, c.transferred_at, c.progress_json, c.archived_at, \
     EXISTS (SELECT 1 FROM favorite f WHERE f.pm_cycle_id = c.id AND f.deleted_at IS NULL) AS is_favorite, \
     (SELECT count(*) FROM pm_cycle_item x JOIN pm_item i ON i.id = x.item_id WHERE x.cycle_id = c.id AND i.archived_at IS NULL) AS items, \
     (SELECT count(*) FROM pm_cycle_item x JOIN pm_item i ON i.id = x.item_id JOIN pm_state s ON s.id = i.state_id WHERE x.cycle_id = c.id AND i.archived_at IS NULL AND s.state_group = 'completed') AS done, \
     (SELECT count(*) FROM pm_cycle_item x JOIN pm_item i ON i.id = x.item_id JOIN pm_state s ON s.id = i.state_id WHERE x.cycle_id = c.id AND i.archived_at IS NULL AND s.state_group = 'cancelled') AS cancelled, \
     c.created_at, c.updated_at FROM pm_cycle c";

/// The counts kept when a cycle's open work moved on.
#[derive(Serialize, Deserialize)]
struct Snapshot {
    items: u32,
    done: u32,
    cancelled: u32,
    /// The details panel as it stood (the burn-down and breakdowns of all its work).
    details: Option<CycleDetails>,
}

fn cycle(r: Row, today: NaiveDate) -> PmCycle {
    let status = status(r.start_date.as_deref(), r.end_date.as_deref(), r.completed_at.is_some(), today);
    // A transferred cycle reads its numbers as they stood (its open work lives on elsewhere).
    let kept: Option<Snapshot> = r.progress_json.as_deref().and_then(|j| serde_json::from_str(j).ok());
    let (items, done, cancelled) = kept.map_or((r.items, r.done, r.cancelled), |k| (k.items, k.done, k.cancelled));
    PmCycle {
        id: r.id,
        project_id: r.project_id,
        name: r.name,
        description: r.description,
        start_date: r.start_date,
        end_date: r.end_date,
        status,
        completed_at: r.completed_at,
        transferred_at: r.transferred_at,
        archived_at: r.archived_at,
        is_favorite: r.is_favorite,
        items,
        done,
        cancelled,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

/// A project's cycles (the archived ones too: the page sorts them), favorites first, then by start.
pub async fn list(pool: &SqlitePool, project_id: &str, today: NaiveDate) -> Result<Vec<PmCycle>> {
    let rows: Vec<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} WHERE c.project_id = ? ORDER BY is_favorite DESC, c.start_date IS NULL, c.start_date, c.created_at")))
        .bind(project_id)
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(|r| cycle(r, today)).collect())
}

/// Every open project's active cycles (the workspace's Active cycles page).
pub async fn active(pool: &SqlitePool, today: NaiveDate) -> Result<Vec<PmCycle>> {
    let day = today.to_string();
    let rows: Vec<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{SELECT} JOIN pm_project m ON m.project_id = c.project_id WHERE m.archived_at IS NULL AND m.cycles = 1 AND c.archived_at IS NULL AND c.completed_at IS NULL \
         AND c.start_date <= ? AND c.end_date >= ? ORDER BY c.end_date"
    )))
    .bind(&day)
    .bind(&day)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| cycle(r, today)).collect())
}

pub async fn get(pool: &SqlitePool, id: &str, today: NaiveDate) -> Result<PmCycle> {
    let row: Option<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} WHERE c.id = ?"))).bind(id).fetch_optional(pool).await?;
    row.map(|r| cycle(r, today)).ok_or_else(|| anyhow!(AppError::not_found("That cycle is gone")))
}

fn clean_name(name: &str) -> Result<String> {
    let n = name.trim();
    if n.is_empty() {
        return Err(invalid("A cycle needs a name"));
    }
    if n.chars().count() > NAME_MAX {
        return Err(invalid(format!("A cycle's name is at most {NAME_MAX} characters")));
    }
    Ok(n.to_owned())
}

/// Both days or none; the start not after the end.
fn clean_dates(start: Option<&str>, end: Option<&str>) -> Result<(Option<String>, Option<String>)> {
    // As a work item's days (YYYY-MM-DD, years 1900 to 2199).
    let day = |d: Option<&str>| -> Result<Option<NaiveDate>> { Ok(items::clean_day(d)?.and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())) };
    match (day(start)?, day(end)?) {
        (None, None) => Ok((None, None)),
        (Some(s), Some(e)) if s <= e => Ok((Some(s.to_string()), Some(e.to_string()))),
        (Some(_), Some(_)) => Err(invalid("A cycle ends on or after its start")),
        _ => Err(invalid("A cycle has both dates, or none (a draft)")),
    }
}

/// No two cycles of a project overlap (their days counted in).
async fn check_overlap(pool: &SqlitePool, project_id: &str, except: Option<&str>, start: Option<&str>, end: Option<&str>) -> Result<()> {
    let (Some(s), Some(e)) = (start, end) else { return Ok(()) };
    // A cycle completed early ends that day: the next may start on it.
    let other: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM pm_cycle WHERE project_id = ? AND id <> coalesce(?, '') AND start_date IS NOT NULL AND start_date <= ? \
         AND CASE WHEN completed_at IS NULL THEN end_date >= ? ELSE end_date > ? END LIMIT 1",
    )
    .bind(project_id)
    .bind(except)
    .bind(e)
    .bind(s)
    .bind(s)
        .fetch_optional(pool)
        .await?;
    match other {
        Some((name,)) => Err(invalid(format!("Those dates overlap “{name}” — cycles of a project don't overlap"))),
        None => Ok(()),
    }
}

#[derive(Debug, Clone, Deserialize, Type)]
pub struct CycleInput {
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

pub async fn create(pool: &SqlitePool, input: CycleInput, actor: &str, today: NaiveDate) -> Result<PmCycle> {
    super::get(pool, &input.project_id).await?;
    let name = clean_name(&input.name)?;
    let (start, end) = clean_dates(input.start_date.as_deref(), input.end_date.as_deref())?;
    check_overlap(pool, &input.project_id, None, start.as_deref(), end.as_deref()).await?;
    let id = uuid::Uuid::now_v7().to_string();
    sqlx::query("INSERT INTO pm_cycle (id, project_id, name, description, start_date, end_date, created_by) VALUES (?, ?, ?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(&input.project_id)
        .bind(&name)
        .bind(input.description.trim().chars().take(2000).collect::<String>())
        .bind(&start)
        .bind(&end)
        .bind(actor)
        .execute(pool)
        .await?;
    super::events::emit(pool, "cycle", "created", &id, actor);
    get(pool, &id, today).await
}

/// The edits; a field left out stays (dates: both, or both cleared with "").
#[derive(Debug, Clone, Default, Deserialize, Type)]
pub struct CyclePatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub is_favorite: Option<bool>,
}

pub async fn update(pool: &SqlitePool, id: &str, patch: CyclePatch, actor: &str, today: NaiveDate) -> Result<PmCycle> {
    let c = get(pool, id, today).await?;
    let name = patch.name.as_deref().map(clean_name).transpose()?;
    let dates = if patch.start_date.is_some() || patch.end_date.is_some() {
        let (s, e) = clean_dates(patch.start_date.as_deref().or(c.start_date.as_deref()), patch.end_date.as_deref().or(c.end_date.as_deref()))?;
        if (&s, &e) == (&c.start_date, &c.end_date) {
            None
        } else if c.status == CycleStatus::Completed {
            return Err(invalid("A completed cycle keeps its dates"));
        } else {
            check_overlap(pool, &c.project_id, Some(id), s.as_deref(), e.as_deref()).await?;
            Some((s, e))
        }
    } else {
        None
    };
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE pm_cycle SET name = coalesce(?, name), description = coalesce(?, description), \
         start_date = CASE WHEN ? THEN ? ELSE start_date END, end_date = CASE WHEN ? THEN ? ELSE end_date END, \
         updated_at = {NOW} WHERE id = ?"
    )))
    .bind(&name)
    .bind(patch.description.as_deref().map(|d| d.trim().chars().take(2000).collect::<String>()))
    .bind(dates.is_some())
    .bind(dates.as_ref().and_then(|(s, _)| s.clone()))
    .bind(dates.is_some())
    .bind(dates.as_ref().and_then(|(_, e)| e.clone()))
    .bind(id)
    .execute(pool)
    .await?;
    // A star is a favorite row (D-079).
    if let Some(on) = patch.is_favorite {
        super::views::pin(pool, super::views::Pin::Cycle, id, on).await?;
    }
    if name.is_some() || patch.description.is_some() || dates.is_some() {
        super::events::emit(pool, "cycle", "updated", id, actor);
    }
    get(pool, id, today).await
}

/// Starts an upcoming cycle today, or ends an active one today (Plane's start and complete now).
pub async fn start_now(pool: &SqlitePool, id: &str, today: NaiveDate) -> Result<PmCycle> {
    let c = get(pool, id, today).await?;
    if c.status != CycleStatus::Upcoming {
        return Err(invalid("Only an upcoming cycle can start now"));
    }
    let day = today.to_string();
    check_overlap(pool, &c.project_id, Some(id), Some(&day), c.end_date.as_deref()).await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE pm_cycle SET start_date = ?, updated_at = {NOW} WHERE id = ?"))).bind(&day).bind(id).execute(pool).await?;
    super::events::emit(pool, "cycle", "updated", id, "me");
    get(pool, id, today).await
}

pub async fn complete_now(pool: &SqlitePool, id: &str, today: NaiveDate) -> Result<PmCycle> {
    let c = get(pool, id, today).await?;
    if c.status != CycleStatus::Active {
        return Err(invalid("Only an active cycle can be completed now"));
    }
    sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE pm_cycle SET end_date = ?, completed_at = {NOW}, updated_at = {NOW} WHERE id = ?"))).bind(today.to_string()).bind(id).execute(pool).await?;
    super::events::emit(pool, "cycle", "updated", id, "me");
    get(pool, id, today).await
}

/// Puts items in a cycle (an item is in one: it moves), each with its activity, in one
/// transaction. Not a completed cycle's; items of the cycle's project only, not archived ones.
pub async fn add_items(pool: &SqlitePool, id: &str, item_ids: &[String], actor: &str, today: NaiveDate) -> Result<PmCycle> {
    let c = get(pool, id, today).await?;
    if c.status == CycleStatus::Completed {
        return Err(invalid(format!("“{}” is completed — work goes into a cycle that isn't", c.name)));
    }
    for item in item_ids {
        let it = items::get(pool, item).await?;
        if it.project_id != c.project_id {
            return Err(invalid(format!("{} is another project's", it.key)));
        }
        if it.archived_at.is_some() {
            return Err(invalid(format!("{} is archived — restore it first", it.key)));
        }
        // A request is planned once it's accepted (D-081).
        if it.state_group == super::settings::StateGroup::Triage {
            return Err(invalid(format!("{} waits in Intake — accept it first", it.key)));
        }
    }
    let mut tx = pool.begin().await?;
    for item in item_ids {
        set_cycle_in(&mut tx, item, Some(&c), actor).await?;
    }
    tx.commit().await?;
    get(pool, id, today).await
}

/// An item into a cycle, or out of its own (none) — with its activity; nothing when unchanged. In
/// the caller's transaction (an item's own edit, a cycle's whole move).
pub(crate) async fn set_cycle_in(tx: &mut Transaction<'_, Sqlite>, item: &str, to: Option<&PmCycle>, actor: &str) -> Result<()> {
    let now: Option<(String, String)> = sqlx::query_as("SELECT c.id, c.name FROM pm_cycle_item x JOIN pm_cycle c ON c.id = x.cycle_id WHERE x.item_id = ?").bind(item).fetch_optional(&mut **tx).await?;
    if now.as_ref().map(|(i, _)| i.as_str()) == to.map(|c| c.id.as_str()) {
        return Ok(());
    }
    match to {
        Some(c) => {
            sqlx::query(sqlx::AssertSqlSafe(format!("INSERT INTO pm_cycle_item (item_id, cycle_id) VALUES (?, ?) ON CONFLICT (item_id) DO UPDATE SET cycle_id = excluded.cycle_id, updated_at = {NOW}")))
                .bind(item)
                .bind(&c.id)
                .execute(&mut **tx)
                .await?;
        }
        None => {
            sqlx::query("DELETE FROM pm_cycle_item WHERE item_id = ?").bind(item).execute(&mut **tx).await?;
        }
    }
    let old = now.map(|(_, n)| n).unwrap_or_default();
    let new = to.map(|c| c.name.clone()).unwrap_or_default();
    items::activity(tx, item, "updated", "cycle", &old, &new, (None, None), actor).await?;
    Ok(())
}

/// Moves a completed cycle's open work (not completed, not cancelled) into a cycle of the project
/// that isn't completed — once, in one transaction; the cycle keeps its numbers and its details
/// as they stood.
pub async fn transfer(pool: &SqlitePool, id: &str, to: &str, actor: &str, today: NaiveDate) -> Result<u32> {
    let from = get(pool, id, today).await?;
    if from.status != CycleStatus::Completed {
        return Err(invalid("Only a completed cycle's work moves on"));
    }
    if from.transferred_at.is_some() {
        return Err(invalid(format!("“{}”'s open work has moved on already", from.name)));
    }
    let target = get(pool, to, today).await?;
    if target.project_id != from.project_id || target.status == CycleStatus::Completed {
        return Err(invalid("Open work moves to a cycle of the same project that isn't completed"));
    }
    let open: Vec<(String,)> = sqlx::query_as(
        "SELECT x.item_id FROM pm_cycle_item x JOIN pm_item i ON i.id = x.item_id JOIN pm_state s ON s.id = i.state_id \
         WHERE x.cycle_id = ? AND i.archived_at IS NULL AND s.state_group NOT IN ('completed', 'cancelled') ORDER BY i.sequence",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    let kept = serde_json::to_string(&Snapshot { items: from.items, done: from.done, cancelled: from.cancelled, details: Some(details(pool, id, today).await?) })?;
    let mut tx = pool.begin().await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE pm_cycle SET progress_json = ?, transferred_at = {NOW}, updated_at = {NOW} WHERE id = ?"))).bind(&kept).bind(id).execute(&mut *tx).await?;
    for (item,) in &open {
        set_cycle_in(&mut tx, item, Some(&target), actor).await?;
    }
    tx.commit().await?;
    Ok(open.len() as u32)
}

/// Archived (completed only) or back.
pub async fn archive(pool: &SqlitePool, id: &str, on: bool, today: NaiveDate) -> Result<PmCycle> {
    let c = get(pool, id, today).await?;
    if on && c.status != CycleStatus::Completed {
        return Err(invalid("Only a completed cycle can be archived"));
    }
    let sql = if on { format!("UPDATE pm_cycle SET archived_at = {NOW}, updated_at = {NOW} WHERE id = ?") } else { format!("UPDATE pm_cycle SET archived_at = NULL, updated_at = {NOW} WHERE id = ?") };
    sqlx::query(sqlx::AssertSqlSafe(sql)).bind(id).execute(pool).await?;
    // Archiving unstars (Plane's).
    if on {
        super::views::pin(pool, super::views::Pin::Cycle, id, false).await?;
    }
    if on != c.archived_at.is_some() {
        super::events::emit(pool, "cycle", "updated", id, "me");
    }
    get(pool, id, today).await
}

/// Deleted; its work items stay, in no cycle — each item's activity says so.
pub async fn delete(pool: &SqlitePool, id: &str, actor: &str, today: NaiveDate) -> Result<()> {
    let c = get(pool, id, today).await?;
    let held: Vec<(String,)> = sqlx::query_as("SELECT item_id FROM pm_cycle_item WHERE cycle_id = ?").bind(id).fetch_all(pool).await?;
    let mut tx = pool.begin().await?;
    for (item,) in &held {
        items::activity(&mut tx, item, "updated", "cycle", &c.name, "", (None, None), actor).await?;
    }
    sqlx::query("DELETE FROM pm_cycle WHERE id = ?").bind(id).execute(&mut *tx).await?;
    tx.commit().await?;
    super::events::emit(pool, "cycle", "deleted", id, actor);
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BurnDay {
    pub day: String,
    /// The ideal line: all the scope at the start, none at the end.
    pub ideal: f64,
    /// What was still open at the day's end (none for days to come).
    pub remaining: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Share {
    pub key: String,
    pub label: String,
    pub total: u32,
    pub done: u32,
}

/// The details panel: the burn-down (work items, or points when the project's active estimate is
/// in points — an item without one weighs nothing) and the breakdowns by assignee, state and label.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CycleDetails {
    pub points: bool,
    pub burndown: Vec<BurnDay>,
    pub assignees: Vec<Share>,
    pub states: Vec<Share>,
    pub labels: Vec<Share>,
}

pub async fn details(pool: &SqlitePool, id: &str, today: NaiveDate) -> Result<CycleDetails> {
    let c = get(pool, id, today).await?;
    // A transferred cycle: its details as they stood when its open work moved on.
    let kept: Option<(Option<String>,)> = sqlx::query_as("SELECT progress_json FROM pm_cycle WHERE id = ?").bind(id).fetch_optional(pool).await?;
    if let Some(d) = kept.and_then(|(j,)| j).and_then(|j| serde_json::from_str::<Snapshot>(&j).ok()).and_then(|k| k.details) {
        return Ok(d);
    }
    // The cycle's items: when done (this Mac's day), their weight (1, or the estimate's number).
    let rows: Vec<(String, Option<String>, Option<String>, String)> = sqlx::query_as(
        "SELECT i.id, CASE WHEN s.state_group = 'completed' THEN date(i.completed_at, 'localtime') END, ep.value, s.state_group \
         FROM pm_cycle_item x JOIN pm_item i ON i.id = x.item_id JOIN pm_state s ON s.id = i.state_id \
         LEFT JOIN pm_estimate_point ep ON ep.id = i.estimate_point_id WHERE x.cycle_id = ? AND i.archived_at IS NULL",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    let numbers: Vec<Option<f64>> = rows.iter().map(|(_, _, v, _)| v.as_deref().and_then(|v| v.trim().parse::<f64>().ok())).collect();
    let kind: Option<(String,)> = sqlx::query_as("SELECT e.kind FROM pm_project m JOIN pm_estimate e ON e.id = m.estimate_id WHERE m.project_id = ?").bind(&c.project_id).fetch_optional(pool).await?;
    let points = kind.is_some_and(|(k,)| k == "points");
    let scope: Vec<(Option<NaiveDate>, f64)> = rows
        .iter()
        .zip(&numbers)
        .filter(|((_, _, _, g), _)| g != "cancelled")
        .map(|((_, done, _, _), n)| (done.as_deref().and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()), if points { n.unwrap_or(0.0) } else { 1.0 }))
        .collect();
    let total: f64 = scope.iter().map(|(_, w)| w).sum();
    let mut burndown = Vec::new();
    if let (Some(s), Some(e)) = (c.start_date.as_deref(), c.end_date.as_deref()) {
        let (s, e) = (NaiveDate::parse_from_str(s, "%Y-%m-%d")?, NaiveDate::parse_from_str(e, "%Y-%m-%d")?);
        let days = (e - s).num_days().max(0);
        for n in 0..=days {
            let day = s + Duration::days(n);
            let remaining = (day <= today).then(|| scope.iter().filter(|(done, _)| done.is_none_or(|d| d > day)).map(|(_, w)| w).sum());
            let ideal = if days == 0 { 0.0 } else { total * (1.0 - n as f64 / days as f64) };
            burndown.push(BurnDay { day: day.to_string(), ideal, remaining });
        }
    }
    let read = |sql: String| async move {
        let rows: Vec<(String, String, u32, u32)> = sqlx::query_as(sqlx::AssertSqlSafe(sql)).bind(id).fetch_all(pool).await?;
        Ok::<_, anyhow::Error>(rows.into_iter().map(|(key, label, total, done)| Share { key, label, total, done }).collect::<Vec<_>>())
    };
    let done = "sum(s.state_group = 'completed')";
    let assignees = read(format!(
        "SELECT coalesce(a.who, 'none'), CASE WHEN a.who IS NULL THEN 'No assignee' WHEN a.who = 'me' THEN 'You' ELSE coalesce(trim(p.name_first || ' ' || coalesce(p.name_last, '')), 'Someone') END, count(*), {done} \
         FROM pm_cycle_item x JOIN pm_item i ON i.id = x.item_id JOIN pm_state s ON s.id = i.state_id \
         LEFT JOIN pm_item_assignee a ON a.item_id = i.id LEFT JOIN person p ON p.id = a.who WHERE x.cycle_id = ? AND i.archived_at IS NULL GROUP BY 1 ORDER BY 3 DESC"
    ))
    .await?;
    let states = read(format!(
        "SELECT s.id, s.name, count(*), {done} FROM pm_cycle_item x JOIN pm_item i ON i.id = x.item_id JOIN pm_state s ON s.id = i.state_id \
         WHERE x.cycle_id = ? AND i.archived_at IS NULL GROUP BY s.id ORDER BY s.sequence"
    ))
    .await?;
    let labels = read(format!(
        "SELECT coalesce(l.id, 'none'), coalesce(l.name, 'No label'), count(*), {done} FROM pm_cycle_item x JOIN pm_item i ON i.id = x.item_id JOIN pm_state s ON s.id = i.state_id \
         LEFT JOIN pm_item_label il ON il.item_id = i.id LEFT JOIN pm_label l ON l.id = il.label_id WHERE x.cycle_id = ? AND i.archived_at IS NULL GROUP BY 1 ORDER BY 3 DESC"
    ))
    .await?;
    Ok(CycleDetails { points, burndown, assignees, states, labels })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::hub::WorkspaceHub;
    use crate::modules::pm::filters::PmRule;
    use crate::modules::pm::items::{ItemInput, ItemPatch};
    use crate::modules::pm::{self, Features, NewPmProject, Network, settings};

    #[test]
    fn a_two_week_cycle_burns_down_and_moves_its_open_work_on_once() {
        let root = std::env::temp_dir().join(format!("newdawn-pm-cycles-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let new = |name: &str, id: &str| NewPmProject { name: name.into(), identifier: id.into(), description: String::new(), emoji: String::new(), cover: String::new(), network: Network::Private, lead: None, features: Features { cycles: true, ..Features::default() } };
            let p = pm::create(&pool, new("VIFC Fintech Hub", "VIFC")).await.unwrap();
            let other = pm::create(&pool, new("Sandbox", "SB")).await.unwrap();
            let done = settings::states(&pool, &p.id).await.unwrap().into_iter().find(|s| s.name == "Done").unwrap().id;
            let today = today();
            let day = |n: i64| (today + Duration::days(n)).to_string();
            let input = |name: &str, s: Option<String>, e: Option<String>| CycleInput { project_id: p.id.clone(), name: name.into(), description: String::new(), start_date: s, end_date: e };

            // Two weeks, this one in the middle; the next right after; a draft; no overlap.
            let a = create(&pool, input("Sprint 1", Some(day(-7)), Some(day(6))), "me", today).await.unwrap();
            assert_eq!(a.status, CycleStatus::Active);
            let b = create(&pool, input("Sprint 2", Some(day(7)), Some(day(20))), "me", today).await.unwrap();
            assert_eq!(b.status, CycleStatus::Upcoming);
            assert!(create(&pool, input("Overlap", Some(day(5)), Some(day(9))), "me", today).await.unwrap_err().to_string().contains("overlap"));
            assert!(create(&pool, input("Half", Some(day(1)), None), "me", today).await.is_err(), "both dates or none");
            assert_eq!(create(&pool, input("Later", None, None), "me", today).await.unwrap().status, CycleStatus::Draft);

            // Five items in it; two Done.
            let mut five = Vec::new();
            for n in 1..=5 {
                five.push(items::create(&pool, ItemInput { project_id: p.id.clone(), name: format!("Task {n}"), ..ItemInput::default() }, "me").await.unwrap().id);
            }
            assert_eq!(add_items(&pool, &a.id, &five, "me", today).await.unwrap().items, 5);
            let elsewhere = items::create(&pool, ItemInput { project_id: other.id.clone(), name: "Not here".into(), ..ItemInput::default() }, "me").await.unwrap();
            assert!(add_items(&pool, &a.id, &[elsewhere.id], "me", today).await.is_err(), "another project's item");
            for id in &five[..2] {
                items::update(&pool, id, ItemPatch { state_id: Some(done.clone()), ..ItemPatch::default() }, "me").await.unwrap();
            }
            assert_eq!(items::get(&pool, &five[0]).await.unwrap().cycle_name.as_deref(), Some("Sprint 1"));
            let d = details(&pool, &a.id, today).await.unwrap();
            assert_eq!(d.burndown.len(), 14);
            assert_eq!((d.burndown[0].ideal, d.burndown[13].ideal), (5.0, 0.0));
            // The cycle began seven days ago: today is its eighth day.
            assert_eq!(d.burndown[7].remaining, Some(3.0), "today: two of five done");
            assert_eq!(d.burndown[6].remaining, Some(5.0), "yesterday: none done yet");
            assert_eq!(d.burndown[8].remaining, None, "days to come");
            assert_eq!(d.states.iter().find(|s| s.label == "Done").map(|s| (s.total, s.done)), Some((2, 2)));
            assert_eq!(d.assignees.iter().map(|s| s.total).sum::<u32>(), 5);
            // The cycle filter.
            let rule = PmRule { field: "cycle".into(), op: "in".into(), values: vec![a.id.clone()] };
            assert_eq!(items::query(&pool, Some(&p.id), &[rule], today, crate::kernel::formats::WeekStart::Monday).await.unwrap().len(), 5);

            // Completed now: the three open items move to Sprint 2 — once.
            assert!(transfer(&pool, &a.id, &b.id, "me", today).await.is_err(), "not completed yet");
            assert_eq!(complete_now(&pool, &a.id, today).await.unwrap().status, CycleStatus::Completed);
            assert_eq!(update(&pool, &a.id, CyclePatch { name: Some("Sprint one".into()), start_date: a.start_date.clone(), end_date: Some(today.to_string()), ..CyclePatch::default() }, "me", today).await.unwrap().name, "Sprint one", "a completed cycle is renamed, its dates as they are");
            let six = items::create(&pool, ItemInput { project_id: p.id.clone(), name: "Task 6".into(), ..ItemInput::default() }, "me").await.unwrap();
            assert!(items::update(&pool, &six.id, ItemPatch { cycle_id: Some(a.id.clone()), ..ItemPatch::default() }, "me").await.is_err(), "a completed cycle takes nothing");
            assert_eq!(transfer(&pool, &a.id, &b.id, "me", today).await.unwrap(), 3);
            assert_eq!(get(&pool, &b.id, today).await.unwrap().items, 3);
            assert!(transfer(&pool, &a.id, &b.id, "me", today).await.unwrap_err().to_string().contains("already"));
            let moved = items::activities(&pool, &five[4]).await.unwrap();
            assert_eq!(moved.last().map(|x| (x.field.as_str(), x.old_value.as_str(), x.new_value.as_str())), Some(("cycle", "Sprint one", "Sprint 2")));

            // Archive (completed only); delete leaves the items, in no cycle; out of a cycle by "".
            assert!(archive(&pool, &b.id, true, today).await.is_err());
            assert!(archive(&pool, &a.id, true, today).await.unwrap().archived_at.is_some());
            items::update(&pool, &five[2], ItemPatch { cycle_id: Some(String::new()), ..ItemPatch::default() }, "me").await.unwrap();
            assert_eq!(get(&pool, &b.id, today).await.unwrap().items, 2);
            delete(&pool, &b.id, "me", today).await.unwrap();
            assert!(items::get(&pool, &five[3]).await.unwrap().cycle_id.is_none());
            assert_eq!(items::activities(&pool, &five[3]).await.unwrap().last().map(|x| (x.field.as_str(), x.old_value.as_str())), Some(("cycle", "Sprint 2")));
            // The transferred cycle keeps its numbers as they stood: five, two done.
            let kept = get(&pool, &a.id, today).await.unwrap();
            assert_eq!((kept.items, kept.done), (5, 2));
            // Completed early, it's out of the way: a cycle can start today.
            assert!(create(&pool, input("Sprint 3", Some(day(0)), Some(day(13))), "me", today).await.is_ok());
            assert_eq!(active(&pool, today).await.unwrap().iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["Sprint 3"], "the workspace's active cycles");
            hub.close_all().await;
        });
        let _ = std::fs::remove_dir_all(root);
    }
}
