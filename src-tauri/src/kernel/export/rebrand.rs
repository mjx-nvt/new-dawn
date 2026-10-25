//! A workspace's own minutes template, made by rules (no model) from the neutral base
//! (`templates/minutes-base.docx`, learned from the VIFC template by
//! `scripts/make_minutes_base.py`) and the workspace's brand: the base's blues become the brand's
//! document palette, its two typefaces the brand's document fonts, its header picture the
//! brand's logo — or the workspace's name when there is none. The slogans and the name in the
//! footer stay placeholders, filled at export.

use std::fs::File;
use std::io::{Cursor, Read, Write};
use std::path::Path;
use std::sync::LazyLock;

use anyhow::{Context, Result};
use regex::{Captures, NoExpand, Regex};

use super::docx::{escape, is_fillable};
use crate::kernel::brand::tokens::DocPalette;

/// A logo as a PNG, with its size in pixels.
pub struct LogoPng {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// What a template is made from.
pub struct DocBrand<'a> {
    /// The document's author (`docProps/core.xml`).
    pub workspace: &'a str,
    pub palette: &'a DocPalette,
    /// Titles; empty keeps the base's.
    pub heading_font: &'a str,
    /// Everything else; empty keeps the base's.
    pub body_font: &'a str,
    /// None: the header shows the workspace's name instead.
    pub logo: Option<LogoPng>,
}

/// The part with the logo, and its relationships.
const HEADER: &str = "word/header1.xml";
const HEADER_RELS: &str = "word/_rels/header1.xml.rels";
/// The base's typefaces.
const BASE_HEADING: &str = "Playfair Display";
const BASE_BODY: &str = "Be Vietnam Pro";
/// Stands in for the logo when there is none: `{{workspace_name}}`, bold, 13 pt — written in the
/// base's title face and deep colour, so the rebrand applies to it like to the rest.
const NAME_RUN: &str = r#"<w:r><w:rPr><w:rFonts w:ascii="Playfair Display" w:cs="Playfair Display" w:eastAsia="Playfair Display" w:hAnsi="Playfair Display"/><w:b/><w:bCs/><w:color w:val="011B65"/><w:sz w:val="26"/><w:szCs w:val="26"/></w:rPr><w:t xml:space="preserve">{{workspace_name}}</w:t></w:r>"#;

/// A colour attribute (text, fill, border) and its value.
static COLOUR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(\sw:(?:color|fill|val)=")([0-9A-Fa-f]{6})""#).unwrap());
/// A typeface attribute of `w:rFonts` holding one of the base's faces.
static FONT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(\sw:(?:ascii|hAnsi|cs|eastAsia)=")(Playfair Display|Be Vietnam Pro)""#).unwrap()
});
/// A picture's size, in both places Word keeps it.
static EXTENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(<wp:extent|<a:ext) cx="\d+" cy="\d+""#).unwrap());
/// The run holding the header's picture.
static PICTURE_RUN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<w:r><w:drawing>.*?</w:drawing></w:r>").unwrap());
static EMBED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"r:embed="([^"]+)""#).unwrap());
static RELATIONSHIP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<Relationship\b[^>]*?\bId="([^"]+)"[^>]*?\bTarget="([^"]+)""#).unwrap()
});
static CREATOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<dc:creator>[^<]*</dc:creator>").unwrap());

/// Writes the minutes template for `brand`, made from `base` (a .docx), to `out` atomically.
pub fn brand_template(base: &[u8], brand: &DocBrand, out: &Path) -> Result<()> {
    let mut archive =
        zip::ZipArchive::new(Cursor::new(base)).context("the base template is not a .docx")?;
    let mut part = |name: &str| -> Result<String> {
        let mut text = String::new();
        archive
            .by_name(name)
            .with_context(|| format!("the base template has no {name}"))?
            .read_to_string(&mut text)?;
        Ok(text)
    };
    let picture = picture_part(&part(HEADER)?, &part(HEADER_RELS)?)
        .context("the base template's header has no picture")?;
    let font = |chosen: &str, base: &str| -> String {
        let chosen = chosen.trim();
        (if chosen.is_empty() { base } else { chosen }).to_owned()
    };
    let (heading, body) = (
        font(brand.heading_font, BASE_HEADING),
        font(brand.body_font, BASE_BODY),
    );
    let tmp = out.with_extension("docx.tmp");
    let written = (|| -> Result<()> {
        let mut writer = zip::ZipWriter::new(
            File::create(&tmp).with_context(|| format!("creating {}", tmp.display()))?,
        );
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index)?;
            let name = entry.name()?.into_owned();
            let changed = if is_fillable(&name) || name == "word/styles.xml" {
                let mut xml = String::new();
                entry
                    .read_to_string(&mut xml)
                    .with_context(|| format!("reading {name}"))?;
                if name == HEADER {
                    xml = header(&xml, brand.logo.as_ref());
                }
                xml = refont(&xml, &heading, &body);
                if is_fillable(&name) {
                    xml = recolour(&xml, brand.palette);
                }
                Some(xml.into_bytes())
            } else if name == "docProps/core.xml" {
                let mut xml = String::new();
                entry.read_to_string(&mut xml)?;
                let creator = format!("<dc:creator>{}</dc:creator>", escape(brand.workspace));
                Some(
                    CREATOR
                        .replace(&xml, NoExpand(&creator))
                        .into_owned()
                        .into_bytes(),
                )
            } else if name == picture {
                brand.logo.as_ref().map(|logo| logo.bytes.clone())
            } else {
                None
            };
            match changed {
                Some(bytes) => {
                    writer.start_file(name, options)?;
                    writer.write_all(&bytes)?;
                }
                None => writer.raw_copy_file(entry)?,
            }
        }
        writer.finish()?.sync_all()?;
        Ok(())
    })();
    if let Err(err) = written {
        let _ = std::fs::remove_file(&tmp);
        return Err(err);
    }
    std::fs::rename(&tmp, out).with_context(|| format!("saving {}", out.display()))?;
    Ok(())
}

/// The header picture's part (`word/media/….png`), through the header's relationships.
fn picture_part(header: &str, rels: &str) -> Option<String> {
    let id = &EMBED.captures(header)?[1];
    RELATIONSHIP
        .captures_iter(rels)
        .find(|c| &c[1] == id)
        .map(|c| format!("word/{}", &c[2]))
}

/// The header with the picture sized for the logo, or with the workspace's name in its place.
fn header(xml: &str, logo: Option<&LogoPng>) -> String {
    match logo {
        Some(logo) => {
            let (cx, cy) = logo_extent(logo.width, logo.height);
            EXTENT
                .replace_all(xml, |c: &Captures| {
                    format!(r#"{} cx="{cx}" cy="{cy}""#, &c[1])
                })
                .into_owned()
        }
        None => PICTURE_RUN.replace(xml, NoExpand(NAME_RUN)).into_owned(),
    }
}

/// The logo's box in the header (EMU) for a `width`×`height` px image: the base's height (made
/// for VIFC's 5.62:1 logo) grown so a squarer logo gets about the same area, up to twice as
/// tall, and never wider than the logo's table cell (2789 twips).
fn logo_extent(width: u32, height: u32) -> (u64, u64) {
    let aspect = f64::from(width.max(1)) / f64::from(height.max(1));
    let h = (276_225.0 * (5.62 / aspect).sqrt())
        .clamp(276_225.0, 552_450.0)
        .round();
    let w = (h * aspect).round();
    if w > 1_771_015.0 {
        return (1_771_015, (1_771_015.0 / aspect).round() as u64);
    }
    (w as u64, h as u64)
}

/// The base's typefaces → the brand's (`heading`, `body`).
fn refont(xml: &str, heading: &str, body: &str) -> String {
    FONT.replace_all(xml, |c: &Captures| {
        let font = if &c[2] == BASE_HEADING { heading } else { body };
        format!("{}{}\"", &c[1], escape(font))
    })
    .into_owned()
}

/// The base's blues → the palette, in one pass with a lookup (a new colour that equals another
/// base colour is never replaced twice). Neutrals stay.
fn recolour(xml: &str, palette: &DocPalette) -> String {
    COLOUR
        .replace_all(xml, |c: &Captures| {
            let new = match c[2].to_ascii_uppercase().as_str() {
                "0132AD" => &palette.accent,
                "011B65" => &palette.deep,
                "F6F9FF" => &palette.tint97,
                "EBF1FF" => &palette.tint93,
                "DFE7F6" => &palette.tint88,
                "C9D3EA" => &palette.tint78,
                _ => return c[0].to_owned(),
            };
            format!("{}{new}\"", &c[1])
        })
        .into_owned()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use quick_xml::events::Event;
    use resvg::tiny_skia::Pixmap;

    use super::*;
    use crate::kernel::brand::tokens::document_palette;
    use crate::kernel::export::{docx, model};

    const SOURCE_COLOURS: [&str; 6] = ["0132AD", "011B65", "F6F9FF", "EBF1FF", "DFE7F6", "C9D3EA"];

    fn base() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("templates/minutes-base.docx")
    }

    /// The base branded for `primary`, with Georgia for titles and "Noto & Sans" for the rest.
    fn branded(primary: &str, logo: Option<LogoPng>) -> PathBuf {
        let out = std::env::temp_dir().join(format!("newdawn-brand-{}.docx", uuid::Uuid::now_v7()));
        let palette = document_palette(primary);
        let brand = DocBrand {
            workspace: "Acme & Co",
            palette: &palette,
            heading_font: "Georgia",
            body_font: "Noto & Sans",
            logo,
        };
        brand_template(&std::fs::read(base()).unwrap(), &brand, &out).unwrap();
        out
    }

    fn read(docx: &Path, name: &str) -> Vec<u8> {
        let mut archive = zip::ZipArchive::new(File::open(docx).unwrap()).unwrap();
        let mut bytes = Vec::new();
        archive
            .by_name(name)
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        bytes
    }

    fn text(docx: &Path, name: &str) -> String {
        String::from_utf8(read(docx, name)).unwrap()
    }

    #[test]
    fn the_logo_fills_the_picture_at_its_own_aspect() {
        let png = Pixmap::new(300, 100).unwrap().encode_png().unwrap();
        let logo = LogoPng {
            bytes: png.clone(),
            width: 300,
            height: 100,
        };
        let out = branded("#0132AD", Some(logo));
        let header = text(&out, HEADER);
        let picture = picture_part(&header, &text(&out, HEADER_RELS)).unwrap();
        let same_fields = docx::placeholders(&out).unwrap() == docx::placeholders(&base()).unwrap();
        let (logo_bytes, base_bytes) = (read(&out, &picture), read(&base(), &picture));
        std::fs::remove_file(&out).unwrap();
        assert!(same_fields, "the base's placeholders, no more, no fewer");
        assert_eq!(logo_bytes, png);
        assert_ne!(base_bytes, png);
        assert!(
            header.contains(r#"<wp:extent cx="1134207" cy="378069""#),
            "{header}"
        );
        assert!(
            header.contains(r#"<a:ext cx="1134207" cy="378069""#),
            "{header}"
        );
        assert_eq!(
            logo_extent(4000, 100),
            (1_771_015, 44_275),
            "capped by the cell"
        );
        assert_eq!(
            logo_extent(100, 100),
            (552_450, 552_450),
            "at most twice as tall"
        );
    }

    #[test]
    fn without_a_logo_the_header_names_the_workspace() {
        let out = branded("#0132AD", None);
        let header = text(&out, HEADER);
        std::fs::remove_file(&out).unwrap();
        assert!(!header.contains("<w:drawing>"), "{header}");
        assert!(header.contains("{{workspace_name}}"));
        assert!(
            header.contains(r#"<w:rFonts w:ascii="Georgia""#),
            "{header}"
        );
    }

    #[test]
    fn another_brand_recolours_refonts_and_still_fills() {
        let out = branded("#C8102E", None);
        let parts = ["word/document.xml", "word/header1.xml", "word/footer1.xml"];
        for name in parts {
            let xml = text(&out, name);
            let left: Vec<String> = COLOUR
                .captures_iter(&xml)
                .map(|c| c[2].to_ascii_uppercase())
                .filter(|hex| SOURCE_COLOURS.contains(&hex.as_str()))
                .collect();
            assert!(left.is_empty(), "{name} keeps {left:?}");
        }
        for name in parts.iter().chain(&["word/styles.xml"]) {
            let xml = text(&out, name);
            assert!(
                !xml.contains(BASE_HEADING) && !xml.contains(BASE_BODY),
                "{name}"
            );
            assert!(xml.contains(r#"w:ascii="Noto &amp; Sans""#), "{name}");
        }
        assert!(text(&out, "word/document.xml").contains(r#"w:hAnsi="Georgia""#));
        assert!(text(&out, "docProps/core.xml").contains("<dc:creator>Acme &amp; Co</dc:creator>"));

        // Every XML part still parses.
        let mut archive = zip::ZipArchive::new(File::open(&out).unwrap()).unwrap();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).unwrap();
            let name = entry.name().unwrap().into_owned();
            if !name.ends_with(".xml") && !name.ends_with(".rels") {
                continue;
            }
            let mut xml = String::new();
            entry.read_to_string(&mut xml).unwrap();
            let mut reader = quick_xml::Reader::from_str(&xml);
            loop {
                match reader.read_event() {
                    Ok(Event::Eof) => break,
                    Ok(_) => {}
                    Err(err) => panic!("{name}: {err}"),
                }
            }
        }

        // And it fills like any template: nothing left unfilled.
        let filled = out.with_extension("filled.docx");
        let rendered = docx::render(&out, &model::sample(), &filled);
        let leftovers: Vec<&str> = parts
            .into_iter()
            .filter(|name| rendered.is_ok() && text(&filled, name).contains("{{"))
            .collect();
        let _ = std::fs::remove_file(&filled);
        std::fs::remove_file(&out).unwrap();
        rendered.unwrap();
        assert!(leftovers.is_empty(), "placeholders left in {leftovers:?}");
    }
}
