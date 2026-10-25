//! A workspace's brand pack (`brand/tokens.json`, schema v2, D-030): read, resolve to the
//! values the UI and the document export use, edit, swap logos, reset to the starter pack.
//! A workspace without a pack gets the bundled starter with its slug (else `default`) copied in
//! on first read, so everything after that edits the workspace's own copy.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use specta::Type;
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use crate::error::AppError;
use crate::events::BrandChanged;
use crate::kernel::hub::WorkspaceHub;

const FILE: &str = "brand/tokens.json";
const SCHEMA: &str = "newdawn-brand-tokens/v2";

/// The file as written (unknown fields are kept across edits).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Pack {
    #[serde(rename = "$schema")]
    schema: String,
    name: String,
    slug: String,
    logo: Logo,
    slogan: Slogan,
    colors: Colors,
    document_fonts: Fonts,
    radius: Radius,
    button: Button,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Logo {
    primary: Option<String>,
    on_dark: Option<String>,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
#[serde(default)]
pub struct Slogan {
    pub en: String,
    pub vi: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct Colors {
    primary: String,
    primary_ramp: Option<Map<String, Value>>,
    secondary: Option<String>,
    accent: Option<String>,
    text: String,
    canvas: String,
    status: Status,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            primary: "#3B5BDB".into(),
            primary_ramp: None,
            secondary: None,
            accent: None,
            text: "#0B1220".into(),
            canvas: "#FFFFFF".into(),
            status: Status::default(),
            extra: Map::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(default)]
pub struct Status {
    pub positive: String,
    pub warning: String,
    pub negative: String,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            positive: "#1F7A4D".into(),
            warning: "#9A6A00".into(),
            negative: "#B3261E".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(default)]
pub struct Fonts {
    pub heading: String,
    pub body: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
}

impl Default for Fonts {
    fn default() -> Self {
        Self {
            heading: "Be Vietnam Pro".into(),
            body: "Be Vietnam Pro".into(),
            files: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(default)]
pub struct Radius {
    pub preset: String,
    pub card: u32,
    pub chip: u32,
    pub control: u32,
    pub modal: u32,
}

impl Default for Radius {
    fn default() -> Self {
        Self::preset("soft")
    }
}

impl Radius {
    /// The three corner styles the Brand screen offers.
    pub fn preset(name: &str) -> Self {
        let (card, chip, control, modal) = match name {
            "sharp" => (4, 3, 4, 6),
            "round" => (16, 999, 999, 20),
            _ => (10, 6, 8, 14),
        };
        Self {
            preset: if matches!(name, "sharp" | "round") {
                name
            } else {
                "soft"
            }
            .into(),
            card,
            chip,
            control,
            modal,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(default)]
pub struct Button {
    /// square | rounded | pill
    pub shape: String,
    /// filled | soft | outline
    pub primary: String,
    /// soft | outline | ghost
    pub secondary: String,
}

impl Default for Button {
    fn default() -> Self {
        Self {
            shape: "rounded".into(),
            primary: "filled".into(),
            secondary: "soft".into(),
        }
    }
}

/// What the UI gets: every value resolved, logos as absolute paths (asset protocol).
#[derive(Debug, Clone, Serialize, Type)]
pub struct BrandView {
    pub name: String,
    /// The starter pack this workspace's brand came from (its slug, or "default").
    pub starter: String,
    pub primary: String,
    /// 50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950.
    pub ramp: Vec<String>,
    pub secondary: Option<String>,
    pub accent: Option<String>,
    pub text: String,
    pub canvas: String,
    pub status: Status,
    pub radius: Radius,
    pub button: Button,
    pub slogan: Slogan,
    pub document_fonts: Fonts,
    pub logo_primary: Option<String>,
    pub logo_on_dark: Option<String>,
    /// Changes whenever a logo file is replaced (its name carries a timestamp): cache-busting.
    pub logo_version: String,
}

/// Fields the Brand screen can change (each `None` = leave as is; `Some("")` clears an
/// optional colour or text).
#[derive(Debug, Clone, Default, Deserialize, Type)]
#[serde(default)]
pub struct BrandPatch {
    pub primary: Option<String>,
    pub secondary: Option<String>,
    pub accent: Option<String>,
    pub text: Option<String>,
    pub canvas: Option<String>,
    pub positive: Option<String>,
    pub warning: Option<String>,
    pub negative: Option<String>,
    /// sharp | soft | round
    pub radius_preset: Option<String>,
    pub button_shape: Option<String>,
    pub button_primary: Option<String>,
    pub button_secondary: Option<String>,
    pub slogan_en: Option<String>,
    pub slogan_vi: Option<String>,
    pub font_heading: Option<String>,
    pub font_body: Option<String>,
}

fn hex(value: &str) -> Result<String> {
    let v = value.trim();
    let ok = v.len() == 7 && v.starts_with('#') && v[1..].chars().all(|c| c.is_ascii_hexdigit());
    if !ok {
        bail!(AppError::invalid(format!(
            "“{value}” isn't a colour — use the form #0132AD"
        )));
    }
    Ok(v.to_ascii_uppercase())
}

fn rgb(hex: &str) -> (f64, f64, f64) {
    let n = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0) as f64;
    (n(1), n(3), n(5))
}

fn mix(hex: &str, with: f64, amount: f64) -> String {
    let (r, g, b) = rgb(hex);
    let m = |c: f64| (c + (with - c) * amount).round().clamp(0.0, 255.0) as u8;
    format!("#{:02X}{:02X}{:02X}", m(r), m(g), m(b))
}

/// 50…950 from one colour: lighter tints towards white, deeper shades towards black.
pub fn ramp(primary: &str) -> Vec<String> {
    [0.94, 0.86, 0.70, 0.50, 0.25]
        .iter()
        .map(|a| mix(primary, 255.0, *a))
        .chain(std::iter::once(primary.to_ascii_uppercase()))
        .chain(
            [0.18, 0.36, 0.52, 0.66, 0.80]
                .iter()
                .map(|a| mix(primary, 0.0, *a)),
        )
        .collect()
}

const STEPS: [&str; 11] = [
    "50", "100", "200", "300", "400", "500", "600", "700", "800", "900", "950",
];

async fn folder(app: &AppHandle, slug: &str) -> Result<PathBuf> {
    Ok(PathBuf::from(
        app.state::<WorkspaceHub>()
            .workspace(slug)
            .await?
            .folder_path,
    ))
}

fn starter_dir(app: &AppHandle, slug: &str) -> Option<(String, PathBuf)> {
    for name in [slug, "default"] {
        let dir = app
            .path()
            .resolve(format!("brands/{name}"), BaseDirectory::Resource)
            .ok()?;
        if dir.join("tokens.json").is_file() {
            return Some((name.to_owned(), dir));
        }
    }
    None
}

/// Copies the starter pack (tokens + logos; templates are the meeting types' business) into
/// the workspace when it has no pack of its own.
fn ensure(app: &AppHandle, folder: &Path, slug: &str) -> Result<()> {
    let file = folder.join(FILE);
    if file.is_file() {
        return Ok(());
    }
    let (_, starter) = starter_dir(app, slug).context("no starter brand pack is bundled")?;
    let brand = folder.join("brand");
    std::fs::create_dir_all(brand.join("logo"))?;
    let tmp = file.with_extension("json.tmp");
    std::fs::copy(starter.join("tokens.json"), &tmp)?;
    std::fs::rename(&tmp, &file)?;
    if let Ok(logos) = std::fs::read_dir(starter.join("logo")) {
        for entry in logos.flatten() {
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                let _ = std::fs::copy(entry.path(), brand.join("logo").join(entry.file_name()));
            }
        }
    }
    Ok(())
}

fn read(folder: &Path) -> Result<Pack> {
    let text = std::fs::read_to_string(folder.join(FILE))
        .with_context(|| format!("reading {}", folder.join(FILE).display()))?;
    serde_json::from_str(&text).context("brand/tokens.json is not valid")
}

fn write(folder: &Path, pack: &Pack) -> Result<()> {
    let file = folder.join(FILE);
    let tmp = file.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(pack)?)?;
    std::fs::rename(&tmp, &file).with_context(|| format!("saving {}", file.display()))?;
    Ok(())
}

fn view(folder: &Path, pack: Pack, starter: String) -> BrandView {
    let primary = pack.colors.primary.to_ascii_uppercase();
    let ramp = pack
        .colors
        .primary_ramp
        .as_ref()
        .and_then(|m| {
            STEPS
                .iter()
                .map(|s| {
                    m.get(*s)
                        .and_then(Value::as_str)
                        .map(str::to_ascii_uppercase)
                })
                .collect::<Option<Vec<_>>>()
        })
        .unwrap_or_else(|| ramp(&primary));
    let logo = |rel: &Option<String>| {
        rel.as_deref()
            .filter(|r| !r.is_empty())
            .map(|r| folder.join("brand").join(r))
            .filter(|p| p.is_file())
            .map(|p| p.display().to_string())
    };
    BrandView {
        name: pack.name,
        starter,
        logo_version: format!(
            "{}|{}",
            pack.logo.primary.as_deref().unwrap_or_default(),
            pack.logo.on_dark.as_deref().unwrap_or_default()
        ),
        logo_primary: logo(&pack.logo.primary),
        logo_on_dark: logo(&pack.logo.on_dark),
        primary,
        ramp,
        secondary: pack.colors.secondary.filter(|s| !s.is_empty()),
        accent: pack.colors.accent.filter(|s| !s.is_empty()),
        text: pack.colors.text,
        canvas: pack.colors.canvas,
        status: pack.colors.status,
        radius: pack.radius,
        button: pack.button,
        slogan: pack.slogan,
        document_fonts: pack.document_fonts,
    }
}

pub async fn get(app: &AppHandle, slug: &str) -> Result<BrandView> {
    let folder = folder(app, slug).await?;
    ensure(app, &folder, slug)?;
    let pack = read(&folder)?;
    let starter = starter_dir(app, slug)
        .map(|(n, _)| n)
        .unwrap_or_else(|| "default".into());
    Ok(view(&folder, pack, starter))
}

fn opt_colour(target: &mut Option<String>, value: &Option<String>) -> Result<()> {
    if let Some(v) = value {
        *target = if v.trim().is_empty() {
            None
        } else {
            Some(hex(v)?)
        };
    }
    Ok(())
}

pub async fn update(app: &AppHandle, slug: &str, patch: BrandPatch) -> Result<BrandView> {
    let folder = folder(app, slug).await?;
    ensure(app, &folder, slug)?;
    let mut pack = read(&folder)?;
    if let Some(p) = &patch.primary {
        pack.colors.primary = hex(p)?;
        pack.colors.primary_ramp = None; // derived again from the new colour
    }
    opt_colour(&mut pack.colors.secondary, &patch.secondary)?;
    opt_colour(&mut pack.colors.accent, &patch.accent)?;
    if let Some(v) = &patch.text {
        pack.colors.text = hex(v)?;
    }
    if let Some(v) = &patch.canvas {
        pack.colors.canvas = hex(v)?;
    }
    if let Some(v) = &patch.positive {
        pack.colors.status.positive = hex(v)?;
    }
    if let Some(v) = &patch.warning {
        pack.colors.status.warning = hex(v)?;
    }
    if let Some(v) = &patch.negative {
        pack.colors.status.negative = hex(v)?;
    }
    if let Some(p) = &patch.radius_preset {
        pack.radius = Radius::preset(p);
    }
    let pick = |target: &mut String, value: &Option<String>, allowed: &[&str]| -> Result<()> {
        if let Some(v) = value {
            if !allowed.contains(&v.as_str()) {
                bail!(AppError::invalid(format!(
                    "“{v}” isn't one of {}",
                    allowed.join(", ")
                )));
            }
            *target = v.clone();
        }
        Ok(())
    };
    pick(
        &mut pack.button.shape,
        &patch.button_shape,
        &["square", "rounded", "pill"],
    )?;
    pick(
        &mut pack.button.primary,
        &patch.button_primary,
        &["filled", "soft", "outline"],
    )?;
    pick(
        &mut pack.button.secondary,
        &patch.button_secondary,
        &["soft", "outline", "ghost"],
    )?;
    let text = |target: &mut String, value: &Option<String>| {
        if let Some(v) = value {
            *target = v.trim().chars().take(200).collect();
        }
    };
    text(&mut pack.slogan.en, &patch.slogan_en);
    text(&mut pack.slogan.vi, &patch.slogan_vi);
    text(&mut pack.document_fonts.heading, &patch.font_heading);
    text(&mut pack.document_fonts.body, &patch.font_body);
    if pack.schema.is_empty() {
        pack.schema = SCHEMA.into();
    }
    write(&folder, &pack)?;
    let _ = BrandChanged {
        slug: slug.to_owned(),
    }
    .emit(app);
    get(app, slug).await
}

/// Copies a picked SVG/PNG into `brand/logo/` as the primary or on-dark logo; `None` clears it.
pub async fn set_logo(
    app: &AppHandle,
    slug: &str,
    source: Option<&str>,
    on_dark: bool,
) -> Result<BrandView> {
    let folder = folder(app, slug).await?;
    ensure(app, &folder, slug)?;
    let mut pack = read(&folder)?;
    let rel = match source {
        None => None,
        Some(source) => {
            let path = Path::new(source);
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(str::to_ascii_lowercase)
                .unwrap_or_default();
            if !path.is_file() || !matches!(ext.as_str(), "svg" | "png") {
                bail!(AppError::invalid("Choose an SVG or PNG file"));
            }
            // A new name each time, so the webview never shows a cached older logo.
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let name = format!(
                "{}-{stamp}.{ext}",
                if on_dark { "logo-dark" } else { "logo" }
            );
            let target = folder.join("brand/logo").join(&name);
            std::fs::create_dir_all(target.parent().context("logo folder")?)?;
            let tmp = target.with_extension(format!("{ext}.tmp"));
            std::fs::copy(path, &tmp).with_context(|| format!("copying {}", path.display()))?;
            std::fs::rename(&tmp, &target)?;
            Some(format!("logo/{name}"))
        }
    };
    let previous = std::mem::replace(
        if on_dark {
            &mut pack.logo.on_dark
        } else {
            &mut pack.logo.primary
        },
        rel,
    );
    write(&folder, &pack)?;
    // The file it replaced is only removed when it was one we made (inside brand/logo).
    if let Some(old) = previous.filter(|p| p.starts_with("logo/") && !p.contains("..")) {
        let still_used = pack.logo.primary.as_deref() == Some(&old)
            || pack.logo.on_dark.as_deref() == Some(&old);
        if !still_used {
            let _ = std::fs::remove_file(folder.join("brand").join(old));
        }
    }
    let _ = BrandChanged {
        slug: slug.to_owned(),
    }
    .emit(app);
    get(app, slug).await
}

/// Back to the starter pack: the workspace's tokens and logos go to the Trash.
pub async fn reset(app: &AppHandle, slug: &str) -> Result<BrandView> {
    let folder = folder(app, slug).await?;
    for rel in [FILE, "brand/logo"] {
        let path = folder.join(rel);
        if path.exists() {
            crate::modules::meetings::review::to_trash(path).await?;
        }
    }
    let _ = BrandChanged {
        slug: slug.to_owned(),
    }
    .emit(app);
    get(app, slug).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ramps_and_checks_colours() {
        let r = ramp("#0132AD");
        assert_eq!(r.len(), 11);
        assert_eq!(r[5], "#0132AD");
        assert_eq!(r[0], "#F0F3FA");
        assert!(r[10].starts_with('#'));
        assert!(hex(" #abcdef ").is_ok_and(|h| h == "#ABCDEF"));
        assert!(hex("#abc").is_err());
        assert!(hex("blue").is_err());
        assert_eq!(Radius::preset("round").chip, 999);
        assert_eq!(Radius::preset("nope").preset, "soft");
    }

    #[test]
    fn the_vifc_pack_parses_and_keeps_unknown_fields() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let text = std::fs::read_to_string(root.join("brands/vifc/tokens.json")).unwrap();
        let pack: Pack = serde_json::from_str(&text).unwrap();
        assert_eq!(pack.colors.primary, "#0132AD");
        assert!(pack.extra.contains_key("meeting_types_starter"));
        let again: Value = serde_json::to_value(&pack).unwrap();
        assert!(again.get("meeting_types_starter").is_some(), "round-trips");
        assert_eq!(again["$schema"], SCHEMA);
    }
}
