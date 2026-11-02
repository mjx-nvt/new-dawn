//! Charts (D-067, 0.10.10): one query for a dashboard's or a record page's chart — the object,
//! a view-style filter, the X field (a select's options in their order, a date bucketed by its
//! granularity, a relation by its label, a boolean as True / False, text as is), the Y
//! aggregate, an optional second dimension that makes series — on the record engine, capped at
//! Twenty's 100 categories and 50 series. Sorting, cumulative sums and the omission of empty
//! categories are done on the result.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::{Row, SqlitePool};

use super::query::{self, Bind, Query};
use super::types::{columns, FieldMeta, FieldType, Json, ObjectDef, RelationKind};
use crate::error::AppError;

/// Twenty's display limits.
const MAX_CATEGORIES: usize = 100;
const MAX_SERIES: usize = 50;

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

/// A dimension: a field, and for a date its granularity (DAY · WEEK · MONTH · QUARTER · YEAR ·
/// DAY_OF_WEEK · MONTH_OF_YEAR · QUARTER_OF_YEAR).
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
pub struct Axis {
    pub field: String,
    #[serde(default)]
    pub granularity: Option<String>,
}

/// The value shown: COUNT · COUNT_EMPTY · COUNT_NOT_EMPTY · COUNT_UNIQUE · SUM · AVG · MIN · MAX ·
/// PERCENT_EMPTY · PERCENT_NOT_EMPTY · COUNT_TRUE · COUNT_FALSE · RATIO (the share of records
/// whose field holds `ratio_value`). A value is a number: Twenty's Earliest / Latest on a date
/// wait for a chart that can show a date.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
pub struct Measure {
    pub op: String,
    #[serde(default)]
    pub field: Option<String>,
    #[serde(default)]
    pub sub: Option<String>,
    #[serde(default)]
    pub ratio_value: Option<String>,
}

/// What a chart widget keeps in its config.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ChartConfig {
    /// The object's name.
    pub source: String,
    /// The view bar's filter JSON (rules and groups), or none.
    #[serde(default)]
    pub filter: Option<Json>,
    /// The categories; none for an aggregate of one value.
    #[serde(default)]
    pub x: Option<Axis>,
    pub y: Measure,
    /// A second dimension: one series a value.
    #[serde(default)]
    pub group: Option<Axis>,
    /// POSITION · ALPHABETICAL · VALUE_DESC · VALUE_ASC
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub omit_zero: bool,
    #[serde(default)]
    pub cumulative: bool,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct Category {
    pub key: String,
    pub label: String,
    /// A select option's colour name, when the dimension is a select.
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct Series {
    pub key: String,
    pub label: String,
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct ChartData {
    pub categories: Vec<Category>,
    pub series: Vec<Series>,
    /// The measure over everything the filter keeps (an aggregate chart's one value).
    pub total: f64,
    /// Some categories or series were left out (Twenty's "Undisplayed data").
    pub truncated: bool,
}

/// What a dimension's keys mean once the rows are back.
enum Keys {
    /// A select: every option in its order, with its colour.
    Options(Vec<(String, String, String)>),
    /// A relation: the target's labels, read for the kept keys once they are known.
    Relation(Box<ObjectDef>, BTreeMap<String, String>),
    Boolean,
    DayOfWeek,
    MonthOfYear,
    Plain,
}

/// The SQL of a dimension, and how its keys read.
async fn dimension(pool: &SqlitePool, object: &ObjectDef, axis: &Axis) -> Result<(String, Keys)> {
    let field = object.field(&axis.field).ok_or_else(|| invalid(format!("No field named {}", axis.field)))?;
    if field.r#type == FieldType::Relation {
        let rel = field.options.relation.as_ref().filter(|r| r.kind == RelationKind::ManyToOne).ok_or_else(|| invalid(format!("{} isn't a relation a chart groups by", field.label)))?;
        let target = super::object(pool, &rel.target).await?;
        return Ok((format!("coalesce(t.\"{}_id\", '')", field.column), Keys::Relation(Box::new(target), BTreeMap::new())));
    }
    let Some(c) = columns(field).into_iter().find(|c| !c.json) else { return Err(invalid(format!("{} can't be a chart's axis", field.label))) };
    let col = format!("t.\"{}\"", c.name);
    Ok(match field.r#type {
        FieldType::Select => (format!("coalesce({col}, '')"), Keys::Options(field.options.select.iter().map(|o| (o.value.clone(), o.label.clone(), o.color.clone())).collect())),
        FieldType::Boolean => (format!("CASE WHEN coalesce({col}, 0) = 0 THEN 'false' ELSE 'true' END"), Keys::Boolean),
        FieldType::Date | FieldType::DateTime => {
            let g = axis.granularity.as_deref().unwrap_or("MONTH");
            // A date-time is UTC text: bucketed in this Mac's own time (an evening record stays
            // on its day); a date is a day already. `%W` counts weeks from the first Monday.
            let when = if field.r#type == FieldType::DateTime { format!("{col}, 'localtime'") } else { col.clone() };
            let expr = match g {
                "DAY" => format!("strftime('%Y-%m-%d', {when})"),
                "WEEK" => format!("strftime('%Y-W%W', {when})"),
                "MONTH" => format!("strftime('%Y-%m', {when})"),
                "QUARTER" => format!("strftime('%Y', {when}) || '-Q' || ((cast(strftime('%m', {when}) as integer) + 2) / 3)"),
                "YEAR" => format!("strftime('%Y', {when})"),
                "DAY_OF_WEEK" => format!("strftime('%w', {when})"),
                "MONTH_OF_YEAR" => format!("strftime('%m', {when})"),
                "QUARTER_OF_YEAR" => format!("'Q' || ((cast(strftime('%m', {when}) as integer) + 2) / 3)"),
                other => return Err(invalid(format!("No granularity named {other}"))),
            };
            let keys = match g {
                "DAY_OF_WEEK" => Keys::DayOfWeek,
                "MONTH_OF_YEAR" => Keys::MonthOfYear,
                _ => Keys::Plain,
            };
            (format!("coalesce({expr}, '')"), keys)
        }
        _ => (format!("coalesce(cast({col} as text), '')"), Keys::Plain),
    })
}

/// The SQL of the measure, its binds, and whether the value is in micros (a currency).
fn measure(object: &ObjectDef, m: &Measure, binds: &mut Vec<Bind>) -> Result<(String, bool)> {
    if m.op == "COUNT" {
        return Ok(("count(*)".into(), false));
    }
    let name = m.field.as_deref().ok_or_else(|| invalid(format!("{} needs a field", m.op)))?;
    let field: &FieldMeta = object.field(name).ok_or_else(|| invalid(format!("No field named {name}")))?;
    let c = query::aggregate_column(field, m.sub.as_deref(), &m.op).ok_or_else(|| invalid(format!("{} can't be aggregated", field.label)))?;
    let micros = c.key == Some("amountMicros");
    let col = format!("t.\"{}\"", c.name);
    let empty = query::empty_sql(&c, &col);
    let sql = match m.op.as_str() {
        "COUNT_EMPTY" => format!("sum(CASE WHEN {empty} THEN 1 ELSE 0 END)"),
        "COUNT_NOT_EMPTY" => format!("sum(CASE WHEN {empty} THEN 0 ELSE 1 END)"),
        "COUNT_UNIQUE" => format!("count(DISTINCT CASE WHEN {empty} THEN NULL ELSE {col} END)"),
        "PERCENT_EMPTY" => format!("100.0 * sum(CASE WHEN {empty} THEN 1 ELSE 0 END) / max(count(*), 1)"),
        "PERCENT_NOT_EMPTY" => format!("100.0 * sum(CASE WHEN {empty} THEN 0 ELSE 1 END) / max(count(*), 1)"),
        "SUM" => format!("sum({col})"),
        "AVG" => format!("avg({col})"),
        "MIN" => format!("min({col})"),
        "MAX" => format!("max({col})"),
        "COUNT_TRUE" => format!("sum(CASE WHEN coalesce({col}, 0) <> 0 THEN 1 ELSE 0 END)"),
        "COUNT_FALSE" => format!("sum(CASE WHEN coalesce({col}, 0) = 0 THEN 1 ELSE 0 END)"),
        "RATIO" => {
            let wanted = m.ratio_value.clone().ok_or_else(|| invalid("A ratio needs the value to count"))?;
            let hit = match field.r#type {
                FieldType::Boolean => format!("coalesce({col}, 0) {} 0", if matches!(wanted.as_str(), "true" | "1") { "<>" } else { "=" }),
                FieldType::MultiSelect => {
                    binds.push(Bind::Text(format!("%\"{}\"%", wanted.replace('%', ""))));
                    format!("{col} LIKE ?")
                }
                _ => {
                    binds.push(Bind::Text(wanted));
                    format!("{col} = ?")
                }
            };
            format!("100.0 * sum(CASE WHEN {hit} THEN 1 ELSE 0 END) / max(count(*), 1)")
        }
        other => return Err(invalid(format!("No aggregate named {other}"))),
    };
    Ok((sql, micros))
}

/// A relation's labels for the kept keys only (never the whole target table).
async fn resolve_labels(pool: &SqlitePool, keys: Keys, kept: &[String]) -> Result<Keys> {
    let Keys::Relation(target, _) = keys else { return Ok(keys) };
    let ids: Vec<&String> = kept.iter().filter(|k| !k.is_empty()).collect();
    if ids.is_empty() {
        return Ok(Keys::Relation(target, BTreeMap::new()));
    }
    let sql = format!("SELECT t.id, {} FROM {} t WHERE t.id IN ({})", target.label_sql("t"), target.meta.table_name, vec!["?"; ids.len()].join(", "));
    let mut q = sqlx::query_as::<_, (String, String)>(query::sql(sql));
    for id in ids {
        q = q.bind(id.clone());
    }
    let labels = q.fetch_all(pool).await?.into_iter().collect();
    Ok(Keys::Relation(target, labels))
}

fn label_of(keys: &Keys, key: &str) -> String {
    const DAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
    const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    match keys {
        Keys::Options(options) => options.iter().find(|(v, _, _)| v == key).map(|(_, l, _)| l.clone()).unwrap_or_else(|| if key.is_empty() { "No value".into() } else { key.to_owned() }),
        Keys::Relation(_, labels) => labels.get(key).cloned().unwrap_or_else(|| if key.is_empty() { "No value".into() } else { "Deleted".into() }),
        Keys::Boolean => if key == "true" { "True".into() } else { "False".into() },
        Keys::DayOfWeek => key.parse::<usize>().ok().and_then(|d| DAYS.get(d)).map(|d| (*d).to_owned()).unwrap_or_else(|| key.to_owned()),
        Keys::MonthOfYear => key.parse::<usize>().ok().and_then(|m| MONTHS.get(m.wrapping_sub(1))).map(|m| (*m).to_owned()).unwrap_or_else(|| key.to_owned()),
        Keys::Plain => if key.is_empty() { "No value".into() } else { key.to_owned() },
    }
}

fn number(row: &sqlx::sqlite::SqliteRow, idx: usize) -> f64 {
    if let Ok(Some(n)) = row.try_get::<Option<f64>, _>(idx) {
        n
    } else if let Ok(Some(n)) = row.try_get::<Option<i64>, _>(idx) {
        n as f64
    } else if let Ok(Some(s)) = row.try_get::<Option<String>, _>(idx) {
        s.parse().unwrap_or(0.0)
    } else {
        0.0
    }
}

/// The chart's categories and series.
pub async fn chart_data(pool: &SqlitePool, object: &ObjectDef, config: &ChartConfig) -> Result<ChartData> {
    let query = Query { filter: config.filter.clone(), sort: Vec::new(), limit: None, offset: None, search: None, deleted: false };
    let mut binds: Vec<Bind> = Vec::new();
    let (value_sql, micros) = measure(object, &config.y, &mut binds)?;
    let scale = if micros { 1e-6 } else { 1.0 };
    let (x_sql, x_keys) = match &config.x {
        Some(axis) => dimension(pool, object, axis).await?,
        None => ("''".to_owned(), Keys::Plain),
    };
    let (g_sql, g_keys) = match &config.group {
        Some(axis) => dimension(pool, object, axis).await?,
        None => ("''".to_owned(), Keys::Plain),
    };
    let where_ = query::where_sql(object, &query, &mut binds)?;
    let sql_text = format!("SELECT {x_sql} AS x, {g_sql} AS g, {value_sql} AS v FROM {} t WHERE {where_} GROUP BY x, g", object.meta.table_name);
    let rows = query::bind_all(sqlx::query(query::sql(sql_text)), &binds).fetch_all(pool).await.context("charting records")?;
    // The one value over everything (the aggregate chart, the pie's centre).
    let mut total_binds: Vec<Bind> = Vec::new();
    let (total_sql, _) = measure(object, &config.y, &mut total_binds)?;
    let total_where = query::where_sql(object, &query, &mut total_binds)?;
    let total_row = query::bind_all(sqlx::query(query::sql(format!("SELECT {total_sql} FROM {} t WHERE {total_where}", object.meta.table_name))), &total_binds).fetch_one(pool).await?;
    let total = number(&total_row, 0) * scale;

    let mut cells: BTreeMap<(String, String), f64> = BTreeMap::new();
    let mut x_seen: Vec<String> = Vec::new();
    let mut g_seen: Vec<String> = Vec::new();
    let (mut x_set, mut g_set) = (BTreeSet::new(), BTreeSet::new());
    for row in &rows {
        let x: String = row.try_get("x").unwrap_or_default();
        let g: String = row.try_get("g").unwrap_or_default();
        let v = number(row, 2) * scale;
        if x_set.insert(x.clone()) {
            x_seen.push(x.clone());
        }
        if g_set.insert(g.clone()) {
            g_seen.push(g.clone());
        }
        cells.insert((x, g), v);
    }
    // The categories: a select's options in their order (every one, unless zeros are omitted),
    // else what the rows hold.
    let mut category_keys: Vec<String> = match &x_keys {
        Keys::Options(options) if config.x.is_some() => {
            let mut keys: Vec<String> = options.iter().map(|(v, _, _)| v.clone()).collect();
            let known: BTreeSet<&String> = keys.iter().collect();
            let extra: Vec<String> = x_seen.iter().filter(|k| !known.contains(k)).cloned().collect();
            keys.extend(extra);
            keys
        }
        _ => {
            let mut keys = x_seen.clone();
            keys.sort();
            keys
        }
    };
    let mut series_keys: Vec<String> = match &g_keys {
        Keys::Options(options) if config.group.is_some() => {
            let mut keys: Vec<String> = options.iter().map(|(v, _, _)| v.clone()).filter(|k| g_set.contains(k)).collect();
            let known: BTreeSet<&String> = keys.iter().collect();
            let extra: Vec<String> = g_seen.iter().filter(|k| !known.contains(k)).cloned().collect();
            keys.extend(extra);
            keys
        }
        _ => {
            let mut keys = g_seen.clone();
            keys.sort();
            keys
        }
    };
    let value_at = |x: &str, g: &str| cells.get(&(x.to_owned(), g.to_owned())).copied().unwrap_or(0.0);
    let totals: BTreeMap<String, f64> = category_keys.iter().map(|x| (x.clone(), series_keys.iter().map(|g| value_at(x, g)).sum::<f64>())).collect();
    let total_of = |x: &String| totals.get(x).copied().unwrap_or(0.0);
    match config.sort.as_deref().unwrap_or("POSITION") {
        "ALPHABETICAL" => category_keys.sort_by_key(|k| label_of(&x_keys, k).to_lowercase()),
        "VALUE_DESC" => category_keys.sort_by(|a, b| total_of(b).partial_cmp(&total_of(a)).unwrap_or(std::cmp::Ordering::Equal)),
        "VALUE_ASC" => category_keys.sort_by(|a, b| total_of(a).partial_cmp(&total_of(b)).unwrap_or(std::cmp::Ordering::Equal)),
        _ => {}
    }
    if config.omit_zero {
        category_keys.retain(|k| total_of(k) != 0.0);
    }
    let truncated = category_keys.len() > MAX_CATEGORIES || series_keys.len() > MAX_SERIES;
    category_keys.truncate(MAX_CATEGORIES);
    series_keys.truncate(MAX_SERIES);
    let x_keys = resolve_labels(pool, x_keys, &category_keys).await?;
    let g_keys = resolve_labels(pool, g_keys, &series_keys).await?;
    let categories = category_keys
        .iter()
        .map(|k| Category {
            key: k.clone(),
            label: label_of(&x_keys, k),
            color: match &x_keys {
                Keys::Options(options) => options.iter().find(|(v, _, _)| v == k).map(|(_, _, c)| c.clone()),
                _ => None,
            },
        })
        .collect();
    let series = series_keys
        .iter()
        .map(|g| {
            let mut running = 0.0;
            let values = category_keys
                .iter()
                .map(|x| {
                    let v = value_at(x, g);
                    if config.cumulative {
                        running += v;
                        running
                    } else {
                        v
                    }
                })
                .collect();
            Series { key: g.clone(), label: if config.group.is_some() { label_of(&g_keys, g) } else { String::new() }, values }
        })
        .collect();
    Ok(ChartData { categories, series, total, truncated })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::hub::WorkspaceHub;
    use crate::kernel::records::query::Actor;
    use serde_json::json;

    /// "Pipeline amount by stage" and "Opportunities closed by month": a sum by a select's
    /// options in their order (every option, zeros kept), a count by a date's months with a
    /// filter, a count by relation, a ratio, and the cumulative and sorted forms.
    #[test]
    fn charts_group_sum_count_and_sort() {
        let root = std::env::temp_dir().join(format!("newdawn-charts-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let actor = Actor::system("test");
            let company = super::super::object(&pool, "company").await.unwrap();
            let keppel = query::create(&pool, &company, &json!({ "name": "Keppel Land" }), &actor).await.unwrap();
            let opportunity = super::super::object(&pool, "opportunity").await.unwrap();
            let stage = opportunity.field("stage").unwrap().options.select.clone();
            let (first, second) = (stage[0].value.clone(), stage[1].value.clone());
            for (name, amount, st, close) in [("A", 100.0, &first, "2026-01-15"), ("B", 250.0, &first, "2026-02-03"), ("C", 50.0, &second, "2026-02-20")] {
                query::create(&pool, &opportunity, &json!({ "name": name, "amount": { "amountMicros": (amount * 1e6) as i64, "currencyCode": "USD" }, "stage": st, "closeDate": close, "company": keppel["id"] }), &actor).await.unwrap();
            }
            let by_stage = chart_data(&pool, &opportunity, &ChartConfig { source: "opportunity".into(), filter: None, x: Some(Axis { field: "stage".into(), granularity: None }), y: Measure { op: "SUM".into(), field: Some("amount".into()), sub: None, ratio_value: None }, group: None, sort: None, omit_zero: false, cumulative: false }).await.unwrap();
            assert_eq!(by_stage.categories.len(), stage.len(), "every option, in order, zeros kept");
            assert_eq!(by_stage.categories[0].label, stage[0].label);
            assert_eq!(by_stage.series[0].values[0], 350.0, "micros became units");
            assert_eq!(by_stage.series[0].values[1], 50.0);
            assert_eq!(by_stage.total, 400.0);
            let by_month = chart_data(&pool, &opportunity, &ChartConfig { source: "opportunity".into(), filter: Some(Json(json!({ "and": [{ "field": "stage", "op": "in", "value": [first] }] }))), x: Some(Axis { field: "closeDate".into(), granularity: Some("MONTH".into()) }), y: Measure { op: "COUNT".into(), field: None, sub: None, ratio_value: None }, group: None, sort: None, omit_zero: true, cumulative: true }).await.unwrap();
            assert_eq!(by_month.categories.iter().map(|c| c.key.as_str()).collect::<Vec<_>>(), ["2026-01", "2026-02"]);
            assert_eq!(by_month.series[0].values, [1.0, 2.0], "cumulative");
            let by_company = chart_data(&pool, &opportunity, &ChartConfig { source: "opportunity".into(), filter: None, x: Some(Axis { field: "company".into(), granularity: None }), y: Measure { op: "COUNT".into(), field: None, sub: None, ratio_value: None }, group: Some(Axis { field: "stage".into(), granularity: None }), sort: Some("VALUE_DESC".into()), omit_zero: true, cumulative: false }).await.unwrap();
            assert_eq!(by_company.categories[0].label, "Keppel Land");
            assert_eq!(by_company.series.len(), 2, "one series a stage seen");
            let ratio = chart_data(&pool, &opportunity, &ChartConfig { source: "opportunity".into(), filter: None, x: None, y: Measure { op: "RATIO".into(), field: Some("stage".into()), sub: None, ratio_value: Some(second.clone()) }, group: None, sort: None, omit_zero: false, cumulative: false }).await.unwrap();
            assert!((ratio.total - 33.33).abs() < 0.1, "{}", ratio.total);
            hub.close_all().await;
        });
        std::fs::remove_dir_all(&root).unwrap();
    }
}
