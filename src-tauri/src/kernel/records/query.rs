//! Reading and writing the records of any object through its metadata (D-060): one query
//! builder for lists (filters, sorts, search, pages), get, create, update, soft delete, restore
//! and permanent delete — with validation per field type, the timeline and the search index.

use std::collections::BTreeMap;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use specta::Type;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};

use super::types::{columns, subs, ColumnKind, FieldMeta, FieldType, Json, ObjectDef, RelationKind, StoredColumn};
use super::{search, timeline};
use crate::error::AppError;

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";
const PAGE: u32 = 60;
const MAX_PAGE: u32 = 5_000;

/// What a list asks for. The filter is Twenty-shaped JSON: `{"and": [...]}`, `{"or": [...]}`,
/// `{"not": {...}}` or `{"field": "city", "sub": null, "op": "contains", "value": "Hanoi"}`
/// (`sub` names a composite's part, e.g. `addressCity`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct Query {
    #[serde(default)]
    pub filter: Option<Json>,
    #[serde(default)]
    pub sort: Vec<Sort>,
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub offset: Option<u32>,
    #[serde(default)]
    pub search: Option<String>,
    /// The deleted records instead of the live ones.
    #[serde(default)]
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Sort {
    pub field: String,
    #[serde(default)]
    pub sub: Option<String>,
    #[serde(default)]
    pub desc: bool,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct Page {
    pub records: Vec<Json>,
    pub total: u32,
}

/// Who writes (Twenty's actor): the owner by hand, an agent by name, an import, a workflow, the
/// API. Kept on every record (`createdBy`) and activity.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct Actor {
    /// MANUAL · AGENT · IMPORT · WORKFLOW · API · SYSTEM
    pub source: String,
    pub name: String,
    #[serde(default)]
    pub member_id: Option<String>,
}

impl Actor {
    pub fn manual(member_id: Option<String>, name: &str) -> Self {
        Self { source: "MANUAL".into(), name: name.into(), member_id }
    }

    pub fn system(name: &str) -> Self {
        Self { source: "SYSTEM".into(), name: name.into(), member_id: None }
    }

    /// A spreadsheet import, named after its file (D-065).
    pub fn import(member_id: Option<String>, file_name: &str) -> Self {
        Self { source: "IMPORT".into(), name: file_name.into(), member_id }
    }
}

#[derive(Debug, Clone)]
enum Bind {
    Text(String),
    Real(f64),
    Int(i64),
    Null,
    /// An SQL expression written by the engine itself (never from input), inlined instead of bound.
    Raw(String),
}

type SqlQuery<'q> = sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments>;

/// Dynamic SQL from the metadata: table and column names come from `object_meta` / `field_meta`
/// (never from typed input); every value is bound.
fn sql(text: String) -> sqlx::AssertSqlSafe<String> {
    sqlx::AssertSqlSafe(text)
}

fn bind_all<'q>(mut q: SqlQuery<'q>, binds: &[Bind]) -> SqlQuery<'q> {
    for b in binds {
        q = match b {
            Bind::Text(s) => q.bind(s.clone()),
            Bind::Real(x) => q.bind(*x),
            Bind::Int(i) => q.bind(*i),
            Bind::Null => q.bind(Option::<String>::None),
            Bind::Raw(_) => q,
        };
    }
    q
}

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

// ---------- filters ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Is,
    IsNot,
    Contains,
    NotContains,
    IsEmpty,
    IsNotEmpty,
    Gte,
    Lte,
    Gt,
    Lt,
    IsBefore,
    IsAfter,
    IsToday,
    IsInPast,
    IsInFuture,
    IsRelative,
    In,
    NotIn,
}

impl Op {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "is" => Self::Is,
            "isNot" => Self::IsNot,
            "contains" => Self::Contains,
            "notContains" | "doesNotContain" => Self::NotContains,
            "isEmpty" => Self::IsEmpty,
            "isNotEmpty" => Self::IsNotEmpty,
            "gte" | "greaterThanOrEqual" => Self::Gte,
            "lte" | "lessThanOrEqual" => Self::Lte,
            "gt" => Self::Gt,
            "lt" => Self::Lt,
            "isBefore" => Self::IsBefore,
            "isAfter" | "isAfterOrEqual" => Self::IsAfter,
            "isToday" => Self::IsToday,
            "isInPast" => Self::IsInPast,
            "isInFuture" => Self::IsInFuture,
            "isRelative" => Self::IsRelative,
            "in" => Self::In,
            "notIn" => Self::NotIn,
            _ => return None,
        })
    }
}

enum Cond {
    And(Vec<Cond>),
    Or(Vec<Cond>),
    Not(Box<Cond>),
    Field { field: String, sub: Option<String>, op: Op, value: Value },
}

fn parse_filter(value: &Value) -> Result<Cond> {
    let map = value.as_object().ok_or_else(|| invalid("A filter is an object"))?;
    if let Some(items) = map.get("and").and_then(Value::as_array) {
        return Ok(Cond::And(items.iter().map(parse_filter).collect::<Result<_>>()?));
    }
    if let Some(items) = map.get("or").and_then(Value::as_array) {
        return Ok(Cond::Or(items.iter().map(parse_filter).collect::<Result<_>>()?));
    }
    if let Some(inner) = map.get("not") {
        return Ok(Cond::Not(Box::new(parse_filter(inner)?)));
    }
    let field = map.get("field").and_then(Value::as_str).ok_or_else(|| invalid("A filter rule names a field"))?;
    let op = map.get("op").and_then(Value::as_str).and_then(Op::parse).ok_or_else(|| invalid("A filter rule has a known operator"))?;
    Ok(Cond::Field {
        field: field.to_owned(),
        sub: map.get("sub").and_then(Value::as_str).map(str::to_owned),
        op,
        value: map.get("value").cloned().unwrap_or(Value::Null),
    })
}

/// The columns a rule applies to: the named part of a composite, else every text part.
fn match_columns(field: &FieldMeta, sub: Option<&str>) -> Vec<StoredColumn> {
    let all = columns(field);
    match sub {
        Some(sub) => all.into_iter().filter(|c| c.key == Some(sub)).collect(),
        None if subs(field.r#type).is_empty() => all,
        None => all.into_iter().filter(|c| c.kind == ColumnKind::Text && !c.json).collect(),
    }
}

fn like_pattern(text: &str) -> String {
    let escaped = text.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    format!("%{escaped}%")
}

fn to_bind(c: &StoredColumn, value: &Value) -> Bind {
    match (c.kind, value) {
        (_, Value::Null) => Bind::Null,
        (ColumnKind::Int, Value::Bool(b)) => Bind::Int(i64::from(*b)),
        (ColumnKind::Int, Value::Number(n)) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)).map(Bind::Int).unwrap_or(Bind::Null),
        (ColumnKind::Real, Value::Number(n)) => n.as_f64().map(Bind::Real).unwrap_or(Bind::Null),
        (ColumnKind::Real | ColumnKind::Int, Value::String(s)) => s.trim().parse::<f64>().map(|f| if c.kind == ColumnKind::Int { Bind::Int(f as i64) } else { Bind::Real(f) }).unwrap_or(Bind::Null),
        (_, Value::String(s)) => Bind::Text(s.clone()),
        (_, Value::Bool(b)) => Bind::Text(if *b { "1".into() } else { "0".into() }),
        (_, Value::Number(n)) => Bind::Text(n.to_string()),
        (_, other) => Bind::Text(other.to_string()),
    }
}

fn is_date(field: &FieldMeta) -> bool {
    matches!(field.r#type, FieldType::Date | FieldType::DateTime)
}

/// The day of a date column in the Mac's time zone (a date-time is stored in UTC).
fn day_sql(field: &FieldMeta, col: &str) -> String {
    if field.r#type == FieldType::DateTime {
        format!("date({col}, 'localtime')")
    } else {
        col.to_owned()
    }
}

fn empty_sql(c: &StoredColumn, col: &str) -> String {
    match c.kind {
        ColumnKind::Text if c.json => format!("({col} IS NULL OR {col} IN ('', '[]', '{{}}', 'null'))"),
        ColumnKind::Text => format!("({col} IS NULL OR {col} = '')"),
        _ => format!("{col} IS NULL"),
    }
}

fn in_sql(c: &StoredColumn, col: &str, values: &[Value], binds: &mut Vec<Bind>) -> String {
    if values.is_empty() {
        return "0".into();
    }
    let marks = vec!["?"; values.len()].join(", ");
    binds.extend(values.iter().map(|v| to_bind(c, v)));
    if c.json {
        format!("EXISTS (SELECT 1 FROM json_each({col}) WHERE json_each.value IN ({marks}))")
    } else {
        format!("{col} IN ({marks})")
    }
}

fn column_sql(field: &FieldMeta, c: &StoredColumn, op: Op, value: &Value, binds: &mut Vec<Bind>) -> Result<String> {
    let col = format!("t.\"{}\"", c.name);
    Ok(match op {
        Op::Is => match value {
            Value::Null => format!("{col} IS NULL"),
            Value::Array(values) => in_sql(c, &col, values, binds),
            _ if field.r#type == FieldType::DateTime => {
                // A day alone is a local day; a full time is an instant read in local time.
                let day_only = value.as_str().is_some_and(|s| s.trim().len() == 10);
                binds.push(to_bind(c, value));
                if day_only {
                    format!("date({col}, 'localtime') = date(?)")
                } else {
                    format!("date({col}, 'localtime') = date(?, 'localtime')")
                }
            }
            _ => {
                binds.push(to_bind(c, value));
                format!("{col} = ?")
            }
        },
        Op::IsNot => match value {
            Value::Null => format!("{col} IS NOT NULL"),
            Value::Array(values) => format!("({col} IS NULL OR NOT {})", in_sql(c, &col, values, binds)),
            _ => {
                binds.push(to_bind(c, value));
                format!("({col} IS NULL OR {col} <> ?)")
            }
        },
        Op::Contains => match value {
            Value::Array(values) => in_sql(c, &col, values, binds),
            _ if c.json => {
                binds.push(to_bind(c, value));
                format!("EXISTS (SELECT 1 FROM json_each({col}) WHERE json_each.value = ?)")
            }
            _ if matches!(field.r#type, FieldType::Select | FieldType::Relation) => {
                binds.push(to_bind(c, value));
                format!("{col} = ?")
            }
            _ => {
                let text = match value {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    _ => return Err(invalid(format!("{}: contains needs text", field.label))),
                };
                binds.push(Bind::Text(like_pattern(&text)));
                format!("{col} LIKE ? ESCAPE '\\'")
            }
        },
        Op::NotContains => format!("({col} IS NULL OR NOT {})", column_sql(field, c, Op::Contains, value, binds)?),
        Op::IsEmpty => empty_sql(c, &col),
        Op::IsNotEmpty => format!("NOT {}", empty_sql(c, &col)),
        Op::Gte | Op::Lte | Op::Gt | Op::Lt | Op::IsBefore | Op::IsAfter => {
            if value.is_null() {
                return Err(invalid(format!("{}: this filter needs a value", field.label)));
            }
            binds.push(to_bind(c, value));
            let sign = match op {
                Op::Gte | Op::IsAfter => ">=",
                Op::Lte => "<=",
                Op::Gt => ">",
                _ => "<",
            };
            format!("{col} {sign} ?")
        }
        Op::IsToday => format!("{} = date('now', 'localtime')", day_sql(field, &col)),
        Op::IsInPast => {
            if field.r#type == FieldType::DateTime {
                format!("{col} < {NOW}")
            } else {
                format!("{col} < date('now', 'localtime')")
            }
        }
        Op::IsInFuture => {
            if field.r#type == FieldType::DateTime {
                format!("{col} > {NOW}")
            } else {
                format!("{col} > date('now', 'localtime')")
            }
        }
        Op::IsRelative => {
            let d = day_sql(field, &col);
            let direction = value.get("direction").and_then(Value::as_str).unwrap_or("PAST").to_ascii_uppercase();
            let amount = value.get("amount").and_then(Value::as_i64).unwrap_or(1).max(1);
            let unit = value.get("unit").and_then(Value::as_str).unwrap_or("DAY").to_ascii_uppercase();
            let modifier = |n: i64| match unit.as_str() {
                "WEEK" => format!("{:+} days", n * 7),
                "MONTH" => format!("{n:+} months"),
                "YEAR" => format!("{n:+} years"),
                _ => format!("{n:+} days"),
            };
            match direction.as_str() {
                "NEXT" => format!("({d} >= date('now', 'localtime') AND {d} <= date('now', 'localtime', '{}'))", modifier(amount)),
                "THIS" => match unit.as_str() {
                    "WEEK" => format!(
                        "({d} >= date('now', 'localtime', '-6 days', 'weekday 1') AND {d} < date('now', 'localtime', '-6 days', 'weekday 1', '+7 days'))"
                    ),
                    "MONTH" => format!("({d} >= date('now', 'localtime', 'start of month') AND {d} < date('now', 'localtime', 'start of month', '+1 month'))"),
                    "YEAR" => format!("({d} >= date('now', 'localtime', 'start of year') AND {d} < date('now', 'localtime', 'start of year', '+1 year'))"),
                    _ => format!("{d} = date('now', 'localtime')"),
                },
                _ => format!("({d} >= date('now', 'localtime', '{}') AND {d} <= date('now', 'localtime'))", modifier(-amount)),
            }
        }
        Op::In => match value {
            Value::Array(values) => in_sql(c, &col, values, binds),
            other => in_sql(c, &col, std::slice::from_ref(other), binds),
        },
        Op::NotIn => format!("({col} IS NULL OR NOT {})", column_sql(field, c, Op::In, value, binds)?),
    })
}

fn cond_sql(object: &ObjectDef, cond: &Cond, binds: &mut Vec<Bind>) -> Result<String> {
    Ok(match cond {
        Cond::And(items) if items.is_empty() => "1".into(),
        Cond::Or(items) if items.is_empty() => "1".into(),
        Cond::And(items) => format!("({})", items.iter().map(|c| cond_sql(object, c, binds)).collect::<Result<Vec<_>>>()?.join(" AND ")),
        Cond::Or(items) => format!("({})", items.iter().map(|c| cond_sql(object, c, binds)).collect::<Result<Vec<_>>>()?.join(" OR ")),
        Cond::Not(inner) => format!("NOT {}", cond_sql(object, inner, binds)?),
        Cond::Field { field, sub, op, value } => {
            let f = object.field(field).ok_or_else(|| invalid(format!("No field named {field}")))?;
            let cols = match_columns(f, sub.as_deref());
            if cols.is_empty() {
                return Err(invalid(format!("{} can't be filtered on", f.label)));
            }
            let parts = cols.iter().map(|c| column_sql(f, c, *op, value, binds)).collect::<Result<Vec<_>>>()?;
            let joiner = if matches!(op, Op::NotContains | Op::IsEmpty | Op::IsNot | Op::NotIn) { " AND " } else { " OR " };
            format!("({})", parts.join(joiner))
        }
    })
}

/// The WHERE clause of a list: live or deleted rows, the filter, the search.
fn where_sql(object: &ObjectDef, query: &Query, binds: &mut Vec<Bind>) -> Result<String> {
    let mut parts = vec![if query.deleted { "t.deleted_at IS NOT NULL".to_owned() } else { "t.deleted_at IS NULL".to_owned() }];
    if let Some(Json(filter)) = &query.filter {
        if !filter.is_null() {
            parts.push(cond_sql(object, &parse_filter(filter)?, binds)?);
        }
    }
    if let Some(text) = query.search.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        binds.push(Bind::Text(object.meta.name_singular.clone()));
        binds.push(Bind::Text(search::match_query(text)));
        parts.push("t.id IN (SELECT record_id FROM search_index WHERE object = ? AND search_index MATCH ?)".into());
    }
    Ok(parts.join(" AND "))
}

fn order_sql(object: &ObjectDef, sorts: &[Sort]) -> Result<String> {
    let mut parts = Vec::new();
    for sort in sorts {
        let field = object.field(&sort.field).ok_or_else(|| invalid(format!("No field named {}", sort.field)))?;
        let cols = match_columns(field, sort.sub.as_deref());
        if cols.is_empty() {
            return Err(invalid(format!("{} can't be sorted on", field.label)));
        }
        for c in cols {
            let collate = if c.kind == ColumnKind::Text { " COLLATE NOCASE" } else { "" };
            parts.push(format!("t.\"{}\"{collate} {} NULLS LAST", c.name, if sort.desc { "DESC" } else { "ASC" }));
        }
    }
    parts.push("t.position ASC".into());
    parts.push("t.created_at ASC".into());
    Ok(parts.join(", "))
}

// ---------- reading ----------

/// The objects this object's many-to-one relations point at, by field name.
async fn targets(pool: &SqlitePool, object: &ObjectDef) -> Result<BTreeMap<String, ObjectDef>> {
    let mut out = BTreeMap::new();
    for field in object.fields.iter().filter(|f| f.is_active && f.r#type == FieldType::Relation) {
        let Some(rel) = &field.options.relation else { continue };
        if rel.kind != RelationKind::ManyToOne {
            continue;
        }
        if let Ok(target) = super::object(pool, &rel.target).await {
            out.insert(field.name.clone(), target);
        }
    }
    Ok(out)
}

fn select_sql(object: &ObjectDef, targets: &BTreeMap<String, ObjectDef>) -> String {
    let mut parts = vec!["t.*".to_owned()];
    for (name, target) in targets {
        let Some(field) = object.field(name) else { continue };
        parts.push(format!(
            "(SELECT {} FROM {} x WHERE x.id = t.\"{}_id\") AS \"{}__label\"",
            target.label_sql("x"),
            target.meta.table_name,
            field.column,
            name
        ));
    }
    parts.join(", ")
}

fn text(row: &SqliteRow, col: &str) -> Option<String> {
    row.try_get::<Option<String>, _>(col).ok().flatten()
}

fn real(row: &SqliteRow, col: &str) -> Option<f64> {
    row.try_get::<Option<f64>, _>(col).ok().flatten()
}

fn int(row: &SqliteRow, col: &str) -> Option<i64> {
    row.try_get::<Option<i64>, _>(col).ok().flatten()
}

fn json_text(row: &SqliteRow, col: &str) -> Value {
    text(row, col).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null)
}

fn column_value(row: &SqliteRow, c: &StoredColumn, field: &FieldMeta) -> Value {
    if c.json {
        return match json_text(row, &c.name) {
            Value::Null if matches!(field.r#type, FieldType::MultiSelect | FieldType::Array) || c.key.is_some_and(|k| k.starts_with("additional") || k == "secondaryLinks") => json!([]),
            v => v,
        };
    }
    match c.kind {
        ColumnKind::Real => real(row, &c.name).map(Value::from).unwrap_or(Value::Null),
        ColumnKind::Int if field.r#type == FieldType::Boolean => Value::Bool(int(row, &c.name).unwrap_or(0) != 0),
        ColumnKind::Int => int(row, &c.name).map(Value::from).unwrap_or(Value::Null),
        ColumnKind::Text => match text(row, &c.name) {
            Some(s) => Value::String(s),
            None if is_date(field) || field.r#type == FieldType::Select => Value::Null,
            None => Value::String(String::new()),
        },
    }
}

fn row_json(object: &ObjectDef, row: &SqliteRow) -> Value {
    let mut map = Map::new();
    map.insert("id".into(), text(row, "id").map(Value::String).unwrap_or(Value::Null));
    map.insert("createdAt".into(), text(row, "created_at").map(Value::String).unwrap_or(Value::Null));
    map.insert("updatedAt".into(), text(row, "updated_at").map(Value::String).unwrap_or(Value::Null));
    map.insert("deletedAt".into(), text(row, "deleted_at").map(Value::String).unwrap_or(Value::Null));
    for field in object.fields.iter().filter(|f| f.is_active) {
        match field.r#type {
            FieldType::Relation => {
                let Some(rel) = &field.options.relation else { continue };
                if rel.kind != RelationKind::ManyToOne {
                    continue;
                }
                let id = text(row, &format!("{}_id", field.column));
                map.insert(format!("{}Id", field.name), id.clone().map(Value::String).unwrap_or(Value::Null));
                let label = text(row, &format!("{}__label", field.name)).unwrap_or_default();
                map.insert(field.name.clone(), id.map(|id| json!({ "id": id, "label": label })).unwrap_or(Value::Null));
            }
            t if !subs(t).is_empty() => {
                let mut part = Map::new();
                for c in columns(field) {
                    if let Some(key) = c.key {
                        part.insert(key.into(), column_value(row, &c, field));
                    }
                }
                map.insert(field.name.clone(), Value::Object(part));
            }
            _ => {
                if let Some(c) = columns(field).first() {
                    map.insert(field.name.clone(), column_value(row, c, field));
                }
            }
        }
    }
    Value::Object(map)
}

pub async fn list(pool: &SqlitePool, object: &ObjectDef, query: &Query) -> Result<Page> {
    let targets = targets(pool, object).await?;
    let mut binds = Vec::new();
    let where_ = where_sql(object, query, &mut binds)?;
    let count_sql = format!("SELECT count(*) FROM {} t WHERE {where_}", object.meta.table_name);
    let (total,): (i64,) = bind_all(sqlx::query(sql(count_sql)), &binds)
        .fetch_one(pool)
        .await
        .map(|row| (row.get::<i64, _>(0),))
        .context("counting records")?;
    let sql_text = format!(
        "SELECT {} FROM {} t WHERE {where_} ORDER BY {} LIMIT ? OFFSET ?",
        select_sql(object, &targets),
        object.meta.table_name,
        order_sql(object, &query.sort)?
    );
    binds.push(Bind::Int(i64::from(query.limit.unwrap_or(PAGE).min(MAX_PAGE))));
    binds.push(Bind::Int(i64::from(query.offset.unwrap_or(0))));
    let rows = bind_all(sqlx::query(sql(sql_text)), &binds).fetch_all(pool).await.context("listing records")?;
    Ok(Page { records: rows.iter().map(|r| Json(row_json(object, r))).collect(), total: total.clamp(0, i64::from(u32::MAX)) as u32 })
}

/// One record by id (a deleted one too when `any` is set).
pub async fn get(pool: &SqlitePool, object: &ObjectDef, id: &str, any: bool) -> Result<Value> {
    let targets = targets(pool, object).await?;
    let sql_text = format!(
        "SELECT {} FROM {} t WHERE t.id = ?{}",
        select_sql(object, &targets),
        object.meta.table_name,
        if any { "" } else { " AND t.deleted_at IS NULL" }
    );
    let row = sqlx::query(sql(sql_text)).bind(id).fetch_optional(pool).await?;
    row.map(|r| row_json(object, &r))
        .ok_or_else(|| anyhow!(AppError::not_found(format!("No {} with that id", object.meta.label_singular.to_lowercase()))))
}

// ---------- writing ----------

fn parse_date(label: &str, s: &str) -> Result<String> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").map(|d| d.to_string()).map_err(|_| invalid(format!("{label}: a date is YYYY-MM-DD")))
}

fn parse_date_time(label: &str, s: &str) -> Result<String> {
    let s = s.trim();
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&chrono::Utc).format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string());
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(format!("{d}T00:00:00.000Z"));
    }
    Err(invalid(format!("{label}: a date and time is ISO-8601")))
}

fn string_list(label: &str, value: &Value) -> Result<Vec<String>> {
    match value {
        Value::Null => Ok(Vec::new()),
        Value::Array(items) => items.iter().map(|v| v.as_str().map(str::to_owned).ok_or_else(|| invalid(format!("{label}: a list of text")))).collect(),
        Value::String(s) => Ok(s.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect()),
        _ => Err(invalid(format!("{label}: a list of text"))),
    }
}

/// Whether a value is one the field takes (an import checks every cell before writing).
pub fn check_value(field: &FieldMeta, value: &Value) -> Result<()> {
    binds_for(field, value).map(|_| ())
}

/// The columns a field's value becomes, checked for its type.
fn binds_for(field: &FieldMeta, value: &Value) -> Result<Vec<(String, Bind)>> {
    let label = field.label.as_str();
    let composite = subs(field.r#type);
    if !composite.is_empty() {
        if !(value.is_object() || value.is_null()) {
            return Err(invalid(format!("{label}: an object with its parts")));
        }
        let mut out = Vec::new();
        for c in columns(field) {
            let part = c.key.and_then(|k| value.get(k)).cloned().unwrap_or(Value::Null);
            let bind = match (c.kind, c.json) {
                (ColumnKind::Text, true) => Bind::Text(match part {
                    Value::Null => "[]".into(),
                    other => other.to_string(),
                }),
                (ColumnKind::Text, false) => Bind::Text(match part {
                    Value::Null => String::new(),
                    Value::String(s) => s,
                    other => other.to_string(),
                }),
                _ => to_bind(&c, &part),
            };
            out.push((c.name, bind));
        }
        return Ok(out);
    }
    let Some(c) = columns(field).into_iter().next() else {
        return Err(invalid(format!("{label} can't be set directly")));
    };
    let bind = match field.r#type {
        FieldType::Text | FieldType::LongText | FieldType::RichText | FieldType::Uuid => match value {
            Value::Null => Bind::Text(String::new()),
            Value::String(s) => Bind::Text(s.clone()),
            other => Bind::Text(other.to_string()),
        },
        FieldType::Date => match value.as_str().map(str::trim).filter(|s| !s.is_empty()) {
            Some(s) => Bind::Text(parse_date(label, s)?),
            None => Bind::Null,
        },
        FieldType::DateTime => match value.as_str().map(str::trim).filter(|s| !s.is_empty()) {
            Some(s) => Bind::Text(parse_date_time(label, s)?),
            None => Bind::Null,
        },
        FieldType::Number | FieldType::Position => match to_bind(&c, value) {
            Bind::Null if !value.is_null() => return Err(invalid(format!("{label}: a number"))),
            b => b,
        },
        FieldType::Rating => match value {
            Value::Null => Bind::Null,
            Value::String(s) if s.starts_with("RATING_") => Bind::Int(s[7..].parse().map_err(|_| invalid(format!("{label}: 1 to 5")))?),
            other => match to_bind(&c, other) {
                Bind::Int(n) if (1..=5).contains(&n) => Bind::Int(n),
                _ => return Err(invalid(format!("{label}: 1 to 5"))),
            },
        },
        FieldType::Boolean => match value {
            Value::Null => Bind::Int(0),
            Value::Bool(b) => Bind::Int(i64::from(*b)),
            Value::Number(n) => Bind::Int(i64::from(n.as_f64().unwrap_or(0.0) != 0.0)),
            Value::String(s) => Bind::Int(i64::from(matches!(s.as_str(), "1" | "true" | "yes"))),
            _ => return Err(invalid(format!("{label}: true or false"))),
        },
        FieldType::Select => match value.as_str().map(str::trim).filter(|s| !s.is_empty()) {
            // A required select cleared goes back to its first option (the triggers of
            // 0021 and the stage column expect a value).
            None if !field.is_nullable => Bind::Text(field.options.select.first().map(|o| o.value.clone()).unwrap_or_default()),
            None => Bind::Text(String::new()),
            Some(v) => {
                if !field.options.select.iter().any(|o| o.value == v) {
                    return Err(invalid(format!("{label}: not one of its options")));
                }
                Bind::Text(v.to_owned())
            }
        },
        FieldType::MultiSelect => {
            let values = string_list(label, value)?;
            if let Some(bad) = values.iter().find(|v| !field.options.select.iter().any(|o| &o.value == *v)) {
                return Err(invalid(format!("{label}: {bad} is not one of its options")));
            }
            Bind::Text(serde_json::to_string(&values)?)
        }
        FieldType::Array => Bind::Text(serde_json::to_string(&string_list(label, value)?)?),
        FieldType::Json => Bind::Text(if value.is_null() { "null".into() } else { value.to_string() }),
        FieldType::Relation => match value {
            Value::Null => Bind::Null,
            Value::String(s) if s.trim().is_empty() => Bind::Null,
            Value::String(s) => Bind::Text(s.clone()),
            Value::Object(o) => o.get("id").and_then(Value::as_str).map(|s| Bind::Text(s.to_owned())).unwrap_or(Bind::Null),
            _ => return Err(invalid(format!("{label}: a record id"))),
        },
        FieldType::Actor | FieldType::FullName | FieldType::Emails | FieldType::Phones | FieldType::Links | FieldType::Domain | FieldType::Address | FieldType::Currency => {
            unreachable!("composites are handled above")
        }
    };
    Ok(vec![(c.name, bind)])
}

fn actor_columns(object: &ObjectDef, actor: &Actor) -> Vec<(String, Bind)> {
    let Some(field) = object.fields.iter().find(|f| f.r#type == FieldType::Actor && f.name == "createdBy") else {
        return Vec::new();
    };
    vec![
        (format!("{}_source", field.column), Bind::Text(actor.source.clone())),
        (format!("{}_name", field.column), Bind::Text(actor.name.clone())),
        (format!("{}_member_id", field.column), actor.member_id.clone().map(Bind::Text).unwrap_or(Bind::Null)),
        (format!("{}_context", field.column), Bind::Text("{}".into())),
    ]
}

/// Columns the table needs on insert that no field sets (New Dawn's own, D-042).
fn insert_defaults(object: &ObjectDef) -> Vec<(String, Bind)> {
    match object.meta.name_singular.as_str() {
        "task" => vec![("origin".into(), Bind::Text("typed".into()))],
        _ => Vec::new(),
    }
}

fn get_bind<'a>(cols: &'a [(String, Bind)], name: &str) -> Option<&'a Bind> {
    cols.iter().find(|(c, _)| c == name).map(|(_, b)| b)
}

/// What New Dawn keeps beside Twenty's columns: a person's single-string name and organisation
/// text (the meeting pipeline reads them), a task's done time (Tasks reads it).
async fn hooks(pool: &SqlitePool, object: &ObjectDef, cols: &mut Vec<(String, Bind)>, creating: bool) -> Result<()> {
    match object.meta.name_singular.as_str() {
        "person" => {
            if let (Some(Bind::Text(first)), Some(Bind::Text(last))) = (get_bind(cols, "name_first"), get_bind(cols, "name_last")) {
                let name = format!("{first} {last}").trim().to_owned();
                cols.push(("name".into(), Bind::Text(name)));
            }
            match get_bind(cols, "company_id") {
                Some(Bind::Text(id)) => {
                    let row: Option<(String,)> = sqlx::query_as("SELECT name FROM company WHERE id = ?").bind(id).fetch_optional(pool).await?;
                    cols.push(("organisation".into(), Bind::Text(row.map(|(n,)| n).unwrap_or_default())));
                }
                Some(Bind::Null) => cols.push(("organisation".into(), Bind::Text(String::new()))),
                _ => {}
            }
        }
        "task" => {
            if let Some(Bind::Text(status)) = get_bind(cols, "status") {
                // Done keeps the time it was first done.
                let bind = if status != "DONE" {
                    Bind::Null
                } else if creating {
                    Bind::Text(chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
                } else {
                    Bind::Raw(format!("coalesce(done_at, {NOW})"))
                };
                cols.push(("done_at".into(), bind));
            }
        }
        _ => {}
    }
    Ok(())
}

/// Relations must point at records that exist; unique fields must be unique among live records.
async fn check(pool: &SqlitePool, object: &ObjectDef, cols: &[(String, Bind)], id: &str) -> Result<()> {
    for field in object.fields.iter().filter(|f| f.is_active) {
        if field.r#type == FieldType::Relation {
            let Some(rel) = &field.options.relation else { continue };
            if rel.kind != RelationKind::ManyToOne {
                continue;
            }
            if let Some(Bind::Text(target_id)) = get_bind(cols, &format!("{}_id", field.column)) {
                let target = super::object(pool, &rel.target).await?;
                let (n,): (i64,) = sqlx::query_as(sql(format!("SELECT count(*) FROM {} WHERE id = ? AND deleted_at IS NULL", target.meta.table_name)))
                    .bind(target_id)
                    .fetch_one(pool)
                    .await?;
                if n == 0 {
                    return Err(invalid(format!("{}: no such {}", field.label, target.meta.label_singular.to_lowercase())));
                }
            }
        }
        if field.is_unique {
            for c in columns(field).into_iter().filter(|c| c.kind == ColumnKind::Text && !c.json) {
                if let Some(Bind::Text(v)) = get_bind(cols, &c.name) {
                    if v.is_empty() {
                        continue;
                    }
                    let (n,): (i64,) = sqlx::query_as(sql(format!(
                        "SELECT count(*) FROM {} WHERE \"{}\" = ? AND id <> ? AND deleted_at IS NULL",
                        object.meta.table_name, c.name
                    )))
                    .bind(v)
                    .bind(id)
                    .fetch_one(pool)
                    .await?;
                    if n > 0 {
                        return Err(invalid(format!("{} must be unique", field.label)));
                    }
                }
            }
        }
    }
    Ok(())
}

fn writable(field: &FieldMeta) -> bool {
    field.is_active && field.name != "createdBy" && !columns(field).is_empty()
}

pub async fn create(pool: &SqlitePool, object: &ObjectDef, data: &Value, actor: &Actor) -> Result<Value> {
    let data = data.as_object().ok_or_else(|| invalid("A record is an object of its fields"))?;
    let id = uuid::Uuid::now_v7().to_string();
    let mut cols: Vec<(String, Bind)> = vec![("id".into(), Bind::Text(id.clone()))];
    cols.extend(actor_columns(object, actor));
    cols.extend(insert_defaults(object));
    for field in object.fields.iter().filter(|f| writable(f)) {
        let value = data.get(&field.name).or(field.default_value.as_ref().map(|j| &j.0));
        match value {
            Some(value) => cols.extend(binds_for(field, value)?),
            // A required field without a value gets its empty shape (an empty name, the first
            // option), as Twenty makes an "Unnamed" record, instead of a NOT NULL failure.
            None if !field.is_nullable => cols.extend(binds_for(field, &Value::Null)?),
            None => {}
        }
    }
    if get_bind(&cols, "position").is_none() {
        let (next,): (f64,) = sqlx::query_as(sql(format!("SELECT coalesce(max(position), 0.0) + 1.0 FROM {}", object.meta.table_name)))
            .fetch_one(pool)
            .await?;
        cols.push(("position".into(), Bind::Real(next)));
    }
    hooks(pool, object, &mut cols, true).await?;
    check(pool, object, &cols, &id).await?;
    let names: Vec<String> = cols.iter().map(|(c, _)| format!("\"{c}\"")).collect();
    let marks = cols.iter().map(|(_, b)| match b { Bind::Raw(e) => e.clone(), _ => "?".into() }).collect::<Vec<_>>().join(", ");
    let sql_text = format!("INSERT INTO {} ({}) VALUES ({marks})", object.meta.table_name, names.join(", "));
    let binds: Vec<Bind> = cols.into_iter().map(|(_, b)| b).collect();
    bind_all(sqlx::query(sql(sql_text)), &binds).execute(pool).await.context("creating the record")?;
    let record = get(pool, object, &id, true).await?;
    timeline::record(pool, object, &id, "created", json!({}), actor).await?;
    search::index(pool, object, &record).await?;
    Ok(record)
}

pub async fn update(pool: &SqlitePool, object: &ObjectDef, id: &str, data: &Value, actor: &Actor) -> Result<Value> {
    let data = data.as_object().ok_or_else(|| invalid("A record is an object of its fields"))?;
    let before = get(pool, object, id, true).await?;
    let mut cols: Vec<(String, Bind)> = Vec::new();
    let mut touched = Vec::new();
    for field in object.fields.iter().filter(|f| writable(f)) {
        if let Some(value) = data.get(&field.name) {
            cols.extend(binds_for(field, value)?);
            touched.push(field.name.clone());
        }
    }
    if cols.is_empty() {
        return Ok(before);
    }
    hooks(pool, object, &mut cols, false).await?;
    check(pool, object, &cols, id).await?;
    let sets: Vec<String> = cols
        .iter()
        .map(|(c, b)| match b {
            Bind::Raw(e) => format!("\"{c}\" = {e}"),
            _ => format!("\"{c}\" = ?"),
        })
        .collect();
    let sql_text = format!("UPDATE {} SET {}, updated_at = {NOW} WHERE id = ?", object.meta.table_name, sets.join(", "));
    let mut binds: Vec<Bind> = cols.into_iter().map(|(_, b)| b).collect();
    binds.push(Bind::Text(id.to_owned()));
    bind_all(sqlx::query(sql(sql_text)), &binds).execute(pool).await.context("updating the record")?;
    let after = get(pool, object, id, true).await?;
    let mut diff = Map::new();
    for name in touched {
        let (b, a) = (before.get(&name).cloned().unwrap_or(Value::Null), after.get(&name).cloned().unwrap_or(Value::Null));
        if b != a {
            diff.insert(name, json!({ "before": b, "after": a }));
        }
    }
    if !diff.is_empty() {
        timeline::record(pool, object, id, "updated", json!({ "diff": diff }), actor).await?;
    }
    search::index(pool, object, &after).await?;
    Ok(after)
}

/// A soft delete: the record waits under Deleted records until restored or deleted for good.
pub async fn delete(pool: &SqlitePool, object: &ObjectDef, id: &str, actor: &Actor) -> Result<()> {
    let done = sqlx::query(sql(format!("UPDATE {} SET deleted_at = {NOW}, updated_at = {NOW} WHERE id = ? AND deleted_at IS NULL", object.meta.table_name)))
        .bind(id)
        .execute(pool)
        .await?;
    if done.rows_affected() == 0 {
        return Err(anyhow!(AppError::not_found("That record isn't there")));
    }
    timeline::record(pool, object, id, "deleted", json!({}), actor).await?;
    search::remove(pool, &object.meta.name_singular, id).await
}

pub async fn restore(pool: &SqlitePool, object: &ObjectDef, id: &str, actor: &Actor) -> Result<Value> {
    sqlx::query(sql(format!("UPDATE {} SET deleted_at = NULL, updated_at = {NOW} WHERE id = ?", object.meta.table_name)))
        .bind(id)
        .execute(pool)
        .await?;
    let record = get(pool, object, id, false).await?;
    timeline::record(pool, object, id, "restored", json!({}), actor).await?;
    search::index(pool, object, &record).await?;
    Ok(record)
}

/// Deletes for good (the database's cascades take the targets, favorites and attachments).
pub async fn destroy(pool: &SqlitePool, object: &ObjectDef, id: &str) -> Result<()> {
    if object.meta.name_singular == "company" {
        // Else the people memory's organisation text would bring the company back at the next open.
        sqlx::query("UPDATE person SET organisation = '' WHERE company_id = ?").bind(id).execute(pool).await?;
    }
    sqlx::query(sql(format!("DELETE FROM {} WHERE id = ?", object.meta.table_name))).bind(id).execute(pool).await?;
    timeline::forget(pool, object, id).await?;
    super::navigation::forget_visits(pool, &object.meta.name_singular, Some(id)).await?;
    search::remove(pool, &object.meta.name_singular, id).await
}

// ---------- aggregates and groups (views, D-061) ----------

/// One footer or group aggregate: a field (a composite's part), and the operation — COUNT,
/// COUNT_EMPTY, COUNT_NOT_EMPTY, COUNT_UNIQUE, PERCENT_EMPTY, PERCENT_NOT_EMPTY, SUM, AVG, MIN, MAX.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AggregateInput {
    pub field: String,
    #[serde(default)]
    pub sub: Option<String>,
    pub op: String,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct AggregateValue {
    pub field: String,
    pub op: String,
    pub value: Json,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct GroupCount {
    /// The select option's value ('' = none).
    pub value: String,
    pub count: u32,
}

fn is_numeric_op(op: &str) -> bool {
    matches!(op, "SUM" | "AVG" | "MIN" | "MAX")
}

/// The column an aggregate reads: the named part, else a composite's number (a sum of amounts)
/// or its first text part.
fn aggregate_column(field: &FieldMeta, sub: Option<&str>, op: &str) -> Option<StoredColumn> {
    let cols = columns(field);
    match sub {
        Some(sub) => cols.into_iter().find(|c| c.key == Some(sub)),
        None if is_numeric_op(op) => cols.iter().find(|c| c.kind != ColumnKind::Text).cloned().or_else(|| cols.into_iter().next()),
        None => cols.iter().find(|c| c.kind == ColumnKind::Text && !c.json).cloned().or_else(|| cols.into_iter().next()),
    }
}

/// The aggregates over the records a query lists, in one SELECT.
pub async fn aggregate(pool: &SqlitePool, object: &ObjectDef, query: &Query, ops: &[AggregateInput]) -> Result<Vec<AggregateValue>> {
    let mut binds = Vec::new();
    let where_ = where_sql(object, query, &mut binds)?;
    let mut selects = vec!["count(*)".to_owned()];
    for op in ops {
        let field = object.field(&op.field).ok_or_else(|| invalid(format!("No field named {}", op.field)))?;
        let c = aggregate_column(field, op.sub.as_deref(), &op.op).ok_or_else(|| invalid(format!("{} can't be aggregated", field.label)))?;
        let col = format!("t.\"{}\"", c.name);
        let empty = empty_sql(&c, &col);
        selects.push(match op.op.as_str() {
            "COUNT" => "count(*)".to_owned(),
            "COUNT_EMPTY" => format!("sum(CASE WHEN {empty} THEN 1 ELSE 0 END)"),
            "COUNT_NOT_EMPTY" => format!("sum(CASE WHEN {empty} THEN 0 ELSE 1 END)"),
            "COUNT_UNIQUE" => format!("count(DISTINCT CASE WHEN {empty} THEN NULL ELSE {col} END)"),
            "PERCENT_EMPTY" => format!("100.0 * sum(CASE WHEN {empty} THEN 1 ELSE 0 END) / max(count(*), 1)"),
            "PERCENT_NOT_EMPTY" => format!("100.0 * sum(CASE WHEN {empty} THEN 0 ELSE 1 END) / max(count(*), 1)"),
            "SUM" => format!("sum({col})"),
            "AVG" => format!("avg({col})"),
            "MIN" => format!("min({col})"),
            "MAX" => format!("max({col})"),
            other => return Err(invalid(format!("No aggregate named {other}"))),
        });
    }
    let sql_text = format!("SELECT {} FROM {} t WHERE {where_}", selects.join(", "), object.meta.table_name);
    let row = bind_all(sqlx::query(sql(sql_text)), &binds).fetch_one(pool).await.context("aggregating records")?;
    let mut out = Vec::new();
    for (i, op) in ops.iter().enumerate() {
        let idx = i + 1;
        let value = if let Ok(Some(n)) = row.try_get::<Option<f64>, _>(idx) {
            Value::from(n)
        } else if let Ok(Some(n)) = row.try_get::<Option<i64>, _>(idx) {
            Value::from(n)
        } else if let Ok(Some(s)) = row.try_get::<Option<String>, _>(idx) {
            Value::String(s)
        } else {
            Value::Null
        };
        out.push(AggregateValue { field: op.field.clone(), op: op.op.clone(), value: Json(value) });
    }
    Ok(out)
}

/// How many records a query lists per value of a select field (a grouped table's headers).
pub async fn group_counts(pool: &SqlitePool, object: &ObjectDef, query: &Query, field_name: &str) -> Result<Vec<GroupCount>> {
    let field = object.field(field_name).ok_or_else(|| invalid(format!("No field named {field_name}")))?;
    if field.r#type != FieldType::Select {
        return Err(invalid(format!("{} isn't a select field", field.label)));
    }
    let Some(c) = columns(field).into_iter().next() else { return Ok(Vec::new()) };
    let mut binds = Vec::new();
    let where_ = where_sql(object, query, &mut binds)?;
    let sql_text = format!("SELECT coalesce(t.\"{}\", '') AS v, count(*) AS n FROM {} t WHERE {where_} GROUP BY v", c.name, object.meta.table_name);
    let rows = bind_all(sqlx::query(sql(sql_text)), &binds).fetch_all(pool).await.context("counting groups")?;
    Ok(rows
        .iter()
        .map(|r| GroupCount { value: r.try_get::<String, _>("v").unwrap_or_default(), count: r.try_get::<i64, _>("n").unwrap_or(0).clamp(0, i64::from(u32::MAX)) as u32 })
        .collect())
}
