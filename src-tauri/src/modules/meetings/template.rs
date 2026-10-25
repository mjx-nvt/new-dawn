//! Meeting types and their minutes templates (D-028). Every meeting type has its own Word
//! template: `<workspace>/brand/templates/meeting-minutes/<type id>.docx`. Minutes use the
//! meeting's type template, else the default type's, else the workspace's older single
//! `template.docx` (D-027), else the bundled starter pack with the workspace's slug. A workspace
//! with no types gets a "Weekly meeting" type on first use, its template that older file, the
//! starter pack, or else one made from the workspace's brand (`kernel/export/rebrand.rs`).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::Value;
use specta::Type;
use sqlx::SqlitePool;
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager};

use crate::error::AppError;
use crate::kernel::ai::whisper::Language;
use crate::kernel::brand::{logo, tokens};
use crate::kernel::export::rebrand::{self, DocBrand, LogoPng};
use crate::kernel::export::{docx, model};
use crate::kernel::hub::WorkspaceHub;
use crate::kernel::profile;

const DIR: &str = "templates/meeting-minutes";
/// The single per-workspace template of D-027 (and the starter pack's file name).
pub const TEMPLATE: &str = "templates/meeting-minutes/template.docx";
const FIRST_TYPE: &str = "Weekly meeting";
/// The neutral template a brand is applied to (`scripts/make_minutes_base.py`).
const BASE: &[u8] = include_bytes!("../../../templates/minutes-base.docx");

#[derive(Debug, Clone, Serialize, Type)]
pub struct TemplateCheck {
    /// Every `{{field}}` the file uses.
    pub fields: Vec<String>,
    /// Fields New Dawn doesn't know (typos): they come out empty.
    pub unknown: Vec<String>,
    /// Top-level fields the file doesn't use (fine, just not shown).
    pub unused: Vec<String>,
    /// Why the file can't be used (not a .docx, unclosed `{{#block}}`…); `None` = usable.
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct TemplateInfo {
    /// The type's own file (None: it has none yet and the default type's is used).
    pub path: Option<String>,
    /// Last change of the file (ISO-8601 UTC).
    pub modified_at: Option<String>,
    pub check: Option<TemplateCheck>,
}

/// A picture of the default type's template filled with a made-up meeting.
#[derive(Debug, Clone, Serialize, Type)]
pub struct TemplatePreview {
    /// The first page as a PNG (Quick Look).
    pub path: String,
    /// Changes with every drawing (the PNG's modified time, ms): cache-busting.
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct MeetingTypeView {
    pub id: String,
    pub name: String,
    pub language: Language,
    pub is_default: bool,
    pub template: TemplateInfo,
}

#[derive(sqlx::FromRow)]
struct TypeRow {
    id: String,
    name: String,
    language: Language,
    is_default: bool,
}

async fn folder_of(hub: &WorkspaceHub, slug: &str) -> Result<PathBuf> {
    Ok(PathBuf::from(hub.workspace(slug).await?.folder_path))
}

/// A type's template file. Ids come from the database (UUID v7); anything that isn't one plain
/// file name is refused so a path can't escape the folder.
fn type_file(folder: &Path, type_id: &str) -> Result<PathBuf> {
    let plain = !type_id.is_empty()
        && type_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-');
    if !plain {
        bail!(AppError::invalid("Bad meeting type id"));
    }
    Ok(folder
        .join("brand")
        .join(DIR)
        .join(format!("{type_id}.docx")))
}

fn legacy_file(folder: &Path) -> PathBuf {
    folder.join("brand").join(TEMPLATE)
}

fn starter_path(app: &AppHandle, slug: &str) -> Option<PathBuf> {
    app.path()
        .resolve(format!("brands/{slug}/{TEMPLATE}"), BaseDirectory::Resource)
        .ok()
        .filter(|p| p.is_file())
}

async fn type_row(pool: &SqlitePool, id: &str) -> Result<TypeRow> {
    let row: Option<TypeRow> =
        sqlx::query_as("SELECT id, name, language, is_default FROM meeting_type WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;
    Ok(row.ok_or_else(|| AppError::not_found("No such meeting type"))?)
}

/// The template the minutes of a meeting of type `type_id` are filled from.
pub async fn path_for(app: &AppHandle, slug: &str, type_id: Option<&str>) -> Result<PathBuf> {
    let hub = app.state::<WorkspaceHub>();
    let folder = folder_of(&hub, slug).await?;
    let pool = hub.pool(slug)?;
    let default: Option<(String,)> =
        sqlx::query_as("SELECT id FROM meeting_type WHERE is_default = 1")
            .fetch_optional(&pool)
            .await?;
    for id in type_id
        .into_iter()
        .chain(default.as_ref().map(|(id,)| id.as_str()))
    {
        let file = type_file(&folder, id)?;
        if file.is_file() {
            return Ok(file);
        }
    }
    let legacy = legacy_file(&folder);
    if legacy.is_file() {
        return Ok(legacy);
    }
    starter_path(app, slug).ok_or_else(|| {
        AppError::not_found(
            "This workspace has no minutes template yet — make one in Workspace → Brand (Update from brand)",
        )
        .into()
    })
}

/// One at a time, so two windows listing types can't both create the first one.
static SEEDING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// A workspace without meeting types gets "Weekly meeting" (default), made from its own
/// template.docx (moved), the starter pack (copied) or, without either, its brand.
pub async fn ensure_types(app: &AppHandle, slug: &str) -> Result<()> {
    let _one = SEEDING.lock().await;
    let hub = app.state::<WorkspaceHub>();
    let pool = hub.pool(slug)?;
    let (count,): (i64,) = sqlx::query_as("SELECT count(*) FROM meeting_type")
        .fetch_one(&pool)
        .await?;
    if count > 0 {
        return Ok(());
    }
    let folder = folder_of(&hub, slug).await?;
    let legacy = legacy_file(&folder);
    let from_legacy = legacy.is_file();
    let id = super::new_id();
    let file = type_file(&folder, &id)?;
    let tmp = if from_legacy {
        copy_beside(&legacy, &file)?
    } else if let Some(starter) = starter_path(app, slug) {
        copy_beside(&starter, &file)?
    } else {
        from_brand(app, slug, &file).await?
    };
    add_first_type(&pool, &id, &tmp, &file).await?;
    if from_legacy {
        // The old single template now lives on as this type's: keep a copy in the Trash.
        super::review::to_trash(legacy).await.ok();
    }
    Ok(())
}

/// Puts `tmp` in place as `file` and adds the default type it belongs to.
async fn add_first_type(pool: &SqlitePool, id: &str, tmp: &Path, file: &Path) -> Result<()> {
    std::fs::rename(tmp, file).with_context(|| format!("saving {}", file.display()))?;
    let inserted = sqlx::query(
        "INSERT INTO meeting_type (id, name, language, is_default) VALUES (?, ?, 'auto', 1)",
    )
    .bind(id)
    .bind(FIRST_TYPE)
    .execute(pool)
    .await;
    if let Err(err) = inserted {
        let _ = std::fs::remove_file(file);
        return Err(err.into());
    }
    Ok(())
}

/// Makes the minutes template from the workspace's brand (its colours, fonts, logo and name on
/// the neutral base) as a temp file beside `target`; the caller puts it in place.
async fn from_brand(app: &AppHandle, slug: &str, target: &Path) -> Result<PathBuf> {
    let brand = tokens::get(app, slug).await?;
    let name = app.state::<WorkspaceHub>().workspace(slug).await?.name;
    let tmp = target.with_extension("docx.tmp");
    let out = tmp.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<()> {
        let logo = brand.logo_primary.as_deref().and_then(|path| {
            match logo::header_png(Path::new(path)) {
                Ok((bytes, width, height)) => Some(LogoPng {
                    bytes,
                    width,
                    height,
                }),
                Err(err) => {
                    tracing::warn!("the minutes template goes without the logo {path}: {err:#}");
                    None
                }
            }
        });
        let dir = out.parent().context("template folder")?;
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        rebrand::brand_template(
            BASE,
            &DocBrand {
                workspace: &name,
                palette: &tokens::document_palette(&brand.primary),
                heading_font: &brand.document_fonts.heading,
                body_font: &brand.document_fonts.body,
                logo,
            },
            &out,
        )
    })
    .await??;
    Ok(tmp)
}

/// The minutes template made again from the brand: it becomes the default type's (the old file
/// goes to the Trash), or the first type's when there are none.
pub async fn generate(app: &AppHandle, slug: &str) -> Result<Vec<MeetingTypeView>> {
    {
        let _one = SEEDING.lock().await;
        let hub = app.state::<WorkspaceHub>();
        let pool = hub.pool(slug)?;
        let folder = folder_of(&hub, slug).await?;
        let default: Option<(String,)> =
            sqlx::query_as("SELECT id FROM meeting_type ORDER BY is_default DESC LIMIT 1")
                .fetch_optional(&pool)
                .await?;
        match default {
            Some((id,)) => {
                let file = type_file(&folder, &id)?;
                put_in_place(from_brand(app, slug, &file).await?, &file).await?;
            }
            None => {
                let id = super::new_id();
                let file = type_file(&folder, &id)?;
                let tmp = from_brand(app, slug, &file).await?;
                add_first_type(&pool, &id, &tmp, &file).await?;
            }
        }
    }
    list(app, slug).await
}

async fn template_info(folder: &Path, type_id: &str) -> Result<TemplateInfo> {
    let file = type_file(folder, type_id)?;
    let own = file.is_file();
    let modified_at = own
        .then(|| std::fs::metadata(&file).and_then(|m| m.modified()).ok())
        .flatten()
        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339());
    let check = if own {
        let f = file.clone();
        Some(tauri::async_runtime::spawn_blocking(move || check(&f)).await?)
    } else {
        None
    };
    Ok(TemplateInfo {
        path: own.then(|| file.display().to_string()),
        modified_at,
        check,
    })
}

/// The types for pickers (default first), making the first one if there are none.
pub async fn summaries(app: &AppHandle, slug: &str) -> Result<Vec<super::MeetingTypeSummary>> {
    ensure_types(app, slug).await?;
    super::meeting_types(&app.state::<WorkspaceHub>().pool(slug)?).await
}

/// The workspace's meeting types (default first) with their templates.
pub async fn list(app: &AppHandle, slug: &str) -> Result<Vec<MeetingTypeView>> {
    ensure_types(app, slug).await?;
    let hub = app.state::<WorkspaceHub>();
    let folder = folder_of(&hub, slug).await?;
    let rows: Vec<TypeRow> = sqlx::query_as(
        "SELECT id, name, language, is_default FROM meeting_type ORDER BY is_default DESC, name COLLATE NOCASE",
    )
    .fetch_all(&hub.pool(slug)?)
    .await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let template = template_info(&folder, &r.id).await?;
        out.push(MeetingTypeView {
            id: r.id,
            name: r.name,
            language: r.language,
            is_default: r.is_default,
            template,
        });
    }
    Ok(out)
}

fn clean_name(name: &str) -> Result<&str> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 100 {
        bail!(AppError::invalid(
            "A meeting type needs a name of 1–100 characters"
        ));
    }
    Ok(name)
}

/// A new type. Its template: the picked .docx (checked), else a copy of the default type's
/// template (or the starter). The first type becomes the default.
pub async fn create(
    app: &AppHandle,
    slug: &str,
    name: &str,
    language: Language,
    template: Option<&str>,
) -> Result<Vec<MeetingTypeView>> {
    let name = clean_name(name)?;
    let source = match template {
        Some(picked) => Some(picked_docx(picked)?),
        None => path_for(app, slug, None).await.ok(),
    };
    let hub = app.state::<WorkspaceHub>();
    let pool = hub.pool(slug)?;
    let folder = folder_of(&hub, slug).await?;
    let id = super::new_id();
    let file = type_file(&folder, &id)?;
    if let Some(source) = source {
        install(&source, &file).await?;
    }
    let (count,): (i64,) = sqlx::query_as("SELECT count(*) FROM meeting_type")
        .fetch_one(&pool)
        .await?;
    let inserted = sqlx::query(
        "INSERT INTO meeting_type (id, name, language, is_default) VALUES (?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(name)
    .bind(language)
    .bind(count == 0)
    .execute(&pool)
    .await;
    if let Err(err) = inserted {
        let _ = std::fs::remove_file(&file); // no orphan template
        return Err(err.into());
    }
    list(app, slug).await
}

pub async fn update(
    app: &AppHandle,
    slug: &str,
    id: &str,
    name: &str,
    language: Language,
) -> Result<Vec<MeetingTypeView>> {
    let name = clean_name(name)?;
    let pool = app.state::<WorkspaceHub>().pool(slug)?;
    type_row(&pool, id).await?;
    sqlx::query(
        "UPDATE meeting_type SET name = ?, language = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?",
    )
    .bind(name)
    .bind(language)
    .bind(id)
    .execute(&pool)
    .await?;
    list(app, slug).await
}

/// New meetings start with this type; meetings without a type use its template.
pub async fn set_default(app: &AppHandle, slug: &str, id: &str) -> Result<Vec<MeetingTypeView>> {
    let pool = app.state::<WorkspaceHub>().pool(slug)?;
    type_row(&pool, id).await?;
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE meeting_type SET is_default = 0 WHERE is_default = 1")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE meeting_type SET is_default = 1, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    list(app, slug).await
}

/// Deletes a type: its meetings keep everything but lose the type; its template goes to the
/// Trash. Deleting the default makes the first remaining type the default. The last type
/// stays (a workspace without types would get "Weekly meeting" back on the next list).
pub async fn delete(app: &AppHandle, slug: &str, id: &str) -> Result<Vec<MeetingTypeView>> {
    let hub = app.state::<WorkspaceHub>();
    let pool = hub.pool(slug)?;
    let row = type_row(&pool, id).await?;
    let (count,): (i64,) = sqlx::query_as("SELECT count(*) FROM meeting_type")
        .fetch_one(&pool)
        .await?;
    if count <= 1 {
        bail!(AppError::invalid(
            "A workspace keeps at least one meeting type — rename this one instead"
        ));
    }
    let file = type_file(&folder_of(&hub, slug).await?, id)?;
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM meeting_type WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    if row.is_default {
        sqlx::query(
            "UPDATE meeting_type SET is_default = 1 WHERE id = \
             (SELECT id FROM meeting_type ORDER BY name COLLATE NOCASE LIMIT 1)",
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    if file.is_file() {
        // The type is gone; its template stays recoverable from the Trash.
        super::review::to_trash(file).await?;
    }
    list(app, slug).await
}

/// Reads a .docx and compares its fields with what New Dawn fills in; also fills it once with
/// sample data, so a broken block structure shows up here rather than at export.
pub fn check(file: &Path) -> TemplateCheck {
    let sample = model::sample();
    let mut known = BTreeSet::new();
    collect_keys(&sample, &mut known);
    let fields = match docx::placeholders(file) {
        Ok(fields) => fields,
        Err(err) => return failed(format!("{err:#}")),
    };
    let tmp = std::env::temp_dir().join(format!("newdawn-check-{}.docx", uuid::Uuid::now_v7()));
    let filled = docx::render(file, &sample, &tmp);
    let _ = std::fs::remove_file(&tmp);
    let root = |f: &String| f.split('.').next().unwrap_or_default().to_owned();
    let used: BTreeSet<String> = fields.iter().map(root).collect();
    TemplateCheck {
        unknown: fields
            .iter()
            .filter(|f| !known.contains(&root(f)))
            .cloned()
            .collect(),
        unused: sample
            .as_object()
            .map(|m| m.keys().filter(|k| !used.contains(*k)).cloned().collect())
            .unwrap_or_default(),
        fields,
        error: filled.err().map(|e| format!("{e:#}")),
    }
}

fn failed(error: String) -> TemplateCheck {
    TemplateCheck {
        fields: Vec::new(),
        unknown: Vec::new(),
        unused: Vec::new(),
        error: Some(error),
    }
}

fn collect_keys(value: &Value, out: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, v) in map {
                out.insert(key.clone());
                collect_keys(v, out);
            }
        }
        Value::Array(items) => items.iter().for_each(|v| collect_keys(v, out)),
        _ => {}
    }
}

/// A .docx the user picked (anything else is refused before it's opened).
fn picked_docx(path: &str) -> Result<PathBuf> {
    let path = PathBuf::from(path);
    if !path.is_file()
        || path
            .extension()
            .is_none_or(|e| !e.eq_ignore_ascii_case("docx"))
    {
        bail!(AppError::invalid("Choose a Word document (.docx)"));
    }
    Ok(path)
}

/// Checks a .docx the user picked, without using it.
pub async fn check_path(path: &str) -> Result<TemplateCheck> {
    let path = picked_docx(path)?;
    Ok(tauri::async_runtime::spawn_blocking(move || check(&path)).await?)
}

/// Copies `source` beside `own` as a temp file (the caller renames it into place).
fn copy_beside(source: &Path, own: &Path) -> Result<PathBuf> {
    let dir = own.parent().context("template folder")?;
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let tmp = own.with_extension("docx.tmp");
    if let Err(err) = std::fs::copy(source, &tmp) {
        let _ = std::fs::remove_file(&tmp);
        return Err(err).with_context(|| format!("copying {}", source.display()));
    }
    Ok(tmp)
}

/// Copies `source` into place as `target` once the copy passes the check; a file it replaces
/// goes to the Trash.
async fn install(source: &Path, target: &Path) -> Result<()> {
    put_in_place(copy_beside(source, target)?, target).await
}

/// Renames `tmp` (beside `target`) over `target` once it passes the check; a file it replaces
/// goes to the Trash.
async fn put_in_place(tmp: PathBuf, target: &Path) -> Result<()> {
    // Check the copy itself: what passes is exactly what gets installed.
    let checked = {
        let t = tmp.clone();
        tauri::async_runtime::spawn_blocking(move || check(&t)).await?
    };
    if let Some(error) = checked.error {
        let _ = std::fs::remove_file(&tmp);
        bail!(AppError::invalid(format!(
            "This template can't be used: {error}"
        )));
    }
    if target.is_file() {
        if let Err(err) = super::review::to_trash(target.to_owned()).await {
            let _ = std::fs::remove_file(&tmp);
            return Err(err);
        }
    }
    std::fs::rename(&tmp, target).with_context(|| {
        format!(
            "saving {} (the previous template is in the Trash)",
            target.display()
        )
    })?;
    Ok(())
}

/// Makes a picked .docx the type's template (checked first; the old one goes to the Trash).
pub async fn replace(
    app: &AppHandle,
    slug: &str,
    type_id: &str,
    source: &str,
) -> Result<Vec<MeetingTypeView>> {
    let source = picked_docx(source)?;
    let hub = app.state::<WorkspaceHub>();
    type_row(&hub.pool(slug)?, type_id).await?;
    install(&source, &type_file(&folder_of(&hub, slug).await?, type_id)?).await?;
    list(app, slug).await
}

/// The type's template starts again: the starter pack when the app ships one for this
/// workspace, else one made from its brand (the current file goes to the Trash).
pub async fn reset(app: &AppHandle, slug: &str, type_id: &str) -> Result<Vec<MeetingTypeView>> {
    let hub = app.state::<WorkspaceHub>();
    type_row(&hub.pool(slug)?, type_id).await?;
    let file = type_file(&folder_of(&hub, slug).await?, type_id)?;
    let tmp = match starter_path(app, slug) {
        Some(starter) => copy_beside(&starter, &file)?,
        None => from_brand(app, slug, &file).await?,
    };
    put_in_place(tmp, &file).await?;
    list(app, slug).await
}

/// Previews share one file: one drawing at a time.
static DRAWING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// The default type's template filled with a made-up meeting around this workspace's own name,
/// slogans and minute-taker, drawn by Quick Look into
/// `brand/templates/meeting-minutes/preview/sample.docx.png`.
pub async fn preview(app: &AppHandle, slug: &str) -> Result<TemplatePreview> {
    ensure_types(app, slug).await?;
    let template = path_for(app, slug, None).await?;
    let hub = app.state::<WorkspaceHub>();
    let workspace = hub.workspace(slug).await?;
    let owner = profile::owner(&hub, slug).await?.unwrap_or_default();
    let slogan = tokens::get(app, slug).await?.slogan;
    let data = model::sample_for(&model::Sender {
        workspace: &workspace.name,
        slogan_en: &slogan.en,
        slogan_vi: &slogan.vi,
        name: &owner.name,
        title: &owner.title,
    });
    let dir = Path::new(&workspace.folder_path)
        .join("brand")
        .join(DIR)
        .join("preview");
    draw(template, data, &dir).await.map_err(|err| {
        tracing::warn!("could not draw the minutes preview: {err:#}");
        AppError::new(
            "preview_failed",
            "Couldn't draw a preview — open the template in Word instead.",
        )
        .into()
    })
}

async fn draw(template: PathBuf, data: Value, dir: &Path) -> Result<TemplatePreview> {
    let _one = DRAWING.lock().await;
    std::fs::create_dir_all(dir)?;
    let sample = dir.join("sample.docx");
    let png = dir.join("sample.docx.png");
    let _ = std::fs::remove_file(&png); // an older drawing never stands in for this one
    let out = sample.clone();
    tauri::async_runtime::spawn_blocking(move || docx::render(&template, &data, &out)).await??;
    let status = tokio::time::timeout(
        Duration::from_secs(20),
        tokio::process::Command::new("/usr/bin/qlmanage")
            .args(["-t", "-s", "1400", "-o"])
            .arg(dir)
            .arg(&sample)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .status(),
    )
    .await
    .context("Quick Look took more than 20 s")??;
    if !png.is_file() {
        bail!("Quick Look made no picture ({status})");
    }
    let drawn = std::fs::metadata(&png)?.modified()?;
    Ok(TemplatePreview {
        path: png.display().to_string(),
        version: drawn.duration_since(UNIX_EPOCH)?.as_millis().to_string(),
    })
}

/// Opens the type's template in Word (made from the template it uses now if it has none of its
/// own yet), or shows it in Finder.
pub async fn open(app: &AppHandle, slug: &str, type_id: &str, reveal: bool) -> Result<()> {
    let hub = app.state::<WorkspaceHub>();
    type_row(&hub.pool(slug)?, type_id).await?;
    let file = type_file(&folder_of(&hub, slug).await?, type_id)?;
    if !file.is_file() {
        let source = path_for(app, slug, Some(type_id)).await?;
        let tmp = copy_beside(&source, &file)?;
        std::fs::rename(&tmp, &file).with_context(|| format!("saving {}", file.display()))?;
    }
    if reveal {
        tauri_plugin_opener::reveal_item_in_dir(file)?;
    } else {
        tauri_plugin_opener::open_path(file, None::<&str>)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_files_stay_in_the_template_folder() {
        let folder = Path::new("/w");
        assert_eq!(
            type_file(folder, "0190-ab").unwrap(),
            Path::new("/w/brand/templates/meeting-minutes/0190-ab.docx")
        );
        for bad in ["", "../x", "a/b", "x.docx", "a b"] {
            assert!(type_file(folder, bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn checks_the_vifc_starter_and_rejects_junk() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let vifc = check(&root.join("brands/vifc").join(TEMPLATE));
        assert_eq!(vifc.error, None);
        assert!(vifc.unknown.is_empty(), "unknown: {:?}", vifc.unknown);
        assert!(vifc.fields.contains(&"decisions".to_owned()));
        assert!(vifc.fields.contains(&"minute_taker".to_owned()));
        let base = check(&root.join("src-tauri/templates/minutes-base.docx"));
        assert_eq!(base.error, None);
        assert!(base.unknown.is_empty(), "unknown: {:?}", base.unknown);
        assert!(base.fields.contains(&"slogan_vi".to_owned()));

        let junk = std::env::temp_dir().join(format!("newdawn-junk-{}.docx", uuid::Uuid::now_v7()));
        std::fs::write(&junk, "not a zip").unwrap();
        assert!(check(&junk).error.is_some());
        std::fs::remove_file(junk).unwrap();

        let names: Vec<String> = model::fields().into_iter().map(|f| f.name).collect();
        assert!(names.contains(&"actions".to_owned()) && names.contains(&"title".to_owned()));
    }
}
