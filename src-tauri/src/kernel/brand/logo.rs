//! Logos for documents, made by rules (no model): the full-white version of a logo for dark
//! backgrounds, and the logo as a PNG for a Word header. An SVG is recoloured as text, a PNG
//! pixel by pixel.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::LazyLock;

use anyhow::{Context, Result};
use regex::{Captures, Regex};
use resvg::tiny_skia::{IntSize, Pixmap, Transform};
use resvg::usvg;

/// Neither side of a rasterised logo is larger than this, in pixels.
const MAX_SIDE: f32 = 4000.0;

/// `<style>` blocks: their CSS is recoloured.
static STYLE_BLOCK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)(<style\b[^>]*>)(.*?)(</style>)").unwrap());
/// `style="…"` attributes: their declarations are recoloured.
static STYLE_ATTR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(\sstyle\s*=\s*)("[^"]*"|'[^']*')"#).unwrap());
/// A CSS declaration of a property that carries a colour.
static DECLARATION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(^|[;{\s])(fill|stroke|stop-color|color|flood-color|lighting-color)(\s*:\s*)([^;{}!]*)",
    )
    .unwrap()
});
/// An attribute that carries a colour.
static ATTRIBUTE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(\s)(fill|stroke|stop-color|color|flood-color|lighting-color)(\s*=\s*)("[^"]*"|'[^']*')"#,
    )
    .unwrap()
});
/// The root element's start tag.
static ROOT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<svg\b[^>]*>").unwrap());

/// How a picked SVG is read: a file it names (`<image href="/…">`, `feImage`) is never opened —
/// a logo can't pull other files into the minutes, or hang on `/dev/zero`. Embedded `data:`
/// images still draw (nested SVGs already ignore files).
fn options() -> usvg::Options<'static> {
    let mut options = usvg::Options::default();
    options.image_href_resolver.resolve_string = Box::new(|_, _| None);
    options
}

/// White in place of one colour value. `none`, `transparent`, `inherit`, `currentColor` and
/// `url(#…)` stay (a gradient's stops turn white themselves); an alpha inside the colour
/// (`#RGBA`, `#RRGGBBAA`, `rgba()`, `hsla()`) is kept.
fn white(value: &str) -> String {
    let v = value.trim().to_ascii_lowercase();
    if matches!(
        v.as_str(),
        "none" | "transparent" | "inherit" | "currentcolor"
    ) || v.starts_with("url(")
    {
        return value.to_owned();
    }
    if let Some(hex) = v
        .strip_prefix('#')
        .filter(|h| h.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        return match hex.len() {
            4 => format!("#FFFFFF{0}{0}", &hex[3..]),
            8 => format!("#FFFFFF{}", &hex[6..]),
            _ => "#FFFFFF".into(),
        };
    }
    let alpha = v
        .strip_suffix(')')
        .filter(|_| v.starts_with("rgb") || v.starts_with("hsl"))
        .and_then(|function| function.split_once('('))
        .and_then(|(_, args)| match args.split_once('/') {
            Some((_, alpha)) => Some(alpha),
            None => args.split(',').nth(3),
        });
    match alpha {
        Some(alpha) => format!("rgba(255,255,255,{})", alpha.trim()),
        None => "#FFFFFF".into(),
    }
}

/// Recolours the colour declarations in a piece of CSS.
fn white_css(css: &str) -> String {
    DECLARATION
        .replace_all(css, |c: &Captures| {
            format!("{}{}{}{}", &c[1], &c[2], &c[3], white(&c[4]))
        })
        .into_owned()
}

/// Splits a quoted attribute value into its quote and its text.
fn unquote(quoted: &str) -> (&str, &str) {
    (&quoted[..1], &quoted[1..quoted.len() - 1])
}

/// The full-white version of an SVG logo: every colour in the colour attributes (`fill`,
/// `stroke`, `stop-color`, `color`, `flood-color`, `lighting-color`), in `style="…"` and in
/// `<style>` blocks becomes white, and the root gets a white `fill` and `color` for the shapes
/// that have none (black by default). Opacity is left alone.
pub fn white_svg(svg: &str) -> Result<String> {
    let out = STYLE_BLOCK.replace_all(svg, |c: &Captures| {
        format!("{}{}{}", &c[1], white_css(&c[2]), &c[3])
    });
    let out = STYLE_ATTR.replace_all(&out, |c: &Captures| {
        let (quote, text) = unquote(&c[2]);
        format!("{}{quote}{}{quote}", &c[1], white_css(text))
    });
    let mut out = ATTRIBUTE
        .replace_all(&out, |c: &Captures| {
            let (quote, text) = unquote(&c[4]);
            format!("{}{}{}{quote}{}{quote}", &c[1], &c[2], &c[3], white(text))
        })
        .into_owned();
    let root = ROOT.find(&out).context("the file has no <svg> element")?;
    let (range, mut tag) = (root.range(), root.as_str().to_owned());
    for name in ["color", "fill"] {
        if !ATTRIBUTE.captures_iter(&tag).any(|c| &c[2] == name) {
            tag.insert_str("<svg".len(), &format!(r##" {name}="#FFFFFF""##));
        }
    }
    out.replace_range(range, &tag);
    usvg::Tree::from_str(&out, &options()).context("the white logo isn't a valid SVG")?;
    Ok(out)
}

/// A logo as pixels. An SVG is drawn `width` px wide, its height by its aspect, neither side
/// above 4000 px; a PNG is decoded as it is.
pub fn raster(bytes: &[u8], svg: bool, width: u32) -> Result<Pixmap> {
    if !svg {
        return Pixmap::decode_png(bytes).context("reading the PNG");
    }
    let mut options = options();
    // Scanning the Mac's fonts takes a moment: only for a logo with live text.
    if bytes.windows(5).any(|w| w == b"<text") {
        options.fontdb_mut().load_system_fonts();
    }
    let tree = usvg::Tree::from_data(bytes, &options).context("reading the SVG")?;
    let size = tree.size();
    let scale = (width as f32 / size.width())
        .min(MAX_SIDE / size.width())
        .min(MAX_SIDE / size.height());
    let side = |length: f32| (length * scale).round().max(1.0) as u32;
    let mut pixmap =
        Pixmap::new(side(size.width()), side(size.height())).context("the logo has no size")?;
    resvg::render(
        &tree,
        Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap)
}

/// The full-white version of a raster logo, as a PNG. With any transparency, every pixel turns
/// white and keeps its alpha. Fully opaque, the most common colour along the border is the
/// background: pixels near it become transparent, the rest white, with a soft edge between.
pub fn white_png(pixmap: &Pixmap) -> Result<Vec<u8>> {
    let data = pixmap.data(); // premultiplied RGBA
    let alpha: Vec<u8> = if data.chunks_exact(4).any(|p| p[3] < 255) {
        data.chunks_exact(4).map(|p| p[3]).collect()
    } else {
        let (w, h) = (pixmap.width() as usize, pixmap.height() as usize);
        let at = |x: usize, y: usize| {
            let i = 4 * (y * w + x);
            [data[i], data[i + 1], data[i + 2]]
        };
        let mut counts = BTreeMap::<[u8; 3], u32>::new();
        let border = (0..w)
            .flat_map(|x| [(x, 0), (x, h - 1)])
            .chain((0..h).flat_map(|y| [(0, y), (w - 1, y)]));
        for (x, y) in border {
            *counts.entry(at(x, y)).or_default() += 1;
        }
        let background = counts
            .into_iter()
            .max_by_key(|(_, n)| *n)
            .map_or([255; 3], |(colour, _)| colour);
        data.chunks_exact(4)
            .map(|p| {
                let distance = (0..3)
                    .map(|i| (f32::from(p[i]) - f32::from(background[i])).powi(2))
                    .sum::<f32>()
                    .sqrt();
                (((distance - 24.0) / (96.0 - 24.0)).clamp(0.0, 1.0) * 255.0).round() as u8
            })
            .collect()
    };
    let size = IntSize::from_wh(pixmap.width(), pixmap.height()).context("an empty image")?;
    Pixmap::from_vec(alpha.iter().flat_map(|&a| [a; 4]).collect(), size)
        .context("the white logo")?
        .encode_png()
        .context("writing the white logo")
}

/// The logo as a PNG for a Word header, with its size in pixels: an SVG drawn 2400 px wide
/// (like the header image of the template it was learned from), a PNG as it is.
pub fn header_png(path: &Path) -> Result<(Vec<u8>, u32, u32)> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let svg = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("svg"));
    let pixmap = raster(&bytes, svg, 2400)?;
    let (width, height) = (pixmap.width(), pixmap.height());
    let png = if svg { pixmap.encode_png()? } else { bytes };
    Ok((png, width, height))
}

#[cfg(test)]
mod tests {
    use super::*;
    use resvg::tiny_skia::Color;

    fn vifc(name: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../brands/vifc/logo")
            .join(name)
    }

    #[test]
    fn the_vifc_logo_turns_into_its_official_white_version() {
        let original =
            std::fs::read_to_string(vifc("smifc_fintech_hub_logo_original_tag_sm.svg")).unwrap();
        let ours = raster(white_svg(&original).unwrap().as_bytes(), true, 600).unwrap();
        let official = raster(
            &std::fs::read(vifc("smifc_fintech_hub_logo_full_white_tag_sm.svg")).unwrap(),
            true,
            600,
        )
        .unwrap();
        assert_eq!(
            (ours.width(), ours.height()),
            (official.width(), official.height())
        );
        assert_eq!(ours.width(), 600);
        let pixels = ours
            .data()
            .chunks_exact(4)
            .zip(official.data().chunks_exact(4));
        let close = pixels
            .filter(|(a, b)| a.iter().zip(*b).all(|(x, y)| x.abs_diff(*y) <= 2))
            .count();
        let total = (ours.width() * ours.height()) as usize;
        assert!(
            close * 1000 >= total * 999,
            "{close} of {total} pixels match"
        );
        let ink: Vec<_> = ours.data().chunks_exact(4).filter(|p| p[3] > 0).collect();
        assert!(ink.len() > total / 20, "the logo is drawn");
        assert!(
            ink.iter()
                .all(|p| p[0] == p[3] && p[1] == p[3] && p[2] == p[3]),
            "all white"
        );
    }

    #[test]
    fn white_svg_recolours_colours_and_keeps_the_rest() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 10 10">
            <style>.a { fill: #0132AD; stroke:red !important } .b{fill-opacity:.5;fill:url(#g)}</style>
            <linearGradient id="g"><stop offset="0" stop-color="#123456" stop-opacity="0.4"/></linearGradient>
            <path class="a" d="M0 0h5v5z" style="fill:rgba(1, 50, 173, 0.5);stroke: currentColor"/>
            <rect width="2" height="2" fill="#0132AD80" stroke='blue' opacity="0.3"/>
        </svg>"##;
        let out = white_svg(svg).unwrap();
        assert!(out.starts_with(r##"<svg color="#FFFFFF" xmlns"##), "{out}");
        assert!(out.contains(r#"fill="none""#), "the root's own fill stays");
        assert!(
            out.contains(".a { fill: #FFFFFF; stroke:#FFFFFF!important }"),
            "{out}"
        );
        assert!(out.contains("fill-opacity:.5;fill:url(#g)"), "{out}");
        assert!(
            out.contains(r##"stop-color="#FFFFFF" stop-opacity="0.4""##),
            "{out}"
        );
        assert!(
            out.contains(r#"style="fill:rgba(255,255,255,0.5);stroke: currentColor""#),
            "{out}"
        );
        assert!(
            out.contains(r##"fill="#FFFFFF80" stroke='#FFFFFF' opacity="0.3""##),
            "{out}"
        );
        assert!(white_svg("<html/>").is_err());
    }

    #[test]
    fn a_png_with_transparency_turns_white_and_keeps_its_alpha() {
        let mut logo = Pixmap::new(4, 2).unwrap();
        // Opaque blue, half-transparent red (premultiplied), the rest transparent.
        logo.data_mut()[..8].copy_from_slice(&[1, 50, 173, 255, 100, 0, 0, 128]);
        let png = logo.encode_png().unwrap();
        let white =
            Pixmap::decode_png(&white_png(&raster(&png, false, 0).unwrap()).unwrap()).unwrap();
        for (before, after) in logo
            .data()
            .chunks_exact(4)
            .zip(white.data().chunks_exact(4))
        {
            assert_eq!(after, [before[3]; 4], "white with the alpha kept");
        }
    }

    #[test]
    fn an_opaque_png_loses_its_background() {
        let mut logo = Pixmap::new(30, 20).unwrap();
        logo.fill(Color::WHITE);
        for y in 6..14 {
            for x in 10..20 {
                let i = 4 * (y * 30 + x);
                logo.data_mut()[i..i + 4].copy_from_slice(&[1, 50, 173, 255]);
            }
        }
        let png = logo.encode_png().unwrap();
        let white =
            Pixmap::decode_png(&white_png(&raster(&png, false, 0).unwrap()).unwrap()).unwrap();
        let at = |x: usize, y: usize| &white.data()[4 * (y * 30 + x)..4 * (y * 30 + x) + 4];
        assert_eq!(at(0, 0), [0; 4], "the background is gone");
        assert_eq!(at(29, 19), [0; 4]);
        assert_eq!(at(15, 10), [255; 4], "the mark is white");
    }

    #[test]
    fn an_svg_never_opens_a_file_it_names() {
        let path = std::env::temp_dir().join(format!("newdawn-href-{}.png", uuid::Uuid::now_v7()));
        let mut red = Pixmap::new(4, 4).unwrap();
        red.fill(Color::from_rgba8(255, 0, 0, 255));
        std::fs::write(&path, red.encode_png().unwrap()).unwrap();
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4"><image href="{}" width="4" height="4"/></svg>"#,
            path.display()
        );
        let read_by_default = usvg::Tree::from_str(&svg, &usvg::Options::default())
            .map(|tree| !tree.root().children().is_empty());
        let ours = raster(svg.as_bytes(), true, 4);
        let white = white_svg(&svg);
        let _ = std::fs::remove_file(&path);
        assert!(
            read_by_default.unwrap(),
            "usvg's own default opens the file"
        );
        assert!(
            ours.unwrap().data().iter().all(|&b| b == 0),
            "ours draws nothing from it"
        );
        assert!(white.is_ok());
    }

    #[test]
    fn header_png_draws_an_svg_and_keeps_a_png() {
        let (png, width, height) =
            header_png(&vifc("smifc_fintech_hub_logo_original_tag_sm.svg")).unwrap();
        assert_eq!((width, height), (2400, 421));
        let decoded = Pixmap::decode_png(&png).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (2400, 421));

        let path = std::env::temp_dir().join(format!("newdawn-logo-{}.png", uuid::Uuid::now_v7()));
        let source = Pixmap::new(3, 2).unwrap().encode_png().unwrap();
        std::fs::write(&path, &source).unwrap();
        let kept = header_png(&path);
        let _ = std::fs::remove_file(&path);
        assert_eq!(kept.unwrap(), (source, 3, 2));
    }
}
