//! The local REST API (D-071): Twenty's paths and JSON shapes on this Mac — `/rest/<objects>`
//! list, get, create (POST), update (PATCH), delete, `/rest/batch/<objects>` — with `filter`,
//! `orderBy`, `limit`, `starting_after` / `ending_before`; a Bearer key names the workspace.
//! Also the workflows' webhook trigger and the docs page.

use std::collections::HashMap;

use axum::body::Bytes;
use axum::extract::{Path, Query, Request, State};
use axum::http::{header::AUTHORIZATION, header::HOST, header::ORIGIN, HeaderMap, HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Map, Value};
use sqlx::SqlitePool;
use tauri::{AppHandle, Manager};

use super::{docs, filter, keys};
use crate::error::AppError;
use crate::kernel::hub::WorkspaceHub;
use crate::kernel::records::types::{subs, FieldType, Json as EngineJson};
use crate::kernel::records::{self, query, Actor, ObjectDef, Query as EngineQuery};
use crate::modules::workflows::triggers;

const DEFAULT_LIMIT: u32 = 60;
const MAX_LIMIT: u32 = 200;
/// Twenty's batch size.
const MAX_BATCH: usize = 60;
/// Fields a request may send but the engine keeps itself.
const KEPT_BY_THE_APP: [&str; 5] = ["id", "createdAt", "updatedAt", "deletedAt", "createdBy"];

/// An error in Twenty's shape: `{ statusCode, error, messages }`.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self { status, message: message.into() }
    }
    fn bad(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }
    fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }
    fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, message)
    }
    #[cfg(test)]
    pub fn status(&self) -> u16 {
        self.status.as_u16()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        match e.downcast_ref::<AppError>() {
            Some(a) if a.code == "not_found" => Self::not_found(a.message.clone()),
            Some(a) if a.code.starts_with("invalid") => Self::bad(a.message.clone()),
            _ => Self::new(StatusCode::INTERNAL_SERVER_ERROR, format!("{e:#}")),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let error = match self.status.as_u16() {
            400 => "BadRequestException",
            401 => "UnauthorizedException",
            403 => "ForbiddenException",
            404 => "NotFoundException",
            _ => "InternalServerErrorException",
        };
        (self.status, Json(json!({ "statusCode": self.status.as_u16(), "error": error, "messages": [self.message] }))).into_response()
    }
}

type Answer = Result<Value, ApiError>;

fn respond(answer: Answer, created: bool) -> Response {
    match answer {
        Ok(v) => (if created { StatusCode::CREATED } else { StatusCode::OK }, Json(v)).into_response(),
        Err(e) => e.into_response(),
    }
}

/// An object the API serves: by its plural (Twenty's paths) or singular name; not a system one.
async fn object_of(pool: &SqlitePool, objects: &str) -> Result<ObjectDef, ApiError> {
    let def = records::object(pool, objects).await.map_err(|_| ApiError::not_found(format!("No object named {objects}")))?;
    if def.meta.is_system || !def.meta.is_active {
        return Err(ApiError::not_found(format!("No object named {objects}")));
    }
    Ok(def)
}

/// `person` → `Person`, for Twenty's `createPerson` / `updatePerson` / `createPeople`.
fn cap(name: &str) -> String {
    let mut c = name.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

fn cursor(offset: u32) -> String {
    format!("c{offset:x}")
}

fn uncursor(text: &str) -> Option<u32> {
    u32::from_str_radix(text.strip_prefix('c')?, 16).ok()
}

/// A request's record as the engine takes it: Twenty's `companyId` as the `company` relation; the
/// app's own fields left out; an unknown field refused.
fn normalize(def: &ObjectDef, body: Value) -> Result<Value, ApiError> {
    let Value::Object(given) = body else { return Err(ApiError::bad("The body is a JSON object of the record's fields")) };
    let mut out = Map::new();
    for (key, value) in given {
        if KEPT_BY_THE_APP.contains(&key.as_str()) {
            continue;
        }
        if def.field(&key).is_some() {
            out.insert(key, value);
            continue;
        }
        if let Some(base) = key.strip_suffix("Id").filter(|b| def.field(b).is_some_and(|f| f.r#type == FieldType::Relation)) {
            out.insert(base.to_owned(), value);
            continue;
        }
        return Err(ApiError::bad(format!("{} has no field “{key}”", def.meta.label_singular)));
    }
    Ok(Value::Object(out))
}

fn body_json(body: &Bytes) -> Result<Value, ApiError> {
    if body.iter().all(u8::is_ascii_whitespace) {
        return Ok(json!({}));
    }
    serde_json::from_slice(body).map_err(|e| ApiError::bad(format!("The body isn't JSON: {e}")))
}

pub async fn list(pool: &SqlitePool, objects: &str, params: &HashMap<String, String>) -> Answer {
    let def = object_of(pool, objects).await?;
    let limit = match params.get("limit") {
        Some(l) => l.trim().parse::<u32>().map_err(|_| ApiError::bad("limit is a number"))?,
        None => DEFAULT_LIMIT,
    }
    .clamp(1, MAX_LIMIT);
    let filter = match params.get("filter").map(|f| f.trim()).filter(|f| !f.is_empty()) {
        Some(f) => Some(EngineJson(filter::parse_filter(f)?)),
        None => None,
    };
    let sort = match params.get("orderBy").map(|o| o.trim()).filter(|o| !o.is_empty()) {
        Some(o) => filter::parse_order(o)?,
        None => Vec::new(),
    };
    let mut limit = limit;
    let offset = match (params.get("starting_after"), params.get("ending_before")) {
        (Some(c), _) => uncursor(c).ok_or_else(|| ApiError::bad("starting_after isn't a cursor"))?.saturating_add(1),
        (None, Some(c)) => {
            // The page before the cursor, never the cursor's own record.
            let k = uncursor(c).ok_or_else(|| ApiError::bad("ending_before isn't a cursor"))?;
            limit = limit.min(k);
            k - limit
        }
        _ => 0,
    };
    let page = query::list(pool, &def, &EngineQuery { filter, sort, limit: Some(limit), offset: Some(offset), search: None, deleted: false }).await?;
    let records: Vec<Value> = page.records.into_iter().map(|r| r.0).collect();
    let n = u32::try_from(records.len()).unwrap_or(u32::MAX);
    let (start, end) = if n > 0 { (Some(cursor(offset)), Some(cursor(offset + n - 1))) } else { (None, None) };
    Ok(json!({
        "data": { def.meta.name_plural.as_str(): records },
        "pageInfo": { "hasNextPage": offset.saturating_add(n) < page.total, "hasPreviousPage": offset > 0, "startCursor": start, "endCursor": end },
        "totalCount": page.total
    }))
}

pub async fn get_one(pool: &SqlitePool, objects: &str, id: &str) -> Answer {
    let def = object_of(pool, objects).await?;
    let record = query::get(pool, &def, id, false).await.map_err(|_| ApiError::not_found(format!("No {} with the id {id}", def.meta.label_singular.to_lowercase())))?;
    Ok(json!({ "data": { def.meta.name_singular.as_str(): record } }))
}

pub async fn create(pool: &SqlitePool, objects: &str, body: Value, actor: &Actor) -> Answer {
    let def = object_of(pool, objects).await?;
    let made = query::create(pool, &def, &normalize(&def, body)?, actor).await?;
    Ok(json!({ "data": { format!("create{}", cap(&def.meta.name_singular)): made } }))
}

pub async fn update(pool: &SqlitePool, objects: &str, id: &str, body: Value, actor: &Actor) -> Answer {
    let def = object_of(pool, objects).await?;
    let current = query::get(pool, &def, id, false).await.map_err(|_| ApiError::not_found(format!("No {} with the id {id}", def.meta.label_singular.to_lowercase())))?;
    let mut data = normalize(&def, body)?;
    // Twenty's PATCH writes the parts it's sent: a composite's other parts stay as they were.
    if let Value::Object(fields) = &mut data {
        for (name, value) in fields.iter_mut() {
            let composite = def.field(name).is_some_and(|f| !subs(f.r#type).is_empty());
            if let (true, Some(old), Value::Object(given)) = (composite, current.get(name.as_str()).and_then(Value::as_object), &mut *value) {
                let mut merged = old.clone();
                merged.extend(std::mem::take(given));
                *given = merged;
            }
        }
    }
    let changed = query::update(pool, &def, id, &data, actor).await?;
    Ok(json!({ "data": { format!("update{}", cap(&def.meta.name_singular)): changed } }))
}

/// A delete as the app's: the record goes to Deleted records.
pub async fn delete(pool: &SqlitePool, objects: &str, id: &str, actor: &Actor) -> Answer {
    let def = object_of(pool, objects).await?;
    query::get(pool, &def, id, false).await.map_err(|_| ApiError::not_found(format!("No {} with the id {id}", def.meta.label_singular.to_lowercase())))?;
    query::delete(pool, &def, id, actor).await?;
    Ok(json!({ "data": { format!("delete{}", cap(&def.meta.name_singular)): { "id": id } } }))
}

/// Up to 60 records made in one call; every one is read before any is written.
pub async fn batch(pool: &SqlitePool, objects: &str, body: Value, actor: &Actor) -> Answer {
    let def = object_of(pool, objects).await?;
    let Value::Array(items) = body else { return Err(ApiError::bad("The body is a JSON array of records")) };
    if items.len() > MAX_BATCH {
        return Err(ApiError::bad(format!("At most {MAX_BATCH} records a call")));
    }
    let mut ready = Vec::with_capacity(items.len());
    for (i, item) in items.into_iter().enumerate() {
        let data = normalize(&def, item).map_err(|e| ApiError::bad(format!("Record {}: {}", i + 1, e.message)))?;
        // Every value checked as the engine will, before anything is written.
        for (name, value) in data.as_object().into_iter().flatten() {
            if let Some(f) = def.field(name) {
                query::check_value(f, value).map_err(|e| ApiError::bad(format!("Record {}: {e:#}", i + 1)))?;
            }
        }
        ready.push(data);
    }
    let mut made = Vec::with_capacity(ready.len());
    for (i, data) in ready.iter().enumerate() {
        let one = query::create(pool, &def, data, actor).await.map_err(ApiError::from).map_err(|e| ApiError::new(e.status, format!("Record {}: {} (the {} before it were made)", i + 1, e.message, i)))?;
        made.push(one);
    }
    Ok(json!({ "data": { format!("create{}", cap(&def.meta.name_plural)): made } }))
}

/// The workspace and key's name a request's Bearer key gives.
async fn authorised(app: &AppHandle, headers: &HeaderMap) -> Result<(SqlitePool, Actor), ApiError> {
    let token = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| ApiError::unauthorized("No API key — send Authorization: Bearer <key>"))?;
    let hub = app.state::<WorkspaceHub>();
    match keys::find(&hub, token).await? {
        Some((slug, name)) => Ok((hub.pool(&slug)?, Actor::api(&name))),
        None => Err(ApiError::unauthorized("This API key isn't valid: unknown, revoked or expired")),
    }
}

async fn list_h(State(app): State<AppHandle>, Path(objects): Path<String>, Query(params): Query<HashMap<String, String>>, headers: HeaderMap) -> Response {
    match authorised(&app, &headers).await {
        Ok((pool, _)) => respond(list(&pool, &objects, &params).await, false),
        Err(e) => e.into_response(),
    }
}

async fn get_h(State(app): State<AppHandle>, Path((objects, id)): Path<(String, String)>, headers: HeaderMap) -> Response {
    match authorised(&app, &headers).await {
        Ok((pool, _)) => respond(get_one(&pool, &objects, &id).await, false),
        Err(e) => e.into_response(),
    }
}

async fn create_h(State(app): State<AppHandle>, Path(objects): Path<String>, headers: HeaderMap, body: Bytes) -> Response {
    match (authorised(&app, &headers).await, body_json(&body)) {
        (Ok((pool, actor)), Ok(json)) => respond(create(&pool, &objects, json, &actor).await, true),
        (Err(e), _) | (_, Err(e)) => e.into_response(),
    }
}

async fn update_h(State(app): State<AppHandle>, Path((objects, id)): Path<(String, String)>, headers: HeaderMap, body: Bytes) -> Response {
    match (authorised(&app, &headers).await, body_json(&body)) {
        (Ok((pool, actor)), Ok(json)) => respond(update(&pool, &objects, &id, json, &actor).await, false),
        (Err(e), _) | (_, Err(e)) => e.into_response(),
    }
}

async fn delete_h(State(app): State<AppHandle>, Path((objects, id)): Path<(String, String)>, headers: HeaderMap) -> Response {
    match authorised(&app, &headers).await {
        Ok((pool, actor)) => respond(delete(&pool, &objects, &id, &actor).await, false),
        Err(e) => e.into_response(),
    }
}

async fn batch_h(State(app): State<AppHandle>, Path(objects): Path<String>, headers: HeaderMap, body: Bytes) -> Response {
    match (authorised(&app, &headers).await, body_json(&body)) {
        (Ok((pool, actor)), Ok(json)) => respond(batch(&pool, &objects, json, &actor).await, true),
        (Err(e), _) | (_, Err(e)) => e.into_response(),
    }
}

/// A workflow's webhook trigger (no key, like Twenty's; this Mac only): the body is the run's
/// trigger data; the run goes on by itself.
async fn workflow_hook_h(State(app): State<AppHandle>, Path((workspace, workflow)): Path<(String, String)>, body: Bytes) -> Response {
    let answer = async {
        let data = body_json(&body)?;
        let hub = app.state::<WorkspaceHub>();
        let pool = hub.pool(&workspace).map_err(|_| ApiError::not_found("No such workspace"))?;
        let run_id = triggers::on_webhook(&pool, &workflow, data).await?.ok_or_else(|| ApiError::not_found("No active workflow with a webhook trigger here"))?;
        Ok::<Value, ApiError>(json!({ "success": true, "workflowRunId": run_id }))
    }
    .await;
    respond(answer, false)
}

async fn docs_h(State(app): State<AppHandle>) -> Html<String> {
    let hub = app.state::<WorkspaceHub>();
    Html(docs::page(&hub, super::running_port().unwrap_or(0)).await)
}

/// A Host or Origin naming this Mac (localhost, 127.0.0.1, [::1]), with or without a port.
fn on_this_mac(v: &HeaderValue) -> bool {
    let v = v.to_str().unwrap_or("");
    let host = v.split_once("://").map_or(v, |(_, rest)| rest);
    let name = if host.starts_with('[') { host.split_once(']').map_or(host, |(n, _)| n).trim_start_matches('[') } else { host.rsplit_once(':').map_or(host, |(n, _)| n) };
    matches!(name, "localhost" | "127.0.0.1" | "::1")
}

/// Only tools on this Mac: the Host must name this Mac (a web page reached by DNS rebinding
/// names its own), and a browser's Origin, when sent, too — so no web page can read the docs or
/// start a workflow through a browser on this Mac.
async fn this_mac_only(req: Request, next: Next) -> Response {
    let h = req.headers();
    if !h.get(HOST).is_some_and(on_this_mac) || h.get(ORIGIN).is_some_and(|o| !on_this_mac(o)) {
        return ApiError::new(StatusCode::FORBIDDEN, "Only tools on this Mac can call the local API").into_response();
    }
    next.run(req).await
}

pub fn router(app: AppHandle) -> Router {
    Router::new()
        .route("/rest/batch/{objects}", post(batch_h))
        .route("/rest/{objects}", get(list_h).post(create_h))
        .route("/rest/{objects}/{id}", get(get_h).patch(update_h).delete(delete_h))
        .route("/webhooks/workflows/{workspace}/{workflow}", post(workflow_hook_h))
        .route("/docs", get(docs_h))
        .merge(super::plane::routes(app.clone()))
        .with_state(app)
        .layer(middleware::from_fn(this_mac_only))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::hub::WorkspaceHub;
    use crate::modules::api::keys;

    /// Twenty-shaped answers: a page with its cursors, a record, the made / changed / deleted
    /// one under Twenty's names, a batch; filters and orders in Twenty's syntax; `companyId`;
    /// errors in Twenty's shape. And a key: found in its workspace, refused once revoked.
    #[test]
    fn rest_answers_like_twenty() {
        tauri::async_runtime::block_on(async {
            let root = std::env::temp_dir().join(format!("newdawn-api-{}", uuid::Uuid::now_v7()));
            let hub = WorkspaceHub::open(root).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let actor = Actor::api("test");
            let no = HashMap::new();
            let made = create(&pool, "companies", json!({ "name": "Keppel Land", "employees": 120 }), &actor).await.unwrap();
            let keppel = made["data"]["createCompany"]["id"].as_str().unwrap().to_owned();
            assert_eq!(made["data"]["createCompany"]["createdBy"]["source"], "API");
            let made_people = batch(&pool, "people", json!([{ "name": { "firstName": "Minh", "lastName": "Tran" }, "companyId": keppel }, { "name": { "firstName": "Lan", "lastName": "Pham" }, "jobTitle": "CFO" }, { "name": { "firstName": "An", "lastName": "Le" } }]), &actor).await.unwrap();
            assert_eq!(made_people["data"]["createPeople"].as_array().unwrap().len(), 3);
            assert_eq!(made_people["data"]["createPeople"][0]["company"]["label"], "Keppel Land", "companyId is the company relation");

            let first = list(&pool, "people", &HashMap::from([("limit".into(), "2".into()), ("orderBy".into(), "name.firstName[AscNullsFirst]".into())])).await.unwrap();
            let names: Vec<&str> = first["data"]["people"].as_array().unwrap().iter().map(|p| p["name"]["firstName"].as_str().unwrap()).collect();
            assert_eq!(names, ["An", "Lan"]);
            assert_eq!((first["totalCount"].as_u64(), first["pageInfo"]["hasNextPage"].as_bool()), (Some(3), Some(true)));
            let after = first["pageInfo"]["endCursor"].as_str().unwrap().to_owned();
            let next = list(&pool, "people", &HashMap::from([("limit".into(), "2".into()), ("orderBy".into(), "name.firstName[AscNullsFirst]".into()), ("starting_after".into(), after)])).await.unwrap();
            assert_eq!(next["data"]["people"][0]["name"]["firstName"], "Minh");
            assert_eq!(next["pageInfo"]["hasNextPage"], false);
            let filtered = list(&pool, "people", &HashMap::from([("filter".into(), r#"or(jobTitle[eq]:"CFO",name.firstName[ilike]:"%Min%")"#.into())])).await.unwrap();
            assert_eq!(filtered["totalCount"], 2);

            let id = made_people["data"]["createPeople"][1]["id"].as_str().unwrap().to_owned();
            assert_eq!(get_one(&pool, "people", &id).await.unwrap()["data"]["person"]["jobTitle"], "CFO");
            let changed = update(&pool, "people", &id, json!({ "jobTitle": "CEO", "id": "ignored" }), &actor).await.unwrap();
            assert_eq!(changed["data"]["updatePerson"]["jobTitle"], "CEO");
            let renamed = update(&pool, "people", &id, json!({ "name": { "firstName": "Lana" } }), &actor).await.unwrap();
            assert_eq!((renamed["data"]["updatePerson"]["name"]["firstName"].as_str(), renamed["data"]["updatePerson"]["name"]["lastName"].as_str()), (Some("Lana"), Some("Pham")), "a part sent, the others kept");
            let before = list(&pool, "people", &HashMap::from([("limit".into(), "2".into()), ("ending_before".into(), "c1".into()), ("orderBy".into(), "name.firstName[AscNullsFirst]".into())])).await.unwrap();
            assert_eq!(before["data"]["people"].as_array().unwrap().len(), 1, "the page before c1 is c0 alone");
            let first = list(&pool, "people", &HashMap::from([("ending_before".into(), "c0".into())])).await.unwrap();
            assert_eq!((first["data"]["people"].as_array().unwrap().len(), first["pageInfo"]["hasPreviousPage"].as_bool()), (0, Some(false)), "nothing before the first");
            assert_eq!(batch(&pool, "opportunities", json!([{ "name": "Fine" }, { "name": "Wrong stage", "stage": "NOPE" }]), &actor).await.unwrap_err().status(), 400);
            let opportunities = list(&pool, "opportunities", &no).await.unwrap();
            assert_eq!(opportunities["totalCount"], 0, "nothing written when one record of the batch can't be");
            assert_eq!(delete(&pool, "people", &id, &actor).await.unwrap()["data"]["deletePerson"]["id"], id.as_str());
            assert_eq!(get_one(&pool, "people", &id).await.unwrap_err().status(), 404, "deleted means gone from the API");

            assert_eq!(list(&pool, "unicorns", &no).await.unwrap_err().status(), 404);
            assert_eq!(list(&pool, "people", &HashMap::from([("filter".into(), "name[between]:1".into())])).await.unwrap_err().status(), 400);
            assert_eq!(create(&pool, "people", json!({ "favouriteColour": "red" }), &actor).await.unwrap_err().status(), 400);
            assert_eq!(batch(&pool, "people", json!({ "not": "a list" }), &actor).await.unwrap_err().status(), 400);

            let made_key = keys::create(&pool, "Zapier", "", Some(30)).await.unwrap();
            assert!(made_key.secret.starts_with("nd_") && made_key.key.prefix == made_key.secret[..10]);
            assert_eq!(keys::find(&hub, &made_key.secret).await.unwrap(), Some(("test".into(), "Zapier".into())));
            assert!(keys::find(&hub, "nd_wrong").await.unwrap().is_none());
            keys::revoke(&pool, &made_key.key.id).await.unwrap();
            assert!(keys::find(&hub, &made_key.secret).await.unwrap().is_none(), "revoked");
            let old = keys::create(&pool, "Old", "", Some(1)).await.unwrap();
            sqlx::query("UPDATE api_key SET expires_at = '2026-01-01T00:00:00.000Z' WHERE id = ?").bind(&old.key.id).execute(&pool).await.unwrap();
            assert!(keys::find(&hub, &old.secret).await.unwrap().is_none(), "expired");
            assert!(on_this_mac(&HeaderValue::from_static("localhost:5123")) && on_this_mac(&HeaderValue::from_static("127.0.0.1")) && on_this_mac(&HeaderValue::from_static("[::1]:5123")));
            assert!(on_this_mac(&HeaderValue::from_static(concat!("http", "://localhost:3000"))));
            assert!(!on_this_mac(&HeaderValue::from_static("evil.example:5123")) && !on_this_mac(&HeaderValue::from_static("null")) && !on_this_mac(&HeaderValue::from_static("localhost.evil.example")));
        });
    }
}
