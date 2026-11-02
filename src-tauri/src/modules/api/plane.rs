//! Plane's API (D-084) on the local API's server (D-071): Plane's v1 paths and JSON under
//! `/api/v1/workspaces/{slug}/` — projects, work items (`issues/`, and `work-items/` as Plane's
//! newer name), states, labels, cycles and modules with their work items, Intake's requests —
//! opened by an `X-API-Key` of that workspace, 60 requests a minute a key (`X-RateLimit-Remaining`,
//! `X-RateLimit-Reset`), lists in Plane's pages (`per_page`, `cursor` = "per_page:page:is_prev").
//! Writes go through the Projects module as "<token> (API)"; colours are the app's names.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Bytes;
use axum::extract::{Path, Query, Request, State};
use axum::http::{HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum::{Extension, Json, Router};
use serde::de::DeserializeOwned;
use serde_json::{json, Map, Value};
use sqlx::SqlitePool;
use tauri::{AppHandle, Manager};

use super::keys;
use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::modules::pm::cycles::{self, CycleInput, CyclePatch, CycleStatus, PmCycle};
use crate::modules::pm::intake::{self, IntakeInput, IntakeStatus, PmIntakeItem};
use crate::modules::pm::items::{self, ItemInput, ItemPatch, PmItem};
use crate::modules::pm::modules::{self as pm_modules, ModuleInput, ModulePatch, PmModule};
use crate::modules::pm::comments;
use crate::modules::pm::settings::{self, LabelInput, PmLabel, PmState, StateGroup, StateInput, StatePatch};
use crate::modules::pm::{self, Features, NewPmProject, Network, PmProject, PmProjectPatch};

/// Plane's rate: 60 requests a minute a key.
const PER_MINUTE: u32 = 60;
/// Plane's page: 100 at most.
const PER_PAGE: usize = 100;

/// An error as Plane's API answers one: `{ "error": … }`, or DRF's `{ "detail": … }` for keys and
/// throttling.
#[derive(Debug)]
pub struct PlaneError {
    status: StatusCode,
    body: Value,
}

impl PlaneError {
    fn error(status: StatusCode, message: impl Into<String>) -> Self {
        Self { status, body: json!({ "error": message.into() }) }
    }
    fn detail(status: StatusCode, message: impl Into<String>) -> Self {
        Self { status, body: json!({ "detail": message.into() }) }
    }
    fn bad(message: impl Into<String>) -> Self {
        Self::error(StatusCode::BAD_REQUEST, message)
    }
    fn missing() -> Self {
        Self::error(StatusCode::NOT_FOUND, "The requested resource does not exist.")
    }
    #[cfg(test)]
    pub fn status(&self) -> u16 {
        self.status.as_u16()
    }
}

impl From<anyhow::Error> for PlaneError {
    fn from(e: anyhow::Error) -> Self {
        match e.downcast_ref::<AppError>() {
            Some(a) if a.code == "not_found" => Self::error(StatusCode::NOT_FOUND, a.message.clone()),
            Some(a) if a.code.starts_with("invalid") => Self::bad(a.message.clone()),
            _ => Self::error(StatusCode::INTERNAL_SERVER_ERROR, format!("{e:#}")),
        }
    }
}

impl IntoResponse for PlaneError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

type Answer = Result<(StatusCode, Value), PlaneError>;

fn ok(v: Value) -> Answer {
    Ok((StatusCode::OK, v))
}

fn made(v: Value) -> Answer {
    Ok((StatusCode::CREATED, v))
}

fn gone() -> Answer {
    Ok((StatusCode::NO_CONTENT, Value::Null))
}

fn not_allowed(m: &Method) -> Answer {
    Err(PlaneError::detail(StatusCode::METHOD_NOT_ALLOWED, format!("Method \"{m}\" not allowed.")))
}

/// Who calls: the workspace its key opens — its id (Plane's `workspace`), its folder (for deletes'
/// files) — and the key's name as the actor.
#[derive(Clone)]
pub struct Caller {
    pub pool: SqlitePool,
    pub workspace_id: String,
    pub folder: String,
    pub actor: String,
}

static WINDOWS: LazyLock<Mutex<HashMap<String, (u64, u32)>>> = LazyLock::new(Default::default);

/// A key's request counted in the minute it falls in: whether it may go on, how many are left,
/// and when the minute ends (seconds since 1970).
fn take(key: &str, now: u64) -> (bool, u32, u64) {
    let window = now / 60;
    let mut all = WINDOWS.lock().unwrap_or_else(PoisonError::into_inner);
    all.retain(|_, (w, _)| *w == window);
    let (_, used) = all.entry(key.to_owned()).or_insert((window, 0));
    let go = *used < PER_MINUTE;
    if go {
        *used += 1;
    }
    (go, PER_MINUTE - *used, (window + 1) * 60)
}

/// The key (`X-API-Key`), its workspace against the address's, Plane's rate; the answer carries
/// the rate's headers.
async fn gate(State(app): State<AppHandle>, mut req: Request, next: Next) -> Response {
    let Some(key) = req.headers().get("x-api-key").and_then(|v| v.to_str().ok()).map(str::trim).filter(|k| !k.is_empty()).map(str::to_owned) else {
        return PlaneError::detail(StatusCode::UNAUTHORIZED, "Authentication credentials were not provided.").into_response();
    };
    let hub = app.state::<WorkspaceHub>();
    let (slug, name) = match keys::find(&hub, &key).await {
        Ok(Some(found)) => found,
        Ok(None) => return PlaneError::detail(StatusCode::UNAUTHORIZED, "Given API token is not valid").into_response(),
        Err(e) => return PlaneError::from(e).into_response(),
    };
    // /api/v1/workspaces/{slug}/…: a key opens its own workspace only.
    if req.uri().path().split('/').nth(4) != Some(slug.as_str()) {
        return PlaneError::detail(StatusCode::FORBIDDEN, "This API token isn't for that workspace.").into_response();
    }
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (go, left, reset) = take(&keys::sha256_hex(&key), now);
    let mut answer = if !go {
        let mut r = PlaneError::detail(StatusCode::TOO_MANY_REQUESTS, format!("Request was throttled. Expected available in {} seconds.", reset.saturating_sub(now))).into_response();
        r.headers_mut().insert("Retry-After", HeaderValue::from(reset.saturating_sub(now)));
        r
    } else {
        let caller = async {
            let pool = hub.pool(&slug)?;
            // The module turned off in this workspace (D-059): its paths aren't served.
            if !crate::modules::switches::enabled(&pool).await?.projects {
                anyhow::bail!(AppError::not_found("The Projects module is off in this workspace."));
            }
            let w = hub.workspace(&slug).await?;
            anyhow::Ok(Caller { pool, workspace_id: w.id, folder: w.folder_path, actor: format!("{name} (API)") })
        }
        .await;
        match caller {
            Ok(caller) => {
                req.extensions_mut().insert(caller);
                next.run(req).await
            }
            Err(e) => PlaneError::from(e).into_response(),
        }
    };
    let headers = answer.headers_mut();
    headers.insert("X-RateLimit-Remaining", HeaderValue::from(left));
    headers.insert("X-RateLimit-Reset", HeaderValue::from(reset));
    answer
}

async fn handle(Extension(c): Extension<Caller>, method: Method, Path(p): Path<HashMap<String, String>>, Query(q): Query<HashMap<String, String>>, body: Bytes) -> Response {
    let answer = match object(&body) {
        Ok(b) => dispatch(&c, &method, &p, &q, &b).await,
        Err(e) => Err(e),
    };
    match answer {
        Ok((StatusCode::NO_CONTENT, _)) => StatusCode::NO_CONTENT.into_response(),
        Ok((status, v)) => (status, Json(v)).into_response(),
        Err(e) => e.into_response(),
    }
}

pub fn routes(app: AppHandle) -> Router<AppHandle> {
    const W: &str = "/api/v1/workspaces/{slug}";
    let mut r = Router::new();
    for path in [
        "/projects/",
        "/projects/{project}/",
        "/projects/{project}/{kind}/",
        "/projects/{project}/{kind}/{id}/",
        "/projects/{project}/{kind}/{id}/{sub}/",
        "/projects/{project}/{kind}/{id}/{sub}/{item}/",
        "/issues/{key}/",
        "/work-items/{key}/",
    ] {
        // Plane's paths end with a slash; the same without it.
        r = r.route(&format!("{W}{path}"), any(handle)).route(&format!("{W}{}", path.trim_end_matches('/')), any(handle));
    }
    r.layer(middleware::from_fn_with_state(app, gate))
}

// ---- Requests ----

fn object(body: &Bytes) -> Result<Map<String, Value>, PlaneError> {
    if body.iter().all(u8::is_ascii_whitespace) {
        return Ok(Map::new());
    }
    match serde_json::from_slice(body) {
        Ok(Value::Object(m)) => Ok(m),
        Ok(_) => Err(PlaneError::bad("The body is a JSON object")),
        Err(e) => Err(PlaneError::bad(format!("JSON parse error - {e}"))),
    }
}

/// A text field: absent → None; null → "" (Plane clears with null); a number as its text.
fn text(b: &Map<String, Value>, k: &str) -> Result<Option<String>, PlaneError> {
    match b.get(k) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(String::new())),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(v @ Value::Number(_)) => Ok(Some(v.to_string())),
        Some(_) => Err(PlaneError::bad(format!("{k}: a string is expected"))),
    }
}

fn texts(b: &Map<String, Value>, k: &str) -> Result<Option<Vec<String>>, PlaneError> {
    match b.get(k) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(Vec::new())),
        Some(Value::Array(a)) => a.iter().map(|v| v.as_str().map(str::to_owned).ok_or_else(|| PlaneError::bad(format!("{k}: a list of ids is expected")))).collect::<Result<_, _>>().map(Some),
        Some(_) => Err(PlaneError::bad(format!("{k}: a list of ids is expected"))),
    }
}

fn flag(b: &Map<String, Value>, k: &str) -> Result<Option<bool>, PlaneError> {
    match b.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(v)) => Ok(Some(*v)),
        Some(_) => Err(PlaneError::bad(format!("{k}: true or false is expected"))),
    }
}

fn number(b: &Map<String, Value>, k: &str) -> Result<Option<f64>, PlaneError> {
    match b.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_f64().map(Some).ok_or_else(|| PlaneError::bad(format!("{k}: a number is expected"))),
    }
}

/// One of an enumeration's values, by its name (`in-progress` read as `in_progress`).
fn choice<T: DeserializeOwned>(b: &Map<String, Value>, k: &str) -> Result<Option<T>, PlaneError> {
    match b.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => serde_json::from_value(Value::String(s.replace('-', "_"))).map(Some).map_err(|_| PlaneError::bad(format!("{k}: \"{s}\" is not a valid choice."))),
        Some(_) => Err(PlaneError::bad(format!("{k}: a string is expected"))),
    }
}

fn required(v: Option<String>, k: &str) -> Result<String, PlaneError> {
    v.filter(|s| !s.trim().is_empty()).ok_or_else(|| PlaneError::bad(format!("{k}: This field is required.")))
}

/// HTML's entities read: named (&amp; &lt; &gt; &quot; &apos; &nbsp;) and numbered (&#8217; &#x2019;).
fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        let read = tail.find(';').filter(|end| *end <= 10).and_then(|end| {
            let ch = match &tail[1..end] {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                "nbsp" => Some(' '),
                n => n.strip_prefix("#x").or_else(|| n.strip_prefix("#X")).map_or_else(|| n.strip_prefix('#').and_then(|d| d.parse().ok()), |h| u32::from_str_radix(h, 16).ok()).and_then(char::from_u32),
            };
            ch.map(|c| (c, end))
        });
        match read {
            Some((c, end)) => {
                out.push(c);
                rest = &tail[end + 1..];
            }
            None => {
                out.push('&');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Plane's `description_html` as the editor's document: its text, a paragraph a line.
fn doc_of(html: &str) -> String {
    let text = text_of(html);
    let content: Vec<Value> = text.lines().map(str::trim).filter(|l| !l.is_empty()).map(|l| json!({ "type": "paragraph", "content": [{ "type": "text", "text": l }] })).collect();
    json!({ "type": "doc", "content": if content.is_empty() { vec![json!({ "type": "paragraph" })] } else { content } }).to_string()
}

/// HTML's text, a line a block.
fn text_of(html: &str) -> String {
    let mut text = String::new();
    let mut tag = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        match (in_tag, ch) {
            (false, '<') => {
                in_tag = true;
                tag.clear();
            }
            (true, '>') => {
                in_tag = false;
                let t = tag.trim_start_matches('/').split([' ', '/']).next().unwrap_or("").to_ascii_lowercase();
                if matches!(t.as_str(), "p" | "br" | "div" | "li" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "blockquote" | "pre") && (tag.starts_with('/') || t == "br") {
                    text.push('\n');
                }
            }
            (true, c) => tag.push(c),
            (false, c) => text.push(c),
        }
    }
    unescape(&text)
}

/// Text compared by its words (line breaks and spaces aside).
fn words(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Text as Plane's `description_html`: a paragraph a line.
fn html_of(text: &str) -> String {
    let esc = |l: &str| l.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    let html: String = text.lines().filter(|l| !l.trim().is_empty()).map(|l| format!("<p>{}</p>", esc(l))).collect();
    if html.is_empty() { "<p></p>".into() } else { html }
}

fn opt(s: &str) -> Value {
    if s.is_empty() { Value::Null } else { json!(s) }
}

// ---- Pages ----

/// Plane's page of a list: `per_page` (100 at most), `cursor` "per_page:page:is_prev", `order_by`
/// (a field, `-` for descending) and `fields` (the keys kept).
fn page(mut all: Vec<Value>, q: &HashMap<String, String>) -> Answer {
    if let Some(by) = q.get("order_by").map(|o| o.trim()).filter(|o| !o.is_empty()) {
        let (key, desc) = by.strip_prefix('-').map_or((by, false), |k| (k, true));
        all.sort_by(|a, b| {
            let o = match (&a[key], &b[key]) {
                (Value::Number(x), Value::Number(y)) => x.as_f64().partial_cmp(&y.as_f64()).unwrap_or(std::cmp::Ordering::Equal),
                (x, y) => x.as_str().unwrap_or("").cmp(y.as_str().unwrap_or("")),
            };
            if desc { o.reverse() } else { o }
        });
    }
    let mut per_page = match q.get("per_page") {
        Some(n) => n.trim().parse::<usize>().map_err(|_| PlaneError::bad("per_page: a number is expected"))?,
        None => PER_PAGE,
    };
    let mut at = 0usize;
    if let Some(c) = q.get("cursor").filter(|c| !c.is_empty()) {
        let parts: Vec<&str> = c.split(':').collect();
        let bad = || PlaneError::bad("Invalid cursor parameter.");
        if parts.len() != 3 {
            return Err(bad());
        }
        per_page = parts[0].parse().map_err(|_| bad())?;
        at = parts[1].parse::<i64>().ok().and_then(|n| usize::try_from(n).ok()).ok_or_else(bad)?;
    }
    let per_page = per_page.clamp(1, PER_PAGE);
    let total = all.len();
    let fields: Option<Vec<&str>> = q.get("fields").map(|f| f.split(',').map(str::trim).filter(|f| !f.is_empty()).collect());
    let results: Vec<Value> = all
        .into_iter()
        .skip(at.saturating_mul(per_page))
        .take(per_page)
        .map(|v| match (&fields, v) {
            (Some(keep), Value::Object(m)) => Value::Object(m.into_iter().filter(|(k, _)| keep.contains(&k.as_str())).collect()),
            (_, v) => v,
        })
        .collect();
    let count = results.len();
    ok(json!({
        "grouped_by": null,
        "sub_grouped_by": null,
        "total_count": total,
        "next_cursor": format!("{per_page}:{}:0", at + 1),
        "prev_cursor": format!("{per_page}:{}:1", at as i64 - 1),
        "next_page_results": at.saturating_mul(per_page).saturating_add(count) < total,
        "prev_page_results": at > 0,
        "count": count,
        "total_pages": total.div_ceil(per_page),
        "total_results": total,
        "extra_stats": null,
        "results": results,
    }))
}

// ---- Plane's JSON ----

pub fn project_json(p: &PmProject, ws: &str, (cycles, modules, default_state): &(u32, u32, Option<String>)) -> Value {
    let f = p.features;
    json!({
        "id": p.id, "total_members": 1, "total_cycles": cycles, "total_modules": modules, "is_member": true,
        "sort_order": p.sort_order, "member_role": 20, "is_deployed": false, "cover_image_url": null,
        "created_at": p.created_at, "updated_at": p.updated_at, "deleted_at": null,
        "name": p.name, "description": p.description, "description_text": null, "description_html": null,
        "network": if p.network == Network::Public { 2 } else { 0 }, "identifier": p.identifier,
        "emoji": opt(&p.emoji), "icon_prop": null, "logo_props": {},
        "module_view": f.modules, "cycle_view": f.cycles, "issue_views_view": f.views, "page_view": f.pages, "intake_view": f.intake,
        "is_time_tracking_enabled": false, "is_issue_type_enabled": false, "guest_view_all_features": false,
        "cover_image": null, "archive_in": p.automations.archive_in, "close_in": p.automations.close_in,
        "archived_at": p.archived_at, "timezone": if p.time_zone.is_empty() { "UTC" } else { p.time_zone.as_str() },
        "created_by": null, "updated_by": null, "workspace": ws,
        "default_assignee": p.default_assignee, "project_lead": p.lead, "estimate": p.estimate_id, "default_state": default_state,
    })
}

pub fn issue_json(i: &PmItem, ws: &str) -> Value {
    json!({
        "id": i.id, "type_id": null, "created_at": i.created_at, "updated_at": i.updated_at, "deleted_at": null,
        "point": null, "name": i.name, "description_html": html_of(&i.description_text), "description_stripped": opt(&i.description_text), "description_binary": null,
        "priority": i.priority, "start_date": i.start_date, "target_date": i.target_date, "sequence_id": i.sequence, "sort_order": i.sort_order,
        "completed_at": i.completed_at, "archived_at": i.archived_at, "is_draft": false, "external_source": null, "external_id": null,
        "created_by": i.created_by, "updated_by": i.updated_by, "project": i.project_id, "workspace": ws,
        "parent": i.parent_id, "state": i.state_id, "estimate_point": i.estimate_point_id, "type": null,
        "assignees": i.assignees, "labels": i.labels,
    })
}

type Times = HashMap<String, (String, String)>;

fn state_json(s: &PmState, ws: &str, times: &Times) -> Value {
    let (created, updated) = times.get(&s.id).cloned().unwrap_or_default();
    json!({
        "id": s.id, "created_at": created, "updated_at": updated, "deleted_at": null, "name": s.name, "description": s.description,
        "color": s.color, "slug": s.name.to_lowercase().split_whitespace().collect::<Vec<_>>().join("-"), "sequence": s.sequence, "group": s.group,
        "is_triage": s.group == StateGroup::Triage, "default": s.is_default, "external_source": null, "external_id": null, "created_by": null, "updated_by": null,
        "project": s.project_id, "workspace": ws,
    })
}

fn label_json(l: &PmLabel, ws: &str, times: &Times) -> Value {
    let (created, updated) = times.get(&l.id).cloned().unwrap_or_default();
    json!({
        "id": l.id, "created_at": created, "updated_at": updated, "deleted_at": null, "name": l.name, "description": l.description,
        "color": l.color, "sort_order": l.sort_order, "external_source": null, "external_id": null, "created_by": null, "updated_by": null,
        "workspace": ws, "project": l.project_id, "parent": l.parent_id,
    })
}

/// Work items by state group: backlog, unstarted, started, completed, cancelled.
type Groups = HashMap<String, [u32; 5]>;

fn counts(json: &mut Value, g: Option<&[u32; 5]>) {
    let g = g.copied().unwrap_or_default();
    json["total_issues"] = json!(g.iter().sum::<u32>());
    for (key, n) in ["backlog_issues", "unstarted_issues", "started_issues", "completed_issues", "cancelled_issues"].iter().zip(g) {
        json[*key] = json!(n);
    }
}

fn cycle_json(c: &PmCycle, ws: &str, groups: &Groups) -> Value {
    let status = match c.status {
        CycleStatus::Active => "CURRENT",
        CycleStatus::Upcoming => "UPCOMING",
        CycleStatus::Completed => "COMPLETED",
        CycleStatus::Draft => "DRAFT",
    };
    let mut v = json!({
        "id": c.id, "created_at": c.created_at, "updated_at": c.updated_at, "deleted_at": null, "name": c.name, "description": c.description,
        "start_date": c.start_date, "end_date": c.end_date, "view_props": {}, "sort_order": 0, "external_source": null, "external_id": null,
        "progress_snapshot": {}, "archived_at": c.archived_at, "logo_props": {}, "timezone": "UTC", "version": 1,
        "created_by": null, "updated_by": null, "project": c.project_id, "workspace": ws, "owned_by": "me", "status": status, "is_favorite": c.is_favorite,
    });
    counts(&mut v, groups.get(&c.id));
    v
}

fn module_json(m: &PmModule, ws: &str, groups: &Groups) -> Value {
    let mut v = json!({
        "id": m.id, "created_at": m.created_at, "updated_at": m.updated_at, "deleted_at": null, "name": m.name, "description": m.description,
        "description_text": null, "description_html": null, "start_date": m.start_date, "target_date": m.target_date,
        "status": serde_json::to_value(m.status).ok().and_then(|s| s.as_str().map(|s| s.replace('_', "-"))), "view_props": {}, "sort_order": m.sort_order,
        "external_source": null, "external_id": null, "archived_at": m.archived_at, "logo_props": {}, "created_by": null, "updated_by": null,
        "project": m.project_id, "workspace": ws, "lead": m.lead, "members": m.members, "is_favorite": m.is_favorite,
    });
    counts(&mut v, groups.get(&m.id));
    v
}

/// Plane's numbers: -2 pending, -1 declined, 0 snoozed, 1 accepted, 2 duplicate.
fn code(status: IntakeStatus) -> i64 {
    match status {
        IntakeStatus::Pending => -2,
        IntakeStatus::Declined => -1,
        IntakeStatus::Snoozed => 0,
        IntakeStatus::Accepted => 1,
        IntakeStatus::Duplicate => 2,
    }
}

fn intake_json(x: &PmIntakeItem, ws: &str) -> Value {
    let status = code(x.status);
    json!({
        "id": x.item.id, "issue": x.item.id, "issue_detail": issue_json(&x.item, ws), "intake": x.item.project_id,
        "status": status, "snoozed_till": x.snoozed_until, "duplicate_to": x.duplicate_of,
        "source": x.source.to_uppercase(), "source_email": x.email_sender, "external_source": null, "external_id": null, "extra": {},
        "created_at": x.item.created_at, "updated_at": x.item.updated_at, "deleted_at": null,
        "created_by": x.item.created_by, "updated_by": x.item.updated_by, "project": x.item.project_id, "workspace": ws,
    })
}

// ---- Reads (the Projects module's own, D-084) ----

async fn times(pool: &SqlitePool, table: &'static str, project: &str) -> Result<Times, PlaneError> {
    Ok(settings::times(pool, table, project).await?)
}

async fn groups(pool: &SqlitePool, link: &'static str, column: &'static str, project: &str) -> Result<Groups, PlaneError> {
    Ok(pm::by_state_group(pool, link, column, project).await?)
}

async fn project_one(pool: &SqlitePool, ws: &str, id: &str) -> Result<Value, PlaneError> {
    let p = pm::get(pool, id).await?;
    Ok(project_json(&p, ws, pm::tallies(pool).await?.get(id).unwrap_or(&(0, 0, None))))
}

/// A work item of the project — not a request waiting in Intake (`intake-issues/` reads those).
fn work(item: PmItem) -> Result<PmItem, PlaneError> {
    if item.state_group == StateGroup::Triage {
        return Err(PlaneError::missing());
    }
    Ok(item)
}

/// "VIFC-12": the project by its identifier (any case), the item by its number.
async fn item_by_key(pool: &SqlitePool, key: &str) -> Result<PmItem, PlaneError> {
    let id = items::by_key(pool, key).await?.ok_or_else(PlaneError::missing)?;
    work(items::get(pool, &id).await?)
}

/// A work item of the project in the address.
async fn item_of(pool: &SqlitePool, project: &str, id: &str) -> Result<PmItem, PlaneError> {
    let item = items::get(pool, id).await.map_err(|_| PlaneError::missing())?;
    if item.project_id != project {
        return Err(PlaneError::missing());
    }
    work(item)
}

/// A webhook's `data`: the thing as the API reads it — a work item with its state, labels and
/// assignees as objects, as Plane's webhooks send it; None once it's gone.
pub async fn data(pool: &SqlitePool, ws: &str, kind: &str, id: &str) -> Option<Value> {
    let today = cycles::today();
    match kind {
        "project" => project_one(pool, ws, id).await.ok(),
        "issue" => {
            let i = items::get(pool, id).await.ok()?;
            let labels = settings::labels(pool, &i.project_id).await.ok()?;
            let mut people = Vec::new();
            for w in &i.assignees {
                people.push(json!({ "id": w, "display_name": items::names(pool, std::slice::from_ref(w)).await.unwrap_or_default() }));
            }
            let mut v = issue_json(&i, ws);
            v["state"] = json!({ "id": i.state_id, "name": i.state_name, "color": i.state_color, "group": i.state_group });
            v["labels"] = json!(i.labels.iter().filter_map(|l| labels.iter().find(|x| x.id == *l)).map(|l| json!({ "id": l.id, "name": l.name, "color": l.color })).collect::<Vec<_>>());
            v["assignees"] = json!(people);
            Some(v)
        }
        "cycle" => {
            let c = cycles::get(pool, id, today).await.ok()?;
            Some(cycle_json(&c, ws, &groups(pool, "pm_cycle_item", "cycle_id", &c.project_id).await.ok()?))
        }
        "module" => {
            let m = pm_modules::get(pool, id).await.ok()?;
            Some(module_json(&m, ws, &groups(pool, "pm_module_item", "module_id", &m.project_id).await.ok()?))
        }
        "issue_comment" => {
            let c = comments::one(pool, id).await.ok()??;
            Some(json!({
                "id": id, "comment_html": html_of(&c.body_text), "comment_stripped": c.body_text, "actor": c.created_by, "access": "INTERNAL",
                "issue": c.item_id, "project": c.project_id, "workspace": ws, "created_at": c.created_at, "updated_at": c.updated_at, "created_by": c.created_by, "updated_by": null,
            }))
        }
        _ => None,
    }
}

// ---- Writes: Plane's fields onto the module's ----

fn features(b: &Map<String, Value>, now: Features) -> Result<Features, PlaneError> {
    Ok(Features {
        cycles: flag(b, "cycle_view")?.unwrap_or(now.cycles),
        modules: flag(b, "module_view")?.unwrap_or(now.modules),
        views: flag(b, "issue_views_view")?.unwrap_or(now.views),
        pages: flag(b, "page_view")?.unwrap_or(now.pages),
        intake: flag(b, "intake_view")?.or(flag(b, "inbox_view")?).unwrap_or(now.intake),
    })
}

fn network(b: &Map<String, Value>) -> Result<Option<Network>, PlaneError> {
    match b.get("network") {
        None | Some(Value::Null) => Ok(None),
        Some(v) => match v.as_i64() {
            Some(2) => Ok(Some(Network::Public)),
            Some(0) => Ok(Some(Network::Private)),
            _ => Err(PlaneError::bad(format!("network: \"{v}\" is not a valid choice."))),
        },
    }
}

async fn create_project(pool: &SqlitePool, ws: &str, b: &Map<String, Value>, actor: &str) -> Answer {
    // Plane's new project: its cycles, modules, views and pages on, Intake off, public.
    let p = pm::create_by(
        pool,
        NewPmProject {
            name: required(text(b, "name")?, "name")?,
            identifier: required(text(b, "identifier")?, "identifier")?,
            description: text(b, "description")?.unwrap_or_default(),
            emoji: text(b, "emoji")?.unwrap_or_default(),
            cover: String::new(),
            network: network(b)?.unwrap_or(Network::Public),
            lead: text(b, "project_lead")?.filter(|l| !l.is_empty()),
            features: features(b, Features { cycles: true, modules: true, views: true, pages: true, intake: false })?,
        },
        actor,
    )
    .await?;
    made(project_one(pool, ws, &p.id).await?)
}

/// A whole number of months (0 = off).
fn months(b: &Map<String, Value>, k: &str) -> Result<Option<u32>, PlaneError> {
    match b.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_u64().and_then(|n| u32::try_from(n).ok()).map(Some).ok_or_else(|| PlaneError::bad(format!("{k}: a whole number of months is expected"))),
    }
}

async fn update_project(pool: &SqlitePool, p: &PmProject, b: &Map<String, Value>, actor: &str) -> Result<(), PlaneError> {
    let mut automations = p.automations.clone();
    if let Some(n) = months(b, "archive_in")? {
        automations.archive_in = n;
    }
    if let Some(n) = months(b, "close_in")? {
        automations.close_in = n;
    }
    pm::update(
        pool,
        &p.id,
        PmProjectPatch {
            name: text(b, "name")?,
            identifier: text(b, "identifier")?,
            description: text(b, "description")?,
            emoji: text(b, "emoji")?,
            cover: None,
            network: network(b)?,
            lead: text(b, "project_lead")?,
            default_assignee: text(b, "default_assignee")?,
            subscribers: None,
            time_zone: text(b, "timezone")?,
            features: Some(features(b, p.features)?).filter(|f| *f != p.features),
            automations: Some(automations).filter(|a| *a != p.automations),
            favorite: None,
            sort_order: None,
        },
        actor,
    )
    .await?;
    Ok(())
}

fn item_patch(b: &Map<String, Value>) -> Result<ItemPatch, PlaneError> {
    Ok(ItemPatch {
        name: text(b, "name")?,
        description: text(b, "description_html")?.map(|h| doc_of(&h)),
        state_id: text(b, "state")?,
        priority: choice(b, "priority")?,
        assignees: texts(b, "assignees")?,
        labels: texts(b, "labels")?,
        start_date: text(b, "start_date")?,
        target_date: text(b, "target_date")?,
        estimate_point_id: text(b, "estimate_point")?,
        parent_id: text(b, "parent")?,
        sort_order: number(b, "sort_order")?,
        cycle_id: None,
        modules: None,
    })
}

/// The work items a body names, each once.
fn issues_of(b: &Map<String, Value>) -> Result<Vec<String>, PlaneError> {
    let mut ids = texts(b, "issues")?.filter(|i| !i.is_empty()).ok_or_else(|| PlaneError::bad("issues: This field is required."))?;
    let mut seen = std::collections::HashSet::new();
    ids.retain(|i| seen.insert(i.clone()));
    Ok(ids)
}

/// A name as the module keeps it (spaces collapsed; the module compares in full, any case).
fn kept(name: &str) -> String {
    name.split_whitespace().collect::<Vec<_>>().join(" ")
}

async fn dispatch(c: &Caller, m: &Method, p: &HashMap<String, String>, q: &HashMap<String, String>, b: &Map<String, Value>) -> Answer {
    // HEAD reads as GET (the body is left off by the server).
    let get = Method::GET;
    let m = if *m == Method::HEAD { &get } else { m };
    let at = |k: &str| p.get(k).map(String::as_str);
    let (pool, ws, actor) = (&c.pool, c.workspace_id.as_str(), c.actor.as_str());
    let today = cycles::today();
    if let Some(key) = at("key") {
        return match *m {
            Method::GET => ok(issue_json(&item_by_key(pool, key).await?, ws)),
            _ => not_allowed(m),
        };
    }
    let Some(project) = at("project") else {
        return match *m {
            Method::GET => {
                let tallies = pm::tallies(pool).await?;
                let all = pm::list(pool).await?.iter().filter(|p| p.archived_at.is_none()).map(|p| project_json(p, ws, tallies.get(&p.id).unwrap_or(&(0, 0, None)))).collect();
                page(all, q)
            }
            Method::POST => create_project(pool, ws, b, actor).await,
            _ => not_allowed(m),
        };
    };
    let current = pm::get(pool, project).await.map_err(|_| PlaneError::missing())?;
    match (at("kind"), at("id"), at("sub"), at("item")) {
        (None, ..) => match *m {
            Method::GET => ok(project_one(pool, ws, project).await?),
            Method::PATCH => {
                update_project(pool, &current, b, actor).await?;
                ok(project_one(pool, ws, project).await?)
            }
            Method::DELETE => {
                pm::delete(pool, &c.folder, project, actor).await?;
                gone()
            }
            _ => not_allowed(m),
        },
        (Some("archive"), None, ..) => match *m {
            Method::POST | Method::DELETE => {
                pm::archive(pool, project, *m == Method::POST, actor).await?;
                gone()
            }
            _ => not_allowed(m),
        },
        (Some("issues" | "work-items"), None, ..) => match *m {
            Method::GET => page(items::list(pool, project, false).await?.iter().map(|i| issue_json(i, ws)).collect(), q),
            Method::POST => {
                let input = ItemInput {
                    project_id: project.to_owned(),
                    name: required(text(b, "name")?, "name")?,
                    description: text(b, "description_html")?.map(|h| doc_of(&h)),
                    state_id: text(b, "state")?.filter(|s| !s.is_empty()),
                    priority: choice(b, "priority")?,
                    assignees: texts(b, "assignees")?.unwrap_or_default(),
                    labels: texts(b, "labels")?.unwrap_or_default(),
                    start_date: text(b, "start_date")?.filter(|s| !s.is_empty()),
                    target_date: text(b, "target_date")?.filter(|s| !s.is_empty()),
                    estimate_point_id: text(b, "estimate_point")?.filter(|s| !s.is_empty()),
                    parent_id: text(b, "parent")?.filter(|s| !s.is_empty()),
                    cycle_id: None,
                    modules: Vec::new(),
                };
                made(issue_json(&items::create(pool, input, actor).await?, ws))
            }
            _ => not_allowed(m),
        },
        (Some("issues" | "work-items"), Some(id), None, _) => {
            let it = item_of(pool, project, id).await?;
            match *m {
                Method::GET => ok(issue_json(&it, ws)),
                Method::PATCH => {
                    // A description sent back as it was read (the HTML of its words) keeps the rich
                    // one: pictures, tables and mentions aren't flattened by a round trip.
                    let mut patch = item_patch(b)?;
                    if let Some(Value::String(html)) = b.get("description_html") {
                        if words(&text_of(html)) == words(&it.description_text) {
                            patch.description = None;
                        }
                    }
                    ok(issue_json(&items::update(pool, id, patch, actor).await?, ws))
                }
                Method::DELETE => {
                    items::delete(pool, &c.folder, id, actor).await?;
                    gone()
                }
                _ => not_allowed(m),
            }
        }
        (Some("states"), None, ..) => match *m {
            Method::GET => {
                let t = times(pool, "pm_state", project).await?;
                // Intake's Triage state is the module's own, never one to pick (D-081).
                page(settings::states(pool, project).await?.iter().filter(|s| s.group != StateGroup::Triage).map(|s| state_json(s, ws, &t)).collect(), q)
            }
            Method::POST => {
                let name = required(text(b, "name")?, "name")?;
                let input = StateInput { name: name.clone(), description: text(b, "description")?.unwrap_or_default(), color: text(b, "color")?.unwrap_or_else(|| "gray".into()), group: choice(b, "group")?.ok_or_else(|| PlaneError::bad("group: This field is required."))? };
                let all = settings::create_state(pool, project, input).await?;
                let t = times(pool, "pm_state", project).await?;
                let s = all.iter().find(|s| settings::same(&s.name, &kept(&name))).ok_or_else(PlaneError::missing)?;
                made(state_json(s, ws, &t))
            }
            _ => not_allowed(m),
        },
        (Some("states"), Some(id), None, _) => {
            let found = settings::states(pool, project).await?.into_iter().find(|s| s.id == id && s.group != StateGroup::Triage).ok_or_else(PlaneError::missing)?;
            match *m {
                Method::GET => ok(state_json(&found, ws, &times(pool, "pm_state", project).await?)),
                Method::PATCH => {
                    if b.contains_key("name") || b.contains_key("description") || b.contains_key("color") {
                        settings::update_state(pool, id, StatePatch { name: text(b, "name")?, description: text(b, "description")?, color: text(b, "color")? }).await?;
                    }
                    if flag(b, "default")? == Some(true) {
                        settings::set_default_state(pool, id).await?;
                    }
                    let s = settings::states(pool, project).await?.into_iter().find(|s| s.id == id).ok_or_else(PlaneError::missing)?;
                    ok(state_json(&s, ws, &times(pool, "pm_state", project).await?))
                }
                Method::DELETE => {
                    settings::delete_state(pool, id).await?;
                    gone()
                }
                _ => not_allowed(m),
            }
        }
        (Some("labels"), None, ..) => match *m {
            Method::GET => {
                let t = times(pool, "pm_label", project).await?;
                page(settings::labels(pool, project).await?.iter().map(|l| label_json(l, ws, &t)).collect(), q)
            }
            Method::POST => {
                let name = required(text(b, "name")?, "name")?;
                let parent = text(b, "parent")?.filter(|s| !s.is_empty());
                let input = LabelInput { name: name.clone(), description: text(b, "description")?.unwrap_or_default(), color: text(b, "color")?.unwrap_or_else(|| "gray".into()), parent_id: parent.clone() };
                let all = settings::create_label(pool, project, input).await?;
                let l = all.iter().find(|l| settings::same(&l.name, &kept(&name)) && l.parent_id == parent).ok_or_else(PlaneError::missing)?;
                made(label_json(l, ws, &times(pool, "pm_label", project).await?))
            }
            _ => not_allowed(m),
        },
        (Some("labels"), Some(id), None, _) => {
            let l = settings::labels(pool, project).await?.into_iter().find(|l| l.id == id).ok_or_else(PlaneError::missing)?;
            match *m {
                Method::GET => ok(label_json(&l, ws, &times(pool, "pm_label", project).await?)),
                Method::PATCH => {
                    let input = LabelInput {
                        name: text(b, "name")?.unwrap_or(l.name.clone()),
                        description: text(b, "description")?.unwrap_or(l.description.clone()),
                        color: text(b, "color")?.unwrap_or(l.color.clone()),
                        parent_id: match text(b, "parent")? {
                            Some(p) if p.is_empty() => None,
                            Some(p) => Some(p),
                            None => l.parent_id.clone(),
                        },
                    };
                    let all = settings::update_label(pool, id, input).await?;
                    let l = all.iter().find(|x| x.id == id).ok_or_else(PlaneError::missing)?;
                    ok(label_json(l, ws, &times(pool, "pm_label", project).await?))
                }
                Method::DELETE => {
                    settings::delete_label(pool, id).await?;
                    gone()
                }
                _ => not_allowed(m),
            }
        }
        (Some("cycles"), None, ..) => match *m {
            Method::GET => {
                let g = groups(pool, "pm_cycle_item", "cycle_id", project).await?;
                page(cycles::list(pool, project, today).await?.iter().filter(|c| c.archived_at.is_none()).map(|c| cycle_json(c, ws, &g)).collect(), q)
            }
            Method::POST => {
                let input = CycleInput {
                    project_id: project.to_owned(),
                    name: required(text(b, "name")?, "name")?,
                    description: text(b, "description")?.unwrap_or_default(),
                    start_date: text(b, "start_date")?.filter(|s| !s.is_empty()).map(|d| d.chars().take(10).collect()),
                    end_date: text(b, "end_date")?.filter(|s| !s.is_empty()).map(|d| d.chars().take(10).collect()),
                };
                let made_one = cycles::create(pool, input, actor, today).await?;
                made(cycle_json(&made_one, ws, &Groups::new()))
            }
            _ => not_allowed(m),
        },
        (Some("cycles"), Some(id), sub, item) => {
            let cycle = cycles::get(pool, id, today).await.map_err(|_| PlaneError::missing())?;
            if cycle.project_id != project {
                return Err(PlaneError::missing());
            }
            match (sub, item, m.clone()) {
                (None, _, Method::GET) => ok(cycle_json(&cycle, ws, &groups(pool, "pm_cycle_item", "cycle_id", project).await?)),
                (None, _, Method::PATCH) => {
                    let patch = CyclePatch {
                        name: text(b, "name")?,
                        description: text(b, "description")?,
                        start_date: text(b, "start_date")?.map(|d| d.chars().take(10).collect()),
                        end_date: text(b, "end_date")?.map(|d| d.chars().take(10).collect()),
                        is_favorite: None,
                    };
                    let c = cycles::update(pool, id, patch, actor, today).await?;
                    ok(cycle_json(&c, ws, &groups(pool, "pm_cycle_item", "cycle_id", project).await?))
                }
                (None, _, Method::DELETE) => {
                    cycles::delete(pool, id, actor, today).await?;
                    gone()
                }
                (Some("cycle-issues"), None, Method::GET) => page(items::list(pool, project, false).await?.iter().filter(|i| i.cycle_id.as_deref() == Some(id)).map(|i| issue_json(i, ws)).collect(), q),
                (Some("cycle-issues"), None, Method::POST) => {
                    let ids = issues_of(b)?;
                    cycles::add_items(pool, id, &ids, actor, today).await?;
                    let mut out = Vec::new();
                    for i in &ids {
                        out.push(issue_json(&items::get(pool, i).await?, ws));
                    }
                    made(json!(out))
                }
                (Some("cycle-issues"), Some(i), Method::DELETE) => {
                    let it = item_of(pool, project, i).await?;
                    if it.cycle_id.as_deref() != Some(id) {
                        return Err(PlaneError::missing());
                    }
                    items::update(pool, i, ItemPatch { cycle_id: Some(String::new()), ..ItemPatch::default() }, actor).await?;
                    gone()
                }
                (Some("cycle-issues"), ..) | (None, ..) => not_allowed(m),
                _ => Err(PlaneError::missing()),
            }
        }
        (Some("modules"), None, ..) => match *m {
            Method::GET => {
                let g = groups(pool, "pm_module_item", "module_id", project).await?;
                page(pm_modules::list(pool, project).await?.iter().filter(|x| x.archived_at.is_none()).map(|x| module_json(x, ws, &g)).collect(), q)
            }
            Method::POST => {
                let input = ModuleInput {
                    project_id: project.to_owned(),
                    name: required(text(b, "name")?, "name")?,
                    description: text(b, "description")?.unwrap_or_default(),
                    start_date: text(b, "start_date")?.filter(|s| !s.is_empty()),
                    target_date: text(b, "target_date")?.filter(|s| !s.is_empty()),
                    status: choice(b, "status")?.unwrap_or(pm_modules::ModuleStatus::Backlog),
                    lead: text(b, "lead")?.filter(|s| !s.is_empty()),
                    members: texts(b, "members")?.unwrap_or_default(),
                };
                made(module_json(&pm_modules::create(pool, input, actor).await?, ws, &Groups::new()))
            }
            _ => not_allowed(m),
        },
        (Some("modules"), Some(id), sub, item) => {
            let module = pm_modules::get(pool, id).await.map_err(|_| PlaneError::missing())?;
            if module.project_id != project {
                return Err(PlaneError::missing());
            }
            match (sub, item, m.clone()) {
                (None, _, Method::GET) => ok(module_json(&module, ws, &groups(pool, "pm_module_item", "module_id", project).await?)),
                (None, _, Method::PATCH) => {
                    let patch = ModulePatch {
                        name: text(b, "name")?,
                        description: text(b, "description")?,
                        start_date: text(b, "start_date")?,
                        target_date: text(b, "target_date")?,
                        status: choice(b, "status")?,
                        lead: text(b, "lead")?,
                        members: texts(b, "members")?,
                        sort_order: number(b, "sort_order")?,
                    };
                    let x = pm_modules::update(pool, id, patch, actor).await?;
                    ok(module_json(&x, ws, &groups(pool, "pm_module_item", "module_id", project).await?))
                }
                (None, _, Method::DELETE) => {
                    pm_modules::delete(pool, id, actor).await?;
                    gone()
                }
                (Some("module-issues"), None, Method::GET) => page(items::list(pool, project, false).await?.iter().filter(|i| i.modules.iter().any(|x| x.id == id)).map(|i| issue_json(i, ws)).collect(), q),
                (Some("module-issues"), None, Method::POST) => {
                    let ids = issues_of(b)?;
                    pm_modules::add_items(pool, id, &ids, actor).await?;
                    let mut out = Vec::new();
                    for i in &ids {
                        out.push(issue_json(&items::get(pool, i).await?, ws));
                    }
                    made(json!(out))
                }
                (Some("module-issues"), Some(i), Method::DELETE) => {
                    let it = item_of(pool, project, i).await?;
                    if !it.modules.iter().any(|x| x.id == id) {
                        return Err(PlaneError::missing());
                    }
                    let keep = it.modules.iter().filter(|x| x.id != id).map(|x| x.id.clone()).collect();
                    items::update(pool, i, ItemPatch { modules: Some(keep), ..ItemPatch::default() }, actor).await?;
                    gone()
                }
                (Some("module-issues"), ..) | (None, ..) => not_allowed(m),
                _ => Err(PlaneError::missing()),
            }
        }
        (Some("intake-issues"), None, ..) => match *m {
            Method::GET => page(intake::list(pool, project, today).await?.iter().map(|x| intake_json(x, ws)).collect(), q),
            Method::POST => {
                // Plane's body: { "issue": { "name", "description_html", "priority" } }.
                let empty = Map::new();
                let issue = b.get("issue").and_then(Value::as_object).unwrap_or(&empty);
                let input = IntakeInput {
                    project_id: project.to_owned(),
                    name: required(text(issue, "name")?, "issue.name")?,
                    description: text(issue, "description_html")?.map(|h| doc_of(&h)),
                    priority: choice(issue, "priority")?,
                    assignees: Vec::new(),
                    labels: Vec::new(),
                    target_date: text(issue, "target_date")?.filter(|s| !s.is_empty()),
                };
                made(intake_json(&intake::create(pool, input, actor, today).await?, ws))
            }
            _ => not_allowed(m),
        },
        (Some("intake-issues"), Some(id), None, _) => {
            let x = intake::get(pool, id, today).await.map_err(|_| PlaneError::missing())?;
            if x.item.project_id != project {
                return Err(PlaneError::missing());
            }
            match *m {
                Method::GET => ok(intake_json(&x, ws)),
                Method::PATCH => {
                    let empty = Map::new();
                    let issue = b.get("issue").and_then(Value::as_object).unwrap_or(&empty);
                    // Everything is checked before anything is written. The status it has already
                    // is nothing to do (a client sending back what it read).
                    let wanted = match b.get("status") {
                        None | Some(Value::Null) => None,
                        Some(v) => Some(v.as_i64().ok_or_else(|| PlaneError::bad("status: a number is expected"))?),
                    }
                    .filter(|n| *n != code(x.status));
                    if !matches!(wanted, None | Some(-1..=2)) {
                        return Err(PlaneError::bad(format!("status: \"{}\" is not a valid choice.", wanted.unwrap_or_default())));
                    }
                    if wanted.is_some() && matches!(x.status, IntakeStatus::Accepted | IntakeStatus::Declined | IntakeStatus::Duplicate) {
                        return Err(PlaneError::bad("This request is closed: its status stays as it is."));
                    }
                    let state = text(issue, "state")?.filter(|s| !s.is_empty());
                    if let (Some(1), Some(s)) = (wanted, &state) {
                        if !settings::states(pool, project).await?.iter().any(|x| x.id == *s && x.group != StateGroup::Triage) {
                            return Err(PlaneError::bad("state: not one of the project's states"));
                        }
                    }
                    let until = match wanted {
                        Some(0) => {
                            let day: String = required(text(b, "snoozed_till")?, "snoozed_till")?.chars().take(10).collect();
                            match chrono::NaiveDate::parse_from_str(&day, "%Y-%m-%d") {
                                Ok(d) if d > today => d.to_string(),
                                Ok(_) => return Err(PlaneError::bad("snoozed_till: a day after today is expected")),
                                Err(_) => return Err(PlaneError::bad("snoozed_till: a date is expected")),
                            }
                        }
                        _ => String::new(),
                    };
                    let duplicate = match wanted {
                        Some(2) => item_of(pool, project, &required(text(b, "duplicate_to")?, "duplicate_to")?).await.map_err(|_| PlaneError::bad("duplicate_to: not a work item of this project"))?.id,
                        _ => String::new(),
                    };
                    // A request's own fields (its title, description, priority) while it waits.
                    if issue.keys().any(|k| matches!(k.as_str(), "name" | "description_html" | "priority")) {
                        let patch = ItemPatch { name: text(issue, "name")?, description: text(issue, "description_html")?.map(|h| doc_of(&h)), priority: choice(issue, "priority")?, ..ItemPatch::default() };
                        items::update(pool, id, patch, actor).await?;
                    }
                    match wanted {
                        Some(1) => {
                            intake::accept(pool, id, state.as_deref(), actor, today).await?;
                        }
                        Some(-1) => {
                            intake::decline(pool, id, actor, today).await?;
                        }
                        Some(0) => {
                            intake::snooze(pool, id, &until, actor, today).await?;
                        }
                        Some(2) => {
                            intake::duplicate(pool, id, &duplicate, actor, today).await?;
                        }
                        _ => {}
                    }
                    ok(intake_json(&intake::get(pool, id, today).await?, ws))
                }
                Method::DELETE => {
                    intake::delete(pool, &c.folder, id, today).await?;
                    gone()
                }
                _ => not_allowed(m),
            }
        }
        _ => Err(PlaneError::missing()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::hub::WorkspaceHub;

    async fn call(c: &Caller, m: Method, path: &[(&str, &str)], query: &[(&str, &str)], body: Value) -> Answer {
        let p = path.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
        let q = query.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
        let b = match body {
            Value::Object(m) => m,
            _ => Map::new(),
        };
        dispatch(c, &m, &p, &q, &b).await
    }

    /// The address's parts below the project.
    fn at<'a>(pid: &'a str, rest: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
        [("project", pid)].into_iter().chain(rest.iter().copied()).collect()
    }

    fn status(a: &Answer) -> u16 {
        match a {
            Ok((s, _)) => s.as_u16(),
            Err(e) => e.status(),
        }
    }

    fn body(a: Answer) -> Value {
        a.map(|(_, v)| v).unwrap_or_else(|e| e.body)
    }

    /// Plane's paths and JSON: a project made and listed in Plane's page; work items (by `issues/`,
    /// `work-items/` and their key) made, paged by cursor, changed and cleared with null; states,
    /// labels, cycles and modules with their work items; an Intake request accepted; errors in
    /// Plane's words; the rate of 60 a minute.
    #[test]
    fn answers_like_plane() {
        let root = std::env::temp_dir().join(format!("newdawn-plane-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            let w = hub.workspace("test").await.unwrap();
            let c = Caller { pool: hub.pool("test").unwrap(), workspace_id: w.id.clone(), folder: w.folder_path, actor: "Script (API)".into() };
            let made = call(&c, Method::POST, &[], &[], json!({ "name": "VIFC Fintech Hub", "identifier": "VIFC" })).await;
            assert_eq!(status(&made), 201);
            let p = body(made);
            assert_eq!((p["identifier"].as_str(), p["network"].as_i64(), p["cycle_view"].as_bool(), p["intake_view"].as_bool()), (Some("VIFC"), Some(2), Some(true), Some(false)));
            assert!(p["default_state"].is_string(), "the project's default state");
            let pid = p["id"].as_str().unwrap().to_owned();
            let list = body(call(&c, Method::GET, &[], &[], Value::Null).await);
            assert_eq!((list["total_count"].as_u64(), list["count"].as_u64(), list["next_page_results"].as_bool(), list["results"][0]["identifier"].as_str()), (Some(1), Some(1), Some(false), Some("VIFC")));

            let first = call(&c, Method::POST, &at(&pid, &[("kind", "issues")]), &[], json!({ "name": "Draft the rules", "priority": "high", "description_html": "<p>Ask <b>Minh</b></p><p>by Friday &amp; Monday</p>" })).await;
            assert_eq!(status(&first), 201);
            let first = body(first);
            assert_eq!((first["sequence_id"].as_u64(), first["priority"].as_str()), (Some(1), Some("high")));
            assert_eq!(first["description_html"], "<p>Ask Minh</p><p>by Friday &amp; Monday</p>");
            let one = first["id"].as_str().unwrap().to_owned();
            let second = body(call(&c, Method::POST, &at(&pid, &[("kind", "work-items")]), &[], json!({ "name": "Book the room" })).await);
            let two = second["id"].as_str().unwrap().to_owned();
            let page1 = body(call(&c, Method::GET, &at(&pid, &[("kind", "issues")]), &[("per_page", "1")], Value::Null).await);
            assert_eq!((page1["count"].as_u64(), page1["next_cursor"].as_str(), page1["next_page_results"].as_bool(), page1["total_pages"].as_u64()), (Some(1), Some("1:1:0"), Some(true), Some(2)));
            let page2 = body(call(&c, Method::GET, &at(&pid, &[("kind", "work-items")]), &[("cursor", "1:1:0")], Value::Null).await);
            assert_eq!((page2["results"][0]["name"].as_str(), page2["prev_page_results"].as_bool(), page2["next_page_results"].as_bool()), (Some("Book the room"), Some(true), Some(false)));
            let names = body(call(&c, Method::GET, &at(&pid, &[("kind", "issues")]), &[("order_by", "-sequence_id"), ("fields", "id,name")], Value::Null).await);
            assert_eq!(names["results"][0], json!({ "id": two, "name": "Book the room" }), "ordered, and only the fields asked for");
            assert_eq!(body(call(&c, Method::GET, &[("key", "vifc-1")], &[], Value::Null).await)["id"], one.as_str());
            assert_eq!(first["workspace"], w.id.as_str(), "Plane's workspace is its id");

            // A description sent back as it was read keeps the rich one (its bold, here).
            let rich = r#"{"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Ask "},{"type":"text","text":"Minh","marks":[{"type":"bold"}]}]}]}"#;
            items::update(&c.pool, &two, ItemPatch { description: Some(rich.into()), ..ItemPatch::default() }, "me").await.unwrap();
            let read = body(call(&c, Method::GET, &at(&pid, &[("kind", "issues"), ("id", two.as_str())]), &[], Value::Null).await);
            call(&c, Method::PATCH, &at(&pid, &[("kind", "issues"), ("id", two.as_str())]), &[], json!({ "name": "Book the big room", "description_html": read["description_html"] })).await.unwrap();
            assert!(items::get(&c.pool, &two).await.unwrap().description.unwrap().0.to_string().contains("bold"), "not flattened by a round trip");

            let states = body(call(&c, Method::GET, &at(&pid, &[("kind", "states")]), &[], Value::Null).await);
            let done = states["results"].as_array().unwrap().iter().find(|s| s["name"] == "Done").unwrap();
            assert_eq!(done["group"], "completed");
            let done = done["id"].as_str().unwrap().to_owned();
            let changed = body(call(&c, Method::PATCH, &at(&pid, &[("kind", "issues"), ("id", one.as_str())]), &[], json!({ "state": done, "target_date": "2026-11-30" })).await);
            assert_eq!((changed["state"].as_str(), changed["target_date"].as_str()), (Some(done.as_str()), Some("2026-11-30")));
            assert!(changed["completed_at"].is_string());
            let cleared = body(call(&c, Method::PATCH, &at(&pid, &[("kind", "issues"), ("id", one.as_str())]), &[], json!({ "target_date": null })).await);
            assert!(cleared["target_date"].is_null(), "null clears, as Plane's");
            let review = call(&c, Method::POST, &at(&pid, &[("kind", "states")]), &[], json!({ "name": "In Review", "group": "started", "color": "blue" })).await;
            assert_eq!((status(&review), body(review)["group"].as_str().map(str::to_owned)), (201, Some("started".into())));
            let legal = body(call(&c, Method::POST, &at(&pid, &[("kind", "labels")]), &[], json!({ "name": "Legal", "color": "red" })).await);
            let legal_id = legal["id"].as_str().unwrap().to_owned();
            assert_eq!(body(call(&c, Method::PATCH, &at(&pid, &[("kind", "labels"), ("id", legal_id.as_str())]), &[], json!({ "color": "green" })).await)["name"], "Legal");

            let today = cycles::today();
            let sprint = body(call(&c, Method::POST, &at(&pid, &[("kind", "cycles")]), &[], json!({ "name": "Sprint 1", "start_date": today.to_string(), "end_date": (today + chrono::Duration::days(6)).to_string() })).await);
            assert_eq!(sprint["status"], "CURRENT");
            let sid = sprint["id"].as_str().unwrap().to_owned();
            assert_eq!(status(&call(&c, Method::POST, &at(&pid, &[("kind", "cycles"), ("id", sid.as_str()), ("sub", "cycle-issues")]), &[], json!({ "issues": [two] })).await), 201);
            assert_eq!(body(call(&c, Method::GET, &at(&pid, &[("kind", "cycles"), ("id", sid.as_str())]), &[], Value::Null).await)["total_issues"], 1);
            assert_eq!(status(&call(&c, Method::DELETE, &at(&pid, &[("kind", "cycles"), ("id", sid.as_str()), ("sub", "cycle-issues"), ("item", two.as_str())]), &[], Value::Null).await), 204);
            assert_eq!(body(call(&c, Method::GET, &at(&pid, &[("kind", "cycles"), ("id", sid.as_str())]), &[], Value::Null).await)["total_issues"], 0);
            let module = body(call(&c, Method::POST, &at(&pid, &[("kind", "modules")]), &[], json!({ "name": "Licensing", "status": "in-progress" })).await);
            assert_eq!(module["status"], "in-progress");
            let mid = module["id"].as_str().unwrap().to_owned();
            call(&c, Method::POST, &at(&pid, &[("kind", "modules"), ("id", mid.as_str()), ("sub", "module-issues")]), &[], json!({ "issues": [two] })).await.unwrap();
            assert_eq!(body(call(&c, Method::GET, &at(&pid, &[("kind", "modules"), ("id", mid.as_str()), ("sub", "module-issues")]), &[], Value::Null).await)["count"], 1);

            call(&c, Method::PATCH, &at(&pid, &[]), &[], json!({ "intake_view": true })).await.unwrap();
            let request = call(&c, Method::POST, &at(&pid, &[("kind", "intake-issues")]), &[], json!({ "issue": { "name": "Add the pilot list", "priority": "medium" } })).await;
            assert_eq!(status(&request), 201);
            let request = body(request);
            assert_eq!(request["status"], -2, "pending");
            let rid = request["id"].as_str().unwrap().to_owned();
            assert_eq!(status(&call(&c, Method::GET, &at(&pid, &[("kind", "issues"), ("id", rid.as_str())]), &[], Value::Null).await), 404, "a request waiting in Intake isn't among the work items");
            let states = body(call(&c, Method::GET, &at(&pid, &[("kind", "states")]), &[], Value::Null).await);
            assert!(states["results"].as_array().unwrap().iter().all(|s| s["group"] != "triage"), "Intake's own state isn't listed");
            let early = call(&c, Method::PATCH, &at(&pid, &[("kind", "intake-issues"), ("id", rid.as_str())]), &[], json!({ "status": 0, "snoozed_till": today.to_string(), "issue": { "name": "Renamed too soon" } })).await;
            assert_eq!(status(&early), 400);
            assert_eq!(items::get(&c.pool, &rid).await.unwrap().name, "Add the pilot list", "checked before anything is written");
            assert_eq!(body(call(&c, Method::PATCH, &at(&pid, &[("kind", "intake-issues"), ("id", rid.as_str())]), &[], json!({ "status": -2 })).await)["status"], -2, "the status it has: nothing to do");
            let accepted = body(call(&c, Method::PATCH, &at(&pid, &[("kind", "intake-issues"), ("id", rid.as_str())]), &[], json!({ "status": 1 })).await);
            assert_eq!(accepted["status"], 1);
            assert_eq!(accepted["issue_detail"]["state"], p["default_state"], "into the project's default state");

            assert_eq!(status(&call(&c, Method::GET, &at(&pid, &[("kind", "issues"), ("id", "nope")]), &[], Value::Null).await), 404);
            let missing = call(&c, Method::POST, &at(&pid, &[("kind", "issues")]), &[], json!({})).await;
            assert_eq!((status(&missing), body(missing)["error"].as_str().map(str::to_owned)), (400, Some("name: This field is required.".into())));
            assert_eq!(status(&call(&c, Method::POST, &at(&pid, &[("kind", "states")]), &[], json!({ "name": "X", "group": "nope" })).await), 400);
            assert_eq!(status(&call(&c, Method::DELETE, &[], &[], Value::Null).await), 405);
            assert_eq!(status(&call(&c, Method::GET, &[("project", "nope")], &[], Value::Null).await), 404);
            assert_eq!(status(&call(&c, Method::DELETE, &at(&pid, &[("kind", "issues"), ("id", one.as_str())]), &[], Value::Null).await), 204);
            assert_eq!(status(&call(&c, Method::GET, &[("key", "VIFC-1")], &[], Value::Null).await), 404);
            hub.close_all().await;
        });
        let _ = std::fs::remove_dir_all(root);

        let key = format!("rate-{}", uuid::Uuid::now_v7());
        for n in 0..60 {
            assert_eq!(take(&key, 120), (true, 59 - n, 180));
        }
        assert_eq!(take(&key, 179), (false, 0, 180), "the 61st in the minute waits");
        assert_eq!(take(&key, 180), (true, 59, 240), "a new minute");
        assert_eq!(doc_of("<p>One</p><br><p>Two &lt;3</p>"), json!({ "type": "doc", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "One" }] }, { "type": "paragraph", "content": [{ "type": "text", "text": "Two <3" }] }] }).to_string());
        assert_eq!(unescape("It&#8217;s &#x2019;ok&#39; &amp; &bogus; &"), "It\u{2019}s \u{2019}ok' & &bogus; &");
    }
}
