//! Workflows (D-068, 0.10.11): Twenty's automations on the record engine. A workflow is a record
//! whose versions hold a trigger and steps as one JSON document; the active version's triggers
//! watch records or a schedule, or wait for a hand (manual — from ⌘K or a record's ⋯); a run
//! keeps its context and the steps done in the database, so it goes on after a quit.

pub mod cron;
pub mod run;
pub mod triggers;

use std::collections::HashSet;
use std::sync::OnceLock;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use specta::Type;
use sqlx::SqlitePool;
use tauri::AppHandle;

use crate::error::AppError;
use crate::kernel::records::types::Json;
use crate::kernel::records::{self, query, Actor};

pub const TRIGGER_TYPES: [&str; 4] = ["RECORD", "MANUAL", "SCHEDULE", "WEBHOOK"];
/// What a workflow never watches nor writes: workflows and their runs (no loops, no fake runs).
pub const OWN_OBJECTS: [&str; 2] = ["workflow", "workflowRun"];
pub const STEP_TYPES: [&str; 14] = [
    "CREATE_RECORD",
    "UPDATE_RECORD",
    "DELETE_RECORD",
    "SEARCH_RECORDS",
    "UPSERT_RECORD",
    "FILTER",
    "IF_ELSE",
    "ITERATOR",
    "DELAY",
    "FORM",
    "AI_AGENT",
    "SEND_EMAIL",
    "HTTP_REQUEST",
    "CODE",
];

/// The app, for the steps that need more than a workspace's pool (the AI step's turn with the
/// local model, D-069); set once when the triggers start.
static APP: OnceLock<AppHandle> = OnceLock::new();

pub fn set_app(app: &AppHandle) {
    let _ = APP.set(app.clone());
}

pub fn app() -> Option<&'static AppHandle> {
    APP.get()
}

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

/// A version as the UI reads it.
#[derive(Debug, Clone, Serialize, Type)]
pub struct Version {
    pub id: String,
    pub workflow_id: String,
    pub name: String,
    /// DRAFT · ACTIVE · DEACTIVATED · ARCHIVED
    pub status: String,
    pub definition: Json,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, sqlx::FromRow)]
struct VersionRow {
    id: String,
    workflow_id: String,
    name: String,
    status: String,
    definition_json: String,
    created_at: String,
    updated_at: String,
}

impl From<VersionRow> for Version {
    fn from(r: VersionRow) -> Self {
        Self { id: r.id, workflow_id: r.workflow_id, name: r.name, status: r.status, definition: Json(serde_json::from_str(&r.definition_json).unwrap_or(json!({}))), created_at: r.created_at, updated_at: r.updated_at }
    }
}

const VERSION_COLUMNS: &str = "id, workflow_id, name, status, definition_json, created_at, updated_at";

/// The document a version holds: one trigger, the steps in order.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Definition {
    #[serde(default)]
    pub trigger: Option<Trigger>,
    #[serde(default)]
    pub steps: Vec<Step>,
}

/// RECORD (an object, an event — CREATED · UPDATED · CREATED_OR_UPDATED · DELETED — and the
/// fields watched), MANUAL (GLOBAL · SINGLE · BULK, an object), SCHEDULE (every N MINUTES ·
/// HOURS · DAYS, or a cron), WEBHOOK (shown; active with the local API).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Trigger {
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub object: String,
    #[serde(default)]
    pub event: String,
    #[serde(default)]
    pub fields: Vec<String>,
    #[serde(default)]
    pub availability: String,
    #[serde(default)]
    pub every: u32,
    #[serde(default)]
    pub unit: String,
    #[serde(default)]
    pub cron: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Step {
    #[serde(default)]
    pub id: String,
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub settings: Value,
    /// IF_ELSE: If, then every Else if.
    #[serde(default)]
    pub branches: Vec<Branch>,
    /// IF_ELSE: Else.
    #[serde(default)]
    pub else_steps: Vec<Step>,
    /// ITERATOR: the steps run once an item.
    #[serde(default)]
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Branch {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub conditions: Vec<Condition>,
    /// AND · OR
    #[serde(default)]
    pub logic: String,
    #[serde(default)]
    pub steps: Vec<Step>,
}

/// `left` and `right` are templates; op: is · isNot · contains · notContains · isEmpty ·
/// isNotEmpty · gt · gte · lt · lte · isBefore · isAfter.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Condition {
    #[serde(default)]
    pub left: String,
    #[serde(default)]
    pub op: String,
    #[serde(default)]
    pub right: String,
}

pub fn parse(definition: &Value) -> Result<Definition> {
    serde_json::from_value(definition.clone()).map_err(|e| invalid(format!("The workflow's definition doesn't read: {e}")))
}

/// A definition that can run (checked at Activate and Test; a draft is saved as it is): a known
/// trigger with what it needs, known step types, ids present and unique, no workflow objects.
pub fn validate(def: &Definition) -> Result<()> {
    if let Some(t) = &def.trigger {
        if !TRIGGER_TYPES.contains(&t.kind.as_str()) {
            return Err(invalid(format!("No trigger named {}", t.kind)));
        }
        if t.kind == "RECORD" && (t.object.is_empty() || t.event.is_empty()) {
            return Err(invalid("A record trigger names an object and an event"));
        }
        if t.kind == "MANUAL" && matches!(t.availability.as_str(), "SINGLE" | "BULK") && t.object.is_empty() {
            return Err(invalid("A workflow run on records names their object"));
        }
        if OWN_OBJECTS.contains(&t.object.as_str()) {
            return Err(invalid("A workflow doesn't watch workflows or their runs"));
        }
        if t.kind == "SCHEDULE" && t.cron.trim().is_empty() && t.every == 0 {
            return Err(invalid("A schedule needs an interval or a cron"));
        }
        if t.kind == "SCHEDULE" && next_time(t, Local::now()).is_none() {
            return Err(invalid("That schedule never comes: the cron reads minute hour day month weekday, the interval within a few centuries"));
        }
    }
    fn walk(steps: &[Step], ids: &mut HashSet<String>) -> Result<()> {
        for s in steps {
            if s.id.trim().is_empty() {
                return Err(invalid("A step needs an id"));
            }
            if !STEP_TYPES.contains(&s.kind.as_str()) {
                return Err(invalid(format!("No step named {}", s.kind)));
            }
            // An AI step's outputs can't take the names its answer already has.
            if s.kind == "AI_AGENT" && s.settings.get("outputs").and_then(Value::as_array).is_some_and(|o| o.iter().any(|x| matches!(x.get("name").and_then(Value::as_str), Some("text" | "model")))) {
                return Err(invalid(format!("{}: an output can't be called text or model", if s.name.is_empty() { &s.id } else { &s.name })));
            }
            // An HTTP step's address, when it's written out, is checked before the run (rule 3).
            if s.kind == "HTTP_REQUEST" {
                if let Some(url) = s.settings.get("url").and_then(Value::as_str).filter(|u| !u.trim().is_empty() && !u.contains("{{")) {
                    run::local_url(url)?;
                }
            }
            if s.settings.get("object").and_then(Value::as_str).is_some_and(|o| OWN_OBJECTS.contains(&o)) {
                return Err(invalid(format!("{}: a step doesn't write workflows or their runs", if s.name.is_empty() { &s.id } else { &s.name })));
            }
            if !ids.insert(s.id.clone()) {
                return Err(invalid(format!("Two steps share the id {}", s.id)));
            }
            for b in &s.branches {
                walk(&b.steps, ids)?;
            }
            walk(&s.else_steps, ids)?;
            walk(&s.steps, ids)?;
        }
        Ok(())
    }
    walk(&def.steps, &mut HashSet::new())
}

/// Every version of a workflow, newest first.
pub async fn versions(pool: &SqlitePool, workflow_id: &str) -> Result<Vec<Version>> {
    let rows: Vec<VersionRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT {VERSION_COLUMNS} FROM workflow_version WHERE workflow_id = ? ORDER BY position DESC"))).bind(workflow_id).fetch_all(pool).await?;
    Ok(rows.into_iter().map(Version::from).collect())
}

async fn version(pool: &SqlitePool, id: &str) -> Result<VersionRow> {
    let row: Option<VersionRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT {VERSION_COLUMNS} FROM workflow_version WHERE id = ?"))).bind(id).fetch_optional(pool).await?;
    row.ok_or_else(|| anyhow!(AppError::not_found("No such workflow version")))
}

pub async fn active_version(pool: &SqlitePool, workflow_id: &str) -> Result<Option<Version>> {
    let row: Option<VersionRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT {VERSION_COLUMNS} FROM workflow_version WHERE workflow_id = ? AND status = 'ACTIVE'"))).bind(workflow_id).fetch_optional(pool).await?;
    Ok(row.map(Version::from))
}

/// The version the builder shows: the draft, else the live one, else the latest — reading never
/// makes a version (the first edit does); a workflow with none gets its first draft.
pub async fn editable(pool: &SqlitePool, workflow_id: &str) -> Result<Version> {
    let row: Option<VersionRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {VERSION_COLUMNS} FROM workflow_version WHERE workflow_id = ? ORDER BY CASE status WHEN 'DRAFT' THEN 0 WHEN 'ACTIVE' THEN 1 ELSE 2 END, position DESC LIMIT 1"
    )))
    .bind(workflow_id)
    .fetch_optional(pool)
    .await?;
    match row {
        Some(r) => Ok(r.into()),
        None => draft(pool, workflow_id).await,
    }
}

/// The draft to edit: the DRAFT, else a new one from the active version (else the latest, else
/// empty).
pub async fn draft(pool: &SqlitePool, workflow_id: &str) -> Result<Version> {
    let existing: Option<VersionRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT {VERSION_COLUMNS} FROM workflow_version WHERE workflow_id = ? AND status = 'DRAFT'"))).bind(workflow_id).fetch_optional(pool).await?;
    if let Some(row) = existing {
        return Ok(row.into());
    }
    let known: Option<(String,)> = sqlx::query_as("SELECT id FROM workflow WHERE id = ? AND deleted_at IS NULL").bind(workflow_id).fetch_optional(pool).await?;
    if known.is_none() {
        return Err(anyhow!(AppError::not_found("No such workflow")));
    }
    let base: Option<(String,)> = sqlx::query_as("SELECT definition_json FROM workflow_version WHERE workflow_id = ? ORDER BY status = 'ACTIVE' DESC, position DESC LIMIT 1").bind(workflow_id).fetch_optional(pool).await?;
    let definition = base.map(|(d,)| d).unwrap_or_else(|| json!({ "trigger": null, "steps": [] }).to_string());
    let (n,): (i64,) = sqlx::query_as("SELECT count(*) FROM workflow_version WHERE workflow_id = ?").bind(workflow_id).fetch_one(pool).await?;
    let id = uuid::Uuid::now_v7().to_string();
    sqlx::query("INSERT INTO workflow_version (id, workflow_id, name, status, definition_json, position) VALUES (?, ?, ?, 'DRAFT', ?, ?)")
        .bind(&id)
        .bind(workflow_id)
        .bind(format!("v{}", n + 1))
        .bind(definition)
        .bind((n + 1) as f64)
        .execute(pool)
        .await?;
    Ok(version(pool, &id).await?.into())
}

/// The builder's save: the draft takes the definition as it is (made first when there is none);
/// Activate and Test check it.
pub async fn save_draft(pool: &SqlitePool, workflow_id: &str, definition: &Value) -> Result<Version> {
    parse(definition)?;
    let d = draft(pool, workflow_id).await?;
    sqlx::query("UPDATE workflow_version SET definition_json = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?").bind(definition.to_string()).bind(&d.id).execute(pool).await?;
    Ok(version(pool, &d.id).await?.into())
}

/// The next time a schedule fires after `from`.
pub fn next_time(t: &Trigger, from: DateTime<Local>) -> Option<DateTime<Local>> {
    if t.cron.trim().is_empty() {
        cron::next_interval(t.every, &t.unit, from)
    } else {
        cron::next_after(&t.cron, from)
    }
}

async fn rebuild_triggers(conn: &mut sqlx::SqliteConnection, workflow_id: &str, version_id: &str, def: &Definition) -> Result<()> {
    sqlx::query("DELETE FROM workflow_automated_trigger WHERE workflow_id = ?").bind(workflow_id).execute(&mut *conn).await?;
    let Some(t) = &def.trigger else { return Ok(()) };
    let (kind, settings, next_at) = match t.kind.as_str() {
        "RECORD" => ("RECORD", json!({ "object": t.object, "event": t.event, "fields": t.fields }), None),
        "SCHEDULE" => ("SCHEDULE", json!({ "every": t.every, "unit": t.unit, "cron": t.cron }), next_time(t, Local::now()).map(|d| d.with_timezone(&chrono::Utc).to_rfc3339_opts(chrono::SecondsFormat::Secs, true))),
        "WEBHOOK" => ("WEBHOOK", json!({}), None),
        _ => return Ok(()),
    };
    sqlx::query("INSERT INTO workflow_automated_trigger (id, workflow_id, version_id, kind, settings_json, next_at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(uuid::Uuid::now_v7().to_string())
        .bind(workflow_id)
        .bind(version_id)
        .bind(kind)
        .bind(settings.to_string())
        .bind(next_at)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// The workflow record's status, through the engine (its search text and timeline follow).
async fn set_workflow_status(pool: &SqlitePool, workflow_id: &str, status: &str) -> Result<()> {
    let obj = records::object(pool, "workflow").await?;
    query::update(pool, &obj, workflow_id, &json!({ "status": status }), &Actor::system("Workflows")).await?;
    Ok(())
}

/// Activate: this version goes live, the one that was goes to the archive, its triggers are
/// set; the workflow reads Active.
pub async fn activate(pool: &SqlitePool, version_id: &str) -> Result<Version> {
    let v = version(pool, version_id).await?;
    let def = parse(&serde_json::from_str(&v.definition_json)?)?;
    validate(&def)?;
    if def.trigger.is_none() {
        return Err(invalid("A workflow needs a trigger before it goes live"));
    }
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE workflow_version SET status = 'ARCHIVED', updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE workflow_id = ? AND status = 'ACTIVE' AND id <> ?").bind(&v.workflow_id).bind(version_id).execute(&mut *tx).await?;
    sqlx::query("UPDATE workflow_version SET status = 'ACTIVE', updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?").bind(version_id).execute(&mut *tx).await?;
    rebuild_triggers(&mut tx, &v.workflow_id, version_id, &def).await?;
    tx.commit().await?;
    set_workflow_status(pool, &v.workflow_id, "ACTIVE").await?;
    Ok(version(pool, version_id).await?.into())
}

/// Deactivate: the version stops answering; its triggers go; the workflow reads Deactivated.
pub async fn deactivate(pool: &SqlitePool, version_id: &str) -> Result<Version> {
    let v = version(pool, version_id).await?;
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE workflow_version SET status = 'DEACTIVATED', updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?").bind(version_id).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM workflow_automated_trigger WHERE version_id = ?").bind(version_id).execute(&mut *tx).await?;
    tx.commit().await?;
    if active_version(pool, &v.workflow_id).await?.is_none() {
        set_workflow_status(pool, &v.workflow_id, "DEACTIVATED").await?;
    }
    Ok(version(pool, version_id).await?.into())
}

pub async fn discard_draft(pool: &SqlitePool, version_id: &str) -> Result<()> {
    sqlx::query("DELETE FROM workflow_version WHERE id = ? AND status = 'DRAFT'").bind(version_id).execute(pool).await?;
    Ok(())
}

/// "Use as draft": a past version's definition becomes the draft.
pub async fn use_as_draft(pool: &SqlitePool, version_id: &str) -> Result<Version> {
    let v = version(pool, version_id).await?;
    let d = draft(pool, &v.workflow_id).await?;
    sqlx::query("UPDATE workflow_version SET definition_json = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?").bind(&v.definition_json).bind(&d.id).execute(pool).await?;
    Ok(version(pool, &d.id).await?.into())
}

/// A workflow the owner runs by hand, and from where.
#[derive(Debug, Clone, Serialize, Type)]
pub struct ManualWorkflow {
    pub workflow_id: String,
    pub name: String,
    /// GLOBAL · SINGLE · BULK
    pub availability: String,
    /// The object its records are (SINGLE · BULK), or empty.
    pub object: String,
}

/// The active manual workflows: all of them, or those that take a record of the object.
pub async fn manual(pool: &SqlitePool, object: Option<&str>) -> Result<Vec<ManualWorkflow>> {
    let rows: Vec<(String, String, String)> = sqlx::query_as("SELECT w.id, w.name, v.definition_json FROM workflow w JOIN workflow_version v ON v.workflow_id = w.id AND v.status = 'ACTIVE' WHERE w.deleted_at IS NULL ORDER BY w.name")
        .fetch_all(pool)
        .await?;
    let mut out = Vec::new();
    for (workflow_id, name, definition) in rows {
        let Ok(def) = parse(&serde_json::from_str(&definition).unwrap_or(Value::Null)) else { continue };
        let Some(t) = def.trigger.filter(|t| t.kind == "MANUAL") else { continue };
        let availability = if t.availability.is_empty() { "GLOBAL".to_owned() } else { t.availability.clone() };
        let fits = match object {
            None => availability == "GLOBAL",
            Some(o) => availability != "GLOBAL" && t.object == o,
        };
        if fits {
            out.push(ManualWorkflow { workflow_id, name, availability, object: t.object });
        }
    }
    Ok(out)
}

/// Runs a workflow by hand — the active version, or the draft for a test — once per record
/// (SINGLE), once with all the records (BULK), once with nothing (GLOBAL). The run ids.
pub async fn start_manual(pool: &SqlitePool, workflow_id: &str, record_ids: &[String], test: bool) -> Result<Vec<String>> {
    let v = if test { draft(pool, workflow_id).await? } else { active_version(pool, workflow_id).await?.ok_or_else(|| invalid("This workflow has no active version — activate one first"))? };
    let def = parse(&v.definition.0)?;
    validate(&def)?;
    let trigger = def.trigger.clone().unwrap_or_default();
    if !test && trigger.kind != "MANUAL" {
        return Err(invalid("This workflow doesn't run by hand — its trigger starts it"));
    }
    let mut records = Vec::new();
    if !trigger.object.is_empty() && !record_ids.is_empty() {
        let obj = records::object(pool, &trigger.object).await?;
        for id in record_ids {
            records.push(query::get(pool, &obj, id, false).await?);
        }
    }
    let mut ids = Vec::new();
    if trigger.availability == "SINGLE" && !records.is_empty() {
        for record in records {
            let run_id = run::start(pool, workflow_id, &v.id, json!({ "record": record })).await?;
            run::execute(pool, &run_id).await?;
            ids.push(run_id);
        }
    } else {
        let data = if trigger.availability == "BULK" { json!({ "records": records }) } else if let Some(first) = records.first() { json!({ "record": first }) } else { json!({}) };
        let run_id = run::start(pool, workflow_id, &v.id, data).await?;
        run::execute(pool, &run_id).await?;
        ids.push(run_id);
    }
    Ok(ids)
}
