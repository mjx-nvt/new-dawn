//! Plane's exports and CSV importer (D-083): work items out as CSV, XLSX or JSON with Plane's
//! columns (what a view's filters keep), and Plane's CSV back in — states, labels, assignees,
//! dates, estimates, cycles, modules, links, comments, parents by identifier — with a preview and
//! a summary. Both are kept in the transfer history (0025's `export_job` / `import_job`).

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::SqlitePool;

use super::filters::PmRule;
use super::items::{self, ItemInput, ItemPatch, PmItem, Priority};
use crate::error::AppError;
use crate::kernel::records::transfer::{Table, read_table, rich_doc};

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow!(AppError::invalid(message))
}

/// Plane's export columns, in its order; the importer reads them back by name (any case).
pub const COLUMNS: [&str; 20] = [
    "Identifier",
    "Name",
    "Description",
    "State Name",
    "State Group",
    "Priority",
    "Assignees",
    "Labels",
    "Start Date",
    "Due Date",
    "Estimate",
    "Cycle Name",
    "Modules",
    "Parent",
    "Subscribers",
    "Links",
    "Comments",
    "Created At",
    "Updated At",
    "Completed At",
];
/// Between the comments in one cell (a comment may hold commas).
const COMMENTS_SEP: &str = " ¦ ";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Csv,
    Xlsx,
    Json,
}

/// The people's names by id ("me" is You).
async fn names(pool: &SqlitePool) -> Result<HashMap<String, String>> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT id, trim(name_first || ' ' || name_last) FROM person WHERE deleted_at IS NULL").fetch_all(pool).await?;
    let mut map: HashMap<String, String> = rows.into_iter().collect();
    map.insert("me".into(), "You".into());
    Ok(map)
}

/// The rows of an export: the headers, then a line an item.
async fn table(pool: &SqlitePool, list: &[PmItem]) -> Result<Vec<Vec<String>>> {
    let people = names(pool).await?;
    let labels: HashMap<String, String> = sqlx::query_as::<_, (String, String)>("SELECT id, name FROM pm_label").fetch_all(pool).await?.into_iter().collect();
    // Links and top comments, each read once.
    let mut links: HashMap<String, Vec<String>> = HashMap::new();
    for (item, url) in sqlx::query_as::<_, (String, String)>("SELECT item_id, url FROM pm_link ORDER BY created_at, id").fetch_all(pool).await? {
        links.entry(item).or_default().push(url);
    }
    let mut comments: HashMap<String, Vec<String>> = HashMap::new();
    for (item, text) in sqlx::query_as::<_, (String, String)>("SELECT item_id, body_text FROM pm_comment WHERE parent_id IS NULL ORDER BY created_at, id").fetch_all(pool).await? {
        comments.entry(item).or_default().push(text.replace('\n', " "));
    }
    let who = |ids: &[String]| ids.iter().filter_map(|w| people.get(w).cloned()).collect::<Vec<_>>().join(", ");
    let mut out = vec![COLUMNS.iter().map(|c| (*c).to_owned()).collect::<Vec<String>>()];
    for i in list {
        out.push(vec![
            i.key.clone(),
            i.name.clone(),
            i.description_text.clone(),
            i.state_name.clone(),
            serde_json::to_value(i.state_group)?.as_str().unwrap_or("").to_owned(),
            serde_json::to_value(i.priority)?.as_str().unwrap_or("").to_owned(),
            who(&i.assignees),
            i.labels.iter().filter_map(|l| labels.get(l).cloned()).collect::<Vec<_>>().join(", "),
            i.start_date.clone().unwrap_or_default(),
            i.target_date.clone().unwrap_or_default(),
            i.estimate_value.clone().unwrap_or_default(),
            i.cycle_name.clone().unwrap_or_default(),
            i.modules.iter().map(|m| m.name.clone()).collect::<Vec<_>>().join(", "),
            i.parent_key.clone().unwrap_or_default(),
            who(&i.subscribers),
            links.get(&i.id).map(|l| l.join(", ")).unwrap_or_default(),
            comments.get(&i.id).map(|c| c.join(COMMENTS_SEP)).unwrap_or_default(),
            i.created_at.clone(),
            i.updated_at.clone(),
            i.completed_at.clone().unwrap_or_default(),
        ]);
    }
    Ok(out)
}

/// A sheet's column letters for a 0-based index (A … Z, AA …).
fn column(mut n: usize) -> String {
    let mut s = String::new();
    loop {
        s.insert(0, (b'A' + (n % 26) as u8) as char);
        if n < 26 {
            return s;
        }
        n = n / 26 - 1;
    }
}

/// A minimal workbook: one sheet, every cell an inline string (never run as a formula), at most
/// the 32,767 characters a cell holds.
fn xlsx(rows: &[Vec<String>]) -> Result<Vec<u8>> {
    let esc = crate::kernel::export::docx::escape;
    let mut sheet = String::from(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>"#);
    for (r, row) in rows.iter().enumerate() {
        sheet.push_str(&format!(r#"<row r="{}">"#, r + 1));
        for (c, cell) in row.iter().enumerate() {
            let text: String = cell.chars().take(32_767).collect();
            sheet.push_str(&format!(r#"<c r="{}{}" t="inlineStr"><is><t xml:space="preserve">{}</t></is></c>"#, column(c), r + 1, esc(&text)));
        }
        sheet.push_str("</row>");
    }
    sheet.push_str("</sheetData></worksheet>");
    let parts: [(&str, String); 5] = [
        ("[Content_Types].xml", r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>"#.into()),
        ("_rels/.rels", r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#.into()),
        ("xl/workbook.xml", r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Work items" sheetId="1" r:id="rId1"/></sheets></workbook>"#.into()),
        ("xl/_rels/workbook.xml.rels", r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#.into()),
        ("xl/worksheets/sheet1.xml", sheet),
    ];
    let mut out = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, body) in parts {
        out.start_file(name, options)?;
        out.write_all(body.as_bytes())?;
    }
    Ok(out.finish()?.into_inner())
}

/// JSON objects with Plane's columns in Plane's order.
fn json(rows: &[Vec<String>]) -> Result<String> {
    let mut out = String::from("[\n");
    for (n, r) in rows.iter().skip(1).enumerate() {
        let fields: Vec<String> = COLUMNS.iter().zip(r).map(|(k, v)| Ok(format!("    {}: {}", serde_json::to_string(k)?, serde_json::to_string(v)?))).collect::<Result<_>>()?;
        out.push_str(&format!("  {{\n{}\n  }}{}\n", fields.join(",\n"), if n + 2 < rows.len() { "," } else { "" }));
    }
    out.push(']');
    Ok(out)
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct PmExportReport {
    pub row_count: u32,
    pub file_path: String,
}

/// The file a format goes to: an absolute path with its extension (the save dialog's).
fn check_path(path: &Path, format: ExportFormat) -> Result<&'static str> {
    let ext = match format {
        ExportFormat::Csv => "csv",
        ExportFormat::Xlsx => "xlsx",
        ExportFormat::Json => "json",
    };
    if !path.is_absolute() || !path.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext)) {
        return Err(invalid(format!("Work items are exported to a .{ext} file")));
    }
    Ok(ext)
}

/// Work items as the rules keep them (a project's, or every open project's, on this Mac's day and
/// the owner's week) — see [`export`].
pub async fn export_query(hub: &crate::kernel::hub::WorkspaceHub, slug: &str, project: Option<&str>, rules: &[PmRule], format: ExportFormat, path: &Path, label: &str) -> Result<PmExportReport> {
    check_path(path, format)?;
    let list = items::query_now(hub, slug, project, rules).await?;
    export(&hub.pool(slug)?, &list, format, path, label).await
}

/// Work items into `path` — and into the history. A CSV opens in Excel as UTF-8 (its mark first);
/// a cell starting like a formula is text (`csv::write`).
pub async fn export(pool: &SqlitePool, list: &[PmItem], format: ExportFormat, path: &Path, label: &str) -> Result<PmExportReport> {
    let ext = check_path(path, format)?;
    let rows = table(pool, list).await?;
    let bytes = match format {
        ExportFormat::Csv => format!("\u{feff}{}", crate::kernel::csv::write(&rows)).into_bytes(),
        ExportFormat::Xlsx => xlsx(&rows)?,
        ExportFormat::Json => json(&rows)?.into_bytes(),
    };
    tokio::fs::write(path, bytes).await.with_context(|| format!("couldn't write {}", path.display()))?;
    let row_count = list.len() as u32;
    sqlx::query("INSERT INTO export_job (id, object, view_name, file_path, row_count) VALUES (?, 'pm_item', ?, ?, ?)")
        .bind(uuid::Uuid::now_v7().to_string())
        .bind(format!("{label} · {}", ext.to_uppercase()))
        .bind(path.to_string_lossy().as_ref())
        .bind(i64::from(row_count))
        .execute(pool)
        .await?;
    Ok(PmExportReport { row_count, file_path: path.to_string_lossy().into_owned() })
}

// ── The importer ──

#[derive(Debug, Clone, Serialize, Type)]
pub struct ImportProblem {
    /// The file's row (the headers are row 1; 0: the file as a whole).
    pub row: u32,
    pub message: String,
}

/// What a file holds before it is imported: Plane's columns found, the others left aside, the
/// rows, and what won't come in as it says (or will come in twice).
#[derive(Debug, Clone, Serialize, Type)]
pub struct PmImportPreview {
    pub columns: Vec<String>,
    pub ignored: Vec<String>,
    pub rows: u32,
    pub problems: Vec<ImportProblem>,
}

/// What an import did (Plane's JSON summary).
#[derive(Debug, Clone, Serialize, Type)]
pub struct PmImportSummary {
    pub created: u32,
    pub skipped: u32,
    pub problems: Vec<ImportProblem>,
}

/// Problems kept in a preview or a summary (and their count past it).
const PROBLEMS_KEPT: usize = 500;

fn cap(mut problems: Vec<ImportProblem>) -> Vec<ImportProblem> {
    if problems.len() > PROBLEMS_KEPT {
        let more = problems.len() - PROBLEMS_KEPT;
        problems.truncate(PROBLEMS_KEPT);
        problems.push(ImportProblem { row: 0, message: format!("…and {more} more") });
    }
    problems
}

/// A row's cell under a Plane column (headers in any case).
struct Cells<'a> {
    at: &'a HashMap<String, usize>,
    row: &'a [String],
}

impl Cells<'_> {
    fn get(&self, column: &str) -> &str {
        self.at.get(&column.to_lowercase()).and_then(|i| self.row.get(*i)).map(|s| s.trim()).unwrap_or("")
    }
    fn list(&self, column: &str, sep: &str) -> Vec<String> {
        self.get(column).split(sep).map(|s| s.trim().to_owned()).filter(|s| !s.is_empty()).collect()
    }
    fn day(&self, column: &str) -> Option<String> {
        items::clean_day(Some(&self.get(column).chars().take(10).collect::<String>())).ok().flatten()
    }
}

fn columns(t: &Table) -> (HashMap<String, usize>, Vec<String>, Vec<String>) {
    let mut at = HashMap::new();
    let (mut found, mut ignored) = (Vec::new(), Vec::new());
    for (i, h) in t.headers.iter().enumerate() {
        match COLUMNS.iter().find(|c| c.eq_ignore_ascii_case(h.trim())) {
            Some(c) => {
                at.insert(c.to_lowercase(), i);
                found.push((*c).to_owned());
            }
            None if !h.trim().is_empty() => ignored.push(h.trim().to_owned()),
            None => {}
        }
    }
    (at, found, ignored)
}

/// "You", or a person of the people memory by name (any case).
fn person(name: &str, people: &HashMap<String, String>) -> Option<String> {
    people.iter().find(|(_, n)| n.eq_ignore_ascii_case(name)).map(|(id, _)| id.clone())
}

fn priority(text: &str) -> Option<Priority> {
    serde_json::from_value(serde_json::Value::String(text.trim().to_lowercase())).ok()
}

/// What an import reads of a project: its people, cycles, modules, estimate points, states and
/// labels — once.
struct Known {
    people: HashMap<String, String>,
    cycles: Vec<super::cycles::PmCycle>,
    modules: Vec<super::modules::PmModule>,
    points: Vec<(String, String)>,
    states: Vec<super::settings::PmState>,
    labels: Vec<super::settings::PmLabel>,
}

async fn context(pool: &SqlitePool, project_id: &str) -> Result<Known> {
    Ok(Known {
        people: names(pool).await?,
        cycles: super::cycles::list(pool, project_id, super::cycles::today()).await?,
        modules: super::modules::list(pool, project_id).await?,
        points: sqlx::query_as("SELECT ep.id, ep.value FROM pm_estimate_point ep JOIN pm_project m ON m.estimate_id = ep.estimate_id WHERE m.project_id = ?").bind(project_id).fetch_all(pool).await?,
        states: super::settings::states(pool, project_id).await?,
        labels: super::settings::labels(pool, project_id).await?,
    })
}

/// What a row brings that won't come in as it says: each a sentence. The import says the same.
fn check_row(c: &Cells, x: &Known, identifiers: &HashMap<String, u32>) -> Vec<String> {
    let mut notes = Vec::new();
    let name = c.get("Name");
    if name.chars().count() > items::NAME_MAX {
        notes.push(format!("Its name is longer than {} characters — skipped", items::NAME_MAX));
    }
    for w in c.list("Assignees", ",").into_iter().chain(c.list("Subscribers", ",")) {
        if person(&w, &x.people).is_none() {
            notes.push(format!("“{w}” isn't in the people memory — left out"));
        }
    }
    if !c.get("Priority").is_empty() && priority(c.get("Priority")).is_none() {
        notes.push(format!("“{}” isn't a priority — None", c.get("Priority")));
    }
    let state = c.get("State Name");
    if !state.is_empty() && !x.states.iter().any(|s| super::settings::same(&s.name, state)) {
        notes.push(format!("The state “{state}” is made"));
    }
    for l in c.list("Labels", ",") {
        if !x.labels.iter().any(|k| super::settings::same(&k.name, &l)) {
            notes.push(format!("The label “{l}” is made"));
        }
    }
    let cycle = c.get("Cycle Name");
    if !cycle.is_empty() && !x.cycles.iter().any(|k| super::settings::same(&k.name, cycle) && k.status != super::cycles::CycleStatus::Completed) {
        notes.push(format!("No cycle “{cycle}” takes work here — left out"));
    }
    for m in c.list("Modules", ",") {
        if !x.modules.iter().any(|k| super::settings::same(&k.name, &m) && k.archived_at.is_none()) {
            notes.push(format!("No module “{m}” here — left out"));
        }
    }
    let estimate = c.get("Estimate");
    if !estimate.is_empty() && !x.points.iter().any(|(_, v)| v == estimate) {
        notes.push(format!("“{estimate}” isn't one of the project's estimates — left out"));
    }
    for col in ["Start Date", "Due Date"] {
        if !c.get(col).is_empty() && c.day(col).is_none() {
            notes.push(format!("“{}” isn't a day — the {} is left out", c.get(col), col.to_lowercase()));
        }
    }
    if let (Some(s), Some(t)) = (c.day("Start Date"), c.day("Due Date")) {
        if t < s {
            notes.push("Its due date comes before its start — the due date is left out".into());
        }
    }
    let parent = c.get("Parent").to_uppercase();
    if !parent.is_empty() && !identifiers.contains_key(&parent) {
        notes.push(format!("Its parent {parent} isn't in the file — left without one"));
    }
    notes
}

/// The file read off the async workers (a workbook's parse can take a while).
async fn read(path: &Path) -> Result<Table> {
    let path = path.to_path_buf();
    tauri::async_runtime::spawn_blocking(move || read_table(&path)).await?
}

/// The project, open (an archived one takes no work items).
async fn open_project(pool: &SqlitePool, project_id: &str) -> Result<()> {
    if super::get(pool, project_id).await?.archived_at.is_some() {
        return Err(invalid("The project is archived — restore it to import into it"));
    }
    Ok(())
}

/// The file's identifiers, by row.
fn identifiers(t: &Table, at: &HashMap<String, usize>) -> HashMap<String, u32> {
    t.rows
        .iter()
        .enumerate()
        .filter_map(|(n, row)| {
            let key = Cells { at, row }.get("Identifier").to_uppercase();
            (!key.is_empty()).then_some((key, n as u32 + 2))
        })
        .collect()
}

pub async fn preview(pool: &SqlitePool, project_id: &str, path: &Path) -> Result<PmImportPreview> {
    open_project(pool, project_id).await?;
    let t = read(path).await?;
    let (at, found, ignored) = columns(&t);
    if !at.contains_key("name") {
        return Err(invalid("A Plane CSV has a Name column — this file has none"));
    }
    let x = context(pool, project_id).await?;
    let keys = identifiers(&t, &at);
    let mut problems = Vec::new();
    // The same file twice makes everything twice: say so.
    let existing: Vec<(String,)> = sqlx::query_as("SELECT name FROM pm_item WHERE project_id = ?").bind(project_id).fetch_all(pool).await?;
    let again = t.rows.iter().filter(|row| existing.iter().any(|(n,)| n == Cells { at: &at, row }.get("Name"))).count();
    if again > 0 {
        problems.push(ImportProblem { row: 0, message: format!("{again} row{} named like work items the project has already — importing makes them again", if again == 1 { " is" } else { "s are" }) });
    }
    for (n, row) in t.rows.iter().enumerate() {
        let c = Cells { at: &at, row };
        let line = n as u32 + 2;
        if c.get("Name").is_empty() {
            problems.push(ImportProblem { row: line, message: "No name — the row is skipped".into() });
            continue;
        }
        problems.extend(check_row(&c, &x, &keys).into_iter().map(|message| ImportProblem { row: line, message }));
    }
    Ok(PmImportPreview { columns: found, ignored, rows: t.rows.len() as u32, problems: cap(problems) })
}

/// The project's state by name (any case) — or one made in the row's state group (Backlog when
/// it names none), so an export of another project comes back with its states.
async fn state_for(pool: &SqlitePool, x: &mut Known, project_id: &str, name: &str, group: &str) -> Result<Option<String>> {
    if name.is_empty() {
        return Ok(None);
    }
    if let Some(s) = x.states.iter().find(|s| super::settings::same(&s.name, name)) {
        return Ok((s.group != super::settings::StateGroup::Triage).then(|| s.id.clone()));
    }
    let group: super::settings::StateGroup = serde_json::from_value(serde_json::Value::String(group.to_lowercase())).unwrap_or(super::settings::StateGroup::Backlog);
    if group == super::settings::StateGroup::Triage {
        return Ok(None);
    }
    x.states = super::settings::create_state(pool, project_id, super::settings::StateInput { name: name.to_owned(), description: String::new(), color: "gray".into(), group }).await?;
    Ok(x.states.iter().find(|s| super::settings::same(&s.name, name)).map(|s| s.id.clone()))
}

/// The project's labels by name (any case), the missing ones made.
async fn labels_for(pool: &SqlitePool, x: &mut Known, project_id: &str, names: &[String]) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for n in names {
        if !x.labels.iter().any(|l| super::settings::same(&l.name, n)) {
            x.labels = super::settings::create_label(pool, project_id, super::settings::LabelInput { name: n.clone(), description: String::new(), color: "gray".into(), parent_id: None }).await?;
        }
        if let Some(l) = x.labels.iter().find(|l| super::settings::same(&l.name, n)) {
            if !out.contains(&l.id) {
                out.push(l.id.clone());
            }
        }
    }
    Ok(out)
}

/// Plane's CSV into a project: a row a work item (a row without a name, or with one too long,
/// skipped), then the parents by Identifier within the file. What doesn't come in as written is
/// in the summary, by row; an item made stays made (a link or a comment that failed is said).
/// Made quietly: Plane's webhooks don't hear of an import's rows, as Plane's bulk import (D-084).
pub async fn import(pool: &SqlitePool, project_id: &str, path: &Path, actor: &str) -> Result<PmImportSummary> {
    super::events::quietly(import_rows(pool, project_id, path, actor)).await
}

async fn import_rows(pool: &SqlitePool, project_id: &str, path: &Path, actor: &str) -> Result<PmImportSummary> {
    open_project(pool, project_id).await?;
    let file_name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let t = read(path).await?;
    let (at, _, _) = columns(&t);
    if !at.contains_key("name") {
        return Err(invalid("A Plane CSV has a Name column — this file has none"));
    }
    let mut x = context(pool, project_id).await?;
    let keys = identifiers(&t, &at);
    let (mut created, mut skipped, mut problems) = (0u32, 0u32, Vec::new());
    // The file's identifiers → the items made, for the parents.
    let mut made: HashMap<String, String> = HashMap::new();
    let mut parents: Vec<(u32, String, String)> = Vec::new();
    for (n, row) in t.rows.iter().enumerate() {
        let c = Cells { at: &at, row };
        let line = n as u32 + 2;
        let name = c.get("Name");
        if name.is_empty() || name.chars().count() > items::NAME_MAX {
            skipped += 1;
            problems.push(ImportProblem { row: line, message: if name.is_empty() { "No name — skipped".into() } else { format!("Its name is longer than {} characters — skipped", items::NAME_MAX) } });
            continue;
        }
        // What the preview said of the row (people, cycle, modules, estimate, dates …), said again.
        let notes: Vec<String> = check_row(&c, &x, &keys).into_iter().filter(|m| !m.ends_with("is made") && !m.contains("isn't in the file")).collect();
        let outcome: Result<PmItem> = async {
            let state_id = state_for(pool, &mut x, project_id, c.get("State Name"), c.get("State Group")).await?;
            let labels = labels_for(pool, &mut x, project_id, &c.list("Labels", ",")).await?;
            let assignees: Vec<String> = c.list("Assignees", ",").iter().filter_map(|w| person(w, &x.people)).collect();
            let cycle_id = x.cycles.iter().find(|k| super::settings::same(&k.name, c.get("Cycle Name")) && k.status != super::cycles::CycleStatus::Completed).map(|k| k.id.clone());
            let module_ids: Vec<String> = c.list("Modules", ",").iter().filter_map(|m| x.modules.iter().find(|k| super::settings::same(&k.name, m) && k.archived_at.is_none()).map(|k| k.id.clone())).collect();
            let estimate = x.points.iter().find(|(_, v)| v == c.get("Estimate")).map(|(id, _)| id.clone());
            let description = (!c.get("Description").is_empty()).then(|| rich_doc(c.get("Description")).to_string());
            let (start, due) = (c.day("Start Date"), c.day("Due Date"));
            items::create(
                pool,
                ItemInput {
                    project_id: project_id.to_owned(),
                    name: name.to_owned(),
                    description,
                    state_id,
                    priority: priority(c.get("Priority")),
                    assignees,
                    labels,
                    target_date: due.filter(|t| start.as_ref().is_none_or(|s| s <= t)),
                    start_date: start,
                    estimate_point_id: estimate,
                    parent_id: None,
                    cycle_id,
                    modules: module_ids,
                },
                actor,
            )
            .await
        }
        .await;
        let item = match outcome {
            Ok(item) => item,
            Err(e) => {
                skipped += 1;
                problems.push(ImportProblem { row: line, message: e.to_string() });
                continue;
            }
        };
        created += 1;
        problems.extend(notes.into_iter().map(|message| ImportProblem { row: line, message }));
        // The item is made: what fails now is said, not undone.
        for w in c.list("Subscribers", ",").iter().filter_map(|w| person(w, &x.people)) {
            if let Err(e) = items::subscribe(pool, &item.id, &w, true).await {
                problems.push(ImportProblem { row: line, message: format!("A subscriber wasn't kept: {e}") });
            }
        }
        for url in c.list("Links", ",") {
            if let Err(e) = super::relations::add_link(pool, &item.id, &url, "", actor).await {
                problems.push(ImportProblem { row: line, message: format!("A link wasn't kept: {e}") });
            }
        }
        for text in c.list("Comments", COMMENTS_SEP.trim()) {
            if let Err(e) = super::comments::add(pool, &item.id, &rich_doc(&text).to_string(), None, actor).await {
                problems.push(ImportProblem { row: line, message: format!("A comment wasn't kept: {e}") });
            }
        }
        let key = c.get("Identifier");
        if !key.is_empty() {
            made.insert(key.to_uppercase(), item.id.clone());
        }
        let parent = c.get("Parent");
        if !parent.is_empty() {
            parents.push((line, item.id, parent.to_uppercase()));
        }
    }
    // Parents, now every row that could be is an item.
    for (line, item, parent) in parents {
        match made.get(&parent) {
            Some(p) => {
                if let Err(e) = items::update(pool, &item, ItemPatch { parent_id: Some(p.clone()), ..ItemPatch::default() }, actor).await {
                    problems.push(ImportProblem { row: line, message: format!("Its parent {parent} wasn't kept: {e}") });
                }
            }
            None if keys.contains_key(&parent) => problems.push(ImportProblem { row: line, message: format!("Its parent {parent} wasn't imported — left without one") }),
            None => problems.push(ImportProblem { row: line, message: format!("Its parent {parent} isn't in the file — left without one") }),
        }
    }
    let problems = cap(problems);
    sqlx::query("INSERT INTO import_job (id, object, file_name, rows_total, created_count, skipped_count, errors_json) VALUES (?, 'pm_item', ?, ?, ?, ?, ?)")
        .bind(uuid::Uuid::now_v7().to_string())
        .bind(&file_name)
        .bind(t.rows.len() as i64)
        .bind(i64::from(created))
        .bind(i64::from(skipped))
        .bind(serde_json::to_string(&problems.iter().map(|p| serde_json::json!({ "row": p.row, "message": p.message })).collect::<Vec<_>>())?)
        .execute(pool)
        .await?;
    Ok(PmImportSummary { created, skipped, problems })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::hub::WorkspaceHub;
    use crate::kernel::formats::WeekStart;
    use crate::modules::pm::{self, Features, NewPmProject, Network, settings};

    #[test]
    fn a_plane_csv_export_imports_back_with_states_labels_parents_and_comments() {
        let root = std::env::temp_dir().join(format!("newdawn-pm-transfer-{}", uuid::Uuid::now_v7()));
        tauri::async_runtime::block_on(async {
            let hub = WorkspaceHub::open(root.clone()).await.unwrap();
            hub.create_workspace("VIFC", "test").await.unwrap();
            let pool = hub.pool("test").unwrap();
            let new = |name: &str, id: &str| NewPmProject { name: name.into(), identifier: id.into(), description: String::new(), emoji: String::new(), cover: String::new(), network: Network::Private, lead: None, features: Features::default() };
            let a = pm::create(&pool, new("VIFC Fintech Hub", "VIFC")).await.unwrap();
            let b = pm::create(&pool, new("Sandbox", "SB")).await.unwrap();
            // In A: a custom state, a label, a parent with a sub-item, a comment, a link.
            settings::create_state(&pool, &a.id, settings::StateInput { name: "In Review".into(), description: String::new(), color: "blue".into(), group: settings::StateGroup::Started }).await.unwrap();
            let review = settings::states(&pool, &a.id).await.unwrap().into_iter().find(|s| s.name == "In Review").unwrap().id;
            let label = settings::create_label(&pool, &a.id, settings::LabelInput { name: "Legal".into(), description: String::new(), color: "red".into(), parent_id: None }).await.unwrap()[0].id.clone();
            let parent = items::create(&pool, ItemInput { project_id: a.id.clone(), name: "Draft the rules".into(), state_id: Some(review), labels: vec![label], ..ItemInput::default() }, "me").await.unwrap();
            let child = items::create(&pool, ItemInput { project_id: a.id.clone(), name: "Section 2, licences".into(), parent_id: Some(parent.id.clone()), ..ItemInput::default() }, "me").await.unwrap();
            // A name that a spreadsheet would run as a formula: text out, itself back in.
            items::create(&pool, ItemInput { project_id: a.id.clone(), name: "=1+1".into(), ..ItemInput::default() }, "me").await.unwrap();
            super::super::comments::add(&pool, &child.id, &rich_doc("Ask Minh, by Friday").to_string(), None, "me").await.unwrap();
            super::super::relations::add_link(&pool, &parent.id, "vifc.gov.vn/sandbox", "", "me").await.unwrap();

            // Out as CSV, then into B.
            let dir = root.join("out");
            std::fs::create_dir_all(&dir).unwrap();
            let file = dir.join("vifc.csv");
            let list = items::query(&pool, Some(&a.id), &[], super::super::cycles::today(), WeekStart::Monday).await.unwrap();
            let report = export(&pool, &list, ExportFormat::Csv, &file, "VIFC").await.unwrap();
            assert_eq!(report.row_count, 3);
            assert!(std::fs::read_to_string(&file).unwrap().contains("'=1+1"), "guarded in the file");
            assert!(export(&pool, &list, ExportFormat::Csv, &dir.join("x.txt"), "VIFC").await.is_err(), "the format's extension");
            let p = preview(&pool, &b.id, &file).await.unwrap();
            assert_eq!((p.rows, p.columns.len()), (3, COLUMNS.len()));
            assert!(p.problems.iter().any(|x| x.message.contains("“In Review” is made")), "{:?}", p.problems);
            let summary = import(&pool, &b.id, &file, "me").await.unwrap();
            assert_eq!((summary.created, summary.skipped), (3, 0), "{:?}", summary.problems);
            assert!(items::list(&pool, &b.id, false).await.unwrap().iter().any(|i| i.name == "=1+1"));
            // Again: the preview says the rows are there already.
            assert!(preview(&pool, &b.id, &file).await.unwrap().problems.iter().any(|x| x.row == 0));
            let back = items::list(&pool, &b.id, false).await.unwrap();
            let rules = back.iter().find(|i| i.name == "Draft the rules").unwrap();
            let sub = back.iter().find(|i| i.name == "Section 2, licences").unwrap();
            assert_eq!((rules.state_name.as_str(), rules.labels.len(), sub.parent_key.as_deref()), ("In Review", 1, Some(rules.key.as_str())));
            assert_eq!(super::super::comments::list(&pool, &sub.id).await.unwrap()[0].body_text, "Ask Minh, by Friday");
            assert_eq!(super::super::relations::links(&pool, &rules.id).await.unwrap().len(), 1);

            // XLSX reads back as the same table; JSON is an array of objects.
            let x = dir.join("vifc.xlsx");
            export(&pool, &list, ExportFormat::Xlsx, &x, "VIFC").await.unwrap();
            assert_eq!(read_table(&x).unwrap().rows.len(), 3);
            let j = dir.join("vifc.json");
            export(&pool, &list, ExportFormat::Json, &j, "VIFC").await.unwrap();
            let parsed: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string(&j).unwrap()).unwrap();
            assert_eq!(parsed[0]["State Name"], "In Review");
            hub.close_all().await;
        });
        let _ = std::fs::remove_dir_all(root);
    }
}
