//! A project's states, labels and estimates (D-073, Plane's project settings).
//! States sit in five fixed groups, none ever empty, one of them the default; labels nest one
//! level (a label group); estimates are points or categories, up to six values, one active.

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::error::AppError;
use unicode_normalization::UnicodeNormalization;

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

/// Twenty's option colours, which the app draws from its palette in light and dark (D-073):
/// states and labels keep a name, never a hex value.
pub const COLORS: [&str; 10] = ["green", "turquoise", "sky", "blue", "purple", "pink", "red", "orange", "yellow", "gray"];

fn clean_color(text: &str) -> Result<String> {
    let c = text.trim().to_lowercase();
    if COLORS.contains(&c.as_str()) {
        Ok(c)
    } else {
        Err(invalid(format!("A colour is one of {}", COLORS.join(", "))))
    }
}

/// A name as kept: one line, precomposed (NFC) so "Đã xong" is stored one way.
fn clean_name(text: &str, what: &str) -> Result<String> {
    let name = text.split_whitespace().collect::<Vec<_>>().join(" ").nfc().collect::<String>();
    if name.is_empty() || name.chars().count() > 80 {
        return Err(invalid(format!("A {what} needs a name of at most 80 characters")));
    }
    Ok(name)
}

/// How two names compare: in full, any case (accents count: "Bán" isn't "Bàn").
fn same(a: &str, b: &str) -> bool {
    a.nfc().collect::<String>().to_lowercase() == b.nfc().collect::<String>().to_lowercase()
}

fn id() -> String {
    uuid::Uuid::now_v7().to_string()
}

// ── States ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum StateGroup {
    Backlog,
    Unstarted,
    Started,
    Completed,
    Cancelled,
    /// Intake's (0.11.8): not shown among the project's states.
    Triage,
}

impl StateGroup {
    fn order(self) -> u8 {
        match self {
            StateGroup::Triage => 0,
            StateGroup::Backlog => 1,
            StateGroup::Unstarted => 2,
            StateGroup::Started => 3,
            StateGroup::Completed => 4,
            StateGroup::Cancelled => 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct PmState {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub color: String,
    #[sqlx(rename = "state_group")]
    pub group: StateGroup,
    pub sequence: f64,
    pub is_default: bool,
}

/// Plane's five to start with: Backlog (the default), Todo, In Progress, Done, Cancelled.
pub async fn add_default_states(tx: &mut Transaction<'_, Sqlite>, project_id: &str) -> Result<()> {
    let states = [
        ("Backlog", "gray", "backlog", 15_000.0, true),
        ("Todo", "sky", "unstarted", 25_000.0, false),
        ("In Progress", "yellow", "started", 35_000.0, false),
        ("Done", "green", "completed", 45_000.0, false),
        ("Cancelled", "gray", "cancelled", 55_000.0, false),
    ];
    for (name, color, group, sequence, is_default) in states {
        sqlx::query("INSERT INTO pm_state (id, project_id, name, color, state_group, sequence, is_default) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(id())
            .bind(project_id)
            .bind(name)
            .bind(color)
            .bind(group)
            .bind(sequence)
            .bind(is_default)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

/// A project's states, group by group (Backlog … Cancelled), each group in its order.
pub async fn states(pool: &SqlitePool, project_id: &str) -> Result<Vec<PmState>> {
    let mut rows: Vec<PmState> = sqlx::query_as("SELECT id, project_id, name, description, color, state_group, sequence, is_default FROM pm_state WHERE project_id = ? ORDER BY sequence").bind(project_id).fetch_all(pool).await?;
    rows.sort_by(|a, b| a.group.order().cmp(&b.group.order()).then(a.sequence.total_cmp(&b.sequence)));
    Ok(rows)
}

async fn state(pool: &SqlitePool, id: &str) -> Result<PmState> {
    let row: Option<PmState> = sqlx::query_as("SELECT id, project_id, name, description, color, state_group, sequence, is_default FROM pm_state WHERE id = ?").bind(id).fetch_optional(pool).await?;
    row.ok_or_else(|| anyhow!(AppError::not_found("That state is gone")))
}

async fn state_name_taken(pool: &SqlitePool, project_id: &str, name: &str, except: Option<&str>) -> Result<bool> {
    let all = states(pool, project_id).await?;
    Ok(all.iter().any(|s| Some(s.id.as_str()) != except && same(&s.name, name)))
}

#[derive(Debug, Clone, Deserialize, Type)]
pub struct StateInput {
    pub name: String,
    pub description: String,
    pub color: String,
    pub group: StateGroup,
}

pub async fn create_state(pool: &SqlitePool, project_id: &str, input: StateInput) -> Result<Vec<PmState>> {
    let name = clean_name(&input.name, "state")?;
    if input.group == StateGroup::Triage {
        return Err(invalid("Intake keeps its own triage state"));
    }
    if state_name_taken(pool, project_id, &name, None).await? {
        return Err(invalid(format!("The project already has a state named “{name}”")));
    }
    // Last in its group.
    let (last,): (Option<f64>,) = sqlx::query_as("SELECT max(sequence) FROM pm_state WHERE project_id = ? AND state_group = ?").bind(project_id).bind(input.group).fetch_one(pool).await?;
    sqlx::query("INSERT INTO pm_state (id, project_id, name, description, color, state_group, sequence) VALUES (?, ?, ?, ?, ?, ?, ?)")
        .bind(id())
        .bind(project_id)
        .bind(&name)
        .bind(input.description.trim())
        .bind(clean_color(&input.color)?)
        .bind(input.group)
        .bind(last.map_or(65_535.0, |l| l + 1_000.0))
        .execute(pool)
        .await?;
    states(pool, project_id).await
}

#[derive(Debug, Clone, Default, Deserialize, Type)]
pub struct StatePatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
}

pub async fn update_state(pool: &SqlitePool, id: &str, patch: StatePatch) -> Result<Vec<PmState>> {
    let mut s = state(pool, id).await?;
    if let Some(name) = patch.name {
        s.name = clean_name(&name, "state")?;
        if state_name_taken(pool, &s.project_id, &s.name, Some(id)).await? {
            return Err(invalid(format!("The project already has a state named “{}”", s.name)));
        }
    }
    if let Some(d) = patch.description {
        s.description = d.trim().to_owned();
    }
    if let Some(c) = patch.color {
        s.color = clean_color(&c)?;
    }
    sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE pm_state SET name = ?, description = ?, color = ?, updated_at = {NOW} WHERE id = ?"))).bind(&s.name).bind(&s.description).bind(&s.color).bind(id).execute(pool).await?;
    states(pool, &s.project_id).await
}

/// The default state is where new items start; exactly one a project.
pub async fn set_default_state(pool: &SqlitePool, id: &str) -> Result<Vec<PmState>> {
    let s = state(pool, id).await?;
    if s.group == StateGroup::Triage {
        return Err(invalid("Intake's triage state can't be the default"));
    }
    // Clear, then set: one default a project (a unique index), row order aside.
    let mut tx = pool.begin().await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE pm_state SET is_default = 0, updated_at = {NOW} WHERE project_id = ? AND is_default = 1"))).bind(&s.project_id).execute(&mut *tx).await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE pm_state SET is_default = 1, updated_at = {NOW} WHERE id = ?"))).bind(id).execute(&mut *tx).await?;
    tx.commit().await?;
    states(pool, &s.project_id).await
}

/// A state goes unless it is the default or the last of its group (every group keeps one).
pub async fn delete_state(pool: &SqlitePool, id: &str) -> Result<Vec<PmState>> {
    let s = state(pool, id).await?;
    if s.is_default {
        return Err(invalid("The default state can't be deleted — make another one the default first"));
    }
    let (used,): (i64,) = sqlx::query_as("SELECT count(*) FROM pm_item WHERE state_id = ?").bind(id).fetch_one(pool).await?;
    if used > 0 {
        return Err(invalid(format!("{used} work item{} {} in “{}” — move them to another state first", if used == 1 { "" } else { "s" }, if used == 1 { "is" } else { "are" }, s.name)));
    }
    let all = states(pool, &s.project_id).await?;
    if s.group != StateGroup::Triage && all.iter().filter(|o| o.group == s.group).count() <= 1 {
        return Err(invalid(format!("“{}” is the last state of its group — each group keeps one", s.name)));
    }
    // An automation that closed into it falls back to the first cancelled state (the key's SET NULL).
    sqlx::query("DELETE FROM pm_state WHERE id = ?").bind(id).execute(pool).await?;
    states(pool, &s.project_id).await
}

/// Moves a state up or down within its group.
pub async fn move_state(pool: &SqlitePool, id: &str, up: bool) -> Result<Vec<PmState>> {
    let s = state(pool, id).await?;
    let group: Vec<PmState> = states(pool, &s.project_id).await?.into_iter().filter(|o| o.group == s.group).collect();
    let at = group.iter().position(|o| o.id == s.id).unwrap_or(0);
    let other = if up { at.checked_sub(1) } else { Some(at + 1).filter(|&i| i < group.len()) };
    if let Some(other) = other.map(|i| &group[i]) {
        let mut tx = pool.begin().await?;
        sqlx::query("UPDATE pm_state SET sequence = ? WHERE id = ?").bind(other.sequence).bind(&s.id).execute(&mut *tx).await?;
        sqlx::query("UPDATE pm_state SET sequence = ? WHERE id = ?").bind(s.sequence).bind(&other.id).execute(&mut *tx).await?;
        tx.commit().await?;
    }
    states(pool, &s.project_id).await
}

// ── Labels ──

#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct PmLabel {
    pub id: String,
    pub project_id: String,
    /// The label group it is in.
    pub parent_id: Option<String>,
    pub name: String,
    pub description: String,
    pub color: String,
    pub sort_order: f64,
}

/// A project's labels: each group followed by its labels, in their order.
pub async fn labels(pool: &SqlitePool, project_id: &str) -> Result<Vec<PmLabel>> {
    Ok(sqlx::query_as("SELECT id, project_id, parent_id, name, description, color, sort_order FROM pm_label WHERE project_id = ? ORDER BY sort_order, name COLLATE NOCASE").bind(project_id).fetch_all(pool).await?)
}

#[derive(Debug, Clone, Deserialize, Type)]
pub struct LabelInput {
    pub name: String,
    pub description: String,
    pub color: String,
    pub parent_id: Option<String>,
}

/// The group a label may go in: a label of the same project that is in no group itself.
async fn check_parent(pool: &SqlitePool, project_id: &str, parent: Option<&str>, me: Option<&str>) -> Result<Option<String>> {
    let Some(parent) = parent.filter(|p| !p.is_empty()) else { return Ok(None) };
    let all = labels(pool, project_id).await?;
    let ok = all.iter().any(|l| l.id == parent && l.parent_id.is_none() && Some(l.id.as_str()) != me);
    let has_children = me.is_some_and(|me| all.iter().any(|l| l.parent_id.as_deref() == Some(me)));
    if !ok || has_children {
        return Err(invalid("A label goes in a group of this project, one level deep"));
    }
    Ok(Some(parent.to_owned()))
}

pub async fn create_label(pool: &SqlitePool, project_id: &str, input: LabelInput) -> Result<Vec<PmLabel>> {
    let name = clean_name(&input.name, "label")?;
    let all = labels(pool, project_id).await?;
    if all.iter().any(|l| same(&l.name, &name)) {
        return Err(invalid(format!("The project already has a label named “{name}”")));
    }
    let parent = check_parent(pool, project_id, input.parent_id.as_deref(), None).await?;
    let last = all.iter().map(|l| l.sort_order).fold(0.0_f64, f64::max);
    sqlx::query("INSERT INTO pm_label (id, project_id, parent_id, name, description, color, sort_order) VALUES (?, ?, ?, ?, ?, ?, ?)")
        .bind(id())
        .bind(project_id)
        .bind(parent)
        .bind(&name)
        .bind(input.description.trim())
        .bind(clean_color(&input.color)?)
        .bind(last + 1_000.0)
        .execute(pool)
        .await?;
    labels(pool, project_id).await
}

pub async fn update_label(pool: &SqlitePool, id: &str, input: LabelInput) -> Result<Vec<PmLabel>> {
    let current: Option<(String,)> = sqlx::query_as("SELECT project_id FROM pm_label WHERE id = ?").bind(id).fetch_optional(pool).await?;
    let (project_id,) = current.ok_or_else(|| anyhow!(AppError::not_found("That label is gone")))?;
    let name = clean_name(&input.name, "label")?;
    if labels(pool, &project_id).await?.iter().any(|l| l.id != id && same(&l.name, &name)) {
        return Err(invalid(format!("The project already has a label named “{name}”")));
    }
    let parent = check_parent(pool, &project_id, input.parent_id.as_deref(), Some(id)).await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE pm_label SET name = ?, description = ?, color = ?, parent_id = ?, updated_at = {NOW} WHERE id = ?")))
        .bind(&name)
        .bind(input.description.trim())
        .bind(clean_color(&input.color)?)
        .bind(parent)
        .bind(id)
        .execute(pool)
        .await?;
    labels(pool, &project_id).await
}

/// Deletes a label (a group with its labels).
pub async fn delete_label(pool: &SqlitePool, id: &str) -> Result<Vec<PmLabel>> {
    let current: Option<(String,)> = sqlx::query_as("SELECT project_id FROM pm_label WHERE id = ?").bind(id).fetch_optional(pool).await?;
    let (project_id,) = current.ok_or_else(|| anyhow!(AppError::not_found("That label is gone")))?;
    sqlx::query("DELETE FROM pm_label WHERE id = ?").bind(id).execute(pool).await?;
    labels(pool, &project_id).await
}

// ── Estimates ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "snake_case")]
pub enum EstimateKind {
    Points,
    Categories,
}

#[derive(Debug, Clone, Serialize, Type, sqlx::FromRow)]
pub struct EstimatePoint {
    pub id: String,
    /// Its place, 1 to 6.
    pub key: u32,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct PmEstimate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub kind: EstimateKind,
    pub points: Vec<EstimatePoint>,
    /// The project's estimate now.
    pub active: bool,
    pub archived_at: Option<String>,
    pub created_at: String,
}

/// The active estimate first, then the ones it replaced, newest first.
pub async fn estimates(pool: &SqlitePool, project_id: &str) -> Result<Vec<PmEstimate>> {
    #[derive(sqlx::FromRow)]
    struct Row {
        id: String,
        name: String,
        description: String,
        kind: EstimateKind,
        active: bool,
        archived_at: Option<String>,
        created_at: String,
    }
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT e.id, e.name, e.description, e.kind, (e.id = coalesce(m.estimate_id, '')) AS active, e.archived_at, e.created_at \
         FROM pm_estimate e JOIN pm_project m ON m.project_id = e.project_id WHERE e.project_id = ? ORDER BY active DESC, e.created_at DESC",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;
    let mut out = Vec::new();
    for r in rows {
        let points: Vec<EstimatePoint> = sqlx::query_as("SELECT id, key, value FROM pm_estimate_point WHERE estimate_id = ? ORDER BY key").bind(&r.id).fetch_all(pool).await?;
        out.push(PmEstimate { id: r.id, name: r.name, description: r.description, kind: r.kind, points, active: r.active, archived_at: r.archived_at, created_at: r.created_at });
    }
    Ok(out)
}

#[derive(Debug, Clone, Deserialize, Type)]
pub struct EstimateInput {
    pub name: String,
    pub kind: EstimateKind,
    /// In order, 1 to 6 of them; points are numbers.
    pub values: Vec<String>,
}

fn clean_values(kind: EstimateKind, values: &[String]) -> Result<Vec<String>> {
    let kept: Vec<String> = values.iter().map(|v| v.trim().to_owned()).collect();
    if kept.is_empty() || kept.len() > 6 || kept.iter().any(String::is_empty) {
        return Err(invalid("An estimate has 1 to 6 values, none empty"));
    }
    let mut seen = std::collections::HashSet::new();
    if !kept.iter().all(|v| seen.insert(v.nfc().collect::<String>().to_lowercase())) {
        return Err(invalid("An estimate's values are all different"));
    }
    if kind == EstimateKind::Points && !kept.iter().all(|v| v.parse::<f64>().is_ok_and(|n| n.is_finite() && n >= 0.0)) {
        return Err(invalid("Points are numbers (1, 2, 3, 5, 8 …)"));
    }
    if kept.iter().any(|v| v.chars().count() > 20) {
        return Err(invalid("An estimate's value is at most 20 characters"));
    }
    Ok(kept)
}

/// A new estimate system becomes the project's; the one it replaces is archived.
pub async fn create_estimate(pool: &SqlitePool, project_id: &str, input: EstimateInput) -> Result<Vec<PmEstimate>> {
    let name = clean_name(&input.name, "estimate")?;
    let values = clean_values(input.kind, &input.values)?;
    let estimate = id();
    let mut tx = pool.begin().await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE pm_estimate SET archived_at = coalesce(archived_at, {NOW}), updated_at = {NOW} WHERE project_id = ?"))).bind(project_id).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO pm_estimate (id, project_id, name, kind) VALUES (?, ?, ?, ?)").bind(&estimate).bind(project_id).bind(&name).bind(input.kind).execute(&mut *tx).await?;
    for (i, v) in values.iter().enumerate() {
        sqlx::query("INSERT INTO pm_estimate_point (id, estimate_id, key, value) VALUES (?, ?, ?, ?)").bind(id()).bind(&estimate).bind(i as i64 + 1).bind(v).execute(&mut *tx).await?;
    }
    sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE pm_project SET estimate_id = ?, updated_at = {NOW} WHERE project_id = ?"))).bind(&estimate).bind(project_id).execute(&mut *tx).await?;
    // Items sized in the old system aren't sized in the new one.
    sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE pm_item SET estimate_point_id = NULL, updated_at = {NOW} WHERE project_id = ? AND estimate_point_id IS NOT NULL"))).bind(project_id).execute(&mut *tx).await?;
    tx.commit().await?;
    estimates(pool, project_id).await
}

/// The active estimate's values, changed in place (a value keeps its place, so what items carry
/// stays theirs); places past the new last go.
pub async fn update_estimate(pool: &SqlitePool, estimate_id: &str, input: EstimateInput) -> Result<Vec<PmEstimate>> {
    let row: Option<(String, EstimateKind)> = sqlx::query_as("SELECT project_id, kind FROM pm_estimate WHERE id = ? AND archived_at IS NULL").bind(estimate_id).fetch_optional(pool).await?;
    let (project_id, kind) = row.ok_or_else(|| invalid("Only the active estimate can be changed"))?;
    if input.kind != kind {
        return Err(invalid("A new kind of estimate is a new estimate"));
    }
    let name = clean_name(&input.name, "estimate")?;
    let values = clean_values(kind, &input.values)?;
    let mut tx = pool.begin().await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE pm_estimate SET name = ?, updated_at = {NOW} WHERE id = ?"))).bind(&name).bind(estimate_id).execute(&mut *tx).await?;
    for (i, v) in values.iter().enumerate() {
        let key = i as i64 + 1;
        let changed = sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE pm_estimate_point SET value = ?, updated_at = {NOW} WHERE estimate_id = ? AND key = ?"))).bind(v).bind(estimate_id).bind(key).execute(&mut *tx).await?;
        if changed.rows_affected() == 0 {
            sqlx::query("INSERT INTO pm_estimate_point (id, estimate_id, key, value) VALUES (?, ?, ?, ?)").bind(id()).bind(estimate_id).bind(key).bind(v).execute(&mut *tx).await?;
        }
    }
    sqlx::query("DELETE FROM pm_estimate_point WHERE estimate_id = ? AND key > ?").bind(estimate_id).bind(values.len() as i64).execute(&mut *tx).await?;
    tx.commit().await?;
    estimates(pool, &project_id).await
}

/// No estimates for the project: the active one is archived.
pub async fn disable_estimates(pool: &SqlitePool, project_id: &str) -> Result<Vec<PmEstimate>> {
    let mut tx = pool.begin().await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE pm_estimate SET archived_at = coalesce(archived_at, {NOW}), updated_at = {NOW} WHERE project_id = ?"))).bind(project_id).execute(&mut *tx).await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE pm_project SET estimate_id = NULL, updated_at = {NOW} WHERE project_id = ?"))).bind(project_id).execute(&mut *tx).await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE pm_item SET estimate_point_id = NULL, updated_at = {NOW} WHERE project_id = ? AND estimate_point_id IS NOT NULL"))).bind(project_id).execute(&mut *tx).await?;
    tx.commit().await?;
    estimates(pool, project_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::hub::WorkspaceHub;
    use crate::modules::pm::{self, Features, NewPmProject, Network};

    #[test]
    fn states_labels_and_estimates_keep_their_rules() {
        let root = std::env::temp_dir().join(format!("newdawn-pm-settings-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let p = pm::create(&pool, NewPmProject { name: "Hub".into(), identifier: "HUB".into(), description: String::new(), emoji: String::new(), cover: String::new(), network: Network::Public, lead: None, features: Features::default() }).await.unwrap();
            // States: the five, in group order; the last of a group and the default stay.
            let all = states(&pool, &p.id).await.unwrap();
            let names: Vec<&str> = all.iter().map(|s| s.name.as_str()).collect();
            assert_eq!(names, ["Backlog", "Todo", "In Progress", "Done", "Cancelled"]);
            let todo = all[1].id.clone();
            assert!(delete_state(&pool, &todo).await.unwrap_err().to_string().contains("last state of its group"));
            assert!(delete_state(&pool, &all[0].id).await.unwrap_err().to_string().contains("default"));
            let more = create_state(&pool, &p.id, StateInput { name: "Ready".into(), description: "Groomed".into(), color: "Blue".into(), group: StateGroup::Unstarted }).await.unwrap();
            let ready = more.iter().find(|s| s.name == "Ready").unwrap().clone();
            assert_eq!((more[2].name.as_str(), ready.color.as_str()), ("Ready", "blue"), "last of its group, after Todo");
            assert!(create_state(&pool, &p.id, StateInput { name: "ready".into(), description: String::new(), color: "gray".into(), group: StateGroup::Started }).await.is_err(), "names are unique");
            let moved = move_state(&pool, &ready.id, true).await.unwrap();
            assert_eq!((moved[1].name.as_str(), moved[2].name.as_str()), ("Ready", "Todo"));
            set_default_state(&pool, &todo).await.unwrap();
            delete_state(&pool, &all[0].id).await.unwrap_err(); // Backlog is now the last of its group
            let after = delete_state(&pool, &ready.id).await.unwrap();
            assert_eq!(after.iter().filter(|s| s.is_default).map(|s| s.name.as_str()).collect::<Vec<_>>(), ["Todo"]);
            assert!(update_state(&pool, &todo, StatePatch { color: Some("#ff0000".into()), ..StatePatch::default() }).await.is_err(), "a colour is a name of the palette");
            // Labels: one level of groups, unique names.
            let l = create_label(&pool, &p.id, LabelInput { name: "Area".into(), description: String::new(), color: "purple".into(), parent_id: None }).await.unwrap();
            let area = l[0].id.clone();
            let l = create_label(&pool, &p.id, LabelInput { name: "Frontend".into(), description: String::new(), color: "green".into(), parent_id: Some(area.clone()) }).await.unwrap();
            let frontend = l.iter().find(|x| x.name == "Frontend").unwrap().id.clone();
            assert!(create_label(&pool, &p.id, LabelInput { name: "Deep".into(), description: String::new(), color: "green".into(), parent_id: Some(frontend.clone()) }).await.is_err(), "one level");
            assert!(update_label(&pool, &area, LabelInput { name: "Area".into(), description: String::new(), color: "purple".into(), parent_id: Some(frontend) }).await.is_err(), "a group can't go in a group");
            assert!(create_label(&pool, &p.id, LabelInput { name: "frontend".into(), description: String::new(), color: "green".into(), parent_id: None }).await.is_err());
            create_label(&pool, &p.id, LabelInput { name: "Đã xong".into(), description: String::new(), color: "green".into(), parent_id: None }).await.unwrap();
            assert!(create_label(&pool, &p.id, LabelInput { name: "đã xong".into(), description: String::new(), color: "green".into(), parent_id: None }).await.is_err(), "any case, beyond ASCII");
            create_label(&pool, &p.id, LabelInput { name: "Bán".into(), description: String::new(), color: "green".into(), parent_id: None }).await.unwrap();
            assert!(create_label(&pool, &p.id, LabelInput { name: "Bàn".into(), description: String::new(), color: "green".into(), parent_id: None }).await.is_ok(), "accents count");
            assert_eq!(delete_label(&pool, &area).await.unwrap().len(), 3, "a group goes with its labels");
            // Estimates: up to six values, points are numbers, one active, edited in place.
            assert!(create_estimate(&pool, &p.id, EstimateInput { name: "Points".into(), kind: EstimateKind::Points, values: vec!["1".into(), "two".into()] }).await.is_err());
            assert!(create_estimate(&pool, &p.id, EstimateInput { name: "Many".into(), kind: EstimateKind::Categories, values: (0..7).map(|i| format!("v{i}")).collect() }).await.is_err());
            let e = create_estimate(&pool, &p.id, EstimateInput { name: "Fibonacci".into(), kind: EstimateKind::Points, values: ["1", "2", "3", "5", "8", "13"].map(String::from).to_vec() }).await.unwrap();
            let first = e[0].clone();
            assert!(first.active && first.points.len() == 6);
            let e = create_estimate(&pool, &p.id, EstimateInput { name: "T-shirt".into(), kind: EstimateKind::Categories, values: ["XS", "S", "M", "L", "XL"].map(String::from).to_vec() }).await.unwrap();
            assert_eq!((e[0].name.as_str(), e[0].active, e[1].active, e[1].archived_at.is_some()), ("T-shirt", true, false, true), "the older one archived");
            let kept = e[0].points[1].id.clone();
            let e = update_estimate(&pool, &e[0].id, EstimateInput { name: "T-shirt".into(), kind: EstimateKind::Categories, values: ["XS", "Small", "M"].map(String::from).to_vec() }).await.unwrap();
            assert_eq!((e[0].points.len(), e[0].points[1].id.as_str(), e[0].points[1].value.as_str()), (3, kept.as_str(), "Small"), "a value keeps its place");
            assert!(update_estimate(&pool, &first.id, EstimateInput { name: "F".into(), kind: EstimateKind::Points, values: vec!["1".into()] }).await.is_err(), "an archived one stays as it was");
            let e = disable_estimates(&pool, &p.id).await.unwrap();
            assert!(e.iter().all(|x| !x.active) && pm::get(&pool, &p.id).await.unwrap().estimate_id.is_none());
            hub.close_all().await;
        });
        let _ = std::fs::remove_dir_all(root);
    }
}
