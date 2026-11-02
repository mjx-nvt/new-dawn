//! Import and export (D-065, 0.10.8): Twenty's spreadsheet import — a CSV / XLSX / XLS file,
//! its columns matched to fields ("Phones / Primary Phone Number" and the like), relations by
//! id or by a field of the other object, existing records updated by an identifier, every row
//! checked before anything is written — and a view's export to CSV with exactly its visible
//! columns, filters and sorts. Runs are kept in `import_job` and `export_job`.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use specta::Type;
use sqlx::SqlitePool;

use super::query::{self, Actor, Query};
use super::types::{subs, FieldMeta, FieldType, ObjectDef, RelationKind};
use super::{object, rich, split_name};
use crate::error::AppError;
use crate::kernel::csv;

/// Twenty's limits: rows a file may hold; records an export may hold.
const MAX_ROWS: usize = 10_000;
const MAX_EXPORT: u32 = 20_000;
/// Rows an export reads at a time.
const PAGE: u32 = 2_000;

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

// ---- the file -----------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Table {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// The first sheet of a workbook, or the CSV: the first row is the headers; cells are trimmed
/// and rows made as wide as the headers; blank rows are dropped.
pub fn read_table(path: &Path) -> Result<Table> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let mut rows = match ext.as_str() {
        "xlsx" | "xlsm" | "xlsb" | "xls" | "ods" => read_workbook(path)?,
        _ => {
            let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
            csv::parse(&String::from_utf8_lossy(&bytes))
        }
    };
    rows.retain(|r| r.iter().any(|c| !c.trim().is_empty()));
    if rows.is_empty() {
        return Err(invalid("The file has no rows"));
    }
    let headers: Vec<String> = rows.remove(0).into_iter().map(|h| h.trim().to_owned()).collect();
    if rows.len() > MAX_ROWS {
        return Err(invalid(format!("Too many records. Up to {MAX_ROWS} allowed")));
    }
    for r in &mut rows {
        r.resize(headers.len(), String::new());
        for c in r.iter_mut() {
            *c = c.trim().to_owned();
        }
    }
    Ok(Table { headers, rows })
}

fn read_workbook(path: &Path) -> Result<Vec<Vec<String>>> {
    use calamine::{open_workbook_auto, Reader};
    let mut book = open_workbook_auto(path).with_context(|| format!("opening {}", path.display()))?;
    let name = book.sheet_names().first().cloned().ok_or_else(|| invalid("The workbook has no sheet"))?;
    let range = book.worksheet_range(&name).with_context(|| format!("reading the sheet {name}"))?;
    Ok(range.rows().map(|r| r.iter().map(cell_text).collect()).collect())
}

fn cell_text(cell: &calamine::Data) -> String {
    use calamine::Data;
    match cell {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Float(f) => number_text(*f),
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_owned(),
        Data::DateTime(dt) => excel_date(dt.as_f64()),
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
        Data::Error(e) => e.to_string(),
    }
}

/// A whole number without ".0"; else the shortest text that reads back.
fn number_text(f: f64) -> String {
    if f.fract() == 0.0 && f.abs() < 1e15 {
        format!("{}", f as i64)
    } else {
        f.to_string()
    }
}

/// An Excel serial date (days since 1899-12-30) as ISO text, with its time when it has one.
fn excel_date(serial: f64) -> String {
    let days = serial.floor() as i64;
    let secs = ((serial - serial.floor()) * 86_400.0).round() as i64;
    // Civil from days (Howard Hinnant): Excel's day 0 is 25 569 days before 1970-01-01.
    let z = days - 25_569 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    if secs == 0 {
        format!("{y:04}-{m:02}-{d:02}")
    } else {
        format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}", secs / 3600, secs % 3600 / 60, secs % 60)
    }
}

// ---- the mapping --------------------------------------------------------------------------

/// A column of the file and the field it fills.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
pub struct ColumnMap {
    pub header: String,
    /// The field's name, "id" for the record's id, or none ("Do not map").
    #[serde(default)]
    pub field: Option<String>,
    /// A composite's part (its key), or none for the whole field.
    #[serde(default)]
    pub sub: Option<String>,
    /// A relation: "id", or the other object's field the cell names (its domain, its email…).
    #[serde(default)]
    pub match_by: Option<String>,
}

/// A field the Match columns step offers, with its parts and — for a relation — the ways a
/// cell can name the other object's record.
#[derive(Debug, Clone, Serialize, Type)]
pub struct FieldChoice {
    pub field: String,
    pub label: String,
    pub r#type: FieldType,
    pub subs: Vec<SubChoice>,
    pub match_options: Vec<MatchChoice>,
    pub required: bool,
    pub unique: bool,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct SubChoice {
    pub key: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct MatchChoice {
    pub value: String,
    pub label: String,
}

/// What the Select file step hands the sheet: the table, a first mapping, the identifier that
/// updates existing records, and the choices.
#[derive(Debug, Clone, Serialize, Type)]
pub struct ImportFile {
    pub file_name: String,
    pub table: Table,
    pub mapping: Vec<ColumnMap>,
    pub identifier: Option<String>,
    pub fields: Vec<FieldChoice>,
}

/// Twenty's labels for the parts of a composite ("Phones / Primary Phone Number").
pub fn sub_label(t: FieldType, key: &str) -> &'static str {
    match key {
        "firstName" => "First Name",
        "lastName" => "Last Name",
        "primaryEmail" => "Primary Email",
        "additionalEmails" => "Additional Emails",
        "primaryPhoneNumber" => "Primary Phone Number",
        "primaryPhoneCountryCode" => "Primary Phone Country Code",
        "primaryPhoneCallingCode" => "Primary Phone Calling Code",
        "additionalPhones" => "Additional Phones",
        "primaryLinkLabel" if t == FieldType::Domain => "Domain Label",
        "primaryLinkLabel" => "Link Label",
        "primaryLinkUrl" if t == FieldType::Domain => "Domain URL",
        "primaryLinkUrl" => "Link URL",
        "secondaryLinks" => "Secondary Links",
        "addressStreet1" => "Address 1",
        "addressStreet2" => "Address 2",
        "addressCity" => "City",
        "addressState" => "State",
        "addressCountry" => "Country",
        "addressPostcode" => "Post Code",
        "addressLat" => "Latitude",
        "addressLng" => "Longitude",
        "amountMicros" => "Amount",
        "currencyCode" => "Currency",
        "source" => "Source",
        "name" => "Name",
        "workspaceMemberId" => "Member Id",
        "context" => "Context",
        _ => "Value",
    }
}

/// The part a whole composite's cell fills ("Emails" → the primary email).
fn whole_key(t: FieldType) -> &'static str {
    match t {
        FieldType::FullName => "firstName",
        FieldType::Emails => "primaryEmail",
        FieldType::Phones => "primaryPhoneNumber",
        FieldType::Links | FieldType::Domain => "primaryLinkUrl",
        FieldType::Address => "addressStreet1",
        FieldType::Currency => "amountMicros",
        _ => "",
    }
}

/// The fields a file can fill: active, with columns of their own, not the engine's.
fn importable(def: &ObjectDef) -> Vec<&FieldMeta> {
    def.fields
        .iter()
        .filter(|f| {
            f.is_active
                && f.name != "createdBy"
                && !matches!(f.r#type, FieldType::Actor | FieldType::Position | FieldType::Uuid)
                && (f.r#type != FieldType::Relation || f.options.relation.as_ref().is_some_and(|r| r.kind == RelationKind::ManyToOne))
        })
        .collect()
}

fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

fn looks_like_uuid(s: &str) -> bool {
    s.len() == 36 && s.bytes().enumerate().all(|(i, b)| if matches!(i, 8 | 13 | 18 | 23) { b == b'-' } else { b.is_ascii_hexdigit() })
}

/// The ways a cell can name the other object's record: its id, its label, its unique fields,
/// a company's domain, a person's email.
fn match_options(target: &ObjectDef) -> Vec<MatchChoice> {
    let mut out = vec![MatchChoice { value: "id".into(), label: "Id".into() }];
    for f in target.fields.iter().filter(|f| f.is_active) {
        let wanted = Some(f.id.as_str()) == target.meta.label_identifier_field_id.as_deref() || f.is_unique || matches!(f.name.as_str(), "domainName" | "emails");
        if wanted && matches!(f.r#type, FieldType::Text | FieldType::FullName | FieldType::Emails | FieldType::Domain | FieldType::Links | FieldType::Uuid) {
            out.push(MatchChoice { value: f.name.clone(), label: f.label.clone() });
        }
    }
    out
}

/// The other objects the importable relations point at, by singular name.
pub async fn targets_of(pool: &SqlitePool, def: &ObjectDef) -> Result<HashMap<String, ObjectDef>> {
    let mut out = HashMap::new();
    for f in importable(def) {
        if let Some(rel) = &f.options.relation {
            if !out.contains_key(&rel.target) {
                out.insert(rel.target.clone(), object(pool, &rel.target).await?);
            }
        }
    }
    Ok(out)
}

/// The choices for the Match columns step.
pub fn field_choices(def: &ObjectDef, targets: &HashMap<String, ObjectDef>) -> Vec<FieldChoice> {
    let mut out = vec![FieldChoice { field: "id".into(), label: "Id".into(), r#type: FieldType::Uuid, subs: Vec::new(), match_options: Vec::new(), required: false, unique: true }];
    for f in importable(def) {
        let match_options = f.options.relation.as_ref().and_then(|r| targets.get(&r.target)).map(match_options).unwrap_or_default();
        let subs = subs(f.r#type).iter().map(|s| SubChoice { key: s.key.into(), label: sub_label(f.r#type, s.key).into() }).collect();
        out.push(FieldChoice { field: f.name.clone(), label: f.label.clone(), r#type: f.r#type, subs, match_options, required: !f.is_nullable, unique: f.is_unique });
    }
    out
}

/// A first mapping from the headers: Twenty's own "Label / Part" headers, field labels and
/// names, a part's key or label on its own, and the usual spreadsheet words (email, phone,
/// website, companyDomain…). A relation named by a bare label matches by id when the cells
/// hold ids, else by the other object's label.
pub fn suggest(def: &ObjectDef, targets: &HashMap<String, ObjectDef>, table: &Table) -> Vec<ColumnMap> {
    // (what a header may read, the field, its part, how a relation matches)
    let mut cands: Vec<(String, &str, Option<&str>, Option<String>)> = vec![("id".into(), "id", None, None), ("recordid".into(), "id", None, None)];
    let (mut link, mut emails, mut phones): (Option<&FieldMeta>, Option<&FieldMeta>, Option<&FieldMeta>) = (None, None, None);
    for f in importable(def) {
        if f.r#type == FieldType::Relation {
            let target = f.options.relation.as_ref().and_then(|r| targets.get(&r.target));
            let by_label = target.and_then(ObjectDef::label_field).map(|l| l.name.clone());
            for text in [f.label.as_str(), f.name.as_str()] {
                cands.push((norm(text), &f.name, None, by_label.clone()));
            }
            for text in [format!("{} id", f.label), format!("{}Id", f.name)] {
                cands.push((norm(&text), &f.name, None, Some("id".into())));
            }
            if let Some(t) = target {
                for other in match_options(t) {
                    for text in [format!("{}{}", f.name, other.label), format!("{} {}", f.label, other.label)] {
                        cands.push((norm(&text), &f.name, None, Some(other.value.clone())));
                    }
                }
                for (word, by) in [("domain", "domainName"), ("website", "domainName"), ("email", "emails")] {
                    if t.field(by).is_some() {
                        cands.push((norm(&format!("{}{word}", f.name)), &f.name, None, Some(by.into())));
                    }
                }
            }
            continue;
        }
        for text in [f.label.as_str(), f.name.as_str()] {
            cands.push((norm(text), &f.name, None, None));
        }
        for s in subs(f.r#type) {
            let sl = sub_label(f.r#type, s.key);
            for text in [format!("{} / {sl}", f.label), format!("{} {sl}", f.label), format!("{}.{}", f.name, s.key), format!("{} {}", f.name, s.key), s.key.to_owned(), sl.to_owned()] {
                cands.push((norm(&text), &f.name, Some(s.key), None));
            }
        }
        match f.r#type {
            FieldType::Links | FieldType::Domain => link = link.or(Some(f)),
            FieldType::Emails => emails = emails.or(Some(f)),
            FieldType::Phones => phones = phones.or(Some(f)),
            _ => {}
        }
    }
    if let Some(f) = emails {
        for w in ["email", "emailaddress", "mail"] {
            cands.push((w.into(), &f.name, Some("primaryEmail"), None));
        }
    }
    if let Some(f) = phones {
        for w in ["phone", "phonenumber", "mobile", "tel"] {
            cands.push((w.into(), &f.name, Some("primaryPhoneNumber"), None));
        }
    }
    if let Some(f) = link {
        for w in ["website", "url", "domain", "link"] {
            cands.push((w.into(), &f.name, Some("primaryLinkUrl"), None));
        }
    }
    if let Some(l) = def.label_field() {
        for w in ["fullname", "title", "label"] {
            cands.push((w.into(), &l.name, None, None));
        }
    }
    table
        .headers
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let n = norm(h);
            let Some((_, field, sub, by)) = cands.iter().find(|c| c.0 == n) else {
                return ColumnMap { header: h.clone(), field: None, sub: None, match_by: None };
            };
            let mut match_by = by.clone();
            if match_by.as_deref().is_some_and(|b| b != "id") && table.rows.iter().find_map(|r| r.get(i).filter(|c| !c.is_empty())).is_some_and(|c| looks_like_uuid(c)) {
                match_by = Some("id".into());
            }
            ColumnMap { header: h.clone(), field: Some((*field).to_owned()), sub: sub.map(str::to_owned), match_by }
        })
        .collect()
}

/// The field that names an existing record to update: the id when mapped, else a person's
/// email or a company's domain, else the first mapped unique field.
pub fn default_identifier(def: &ObjectDef, mapping: &[ColumnMap]) -> Option<String> {
    let mapped = |name: &str| mapping.iter().any(|m| m.field.as_deref() == Some(name));
    if mapped("id") {
        return Some("id".into());
    }
    for name in ["emails", "domainName"] {
        if def.field(name).is_some() && mapped(name) {
            return Some(name.into());
        }
    }
    def.fields.iter().find(|f| f.is_unique && f.is_active && mapped(&f.name)).map(|f| f.name.clone())
}

/// The Select file step: the table read, a mapping proposed, the choices listed.
pub async fn open(pool: &SqlitePool, def: &ObjectDef, path: &Path) -> Result<ImportFile> {
    // Reading and parsing a file is work for a CPU, not for the async workers.
    let file = path.to_path_buf();
    let table = tauri::async_runtime::spawn_blocking(move || read_table(&file)).await??;
    let targets = targets_of(pool, def).await?;
    let mapping = suggest(def, &targets, &table);
    let identifier = default_identifier(def, &mapping);
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file").to_owned();
    Ok(ImportFile { file_name, table, mapping, identifier, fields: field_choices(def, &targets) })
}

// ---- the rows -----------------------------------------------------------------------------

/// A select's option by its value or its label, any case.
fn option_value(f: &FieldMeta, text: &str) -> Option<String> {
    f.options.select.iter().find(|o| o.value.eq_ignore_ascii_case(text) || o.label.eq_ignore_ascii_case(text)).map(|o| o.value.clone())
}

/// A list cell: a JSON array, or items apart by `;` or `,`.
fn list_cell(t: &str) -> Vec<String> {
    match serde_json::from_str::<Value>(t) {
        Ok(Value::Array(items)) => items.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect(),
        _ => t.split([';', ',']).map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect(),
    }
}

/// Plain lines as the editor's document.
fn rich_doc(text: &str) -> Value {
    let paragraphs: Vec<Value> = text
        .lines()
        .map(|l| if l.trim().is_empty() { json!({ "type": "paragraph" }) } else { json!({ "type": "paragraph", "content": [{ "type": "text", "text": l }] }) })
        .collect();
    json!({ "type": "doc", "content": paragraphs })
}

/// A cell's value as the engine takes it: a part of a composite (as `{ part: value }`), or the
/// whole field. Empty is `Null` — cleared on an update, left to its default on a new record.
fn parse_cell(f: &FieldMeta, sub: Option<&str>, text: &str) -> std::result::Result<Value, String> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(Value::Null);
    }
    let label = f.label.as_str();
    let parts = subs(f.r#type);
    if !parts.is_empty() {
        if f.r#type == FieldType::FullName && sub.is_none() {
            let (first, last) = split_name(t);
            return Ok(json!({ "firstName": first, "lastName": last }));
        }
        let key = sub.unwrap_or_else(|| whole_key(f.r#type));
        let part = parts.iter().find(|p| p.key == key).ok_or_else(|| format!("{label} has no part named {key}"))?;
        let value = match (f.r#type, key) {
            (FieldType::Currency, "amountMicros") => {
                let n: f64 = t.replace([',', ' '], "").parse().map_err(|_| format!("{label}: a number"))?;
                Value::from((n * 1_000_000.0).round() as i64)
            }
            (FieldType::Address, "addressLat" | "addressLng") => Value::from(t.parse::<f64>().map_err(|_| format!("{label}: a number"))?),
            (FieldType::Emails, "primaryEmail") => {
                if !t.contains('@') {
                    return Err(format!("{label}: {t} is not an email"));
                }
                Value::String(t.to_lowercase())
            }
            _ if part.json => match serde_json::from_str::<Value>(t) {
                Ok(v @ Value::Array(_)) => v,
                _ => Value::Array(list_cell(t).into_iter().map(Value::String).collect()),
            },
            _ => Value::String(t.to_owned()),
        };
        return Ok(json!({ key: value }));
    }
    Ok(match f.r#type {
        FieldType::RichText => Value::String(rich_doc(t).to_string()),
        FieldType::Boolean => match t.to_ascii_lowercase().as_str() {
            "true" | "yes" | "1" | "x" | "✓" => Value::Bool(true),
            "false" | "no" | "0" => Value::Bool(false),
            _ => return Err(format!("{label}: TRUE or FALSE")),
        },
        FieldType::Number => Value::from(t.replace([',', ' '], "").parse::<f64>().map_err(|_| format!("{label}: a number"))?),
        FieldType::Rating => Value::String(if t.starts_with("RATING_") { t.to_owned() } else { format!("RATING_{t}") }),
        FieldType::Select => Value::String(option_value(f, t).ok_or_else(|| format!("{label}: {t} is not one of its options"))?),
        FieldType::MultiSelect => {
            let mut out = Vec::new();
            for item in list_cell(t) {
                out.push(Value::String(option_value(f, &item).ok_or_else(|| format!("{label}: {item} is not one of its options"))?));
            }
            Value::Array(out)
        }
        FieldType::Array => Value::Array(list_cell(t).into_iter().map(Value::String).collect()),
        FieldType::Json => serde_json::from_str(t).map_err(|_| format!("{label}: not JSON"))?,
        FieldType::DateTime if t.len() == 10 => Value::String(format!("{t}T00:00:00Z")),
        _ => Value::String(t.to_owned()),
    })
}

/// A URL as a host: no scheme, no www, no trailing slash, lower case.
fn host_of(url: &str) -> String {
    let lower = url.trim().to_lowercase();
    let no_scheme = lower.split_once("://").map_or(lower.as_str(), |(_, rest)| rest);
    no_scheme.strip_prefix("www.").unwrap_or(no_scheme).trim_end_matches('/').to_owned()
}

pub type Lookups = HashMap<(String, String, String), Option<String>>;

/// The id of the record whose `by` (its id, or one of its fields) reads `text` — remembered
/// per run, so a company named on a hundred rows is looked up once.
pub async fn find_by(pool: &SqlitePool, obj: &ObjectDef, by: &str, text: &str, cache: &mut Lookups) -> Result<Option<String>> {
    let key = (obj.meta.name_singular.clone(), by.to_owned(), text.trim().to_lowercase());
    if let Some(hit) = cache.get(&key) {
        return Ok(hit.clone());
    }
    let table = &obj.meta.table_name;
    let found: Option<(String,)> = if by == "id" {
        sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT id FROM {table} WHERE id = ? AND deleted_at IS NULL"))).bind(text.trim()).fetch_optional(pool).await?
    } else {
        let Some(f) = obj.field(by) else { return Ok(None) };
        let (expr, value) = match f.r#type {
            FieldType::FullName => (format!("lower(trim(\"{c}_first\" || ' ' || \"{c}_last\"))", c = f.column), text.trim().to_lowercase()),
            FieldType::Emails => (format!("lower(\"{}_primary\")", f.column), text.trim().to_lowercase()),
            // The stored URL as a host, like `host_of` (the scheme literals are joined in SQL:
            // they are prefixes stripped from stored text, not addresses the app reaches).
            FieldType::Links | FieldType::Domain => (format!("rtrim(replace(replace(replace(lower(\"{}_url\"), 'https:' || '//', ''), 'http:' || '//', ''), 'www.', ''), '/')", f.column), host_of(text)),
            FieldType::Phones => (format!("\"{}_number\"", f.column), text.trim().to_owned()),
            _ => (format!("lower(trim(\"{}\"))", f.column), text.trim().to_lowercase()),
        };
        sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT id FROM {table} WHERE {expr} = ? AND deleted_at IS NULL LIMIT 1"))).bind(value).fetch_optional(pool).await?
    };
    let id = found.map(|(id,)| id);
    cache.insert(key, id.clone());
    Ok(id)
}

fn by_label(obj: &ObjectDef, by: &str) -> String {
    if by == "id" {
        "id".into()
    } else {
        obj.field(by).map(|f| f.label.to_lowercase()).unwrap_or_else(|| by.to_owned())
    }
}

/// The text a field's value compares by: a composite's main part, else the value itself.
fn key_text(data: &Map<String, Value>, f: &FieldMeta) -> Option<String> {
    let v = data.get(&f.name)?;
    let text = match v {
        Value::Object(o) if f.r#type == FieldType::FullName => format!("{} {}", o.get("firstName").and_then(Value::as_str).unwrap_or(""), o.get("lastName").and_then(Value::as_str).unwrap_or("")),
        Value::Object(o) => o.get(whole_key(f.r#type)).and_then(Value::as_str).unwrap_or("").to_owned(),
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    };
    let text = if matches!(f.r#type, FieldType::Links | FieldType::Domain) { host_of(&text) } else { text.trim().to_lowercase() };
    (!text.is_empty()).then_some(text)
}

fn is_blank(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::String(s) => s.trim().is_empty(),
        Value::Object(o) => o.values().all(is_blank),
        Value::Array(a) => a.is_empty(),
        _ => false,
    }
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct CellError {
    /// The column in error, or none for the row as a whole.
    pub column: Option<u32>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct RowPreview {
    pub index: u32,
    /// create · update · error
    pub action: String,
    pub errors: Vec<CellError>,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct Preview {
    pub rows: Vec<RowPreview>,
    pub creates: u32,
    pub updates: u32,
    pub errors: u32,
}

struct Prepared {
    data: Value,
    existing: Option<String>,
    errors: Vec<CellError>,
}

/// Every row in the engine's shape, checked: parts merged, relations resolved, the identifier
/// matched to an existing record, the engine's own rules per value, required fields,
/// duplicates in the file and against the records.
async fn prepare(pool: &SqlitePool, def: &ObjectDef, table: &Table, mapping: &[ColumnMap], identifier: Option<&str>) -> Result<Vec<Prepared>> {
    let targets = targets_of(pool, def).await?;
    let mut lookups = Lookups::new();
    let column_of = |name: &str| mapping.iter().position(|m| m.field.as_deref() == Some(name)).map(|c| c as u32);
    let mut out = Vec::with_capacity(table.rows.len());
    for row in &table.rows {
        let mut data = Map::new();
        let mut errors = Vec::new();
        let mut existing = None;
        for (col, m) in mapping.iter().enumerate() {
            let Some(name) = m.field.as_deref() else { continue };
            let text = row.get(col).map(String::as_str).unwrap_or("").trim();
            let column = Some(col as u32);
            if name == "id" {
                if text.is_empty() {
                    continue;
                }
                if !looks_like_uuid(text) {
                    errors.push(CellError { column, message: "Id: not a record id".into() });
                } else if identifier == Some("id") {
                    existing = find_by(pool, def, "id", text, &mut lookups).await?;
                    if existing.is_none() {
                        errors.push(CellError { column, message: format!("No {} with this id", def.meta.label_singular.to_lowercase()) });
                    }
                }
                continue;
            }
            let Some(f) = def.field(name) else {
                errors.push(CellError { column, message: format!("No field named {name}") });
                continue;
            };
            if f.r#type == FieldType::Relation {
                if text.is_empty() {
                    data.insert(f.name.clone(), Value::Null);
                    continue;
                }
                let Some(target) = f.options.relation.as_ref().and_then(|r| targets.get(&r.target)) else { continue };
                let by = m.match_by.as_deref().unwrap_or("id");
                match find_by(pool, target, by, text, &mut lookups).await? {
                    Some(id) => {
                        data.insert(f.name.clone(), Value::String(id));
                    }
                    None => errors.push(CellError { column, message: format!("Can't connect to {}: no record whose {} is {text}", target.meta.label_singular.to_lowercase(), by_label(target, by)) }),
                }
                continue;
            }
            match parse_cell(f, m.sub.as_deref(), text) {
                Err(message) => errors.push(CellError { column, message }),
                Ok(Value::Null) => {
                    data.entry(f.name.clone()).or_insert(Value::Null);
                }
                Ok(Value::Object(parts)) if !subs(f.r#type).is_empty() => {
                    let slot = data.entry(f.name.clone()).or_insert_with(|| Value::Object(Map::new()));
                    if !slot.is_object() {
                        *slot = Value::Object(Map::new());
                    }
                    if let Value::Object(o) = slot {
                        o.extend(parts);
                    }
                }
                Ok(v) => {
                    data.insert(f.name.clone(), v);
                }
            }
        }
        // The identifier names the record to update.
        if let Some(f) = identifier.filter(|i| *i != "id").and_then(|i| def.field(i)) {
            if let Some(text) = key_text(&data, f) {
                existing = find_by(pool, def, &f.name, &text, &mut lookups).await?;
            }
        }
        // The engine's own rules per value (a date's shape, a select's option…).
        for (name, value) in &data {
            if let Some(f) = def.field(name) {
                if let Err(e) = query::check_value(f, value) {
                    errors.push(CellError { column: column_of(name), message: e.to_string() });
                }
            }
        }
        // Required fields on a new record; an emptied required cell on an update.
        for f in importable(def).into_iter().filter(|f| !f.is_nullable && f.r#type != FieldType::Select) {
            let empty = match data.get(&f.name) {
                None => existing.is_none(),
                Some(v) => is_blank(v),
            };
            if empty {
                errors.push(CellError { column: column_of(&f.name), message: format!("{} is required", f.label) });
            }
        }
        // A new record keeps its defaults where the file is blank; an update clears the cell.
        if existing.is_none() {
            data.retain(|_, v| !v.is_null());
        }
        out.push(Prepared { data: Value::Object(data), existing, errors });
    }
    // Duplicates in the file: the identifier and every unique field, by their text.
    let keyed: Vec<&FieldMeta> = importable(def).into_iter().filter(|f| f.is_unique || Some(f.name.as_str()) == identifier).filter(|f| column_of(&f.name).is_some()).collect();
    for f in &keyed {
        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut dups: Vec<usize> = Vec::new();
        for (i, p) in out.iter().enumerate() {
            let Some(text) = p.data.as_object().and_then(|d| key_text(d, f)) else { continue };
            if let Some(first) = seen.insert(text, i) {
                dups.push(first);
                dups.push(i);
            }
        }
        for i in dups {
            let message = format!("{}: the same value is on another row", f.label);
            if !out[i].errors.iter().any(|e| e.message == message) {
                out[i].errors.push(CellError { column: column_of(&f.name), message });
            }
        }
    }
    // A unique field already used by another record (not the row's own).
    for f in keyed.iter().filter(|f| f.is_unique) {
        for p in &mut out {
            let Some(text) = p.data.as_object().and_then(|d| key_text(d, f)) else { continue };
            if let Some(id) = find_by(pool, def, &f.name, &text, &mut lookups).await? {
                if p.existing.as_deref() != Some(id.as_str()) {
                    p.errors.push(CellError { column: column_of(&f.name), message: format!("{}: already used by another {}", f.label, def.meta.label_singular.to_lowercase()) });
                }
            }
        }
    }
    Ok(out)
}

/// The Validate data step: what each row would do.
pub async fn validate(pool: &SqlitePool, def: &ObjectDef, table: &Table, mapping: &[ColumnMap], identifier: Option<&str>) -> Result<Preview> {
    let rows = prepare(pool, def, table, mapping, identifier).await?;
    let (mut creates, mut updates, mut errors) = (0, 0, 0);
    let rows = rows
        .into_iter()
        .enumerate()
        .map(|(i, p)| {
            let action = if !p.errors.is_empty() {
                errors += 1;
                "error"
            } else if p.existing.is_some() {
                updates += 1;
                "update"
            } else {
                creates += 1;
                "create"
            };
            RowPreview { index: i as u32, action: action.into(), errors: p.errors }
        })
        .collect();
    Ok(Preview { rows, creates, updates, errors })
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct RowError {
    /// The row's number in the file (the first data row is 1).
    pub row: u32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct ImportReport {
    pub created: u32,
    pub updated: u32,
    pub skipped: u32,
    pub errors: Vec<RowError>,
}

/// Writes the rows without errors — the others are skipped and reported — and keeps the run.
pub async fn run(pool: &SqlitePool, def: &ObjectDef, file_name: &str, table: &Table, mapping: &[ColumnMap], identifier: Option<&str>, actor: &Actor) -> Result<ImportReport> {
    let rows = prepare(pool, def, table, mapping, identifier).await?;
    let mut report = ImportReport { created: 0, updated: 0, skipped: 0, errors: Vec::new() };
    for (i, p) in rows.into_iter().enumerate() {
        let row = i as u32 + 1;
        if let Some(e) = p.errors.first() {
            report.skipped += 1;
            report.errors.push(RowError { row, message: e.message.clone() });
            continue;
        }
        let written = match &p.existing {
            Some(id) => query::update(pool, def, id, &p.data, actor).await.map(|_| true),
            None => query::create(pool, def, &p.data, actor).await.map(|_| false),
        };
        match written {
            Ok(true) => report.updated += 1,
            Ok(false) => report.created += 1,
            Err(e) => {
                report.skipped += 1;
                report.errors.push(RowError { row, message: e.to_string() });
            }
        }
    }
    sqlx::query("INSERT INTO import_job (id, object, file_name, rows_total, created_count, updated_count, skipped_count, errors_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(uuid::Uuid::now_v7().to_string())
        .bind(&def.meta.name_singular)
        .bind(file_name)
        .bind(table.rows.len() as i64)
        .bind(i64::from(report.created))
        .bind(i64::from(report.updated))
        .bind(i64::from(report.skipped))
        // The first hundred errors are kept; the report the sheet shows has them all.
        .bind(serde_json::to_string(&report.errors.iter().take(100).collect::<Vec<_>>())?)
        .execute(pool)
        .await?;
    Ok(report)
}

// ---- export -------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Type)]
pub struct ExportReport {
    pub row_count: u32,
    pub file_path: String,
}

enum Col<'a> {
    Id,
    Whole(&'a FieldMeta),
    Part(&'a FieldMeta, &'static str),
    RelationLabel(&'a FieldMeta),
    RelationId(&'a FieldMeta),
}

/// The columns a view's visible fields become: Id first; a composite as one column a part
/// ("Phones / Primary Phone Number"); a many-to-one relation as its label and its id.
fn export_columns<'a>(def: &'a ObjectDef, fields: &[String]) -> Vec<(String, Col<'a>)> {
    let mut cols: Vec<(String, Col)> = vec![("Id".into(), Col::Id)];
    for name in fields {
        let Some(f) = def.field(name) else { continue };
        if f.r#type == FieldType::Relation {
            if f.options.relation.as_ref().is_some_and(|r| r.kind == RelationKind::ManyToOne) {
                cols.push((f.label.clone(), Col::RelationLabel(f)));
                cols.push((format!("{} / Id", f.label), Col::RelationId(f)));
            }
            continue;
        }
        let parts = subs(f.r#type);
        if parts.is_empty() {
            cols.push((f.label.clone(), Col::Whole(f)));
            continue;
        }
        for p in parts.iter().filter(|p| !matches!(p.key, "workspaceMemberId" | "context" | "addressLat" | "addressLng")) {
            cols.push((format!("{} / {}", f.label, sub_label(f.r#type, p.key)), Col::Part(f, p.key)));
        }
    }
    cols
}

fn cell_of(col: &Col, record: &Value) -> String {
    let text = |v: Option<&Value>| v.and_then(Value::as_str).unwrap_or("").to_owned();
    match col {
        Col::Id => text(record.get("id")),
        Col::RelationLabel(f) => text(record.get(&f.name).and_then(|v| v.get("label"))),
        Col::RelationId(f) => text(record.get(&f.name).and_then(|v| v.get("id"))),
        Col::Part(f, key) => match (f.r#type, *key, record.get(&f.name).and_then(|v| v.get(*key))) {
            (_, _, None | Some(Value::Null)) => String::new(),
            (FieldType::Currency, "amountMicros", Some(v)) => v.as_f64().map(|m| number_text(m / 1_000_000.0)).unwrap_or_default(),
            (_, _, Some(Value::String(s))) => s.clone(),
            (_, _, Some(Value::Array(a))) if a.is_empty() => String::new(),
            (_, _, Some(v)) => v.to_string(),
        },
        Col::Whole(f) => match (f.r#type, record.get(&f.name)) {
            (_, None | Some(Value::Null)) => String::new(),
            (FieldType::Boolean, Some(v)) => if v.as_bool().unwrap_or(false) { "TRUE" } else { "FALSE" }.into(),
            (FieldType::Rating, Some(Value::Number(n))) => format!("RATING_{n}"),
            (FieldType::RichText, Some(Value::String(s))) => rich::plain(s),
            (FieldType::Number, Some(Value::Number(n))) => number_text(n.as_f64().unwrap_or(0.0)),
            (_, Some(Value::String(s))) => s.clone(),
            (_, Some(Value::Array(a))) if a.is_empty() => String::new(),
            (_, Some(v)) => v.to_string(),
        },
    }
}

/// The view's records as a CSV file: its visible fields in order, with the query the view bar
/// sends (its filters, sorts and search). Twenty's cap of 20 000 rows.
pub async fn export_view(pool: &SqlitePool, def: &ObjectDef, view_name: &str, mut query: Query, fields: &[String], path: &Path) -> Result<ExportReport> {
    let cols = export_columns(def, fields);
    let mut lines: Vec<Vec<String>> = vec![cols.iter().map(|(h, _)| h.clone()).collect()];
    query.limit = Some(PAGE);
    query.offset = Some(0);
    loop {
        let page = query::list(pool, def, &query).await?;
        let n = page.records.len();
        for r in &page.records {
            lines.push(cols.iter().map(|(_, c)| cell_of(c, &r.0)).collect());
        }
        let offset = query.offset.unwrap_or(0) + n as u32;
        if n < PAGE as usize || offset >= page.total.min(MAX_EXPORT) {
            break;
        }
        query.offset = Some(offset);
    }
    let (file, text) = (path.to_path_buf(), csv::write(&lines));
    tauri::async_runtime::spawn_blocking(move || std::fs::write(&file, text).with_context(|| format!("writing {}", file.display()))).await??;
    let row_count = (lines.len() - 1) as u32;
    sqlx::query("INSERT INTO export_job (id, object, view_name, file_path, row_count) VALUES (?, ?, ?, ?, ?)")
        .bind(uuid::Uuid::now_v7().to_string())
        .bind(&def.meta.name_singular)
        .bind(view_name)
        .bind(path.to_string_lossy().as_ref())
        .bind(i64::from(row_count))
        .execute(pool)
        .await?;
    Ok(ExportReport { row_count, file_path: path.to_string_lossy().into_owned() })
}

// ---- jobs ---------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Type)]
pub struct TransferJob {
    pub id: String,
    /// import · export
    pub kind: String,
    /// The object's singular name.
    pub object: String,
    /// The file imported, or the view exported.
    pub label: String,
    pub detail: String,
    pub file_path: String,
    pub created_at: String,
}

#[derive(sqlx::FromRow)]
struct JobRow {
    id: String,
    kind: String,
    object: String,
    label: String,
    a: i64,
    b: i64,
    c: i64,
    d: i64,
    file_path: String,
    created_at: String,
}

/// The last fifty imports and exports, newest first.
pub async fn jobs(pool: &SqlitePool) -> Result<Vec<TransferJob>> {
    let rows: Vec<JobRow> = sqlx::query_as(
        "SELECT * FROM (SELECT id, 'import' AS kind, object, file_name AS label, created_count AS a, updated_count AS b, skipped_count AS c, rows_total AS d, '' AS file_path, created_at FROM import_job \
         UNION ALL SELECT id, 'export', object, view_name, row_count, 0, 0, 0, file_path, created_at FROM export_job) ORDER BY created_at DESC LIMIT 50",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let detail = if r.kind == "import" { format!("{} created · {} updated · {} skipped of {}", r.a, r.b, r.c, r.d) } else { format!("{} records", r.a) };
            TransferJob { id: r.id, kind: r.kind, object: r.object, label: r.label, detail, file_path: r.file_path, created_at: r.created_at }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::hub::WorkspaceHub;

    #[test]
    fn excel_serials_and_numbers_read_as_text() {
        assert_eq!(excel_date(45_658.0), "2025-01-01");
        assert_eq!(excel_date(25_569.0), "1970-01-01");
        assert_eq!(excel_date(45_658.5), "2025-01-01T12:00:00");
        assert_eq!(number_text(1_234.0), "1234");
        assert_eq!(number_text(12.5), "12.5");
        assert_eq!(host_of(concat!("https", "://www.Keppel.com/")), "keppel.com");
        assert!(looks_like_uuid("01a198a3-61da-7178-80a5-fbbafdf2e1f6"));
    }

    /// A Twenty-style People CSV (firstName, lastName, email, jobTitle, companyDomain) maps by
    /// itself, imports with the companies linked, and a second import of the same emails with
    /// new titles updates instead of duplicating; a domain nobody has is an error on its row;
    /// the export of the people view has exactly its columns, the Id first.
    #[test]
    fn a_people_file_imports_updates_and_exports() {
        let root = std::env::temp_dir().join(format!("newdawn-transfer-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let actor = Actor::import(None, "people.csv");
            let company = object(&pool, "company").await.unwrap();
            query::create(&pool, &company, &json!({ "name": "Keppel Land", "domainName": { "primaryLinkUrl": "www.keppel.com" } }), &actor).await.unwrap();
            let person = object(&pool, "person").await.unwrap();
            let file = root.join("people.csv");
            std::fs::write(&file, "firstName,lastName,email,jobTitle,companyDomain\nMarina,Tran,marina@keppel.com,Leasing Manager,keppel.com\nDuc,Le,duc@keppel.com,CEO,www.keppel.com\nNo,Body,nobody@x.com,Clerk,nowhere.example\n").unwrap();
            let opened = open(&pool, &person, &file).await.unwrap();
            let fields: Vec<Option<&str>> = opened.mapping.iter().map(|m| m.field.as_deref()).collect();
            assert_eq!(fields, [Some("name"), Some("name"), Some("emails"), Some("jobTitle"), Some("company")]);
            assert_eq!(opened.mapping[0].sub.as_deref(), Some("firstName"));
            assert_eq!(opened.mapping[2].sub.as_deref(), Some("primaryEmail"));
            assert_eq!(opened.mapping[4].match_by.as_deref(), Some("domainName"));
            assert_eq!(opened.identifier.as_deref(), Some("emails"));
            let preview = validate(&pool, &person, &opened.table, &opened.mapping, opened.identifier.as_deref()).await.unwrap();
            assert_eq!((preview.creates, preview.updates, preview.errors), (2, 0, 1));
            assert_eq!(preview.rows[2].errors[0].column, Some(4));
            assert!(preview.rows[2].errors[0].message.starts_with("Can't connect to company"), "{}", preview.rows[2].errors[0].message);
            let report = run(&pool, &person, "people.csv", &opened.table, &opened.mapping, opened.identifier.as_deref(), &actor).await.unwrap();
            assert_eq!((report.created, report.updated, report.skipped), (2, 0, 1));
            let (linked,): (i64,) = sqlx::query_as("SELECT count(*) FROM person WHERE company_id = (SELECT id FROM company WHERE name = 'Keppel Land')").fetch_one(&pool).await.unwrap();
            assert_eq!(linked, 2, "both people point at the company");
            // The same people with new titles.
            std::fs::write(&file, "firstName,lastName,email,jobTitle\nMarina,Tran,MARINA@keppel.com,Head of Leasing\nDuc,Le,duc@keppel.com,Chairman\n").unwrap();
            let again = open(&pool, &person, &file).await.unwrap();
            let report = run(&pool, &person, "people.csv", &again.table, &again.mapping, again.identifier.as_deref(), &actor).await.unwrap();
            assert_eq!((report.created, report.updated, report.skipped), (0, 2, 0));
            let (people,): (i64,) = sqlx::query_as("SELECT count(*) FROM person WHERE deleted_at IS NULL").fetch_one(&pool).await.unwrap();
            assert_eq!(people, 2, "updated, not duplicated");
            let (title,): (String,) = sqlx::query_as("SELECT title FROM person WHERE emails_primary = 'duc@keppel.com'").fetch_one(&pool).await.unwrap();
            assert_eq!(title, "Chairman");
            // The export: Id, then the view's fields, a composite as its parts, a relation as label and id.
            let out = root.join("people-out.csv");
            let query = Query { filter: None, sort: Vec::new(), limit: None, offset: None, search: None, deleted: false };
            let exported = export_view(&pool, &person, "All People", query, &["name".into(), "emails".into(), "jobTitle".into(), "company".into()], &out).await.unwrap();
            assert_eq!(exported.row_count, 2);
            let rows = csv::parse(&std::fs::read_to_string(&out).unwrap());
            assert_eq!(rows[0], ["Id", "Name / First Name", "Name / Last Name", "Emails / Primary Email", "Emails / Additional Emails", "Job title", "Company", "Company / Id"]);
            assert!(rows[1..].iter().any(|r| r[1] == "Duc" && r[5] == "Chairman" && r[6] == "Keppel Land" && looks_like_uuid(&r[7])));
            let jobs = jobs(&pool).await.unwrap();
            assert_eq!(jobs.len(), 3);
            assert_eq!(jobs[0].kind, "export");
            assert_eq!(jobs[0].detail, "2 records");
            assert_eq!(jobs[1].detail, "0 created · 2 updated · 0 skipped of 2");
            hub.close_all().await;
        });
        std::fs::remove_dir_all(&root).unwrap();
    }
}
