//! The assistant's tools (D-070): lookups that run at once on the record engine — find, count,
//! aggregate, open — and changes that come back as proposals the owner applies. What a tool
//! gives the model is short: a record's id, label, link and its main fields.

use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use serde_json::{json, Map, Value};
use sqlx::SqlitePool;

use crate::error::AppError;
use crate::kernel::ai::ollama::ToolCall;
use crate::kernel::records::query::{self, AggregateInput, Sort};
use crate::kernel::records::types::{FieldMeta, FieldType, Json, RelationKind};
use crate::kernel::records::{self, timeline, transfer, Actor, ObjectDef, Query};

/// Every tool, in the order the settings show them.
pub const ALL: [&str; 7] = ["find_records", "count_records", "aggregate", "open_record", "propose_create", "propose_update", "propose_note"];
/// The ones that only read (a workflow's agent gets no others).
pub const READ: [&str; 4] = ["find_records", "count_records", "aggregate", "open_record"];
/// Objects the assistant neither reads nor writes: the workflows' own and the dashboards.
pub const LEFT_OUT: [&str; 3] = ["workflow", "workflowRun", "dashboard"];
const FIND_LIMIT: u64 = 20;
/// How many records an "inactive for N days" lookup looks through.
const SCAN: u32 = 2000;
const OPS: [&str; 12] = ["is", "isNot", "contains", "notContains", "gt", "gte", "lt", "lte", "isEmpty", "isNotEmpty", "isBefore", "isAfter"];

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

/// A number the model wrote as a number or as text ("30").
fn number(v: Option<&Value>) -> Option<i64> {
    let v = v?;
    v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)).or_else(|| v.as_str()?.trim().parse().ok())
}

/// The tools an agent may use, as Ollama's `tools` (JSON schema functions).
pub fn schema(allowed: &[String]) -> Value {
    let object = json!({ "type": "string", "description": "The object's singular name, e.g. opportunity, company, person" });
    let rules = json!({
        "type": "array",
        "description": "Filters, all of which hold. A select takes its option's label or value; a relation the related record's name or id; an amount is in units; a date is YYYY-MM-DD.",
        "items": { "type": "object", "properties": { "field": { "type": "string" }, "op": { "type": "string", "enum": OPS }, "value": {} }, "required": ["field", "op"] }
    });
    let inactive = json!({ "type": "integer", "description": "Only records with no activity (a change, a note or a task) for this many days" });
    let fields = json!({ "type": "object", "description": "Field names and their values (an amount in units, a select by its label, a relation by the record's name or id, a name as text)" });
    let record = json!({ "type": "string", "description": "The record's id from a lookup, or its name" });
    let sort = json!({ "type": "object", "description": "Order: a field and whether largest or latest first", "properties": { "field": { "type": "string" }, "desc": { "type": "boolean" } } });
    let all = [
        ("find_records", "Find records of an object, with filters and an order. Gives up to 20 with their links.", json!({ "object": object, "rules": rules, "inactive_days": inactive, "sort": sort, "limit": { "type": "integer" } }), vec!["object"]),
        ("count_records", "Count the records of an object that match filters.", json!({ "object": object, "rules": rules, "inactive_days": inactive }), vec!["object"]),
        ("aggregate", "Sum, average, smallest or largest value of a number or amount field over the records that match.", json!({ "object": object, "field": { "type": "string" }, "op": { "type": "string", "enum": ["SUM", "AVG", "MIN", "MAX"] }, "rules": rules }), vec!["object", "field", "op"]),
        ("open_record", "One record with all its fields and its last activity.", json!({ "object": object, "id": record }), vec!["object", "id"]),
        ("propose_create", "Propose a new record; the owner applies it.", json!({ "object": object, "fields": fields }), vec!["object", "fields"]),
        ("propose_update", "Propose changes to a record; the owner applies them.", json!({ "object": object, "id": record, "fields": fields }), vec!["object", "id", "fields"]),
        ("propose_note", "Propose a note on a record; the owner applies it.", json!({ "object": object, "id": record, "title": { "type": "string" }, "body": { "type": "string" } }), vec!["object", "id", "title"]),
    ];
    Value::Array(
        all.into_iter()
            .filter(|(name, ..)| allowed.iter().any(|a| a == name))
            .map(|(name, description, properties, required)| json!({ "type": "function", "function": { "name": name, "description": description, "parameters": { "type": "object", "properties": properties, "required": required } } }))
            .collect(),
    )
}

/// The fields the assistant reads and writes: no system ones, positions, actors or lists.
fn usable(def: &ObjectDef) -> impl Iterator<Item = &FieldMeta> {
    def.fields.iter().filter(|f| f.is_active && !f.is_system && !matches!(f.r#type, FieldType::Position | FieldType::Actor) && !(f.r#type == FieldType::Relation && f.options.relation.as_ref().is_some_and(|r| r.kind != RelationKind::ManyToOne)))
}

fn kind_of(f: &FieldMeta) -> String {
    match f.r#type {
        FieldType::Select | FieldType::MultiSelect => {
            let options: Vec<String> = f.options.select.iter().map(|o| o.label.clone()).collect();
            format!("{} of {}", if f.r#type == FieldType::Select { "one" } else { "any" }, options.join(", "))
        }
        FieldType::Relation => format!("→ {}", f.options.relation.as_ref().map(|r| r.target.as_str()).unwrap_or("?")),
        FieldType::Currency => "amount".into(),
        other => other.as_str().to_lowercase().replace('_', " "),
    }
}

/// The objects and their fields, for the system prompt.
pub async fn catalogue(pool: &SqlitePool) -> Result<String> {
    let mut out = String::from("The objects, by singular name, with their fields:\n");
    for meta in records::objects(pool).await?.into_iter().filter(|m| m.is_active && !m.is_system && !LEFT_OUT.contains(&m.name_singular.as_str())) {
        let def = records::object(pool, &meta.name_singular).await?;
        let fields: Vec<String> = usable(&def).map(|f| format!("{} ({})", f.name, kind_of(f))).collect();
        out.push_str(&format!("- {} ({}): {}\n", meta.name_singular, meta.label_plural, fields.join(", ")));
    }
    Ok(out)
}

/// An object the assistant works on — not a system one, a workflow's, a dashboard or one turned
/// off — by its singular or plural name.
pub async fn object_named(pool: &SqlitePool, name: &str) -> Result<ObjectDef> {
    let name = name.trim();
    let def = records::object(pool, name).await.map_err(|_| invalid(format!("No object named “{name}”")))?;
    if def.meta.is_system || !def.meta.is_active || LEFT_OUT.contains(&def.meta.name_singular.as_str()) {
        return Err(invalid(format!("The assistant doesn't work on {}", def.meta.label_plural.to_lowercase())));
    }
    Ok(def)
}

async fn object(pool: &SqlitePool, args: &Value) -> Result<ObjectDef> {
    object_named(pool, args.get("object").and_then(Value::as_str).unwrap_or("")).await
}

fn field<'a>(def: &'a ObjectDef, name: &str) -> Result<&'a FieldMeta> {
    usable(def).find(|f| f.name == name || f.label.eq_ignore_ascii_case(name)).ok_or_else(|| {
        let names: Vec<&str> = usable(def).map(|f| f.name.as_str()).collect();
        invalid(format!("{} has no field “{name}” — its fields: {}", def.meta.label_singular, names.join(", ")))
    })
}

fn looks_like_id(text: &str) -> bool {
    text.len() == 36 && text.chars().filter(|c| *c == '-').count() == 4
}

/// The label of a record as the engine returns it (its label field, a name's parts joined).
pub fn label_of(def: &ObjectDef, record: &Value) -> String {
    let Some(f) = def.meta.label_identifier_field_id.as_ref().and_then(|id| def.fields.iter().find(|f| &f.id == id)) else {
        return record.get("id").and_then(Value::as_str).unwrap_or("").to_owned();
    };
    match record.get(&f.name) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Object(o)) => o.values().filter_map(Value::as_str).map(str::trim).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" "),
        _ => String::new(),
    }
}

/// A record of an object by its id, or by its name (the model may give either).
async fn record_named(pool: &SqlitePool, def: &ObjectDef, given: &str) -> Result<Value> {
    let given = given.trim();
    let singular = def.meta.label_singular.to_lowercase();
    let id = if looks_like_id(given) {
        given.to_owned()
    } else {
        let by = def.meta.label_identifier_field_id.as_ref().and_then(|id| def.fields.iter().find(|x| &x.id == id)).map(|x| x.name.clone()).unwrap_or_else(|| "name".into());
        transfer::find_by(pool, def, &by, given, &mut transfer::Lookups::new()).await?.ok_or_else(|| invalid(format!("No {singular} named “{given}”")))?
    };
    query::get(pool, def, &id, false).await.map_err(|_| invalid(format!("No {singular} with the id {id}")))
}

/// A value the model wrote, as a filter takes it: a select's label as its value, an amount in
/// units as micros, a relation by name as the record's id.
async fn filter_value(pool: &SqlitePool, f: &FieldMeta, value: &Value) -> Result<Value> {
    match f.r#type {
        FieldType::Select | FieldType::MultiSelect => {
            let one = |v: &Value| -> Value {
                let text = v.as_str().unwrap_or("").trim();
                f.options.select.iter().find(|o| o.value.eq_ignore_ascii_case(text) || o.label.eq_ignore_ascii_case(text)).map_or_else(|| v.clone(), |o| Value::String(o.value.clone()))
            };
            Ok(match value {
                Value::Array(a) => Value::Array(a.iter().map(one).collect()),
                v => one(v),
            })
        }
        FieldType::Currency => Ok(units(value).map_or(Value::Null, |u| json!((u * 1_000_000.0).round()))),
        FieldType::Relation => relation_id(pool, f, value).await,
        _ => Ok(value.clone()),
    }
}

/// An amount in units, from a number, a text or an object with its amount.
fn units(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().replace(',', "").parse::<f64>().ok(),
        Value::Object(o) => o.get("amountMicros").and_then(Value::as_f64).map(|m| m / 1_000_000.0).or_else(|| o.get("amount").or_else(|| o.get("value")).and_then(units)),
        _ => None,
    }
}

/// A relation's value: an id as it is, a name looked up among the target's records.
async fn relation_id(pool: &SqlitePool, f: &FieldMeta, value: &Value) -> Result<Value> {
    let Some(text) = value.as_str().map(str::trim).filter(|t| !t.is_empty()) else { return Ok(value.clone()) };
    if looks_like_id(text) {
        return Ok(Value::String(text.to_owned()));
    }
    let target = f.options.relation.as_ref().map(|r| r.target.clone()).unwrap_or_default();
    let other = records::object(pool, &target).await?;
    let found = record_named(pool, &other, text).await?;
    Ok(found.get("id").cloned().unwrap_or(Value::Null))
}

/// A value the model wrote, as a write takes it — all of a composite: an amount with its
/// currency (the record's, else the field's), a name split in two, an email, a phone or a link
/// as its first part, a rich text as the editor's document.
async fn write_value(pool: &SqlitePool, f: &FieldMeta, value: &Value, existing: Option<&Value>) -> Result<Value> {
    Ok(match (f.r#type, value) {
        (FieldType::Currency, v) => {
            let amount = units(v).ok_or_else(|| invalid(format!("{}: an amount in units", f.label)))?;
            let given = v.get("currencyCode").or_else(|| v.get("currency")).and_then(Value::as_str).filter(|c| !c.is_empty());
            let kept = existing.and_then(|r| r.get(&f.name)).and_then(|c| c.get("currencyCode")).and_then(Value::as_str).filter(|c| !c.is_empty());
            let code = given.or(kept).or(f.options.currency.as_deref()).unwrap_or("");
            json!({ "amountMicros": (amount * 1_000_000.0).round(), "currencyCode": code })
        }
        (FieldType::FullName, Value::String(s)) => {
            let (first, last) = records::split_name(s);
            json!({ "firstName": first, "lastName": last })
        }
        (FieldType::Emails, Value::String(s)) => json!({ "primaryEmail": s.trim() }),
        (FieldType::Phones, Value::String(s)) => json!({ "primaryPhoneNumber": s.trim() }),
        (FieldType::Links | FieldType::Domain, Value::String(s)) => json!({ "primaryLinkUrl": s.trim() }),
        (FieldType::RichText, Value::String(s)) => transfer::rich_doc(s),
        _ => filter_value(pool, f, value).await?,
    })
}

/// The rules as the engine's filter.
async fn filter(pool: &SqlitePool, def: &ObjectDef, rules: Option<&Value>) -> Result<Option<Json>> {
    let Some(Value::Array(rules)) = rules else { return Ok(None) };
    let mut and = Vec::new();
    for rule in rules {
        let name = rule.get("field").and_then(Value::as_str).unwrap_or("");
        let f = field(def, name)?;
        let op = rule.get("op").and_then(Value::as_str).unwrap_or("is");
        if !OPS.contains(&op) {
            return Err(invalid(format!("No operator “{op}” — use one of {}", OPS.join(", "))));
        }
        let value = rule.get("value").cloned().unwrap_or(Value::Null);
        let sub = sub_of(f);
        let value = filter_value(pool, f, &value).await?;
        and.push(json!({ "field": f.name, "sub": sub, "op": op, "value": value }));
    }
    Ok((!and.is_empty()).then(|| Json(json!({ "and": and }))))
}

/// The part of a composite a filter or an order reads.
fn sub_of(f: &FieldMeta) -> Option<&'static str> {
    match f.r#type {
        FieldType::Currency => Some("amountMicros"),
        FieldType::Emails => Some("primaryEmail"),
        FieldType::Links | FieldType::Domain => Some("primaryLinkUrl"),
        FieldType::Phones => Some("primaryPhoneNumber"),
        FieldType::FullName => Some("firstName"),
        _ => None,
    }
}

/// A record in a few words: id, label, link and its main fields, as the owner reads them.
fn brief(def: &ObjectDef, record: &Value, every_field: bool) -> Value {
    let id = record.get("id").and_then(Value::as_str).unwrap_or("");
    let mut out = Map::new();
    out.insert("id".into(), json!(id));
    out.insert("label".into(), json!(label_of(def, record)));
    out.insert("link".into(), json!(format!("/crm/{}/{id}", def.meta.name_plural)));
    for f in usable(def).take(if every_field { 60 } else { 8 }) {
        let v = record.get(&f.name).cloned().unwrap_or(Value::Null);
        let shown = match (f.r#type, &v) {
            (_, Value::Null) => continue,
            (FieldType::RichText, _) => continue,
            (FieldType::Relation, Value::Object(o)) => o.get("label").cloned().unwrap_or(Value::Null),
            (FieldType::Currency, Value::Object(o)) => match o.get("amountMicros").and_then(Value::as_f64) {
                Some(m) => json!(format!("{} {}", m / 1_000_000.0, o.get("currencyCode").and_then(Value::as_str).unwrap_or("")).trim()),
                None => continue,
            },
            (FieldType::Select, Value::String(s)) => json!(f.options.select.iter().find(|o| &o.value == s).map_or(s.as_str(), |o| o.label.as_str())),
            (_, Value::Object(o)) => {
                let text: Vec<&str> = o.values().filter_map(Value::as_str).filter(|s| !s.is_empty()).collect();
                if text.is_empty() {
                    continue;
                }
                json!(text.join(" "))
            }
            (_, Value::String(s)) if s.is_empty() => continue,
            (_, Value::String(s)) if s.chars().count() > 200 => json!(format!("{}…", s.chars().take(200).collect::<String>())),
            _ => v,
        };
        out.insert(f.name.clone(), shown);
    }
    Value::Object(out)
}

/// What a lookup found: how many, the records, the last activity when it filtered on it, and how
/// many it looked through when the matches went past what it reads.
struct Found {
    total: u32,
    records: Vec<Value>,
    last: Option<std::collections::HashMap<String, String>>,
    scanned: Option<(u32, u32)>,
}

/// The records that match, the inactive ones only when asked (looked through up to SCAN).
async fn matching(pool: &SqlitePool, def: &ObjectDef, args: &Value, limit: u32) -> Result<Found> {
    let filter = filter(pool, def, args.get("rules")).await?;
    let inactive = number(args.get("inactive_days")).filter(|d| *d > 0);
    let sort = match args.get("sort") {
        Some(Value::Object(o)) => {
            let f = field(def, o.get("field").and_then(Value::as_str).unwrap_or(""))?;
            vec![Sort { field: f.name.clone(), sub: sub_of(f).map(str::to_owned), desc: o.get("desc").and_then(Value::as_bool).unwrap_or(false) }]
        }
        _ => Vec::new(),
    };
    let query = Query { filter, sort, limit: Some(if inactive.is_some() { SCAN } else { limit }), offset: None, search: None, deleted: false };
    let page = query::list(pool, def, &query).await?;
    let records: Vec<Value> = page.records.into_iter().map(|r| r.0).collect();
    let Some(days) = inactive else { return Ok(Found { total: page.total, records, last: None, scanned: None }) };
    let ids: Vec<String> = records.iter().filter_map(|r| r.get("id").and_then(Value::as_str)).map(str::to_owned).collect();
    let last = timeline::last_activity(pool, def, &ids).await?;
    let since = (Utc::now() - Duration::try_days(days.min(36_500)).unwrap_or_default()).format("%Y-%m-%dT%H:%M:%S").to_string();
    let quiet: Vec<Value> = records.into_iter().filter(|r| r.get("id").and_then(Value::as_str).and_then(|id| last.get(id)).is_none_or(|at| at.as_str() < since.as_str())).collect();
    Ok(Found { total: u32::try_from(quiet.len()).unwrap_or(u32::MAX), records: quiet, last: Some(last), scanned: (page.total > SCAN).then_some((SCAN, page.total)) })
}

async fn find(pool: &SqlitePool, args: &Value) -> Result<Value> {
    let def = object(pool, args).await?;
    let limit = number(args.get("limit")).unwrap_or(10).clamp(1, FIND_LIMIT as i64) as u32;
    let found = matching(pool, &def, args, limit).await?;
    let shown: Vec<Value> = found
        .records
        .iter()
        .take(limit as usize)
        .map(|r| {
            let mut b = brief(&def, r, false);
            if let (Some(last), Some(id)) = (&found.last, r.get("id").and_then(Value::as_str)) {
                b["lastActivity"] = json!(last.get(id).map(|at| at.get(..10).unwrap_or(at)));
            }
            b
        })
        .collect();
    let mut out = json!({ "object": def.meta.name_singular, "total": found.total, "records": shown, "more": found.total as usize > shown.len() });
    if let Some((scanned, matching)) = found.scanned {
        out["scanned"] = json!(scanned);
        out["matching"] = json!(matching);
    }
    Ok(out)
}

async fn count(pool: &SqlitePool, args: &Value) -> Result<Value> {
    let def = object(pool, args).await?;
    let found = matching(pool, &def, args, 1).await?;
    let mut out = json!({ "object": def.meta.name_singular, "count": found.total });
    if let Some((scanned, matching)) = found.scanned {
        out["scanned"] = json!(scanned);
        out["matching"] = json!(matching);
    }
    Ok(out)
}

async fn aggregate(pool: &SqlitePool, args: &Value) -> Result<Value> {
    let def = object(pool, args).await?;
    let f = field(&def, args.get("field").and_then(Value::as_str).unwrap_or(""))?;
    if !matches!(f.r#type, FieldType::Number | FieldType::Currency) {
        return Err(invalid(format!("{} isn't a number or an amount — count the records instead", f.label)));
    }
    let op = args.get("op").and_then(Value::as_str).unwrap_or("SUM").to_uppercase();
    if !matches!(op.as_str(), "SUM" | "AVG" | "MIN" | "MAX") {
        return Err(invalid("The operation is SUM, AVG, MIN or MAX"));
    }
    let money = f.r#type == FieldType::Currency;
    let query = Query { filter: filter(pool, &def, args.get("rules")).await?, sort: Vec::new(), limit: None, offset: None, search: None, deleted: false };
    let values = query::aggregate(pool, &def, &query, &[AggregateInput { field: f.name.clone(), sub: money.then(|| "amountMicros".to_owned()), op: op.clone() }]).await?;
    let value = values.first().map(|v| v.value.0.clone()).unwrap_or(Value::Null);
    let value = match (money, value.as_f64()) {
        (true, Some(m)) => json!(m / 1_000_000.0),
        _ => value,
    };
    let count = query::list(pool, &def, &Query { limit: Some(1), ..query }).await?.total;
    Ok(json!({ "object": def.meta.name_singular, "field": f.name, "op": op, "value": value, "records": count }))
}

async fn open(pool: &SqlitePool, args: &Value) -> Result<Value> {
    let def = object(pool, args).await?;
    let record = record_named(pool, &def, args.get("id").and_then(Value::as_str).unwrap_or("")).await?;
    let id = record.get("id").and_then(Value::as_str).unwrap_or("").to_owned();
    let mut b = brief(&def, &record, true);
    let last = timeline::last_activity(pool, &def, std::slice::from_ref(&id)).await?;
    b["lastActivity"] = json!(last.get(&id).map(|at| at.get(..10).unwrap_or(at)));
    Ok(b)
}

/// How a proposed value reads for the owner: a select by its label, a relation by the name
/// given, an amount in units.
fn shown_value(f: &FieldMeta, given: &Value, value: &Value) -> String {
    match f.r#type {
        FieldType::Select => value.as_str().and_then(|v| f.options.select.iter().find(|o| o.value == v)).map_or_else(|| text(value), |o| o.label.clone()),
        FieldType::Relation => text(given),
        FieldType::Currency => value.get("amountMicros").and_then(Value::as_f64).map(|m| format!("{} {}", m / 1_000_000.0, value.get("currencyCode").and_then(Value::as_str).unwrap_or("")).trim().to_owned()).unwrap_or_default(),
        FieldType::RichText => text(given),
        _ => text(value),
    }
}

fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        Value::Object(o) => o.values().filter_map(Value::as_str).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" "),
        other => other.to_string(),
    }
}

/// The fields of a proposed record, checked as the engine would and shown as the owner reads them.
async fn proposed_fields(pool: &SqlitePool, def: &ObjectDef, fields: Option<&Value>, existing: Option<&Value>) -> Result<(Map<String, Value>, Vec<String>)> {
    let Some(Value::Object(given)) = fields else { return Err(invalid("No fields given")) };
    let (mut data, mut shown) = (Map::new(), Vec::new());
    for (name, value) in given {
        let f = field(def, name)?;
        let written = write_value(pool, f, value, existing).await?;
        query::check_value(f, &written).map_err(|e| invalid(format!("{e:#}")))?;
        let reads = shown_value(f, value, &written);
        shown.push(format!("{}: {}", f.label, if reads.is_empty() { "(empty)".into() } else { reads }));
        data.insert(f.name.clone(), written);
    }
    Ok((data, shown))
}

/// The object a note is attached to through the note's targets (not a note itself).
async fn note_target_field(pool: &SqlitePool, def: &ObjectDef) -> Result<()> {
    let targets = records::object(pool, "noteTarget").await?;
    if def.meta.name_singular == "note" || targets.field(&def.meta.name_singular).is_none() {
        return Err(invalid(format!("A note can't be attached to {}", def.meta.label_plural.to_lowercase())));
    }
    Ok(())
}

/// A proposal and what the model is told about it.
async fn propose(pool: &SqlitePool, kind: &str, args: &Value) -> Result<(Value, Value)> {
    let def = object(pool, args).await?;
    let singular = def.meta.label_singular.to_lowercase();
    let plural = def.meta.name_plural.clone();
    let proposal = match kind {
        "propose_create" => {
            let (data, shown) = proposed_fields(pool, &def, args.get("fields"), None).await?;
            json!({ "kind": "create", "object": def.meta.name_singular, "plural": plural, "fields": data, "summary": format!("Create {} {singular} — {}", if singular.starts_with(['a', 'e', 'i', 'o', 'u']) { "an" } else { "a" }, shown.join(" · ")) })
        }
        "propose_update" => {
            let record = record_named(pool, &def, args.get("id").and_then(Value::as_str).unwrap_or("")).await?;
            let (data, shown) = proposed_fields(pool, &def, args.get("fields"), Some(&record)).await?;
            let label = label_of(&def, &record);
            json!({ "kind": "update", "object": def.meta.name_singular, "plural": plural, "id": record["id"], "label": label, "fields": data, "summary": format!("Change {label} — {}", shown.join(" · ")) })
        }
        _ => {
            note_target_field(pool, &def).await?;
            let record = record_named(pool, &def, args.get("id").and_then(Value::as_str).unwrap_or("")).await?;
            let title = args.get("title").and_then(Value::as_str).unwrap_or("").trim();
            if title.is_empty() {
                return Err(invalid("A note needs a title"));
            }
            let body = args.get("body").and_then(Value::as_str).unwrap_or("").trim();
            let label = label_of(&def, &record);
            json!({ "kind": "note", "object": def.meta.name_singular, "plural": "notes", "id": record["id"], "label": label, "title": title, "body": body, "summary": format!("Add a note to {label} — “{title}”") })
        }
    };
    let mut told = json!({ "proposed": proposal["summary"], "status": "Waiting for the owner to apply it — say it is proposed, not done" });
    // The record's own link, so an answer never makes one up.
    if let Some(id) = proposal.get("id").and_then(Value::as_str) {
        told["record"] = json!({ "label": proposal["label"], "link": format!("/crm/{}/{id}", def.meta.name_plural) });
    }
    Ok((told, proposal))
}

/// What a tool call gave: the result for the model, and a proposal when it was one.
pub struct Outcome {
    pub result: Value,
    pub proposal: Option<Value>,
}

/// Runs a tool the agent may use; a refusal or a mistake comes back as `{"error": …}` for the
/// model to read and correct, never as a failed turn.
pub async fn run(pool: &SqlitePool, allowed: &[String], call: &ToolCall) -> Outcome {
    let name = call.name.as_str();
    if !allowed.iter().any(|a| a == name) {
        return Outcome { result: json!({ "error": format!("No tool named {name} here") }), proposal: None };
    }
    let args = &call.arguments;
    let done = match name {
        "find_records" => find(pool, args).await.map(|r| (r, None)),
        "count_records" => count(pool, args).await.map(|r| (r, None)),
        "aggregate" => aggregate(pool, args).await.map(|r| (r, None)),
        "open_record" => open(pool, args).await.map(|r| (r, None)),
        "propose_create" | "propose_update" | "propose_note" => propose(pool, name, args).await.map(|(r, p)| (r, Some(p))),
        _ => Err(invalid(format!("No tool named {name}"))),
    };
    match done {
        Ok((result, proposal)) => Outcome { result, proposal },
        Err(e) => Outcome { result: json!({ "error": format!("{e:#}") }), proposal: None },
    }
}

/// Applies a proposal through the engine, as the agent: the id of what it made or changed. The
/// object is checked again (it may have been turned off since), an update never lands on a
/// deleted record, and a note is never left without its record.
pub async fn apply(pool: &SqlitePool, proposal: &Value, actor: &Actor) -> Result<String> {
    let object_name = proposal.get("object").and_then(Value::as_str).unwrap_or("");
    let def = object_named(pool, object_name).await?;
    let fields = proposal.get("fields").cloned().unwrap_or(json!({}));
    let id = proposal.get("id").and_then(Value::as_str).unwrap_or("");
    match proposal.get("kind").and_then(Value::as_str).unwrap_or("") {
        "create" => {
            let made = query::create(pool, &def, &fields, actor).await?;
            Ok(made.get("id").and_then(Value::as_str).unwrap_or("").to_owned())
        }
        "update" => {
            query::get(pool, &def, id, false).await.map_err(|_| invalid("The record is gone"))?;
            query::update(pool, &def, id, &fields, actor).await?;
            Ok(id.to_owned())
        }
        "note" => {
            note_target_field(pool, &def).await?;
            query::get(pool, &def, id, false).await.map_err(|_| invalid("The record is gone"))?;
            let notes = records::object(pool, "note").await?;
            let body = proposal.get("body").and_then(Value::as_str).unwrap_or("");
            let mut data = json!({ "title": proposal.get("title").and_then(Value::as_str).unwrap_or("Note") });
            if !body.is_empty() {
                data["body"] = transfer::rich_doc(body);
            }
            let note = query::create(pool, &notes, &data, actor).await?;
            let note_id = note.get("id").and_then(Value::as_str).unwrap_or("").to_owned();
            let targets = records::object(pool, "noteTarget").await?;
            if let Err(e) = query::create(pool, &targets, &json!({ "note": note_id, object_name: id }), actor).await {
                // Not left alone: a retry would make a second one.
                let _ = query::destroy(pool, &notes, &note_id).await;
                return Err(e);
            }
            Ok(note_id)
        }
        other => Err(invalid(format!("Nothing to apply for “{other}”"))),
    }
}
