//! A run (D-068): a version's steps carried out on the record engine. The context — the
//! trigger's data and every step's output, by step id — and the steps done are written after
//! each step; a delay or a form leaves the run WAITING, and it goes on from there, after a quit
//! too. Templates `{{path}}` read the context (`trigger.record.stage`, `s1.id`,
//! `iterator.currentItem.name`, `today+3d`, `now`).

use std::future::Future;
use std::pin::Pin;

use anyhow::{anyhow, Context as _, Result};
use chrono::{DateTime, Duration, Local, SecondsFormat, TimeZone, Utc};
use serde::Serialize;
use serde_json::{json, Map, Value};
use specta::Type;
use sqlx::SqlitePool;

use super::{parse, Condition, Definition, Step};
use crate::error::AppError;
use crate::kernel::records::types::{FieldType, Json};
use crate::kernel::records::{self, query, transfer, Actor, ObjectDef, Query};

const SEARCH_LIMIT: u64 = 200;
const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

/// A run as its page shows it.
#[derive(Debug, Clone, Serialize, Type)]
pub struct RunDetail {
    pub id: String,
    pub name: String,
    /// NOT_STARTED · RUNNING · COMPLETED · FAILED · WAITING
    pub status: String,
    pub workflow_id: String,
    pub workflow_name: String,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub error: String,
    /// The FORM step waited for, or empty.
    pub waiting_for: String,
    pub wait_until: Option<String>,
    pub trigger: Json,
    pub context: Json,
    pub done: Vec<String>,
    pub definition: Json,
}

struct State {
    id: String,
    now: DateTime<Utc>,
    workflow_name: String,
    definition: Definition,
    context: Map<String, Value>,
    done: Vec<String>,
}

enum Flow {
    Done,
    Wait,
    /// A filter said no: the rest is skipped, the run completes.
    Halt,
}

#[derive(sqlx::FromRow)]
struct DetailRow {
    id: String,
    name: String,
    status: String,
    workflow_id: Option<String>,
    workflow_name: Option<String>,
    started_at: Option<String>,
    ended_at: Option<String>,
    error: String,
    waiting_for: String,
    wait_until: Option<String>,
    trigger_json: String,
    context_json: String,
    done_json: String,
    definition_json: String,
    has_version: bool,
}

const DETAIL_SQL: &str = "SELECT r.id, r.name, r.status, r.workflow_id, w.name AS workflow_name, r.started_at, r.ended_at, r.error, r.waiting_for, r.wait_until, \
     r.trigger_json, r.context_json, r.done_json, coalesce(v.definition_json, '{}') AS definition_json, v.id IS NOT NULL AS has_version \
     FROM workflow_run r LEFT JOIN workflow w ON w.id = r.workflow_id LEFT JOIN workflow_version v ON v.id = r.version_id WHERE r.id = ?";

pub async fn detail(pool: &SqlitePool, run_id: &str) -> Result<RunDetail> {
    let r: Option<DetailRow> = sqlx::query_as(DETAIL_SQL).bind(run_id).fetch_optional(pool).await?;
    let r = r.ok_or_else(|| anyhow!(AppError::not_found("No such run")))?;
    let read = |s: &str| serde_json::from_str::<Value>(s).unwrap_or(Value::Null);
    Ok(RunDetail {
        id: r.id,
        name: r.name,
        status: r.status,
        workflow_id: r.workflow_id.unwrap_or_default(),
        workflow_name: r.workflow_name.unwrap_or_default(),
        started_at: r.started_at,
        ended_at: r.ended_at,
        error: r.error,
        waiting_for: r.waiting_for,
        wait_until: r.wait_until,
        trigger: Json(read(&r.trigger_json)),
        context: Json(read(&r.context_json)),
        done: serde_json::from_str(&r.done_json).unwrap_or_default(),
        definition: Json(read(&r.definition_json)),
    })
}

async fn load(pool: &SqlitePool, run_id: &str, now: DateTime<Utc>) -> Result<State> {
    let r: Option<DetailRow> = sqlx::query_as(DETAIL_SQL).bind(run_id).fetch_optional(pool).await?;
    let r = r.ok_or_else(|| anyhow!(AppError::not_found("No such run")))?;
    if !r.has_version {
        // A test of a draft that was discarded since: nothing is left to carry on.
        return Err(invalid("This run's version is gone (a test of a draft discarded since)"));
    }
    let definition = parse(&serde_json::from_str(&r.definition_json).unwrap_or(json!({})))?;
    let context = serde_json::from_str::<Value>(&r.context_json).ok().and_then(|v| v.as_object().cloned()).unwrap_or_default();
    Ok(State { id: r.id, now, workflow_name: r.workflow_name.unwrap_or_default(), definition, context, done: serde_json::from_str(&r.done_json).unwrap_or_default() })
}

/// A new run of a version with the trigger's data, as a workflow-run record, not started yet.
pub async fn start(pool: &SqlitePool, workflow_id: &str, version_id: &str, trigger: Value) -> Result<String> {
    let name: Option<(String,)> = sqlx::query_as("SELECT name FROM workflow WHERE id = ?").bind(workflow_id).fetch_optional(pool).await?;
    let workflow_name = name.map(|(n,)| n).ok_or_else(|| anyhow!(AppError::not_found("No such workflow")))?;
    let runs = records::object(pool, "workflowRun").await?;
    let actor = Actor::workflow(&workflow_name);
    let made = query::create(
        pool,
        &runs,
        &json!({ "name": format!("{workflow_name} · {}", Local::now().format("%-d %b %Y %H:%M")), "status": "NOT_STARTED", "workflow": workflow_id, "startedAt": Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true) }),
        &actor,
    )
    .await?;
    let id = made.get("id").and_then(Value::as_str).unwrap_or_default().to_owned();
    sqlx::query("UPDATE workflow_run SET version_id = ?, trigger_json = ?, context_json = ? WHERE id = ?")
        .bind(version_id)
        .bind(trigger.to_string())
        .bind(json!({ "trigger": trigger }).to_string())
        .bind(&id)
        .execute(pool)
        .await?;
    Ok(id)
}

async fn persist(pool: &SqlitePool, state: &State) -> Result<()> {
    sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE workflow_run SET context_json = ?, done_json = ?, updated_at = {NOW} WHERE id = ?")))
        .bind(Value::Object(state.context.clone()).to_string())
        .bind(serde_json::to_string(&state.done)?)
        .bind(&state.id)
        .execute(pool)
        .await?;
    Ok(())
}

async fn set_status(pool: &SqlitePool, run_id: &str, status: &str, error: &str, ended: bool, wait_until: Option<&str>, waiting_for: &str) -> Result<()> {
    sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE workflow_run SET status = ?, error = ?, ended_at = CASE WHEN ? THEN {NOW} ELSE NULL END, wait_until = ?, waiting_for = ?, updated_at = {NOW} WHERE id = ?")))
        .bind(status)
        .bind(error)
        .bind(ended)
        .bind(wait_until)
        .bind(waiting_for)
        .bind(run_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Takes the run for this pass, from the statuses given only — a start from NOT_STARTED, a
/// resume or a form's answer from WAITING, a retry from FAILED — so a cancelled or finished run
/// never comes back and a run is never in two hands at once.
async fn claim(pool: &SqlitePool, run_id: &str, from: &[&str]) -> Result<()> {
    let marks = vec!["?"; from.len()].join(", ");
    let mut q = sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE workflow_run SET status = 'RUNNING', error = '', ended_at = NULL, wait_until = NULL, waiting_for = '', updated_at = {NOW} WHERE id = ? AND status IN ({marks})"
    )))
    .bind(run_id);
    for status in from {
        q = q.bind(*status);
    }
    if q.execute(pool).await?.rows_affected() == 0 {
        return Err(invalid("This run has moved on — read it again"));
    }
    Ok(())
}

/// Starts a new run: RUNNING, then COMPLETED, FAILED (with the error), or WAITING (a delay's
/// end, or a form).
pub async fn execute(pool: &SqlitePool, run_id: &str) -> Result<RunDetail> {
    execute_at(pool, run_id, Utc::now(), &["NOT_STARTED"]).await
}

/// Carries a run on from where it is, at a given time (the scheduler's tick; tests).
async fn execute_at(pool: &SqlitePool, run_id: &str, now: DateTime<Utc>, from: &[&str]) -> Result<RunDetail> {
    claim(pool, run_id, from).await?;
    let outcome = match load(pool, run_id, now).await {
        Ok(mut state) => {
            let steps = state.definition.steps.clone();
            walk(pool, &mut state, &steps).await
        }
        Err(e) => Err(e),
    };
    match outcome {
        Ok(Flow::Done) | Ok(Flow::Halt) => set_status(pool, run_id, "COMPLETED", "", true, None, "").await?,
        Ok(Flow::Wait) => {}
        Err(e) => set_status(pool, run_id, "FAILED", &format!("{e:#}"), true, None, "").await?,
    }
    detail(pool, run_id).await
}

/// The steps in order, skipping the ones done; a container's inner steps, then itself.
fn walk<'a>(pool: &'a SqlitePool, state: &'a mut State, steps: &'a [Step]) -> Pin<Box<dyn Future<Output = Result<Flow>> + Send + 'a>> {
    Box::pin(async move {
        for step in steps {
            if state.done.contains(&step.id) {
                continue;
            }
            let flow = match step.kind.as_str() {
                "IF_ELSE" => {
                    // The route is picked once: a run that waits or is retried inside it goes on
                    // in the same route, whatever the context says by then.
                    let kept = state.context.get(&step.id).and_then(|v| v.get("index")).and_then(Value::as_i64);
                    let branch = match kept {
                        Some(i) => usize::try_from(i).ok(),
                        None => {
                            let ctx = Value::Object(state.context.clone());
                            // A route without conditions never holds, so Else stays reachable.
                            let picked = step.branches.iter().position(|b| !b.conditions.is_empty() && holds(&b.conditions, &b.logic, &ctx));
                            let name = picked.map(|i| step.branches[i].name.clone()).unwrap_or_else(|| "Else".into());
                            state.context.insert(step.id.clone(), json!({ "branch": name, "index": picked.map_or(-1, |i| i as i64) }));
                            persist(pool, state).await?;
                            picked
                        }
                    };
                    let inner = match branch.and_then(|i| step.branches.get(i)) {
                        Some(b) => &b.steps,
                        None => &step.else_steps,
                    };
                    walk(pool, state, inner).await?
                }
                "ITERATOR" => iterate(pool, state, step).await?,
                _ => run_step(pool, state, step).await?,
            };
            match flow {
                Flow::Done => {
                    if !state.done.contains(&step.id) {
                        state.done.push(step.id.clone());
                    }
                    persist(pool, state).await?;
                }
                Flow::Wait | Flow::Halt => {
                    persist(pool, state).await?;
                    return Ok(flow);
                }
            }
        }
        Ok(Flow::Done)
    })
}

fn inner_ids(steps: &[Step], out: &mut Vec<String>) {
    for s in steps {
        out.push(s.id.clone());
        for b in &s.branches {
            inner_ids(&b.steps, out);
        }
        inner_ids(&s.else_steps, out);
        inner_ids(&s.steps, out);
    }
}

/// Once an item: the inner steps, their done marks and outputs cleared between items (a delay
/// waits, a form asks, an inner iterator starts over for each), the index kept so a run goes on
/// with the item it was at. The item is `{{iterator.currentItem}}` and `{{<id>.currentItem}}`;
/// an enclosing iterator's comes back after the loop. A filter inside stops that item only.
async fn iterate(pool: &SqlitePool, state: &mut State, step: &Step) -> Result<Flow> {
    let ctx = Value::Object(state.context.clone());
    let items = match resolve_value(&step.settings.get("items").cloned().unwrap_or(Value::Null), &ctx) {
        Value::Array(items) => items,
        Value::Null => Vec::new(),
        _ => return Err(invalid(format!("{}: Iterator expects a list", step.name))),
    };
    let mut inner = Vec::new();
    inner_ids(&step.steps, &mut inner);
    let entry = state.context.get(&step.id).cloned();
    let resumed = entry.is_some();
    let start = entry.as_ref().and_then(|v| v.get("index")).and_then(Value::as_u64).map_or(0, |i| usize::try_from(i).unwrap_or(usize::MAX));
    let outer = match &entry {
        Some(e) => e.get("outer").cloned().filter(|v| !v.is_null()),
        None => state.context.get("iterator").cloned(),
    };
    for (i, item) in items.iter().enumerate().skip(start) {
        if i != start || !resumed {
            state.done.retain(|d| !inner.contains(d));
            for id in &inner {
                state.context.remove(id);
            }
        }
        state.context.insert(step.id.clone(), json!({ "index": i, "count": items.len(), "currentItem": item, "outer": outer }));
        state.context.insert("iterator".into(), json!({ "currentItem": item, "index": i }));
        persist(pool, state).await?;
        match walk(pool, state, &step.steps).await? {
            Flow::Wait => return Ok(Flow::Wait),
            Flow::Done | Flow::Halt => {}
        }
    }
    match outer {
        Some(o) => state.context.insert("iterator".into(), o),
        None => state.context.remove("iterator"),
    };
    state.context.insert(step.id.clone(), json!({ "index": items.len(), "count": items.len() }));
    Ok(Flow::Done)
}

async fn object_of(pool: &SqlitePool, settings: &Value) -> Result<ObjectDef> {
    let name = settings.get("object").and_then(Value::as_str).filter(|s| !s.is_empty()).ok_or_else(|| invalid("The step names no object"))?;
    records::object(pool, name).await
}

/// One step: its output goes into the context under its id.
async fn run_step(pool: &SqlitePool, state: &mut State, step: &Step) -> Result<Flow> {
    let ctx = Value::Object(state.context.clone());
    let s = &step.settings;
    let actor = Actor::workflow(&state.workflow_name);
    let label = if step.name.is_empty() { step.kind.clone() } else { step.name.clone() };
    let output = match step.kind.as_str() {
        "CREATE_RECORD" => {
            let def = object_of(pool, s).await?;
            let made = match state.context.get(&step.id).filter(|v| v.get("id").is_some()) {
                // Made before a target failed to link: a retry doesn't make it twice.
                Some(v) => v.clone(),
                None => {
                    let data = resolve_value(&s.get("fields").cloned().unwrap_or(json!({})), &ctx);
                    require_filled(&def, &data, &label)?;
                    let made = query::create(pool, &def, &data, &actor).await.with_context(|| format!("{label}: creating the {}", def.meta.label_singular.to_lowercase()))?;
                    state.context.insert(step.id.clone(), made.clone());
                    persist(pool, state).await?;
                    made
                }
            };
            link_targets(pool, &def, &made, s.get("targets"), &ctx, &actor).await?;
            made
        }
        "UPDATE_RECORD" => {
            let def = object_of(pool, s).await?;
            let id = key_text(&resolve_value(s.get("record_id").unwrap_or(&Value::Null), &ctx));
            if id.is_empty() {
                return Err(invalid(format!("{label}: no record to update")));
            }
            let data = resolve_value(&s.get("fields").cloned().unwrap_or(json!({})), &ctx);
            query::update(pool, &def, &id, &data, &actor).await.with_context(|| format!("{label}: updating the {}", def.meta.label_singular.to_lowercase()))?
        }
        "DELETE_RECORD" => {
            let def = object_of(pool, s).await?;
            let id = key_text(&resolve_value(s.get("record_id").unwrap_or(&Value::Null), &ctx));
            if id.is_empty() {
                return Err(invalid(format!("{label}: no record to delete")));
            }
            let gone = query::get(pool, &def, &id, false).await?;
            query::delete(pool, &def, &id, &actor).await?;
            gone
        }
        "SEARCH_RECORDS" => {
            let def = object_of(pool, s).await?;
            let filter = resolve_value(&s.get("filter").cloned().unwrap_or(Value::Null), &ctx);
            let limit = s.get("limit").and_then(Value::as_u64).unwrap_or(SEARCH_LIMIT).clamp(1, SEARCH_LIMIT) as u32;
            let query = Query { filter: if filter.is_null() { None } else { Some(Json(filter)) }, sort: Vec::new(), limit: Some(limit), offset: None, search: None, deleted: false };
            let page = query::list(pool, &def, &query).await.with_context(|| format!("{label}: searching"))?;
            Value::Array(page.records.into_iter().map(|r| r.0).collect())
        }
        "UPSERT_RECORD" => {
            let def = object_of(pool, s).await?;
            let identifier = s.get("identifier").and_then(Value::as_str).unwrap_or("id").to_owned();
            let data = resolve_value(&s.get("fields").cloned().unwrap_or(json!({})), &ctx);
            let key = data.get(&identifier).map(key_text).unwrap_or_default();
            let existing = if key.is_empty() {
                None
            } else if identifier == "id" {
                query::get(pool, &def, &key, false).await.ok().map(|_| key.clone())
            } else {
                transfer::find_by(pool, &def, &identifier, &key, &mut transfer::Lookups::new()).await?
            };
            match existing {
                Some(id) => query::update(pool, &def, &id, &data, &actor).await?,
                None => {
                    require_filled(&def, &data, &label)?;
                    let made = query::create(pool, &def, &data, &actor).await?;
                    link_targets(pool, &def, &made, s.get("targets"), &ctx, &actor).await?;
                    made
                }
            }
        }
        "FILTER" => {
            let conditions: Vec<Condition> = serde_json::from_value(s.get("conditions").cloned().unwrap_or(json!([]))).unwrap_or_default();
            let logic = s.get("logic").and_then(Value::as_str).unwrap_or("AND");
            if holds(&conditions, logic, &ctx) {
                json!({ "passed": true })
            } else {
                state.context.insert(step.id.clone(), json!({ "passed": false }));
                return Ok(Flow::Halt);
            }
        }
        "DELAY" => {
            let until = match state.context.get(&step.id).and_then(|v| v.get("until")).and_then(Value::as_str) {
                Some(u) => u.to_owned(),
                None => {
                    let until_text = resolve_text(s.get("until").unwrap_or(&Value::Null), &ctx);
                    let until = if until_text.is_empty() {
                        let amount = s.get("amount").and_then(Value::as_i64).unwrap_or(0).max(0);
                        let span = match s.get("unit").and_then(Value::as_str).unwrap_or("MINUTES") {
                            "SECONDS" => Duration::try_seconds(amount),
                            "HOURS" => Duration::try_hours(amount),
                            "DAYS" => Duration::try_days(amount),
                            _ => Duration::try_minutes(amount),
                        };
                        span.and_then(|d| state.now.checked_add_signed(d)).ok_or_else(|| invalid(format!("{label}: that wait is too long")))?
                    } else {
                        parse_when(&until_text).ok_or_else(|| invalid(format!("{label}: the date doesn't read")))?
                    };
                    let text = until.to_rfc3339_opts(SecondsFormat::Secs, true);
                    state.context.insert(step.id.clone(), json!({ "until": text }));
                    persist(pool, state).await?;
                    text
                }
            };
            let due = parse_when(&until).map(|t| t <= state.now).unwrap_or(true);
            if !due {
                set_status(pool, &state.id, "WAITING", "", false, Some(&until), "").await?;
                return Ok(Flow::Wait);
            }
            json!({ "until": until })
        }
        "FORM" => {
            if state.context.get(&step.id).and_then(|v| v.get("answers")).is_some() {
                state.context.get(&step.id).cloned().unwrap_or(Value::Null)
            } else {
                set_status(pool, &state.id, "WAITING", "", false, None, &step.id).await?;
                return Ok(Flow::Wait);
            }
        }
        other => return Err(invalid(format!("No step named {other}"))),
    };
    state.context.insert(step.id.clone(), output);
    Ok(Flow::Done)
}

/// A new note or task attached to records (Twenty's targets): `[{ "object": "person", "id": "{{…}}" }]`.
async fn link_targets(pool: &SqlitePool, def: &ObjectDef, made: &Value, targets: Option<&Value>, ctx: &Value, actor: &Actor) -> Result<()> {
    let Some(Value::Array(targets)) = targets else { return Ok(()) };
    let junction_name = match def.meta.name_singular.as_str() {
        "task" => "taskTarget",
        "note" => "noteTarget",
        _ => return Ok(()),
    };
    let junction = records::object(pool, junction_name).await?;
    let made_id = made.get("id").and_then(Value::as_str).unwrap_or_default();
    for t in targets {
        let object = t.get("object").and_then(Value::as_str).unwrap_or("");
        let id = key_text(&resolve_value(t.get("id").unwrap_or(&Value::Null), ctx));
        if object.is_empty() || id.is_empty() || junction.field(object).is_none() {
            continue;
        }
        query::create(pool, &junction, &json!({ def.meta.name_singular.as_str(): made_id, object: id }), actor).await?;
    }
    Ok(())
}

/// The fields a new record can't do without (a title, a name), refused when a template came out
/// empty — the engine itself lets "+ New" make an untitled record, which a run never fills in.
fn require_filled(def: &ObjectDef, data: &Value, label: &str) -> Result<()> {
    for f in def.fields.iter().filter(|f| f.is_active && !f.is_nullable && matches!(f.r#type, FieldType::Text | FieldType::FullName)) {
        let filled = match data.get(&f.name).or(f.default_value.as_ref().map(|j| &j.0)) {
            Some(Value::String(s)) => !s.trim().is_empty(),
            Some(Value::Object(o)) => o.values().any(|v| v.as_str().is_some_and(|s| !s.trim().is_empty())),
            _ => false,
        };
        if !filled {
            return Err(invalid(format!("{label}: {} came out empty", f.label)));
        }
    }
    Ok(())
}

/// The text a value compares or matches by: a relation's id, a composite's first text part.
fn key_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.trim().to_owned(),
        Value::Object(o) => o.get("id").or_else(|| o.values().find(|x| x.is_string())).and_then(Value::as_str).unwrap_or("").trim().to_owned(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

// ---- templates --------------------------------------------------------------------------

fn parse_when(text: &str) -> Option<DateTime<Utc>> {
    if let Ok(t) = DateTime::parse_from_rfc3339(text) {
        return Some(t.with_timezone(&Utc));
    }
    chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d").ok().and_then(|d| d.and_hms_opt(0, 0, 0)).and_then(|n| Local.from_local_datetime(&n).single()).map(|t| t.with_timezone(&Utc))
}

/// `today`, `now`, with `+3d` / `-2w` / `+1h` / `+30m` after them. Anything that doesn't read
/// — or runs off the calendar — is no date (an empty value), never a crash.
fn date_expr(path: &str) -> Option<Value> {
    let (base, rest) = ["today", "now"].iter().find_map(|b| path.strip_prefix(b).map(|r| (*b, r)))?;
    let mut span = Duration::zero();
    if !rest.is_empty() {
        let negative = rest.starts_with('-');
        let body = rest.strip_prefix(['+', '-'])?;
        let unit = body.chars().last()?;
        let n: i64 = body.strip_suffix(unit)?.parse().ok()?;
        let n = if negative { n.checked_neg()? } else { n };
        span = match unit {
            'd' => Duration::try_days(n),
            'w' => Duration::try_weeks(n),
            'h' => Duration::try_hours(n),
            'm' => Duration::try_minutes(n),
            _ => None,
        }?;
    }
    Some(if base == "today" {
        Value::String(Local::now().checked_add_signed(span)?.format("%Y-%m-%d").to_string())
    } else {
        Value::String(Utc::now().checked_add_signed(span)?.to_rfc3339_opts(SecondsFormat::Secs, true))
    })
}

/// The value at a dotted path in the context (`trigger.record.stage`, `s1.id`, `records.0.name`).
fn lookup(path: &str, ctx: &Value) -> Value {
    let path = path.trim();
    if let Some(v) = date_expr(path) {
        return v;
    }
    let mut cur = ctx;
    for part in path.split('.') {
        cur = match cur {
            Value::Object(o) => match o.get(part) {
                Some(v) => v,
                None => return Value::Null,
            },
            Value::Array(a) => match part.parse::<usize>().ok().and_then(|i| a.get(i)) {
                Some(v) => v,
                None => return Value::Null,
            },
            _ => return Value::Null,
        };
    }
    cur.clone()
}

/// A value as text in a sentence: a relation by its label, an amount in units with its
/// currency, a name or an address by its parts, a list by its count.
fn text_of(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Object(o) => {
            if let Some(t) = o.get("label").or_else(|| o.get("id")).and_then(Value::as_str) {
                return t.to_owned();
            }
            if let Some(m) = o.get("amountMicros").and_then(Value::as_f64) {
                let code = o.get("currencyCode").and_then(Value::as_str).unwrap_or("");
                return format!("{} {code}", m / 1_000_000.0).trim().to_owned();
            }
            o.values().filter_map(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ")
        }
        Value::Array(a) => a.len().to_string(),
    }
}

/// `{{…}}` inside text replaced by their text.
fn render(template: &str, ctx: &Value) -> String {
    let mut out = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find("}}") {
            Some(end) => {
                out.push_str(&text_of(&lookup(&after[..end], ctx)));
                rest = &after[end + 2..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// A setting's value with its templates resolved: one `{{path}}` alone keeps the value's own
/// type (a record, a list, a number); text with templates becomes text; objects and lists
/// resolve inside.
pub fn resolve_value(v: &Value, ctx: &Value) -> Value {
    match v {
        Value::String(s) => {
            let t = s.trim();
            if t.starts_with("{{") && t.ends_with("}}") && t.matches("{{").count() == 1 {
                lookup(&t[2..t.len() - 2], ctx)
            } else {
                Value::String(render(s, ctx))
            }
        }
        Value::Array(a) => Value::Array(a.iter().map(|x| resolve_value(x, ctx)).collect()),
        Value::Object(o) => Value::Object(o.iter().map(|(k, x)| (k.clone(), resolve_value(x, ctx))).collect()),
        other => other.clone(),
    }
}

fn resolve_text(v: &Value, ctx: &Value) -> String {
    text_of(&resolve_value(v, ctx))
}

/// Whether the conditions hold — all of them (AND) or any (OR); numbers compare as numbers,
/// text any case, dates as ISO text.
pub fn holds(conditions: &[Condition], logic: &str, ctx: &Value) -> bool {
    if conditions.is_empty() {
        return true;
    }
    let one = |c: &Condition| {
        let left = resolve_value(&Value::String(c.left.clone()), ctx);
        let right = resolve_value(&Value::String(c.right.clone()), ctx);
        let (l, r) = (text_of(&left), text_of(&right));
        let num = |s: &str| s.trim().parse::<f64>().ok();
        let empty = left.is_null() || l.trim().is_empty() || matches!(&left, Value::Array(a) if a.is_empty());
        let (ll, rl) = (l.to_lowercase(), r.to_lowercase());
        let same = ll == rl || (!empty && left == right);
        let has = ll.contains(&rl) || matches!(&left, Value::Array(a) if a.iter().any(|x| text_of(x).to_lowercase() == rl));
        match c.op.as_str() {
            "is" => same,
            "isNot" => !same,
            "contains" => has,
            "notContains" => !has,
            "isEmpty" => empty,
            "isNotEmpty" => !empty,
            "gt" | "isAfter" => match (num(&l), num(&r)) {
                (Some(a), Some(b)) => a > b,
                _ => l > r,
            },
            "gte" => match (num(&l), num(&r)) {
                (Some(a), Some(b)) => a >= b,
                _ => l >= r,
            },
            "lt" | "isBefore" => match (num(&l), num(&r)) {
                (Some(a), Some(b)) => a < b,
                _ => l < r,
            },
            "lte" => match (num(&l), num(&r)) {
                (Some(a), Some(b)) => a <= b,
                _ => l <= r,
            },
            _ => false,
        }
    };
    if logic.eq_ignore_ascii_case("OR") { conditions.iter().any(one) } else { conditions.iter().all(one) }
}

// ---- the owner's hand -------------------------------------------------------------------

/// A form's answers — only while the run waits on that form: the run goes on.
pub async fn answer_form(pool: &SqlitePool, run_id: &str, step_id: &str, answers: Value) -> Result<RunDetail> {
    let now = detail(pool, run_id).await?;
    if now.status != "WAITING" || now.waiting_for != step_id {
        return Err(invalid("This run isn't waiting for that form any more"));
    }
    let mut state = load(pool, run_id, Utc::now()).await?;
    state.context.insert(step_id.to_owned(), json!({ "answers": answers }));
    persist(pool, &state).await?;
    execute_at(pool, run_id, Utc::now(), &["WAITING"]).await
}

/// A failed run tried again from the step that failed.
pub async fn retry(pool: &SqlitePool, run_id: &str) -> Result<RunDetail> {
    execute_at(pool, run_id, Utc::now(), &["FAILED"]).await
}

/// A run that waits (or never started) is stopped; a running one finishes its step first.
pub async fn cancel(pool: &SqlitePool, run_id: &str) -> Result<RunDetail> {
    let done = sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE workflow_run SET status = 'FAILED', error = 'Cancelled', ended_at = {NOW}, wait_until = NULL, waiting_for = '', updated_at = {NOW} WHERE id = ? AND status IN ('WAITING', 'NOT_STARTED')"
    )))
    .bind(run_id)
    .execute(pool)
    .await?;
    if done.rows_affected() == 0 {
        return Err(invalid("Only a run that waits can be cancelled"));
    }
    detail(pool, run_id).await
}

/// The waiting runs whose delay is over (their workflow not in Deleted records): carried on.
/// How many.
pub async fn resume_due(pool: &SqlitePool, now: DateTime<Utc>) -> Result<u32> {
    let due: Vec<(String,)> = sqlx::query_as(
        "SELECT r.id FROM workflow_run r JOIN workflow w ON w.id = r.workflow_id AND w.deleted_at IS NULL WHERE r.status = 'WAITING' AND r.wait_until IS NOT NULL AND r.wait_until <= ?",
    )
    .bind(now.to_rfc3339_opts(SecondsFormat::Secs, true))
    .fetch_all(pool)
    .await?;
    let mut n = 0;
    for (id,) in due {
        if let Err(e) = execute_at(pool, &id, now, &["WAITING"]).await {
            tracing::warn!("resuming run {id}: {e:#}");
        }
        n += 1;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::hub::WorkspaceHub;
    use crate::kernel::records::events::{self, RecordEvent};
    use crate::modules::workflows::{self, triggers, Definition, Trigger};

    async fn workspace(tag: &str) -> SqlitePool {
        let root = std::env::temp_dir().join(format!("newdawn-workflows-{tag}-{}", uuid::Uuid::now_v7()));
        let hub = WorkspaceHub::open(root).await.unwrap();
        hub.create_workspace("VIFC", "test").await.unwrap();
        hub.pool("test").unwrap()
    }

    async fn workflow(pool: &SqlitePool, name: &str, definition: Value) -> String {
        let def = records::object(pool, "workflow").await.unwrap();
        let made = query::create(pool, &def, &json!({ "name": name, "status": "DRAFT" }), &Actor::system("test")).await.unwrap();
        let id = made["id"].as_str().unwrap().to_owned();
        let draft = workflows::save_draft(pool, &id, &definition).await.unwrap();
        workflows::activate(pool, &draft.id).await.unwrap();
        id
    }

    /// Plan 0.10.11's "done when": an opportunity's stage becomes Customer → a task "Send welcome
    /// note" due in 3 days on its point of contact, by way of the engine's record event. The
    /// workflow's own write starts nothing, and a stage left alone starts nothing.
    #[test]
    fn a_stage_change_makes_a_task_for_the_point_of_contact() {
        tauri::async_runtime::block_on(async {
            let pool = workspace("stage").await;
            let actor = Actor::system("test");
            // The channel is the process's one: the other tests' records come through it too,
            // so the test takes the events it knows by their record or their title.
            let mut events = events::subscribe().expect("the first subscriber");
            let mut take = |want: &dyn Fn(&RecordEvent) -> bool| loop {
                match events.try_recv() {
                    Ok(e) if want(&e) => break e,
                    Ok(_) => continue,
                    Err(_) => panic!("the engine told nothing"),
                }
            };
            let person = records::object(&pool, "person").await.unwrap();
            let minh = query::create(&pool, &person, &json!({ "name": { "firstName": "Minh", "lastName": "Tran" } }), &actor).await.unwrap();
            let minh_id = minh["id"].as_str().unwrap().to_owned();
            let opportunity = records::object(&pool, "opportunity").await.unwrap();
            let deal = query::create(&pool, &opportunity, &json!({ "name": "Bank portal", "stage": "PROPOSAL", "pointOfContact": minh_id }), &actor).await.unwrap();
            let deal_id = deal["id"].as_str().unwrap().to_owned();

            workflow(
                &pool,
                "Welcome customers",
                json!({
                    "trigger": { "type": "RECORD", "object": "opportunity", "event": "UPDATED", "fields": ["stage"] },
                    "steps": [
                        { "id": "gate", "type": "FILTER", "name": "Only customers", "settings": { "conditions": [{ "left": "{{trigger.record.stage}}", "op": "is", "right": "CUSTOMER" }] } },
                        { "id": "welcome", "type": "CREATE_RECORD", "name": "Welcome task", "settings": {
                            "object": "task",
                            "fields": { "title": "Send welcome note to {{trigger.record.pointOfContact.label}}", "dueAt": "{{today+3d}}", "status": "TODO" },
                            "targets": [{ "object": "person", "id": "{{trigger.record.pointOfContact.id}}" }]
                        } }
                    ]
                }),
            )
            .await;
            let triggers_set: (i64,) = sqlx::query_as("SELECT count(*) FROM workflow_automated_trigger WHERE kind = 'RECORD'").fetch_one(&pool).await.unwrap();
            assert_eq!(triggers_set.0, 1);

            // Another field changes: the trigger watches the stage only.
            query::update(&pool, &opportunity, &deal_id, &json!({ "name": "Bank portal v2" }), &actor).await.unwrap();
            let event = take(&|e| e.id == deal_id && e.event == "updated");
            assert_eq!(triggers::on_record(&pool, &event).await.unwrap(), 0, "the stage didn't change");

            query::update(&pool, &opportunity, &deal_id, &json!({ "stage": "CUSTOMER" }), &actor).await.unwrap();
            let event = take(&|e| e.id == deal_id && e.event == "updated");
            assert_eq!(triggers::on_record(&pool, &event).await.unwrap(), 1);

            let task = records::object(&pool, "task").await.unwrap();
            let tasks = query::list(&pool, &task, &Query::default()).await.unwrap();
            assert_eq!(tasks.total, 1);
            let made = &tasks.records[0].0;
            assert_eq!(made["title"], "Send welcome note to Minh Tran");
            assert_eq!(made["dueAt"], (Local::now() + Duration::days(3)).format("%Y-%m-%d").to_string());
            assert_eq!(made["createdBy"]["source"], "WORKFLOW");
            let target: (String,) = sqlx::query_as("SELECT person_id FROM task_target WHERE task_id = ?").bind(made["id"].as_str().unwrap()).fetch_one(&pool).await.unwrap();
            assert_eq!(target.0, minh_id, "the task is on the point of contact");

            // A workflow's own writes are never announced (a run would only skip them).
            while let Ok(e) = events.try_recv() {
                assert_ne!(e.source, "WORKFLOW");
            }

            let runs: Vec<(String, String)> = sqlx::query_as("SELECT status, done_json FROM workflow_run ORDER BY created_at").fetch_all(&pool).await.unwrap();
            assert_eq!(runs.len(), 1);
            assert_eq!(runs[0].0, "COMPLETED");
            assert_eq!(runs[0].1, r#"["gate","welcome"]"#);
            let detail = detail(&pool, &sqlx::query_as::<_, (String,)>("SELECT id FROM workflow_run").fetch_one(&pool).await.unwrap().0).await.unwrap();
            assert_eq!(detail.context.0["welcome"]["title"], "Send welcome note to Minh Tran");
            assert_eq!(detail.trigger.0["record"]["stage"], "CUSTOMER");
            assert_eq!(detail.trigger.0["before"]["stage"], "PROPOSAL");
        });
    }

    /// A manual run with a delay waits in the database and goes on when its time comes — from
    /// the scheduler's tick, as after a quit; the filter's "no" ends a run early; if/else picks
    /// its branch; the iterator runs once an item.
    #[test]
    fn a_delayed_run_waits_and_resumes() {
        tauri::async_runtime::block_on(async {
            let pool = workspace("delay").await;
            let id = workflow(
                &pool,
                "Nightly",
                json!({
                    "trigger": { "type": "MANUAL", "availability": "GLOBAL" },
                    "steps": [
                        { "id": "wait", "type": "DELAY", "settings": { "amount": 5, "unit": "MINUTES" } },
                        { "id": "pick", "type": "IF_ELSE", "branches": [{ "name": "Morning", "conditions": [{ "left": "{{trigger.kind}}", "op": "is", "right": "morning" }], "steps": [
                            { "id": "c1", "type": "CREATE_RECORD", "settings": { "object": "company", "fields": { "name": "Morning Co" } } }
                        ] }], "else_steps": [
                            { "id": "c2", "type": "CREATE_RECORD", "settings": { "object": "company", "fields": { "name": "Evening Co" } } }
                        ] },
                        { "id": "each", "type": "ITERATOR", "settings": { "items": ["A", "B"] }, "steps": [
                            { "id": "c3", "type": "CREATE_RECORD", "settings": { "object": "company", "fields": { "name": "Branch {{iterator.currentItem}} #{{iterator.index}}" } } }
                        ] },
                        { "id": "stop", "type": "FILTER", "settings": { "conditions": [{ "left": "{{c2.name}}", "op": "isEmpty", "right": "" }] } },
                        { "id": "never", "type": "CREATE_RECORD", "settings": { "object": "company", "fields": { "name": "Never" } } }
                    ]
                }),
            )
            .await;
            let listed = workflows::manual(&pool, None).await.unwrap();
            assert_eq!(listed.len(), 1);
            assert_eq!(listed[0].availability, "GLOBAL");
            assert!(workflows::manual(&pool, Some("company")).await.unwrap().is_empty(), "a global workflow isn't offered on a record");

            let runs = workflows::start_manual(&pool, &id, &[], false).await.unwrap();
            assert_eq!(runs.len(), 1);
            let run = detail(&pool, &runs[0]).await.unwrap();
            assert_eq!(run.status, "WAITING");
            let until = run.wait_until.clone().expect("a delay's end");
            assert!(run.done.is_empty());

            assert_eq!(resume_due(&pool, Utc::now()).await.unwrap(), 0, "not yet");
            assert_eq!(resume_due(&pool, Utc::now() + Duration::minutes(6)).await.unwrap(), 1);
            let run = detail(&pool, &runs[0]).await.unwrap();
            assert_eq!(run.status, "COMPLETED", "{}", run.error);
            assert_eq!(run.done, ["wait", "c2", "pick", "c3", "each"], "the filter said no, so it and the step after it aren't done");
            assert_eq!(run.context.0["wait"]["until"], until);
            assert_eq!(run.context.0["pick"]["branch"], "Else");
            assert_eq!(run.context.0["stop"]["passed"], false);
            let company = records::object(&pool, "company").await.unwrap();
            let names: Vec<String> = query::list(&pool, &company, &Query::default()).await.unwrap().records.iter().map(|r| r.0["name"].as_str().unwrap().to_owned()).collect();
            let mut sorted = names.clone();
            sorted.sort();
            assert_eq!(sorted, ["Branch A #0", "Branch B #1", "Evening Co"]);
        });
    }

    /// Each item of an iterator asks its form again (the item before's outputs are gone), a form
    /// is answered only while the run waits on it, and a finished run is neither retried nor
    /// cancelled.
    #[test]
    fn an_iterator_asks_its_form_once_an_item() {
        tauri::async_runtime::block_on(async {
            let pool = workspace("form").await;
            let id = workflow(
                &pool,
                "Branches",
                json!({
                    "trigger": { "type": "MANUAL", "availability": "GLOBAL" },
                    "steps": [
                        { "id": "each", "type": "ITERATOR", "settings": { "items": ["A", "B"] }, "steps": [
                            { "id": "ask", "type": "FORM", "settings": { "fields": [{ "name": "n", "label": "Name", "type": "text" }] } },
                            { "id": "make", "type": "CREATE_RECORD", "settings": { "object": "company", "fields": { "name": "{{ask.answers.n}} {{iterator.currentItem}}" } } }
                        ] }
                    ]
                }),
            )
            .await;
            let run_id = workflows::start_manual(&pool, &id, &[], false).await.unwrap().remove(0);
            let run = detail(&pool, &run_id).await.unwrap();
            assert_eq!((run.status.as_str(), run.waiting_for.as_str()), ("WAITING", "ask"));
            assert!(answer_form(&pool, &run_id, "make", json!({ "n": "x" })).await.is_err(), "only the form it waits on");
            let run = answer_form(&pool, &run_id, "ask", json!({ "n": "North" })).await.unwrap();
            assert_eq!((run.status.as_str(), run.waiting_for.as_str()), ("WAITING", "ask"), "the second item asks again");
            let run = answer_form(&pool, &run_id, "ask", json!({ "n": "South" })).await.unwrap();
            assert_eq!(run.status, "COMPLETED", "{}", run.error);
            let company = records::object(&pool, "company").await.unwrap();
            let mut names: Vec<String> = query::list(&pool, &company, &Query::default()).await.unwrap().records.iter().map(|r| r.0["name"].as_str().unwrap().to_owned()).collect();
            names.sort();
            assert_eq!(names, ["North A", "South B"]);
            assert!(retry(&pool, &run_id).await.is_err(), "a finished run isn't retried");
            assert!(cancel(&pool, &run_id).await.is_err(), "nor cancelled");
        });
    }

    /// What the owner types can be anything: a date that runs off the calendar, a wait of
    /// forever, a schedule that never comes — each is an empty value or a plain refusal.
    #[test]
    fn extreme_input_fails_plainly() {
        let ctx = json!({});
        assert_eq!(resolve_value(&json!("{{today+3é}}"), &ctx), Value::Null);
        assert_eq!(resolve_value(&json!("{{today+999999999999999d}}"), &ctx), Value::Null);
        assert_eq!(resolve_value(&json!("{{now--9223372036854775808m}}"), &ctx), Value::Null);
        let forever = Trigger { kind: "SCHEDULE".into(), every: u32::MAX, unit: "DAYS".into(), ..Default::default() };
        assert!(workflows::validate(&Definition { trigger: Some(forever), steps: Vec::new() }).is_err());
        tauri::async_runtime::block_on(async {
            let pool = workspace("extreme").await;
            let id = workflow(&pool, "Forever", json!({ "trigger": { "type": "MANUAL", "availability": "GLOBAL" }, "steps": [{ "id": "w", "type": "DELAY", "settings": { "amount": i64::MAX, "unit": "DAYS" } }] })).await;
            let run_id = workflows::start_manual(&pool, &id, &[], false).await.unwrap().remove(0);
            let run = detail(&pool, &run_id).await.unwrap();
            assert_eq!(run.status, "FAILED");
            assert!(run.error.contains("too long"), "{}", run.error);
        });
    }

    #[test]
    fn templates_and_conditions_read_the_context() {
        let ctx = json!({ "trigger": { "record": { "stage": "CUSTOMER", "amount": { "amountMicros": 1500000 }, "pointOfContact": { "id": "p1", "label": "Minh" }, "tags": ["a", "b"] } }, "s1": [{ "name": "x" }] });
        assert_eq!(resolve_value(&json!("{{trigger.record.pointOfContact}}"), &ctx), json!({ "id": "p1", "label": "Minh" }));
        assert_eq!(resolve_value(&json!("Hi {{trigger.record.pointOfContact.label}}, stage {{trigger.record.stage}}"), &ctx), json!("Hi Minh, stage CUSTOMER"));
        assert_eq!(resolve_value(&json!("{{s1.0.name}}"), &ctx), json!("x"));
        assert_eq!(resolve_value(&json!("{{nothing.here}}"), &ctx), Value::Null);
        assert_eq!(resolve_value(&json!("{{today}}"), &ctx), json!(Local::now().format("%Y-%m-%d").to_string()));
        assert_eq!(resolve_value(&json!("{{today-1w}}"), &ctx), json!((Local::now() - Duration::weeks(1)).format("%Y-%m-%d").to_string()));
        let c = |left: &str, op: &str, right: &str| Condition { left: left.into(), op: op.into(), right: right.into() };
        assert!(holds(&[c("{{trigger.record.stage}}", "is", "customer")], "AND", &ctx));
        assert!(holds(&[c("{{trigger.record.amount.amountMicros}}", "gt", "1000000")], "AND", &ctx));
        assert!(!holds(&[c("{{trigger.record.amount.amountMicros}}", "lt", "1000000")], "AND", &ctx));
        assert!(holds(&[c("{{trigger.record.tags}}", "contains", "B")], "AND", &ctx));
        assert!(holds(&[c("{{trigger.record.missing}}", "isEmpty", "")], "AND", &ctx));
        assert!(holds(&[c("{{trigger.record.stage}}", "is", "NEW"), c("{{trigger.record.stage}}", "isNot", "NEW")], "OR", &ctx));
        assert!(!holds(&[c("{{trigger.record.stage}}", "is", "NEW"), c("{{trigger.record.stage}}", "isNot", "NEW")], "AND", &ctx));
    }
}
