//! Word template engine: fills a tenant's designed `.docx` (D-006, docs/document-export.md §2).
//!
//! Template language, written as ordinary text in Word:
//! - `{{field}}` / `{{a.b}}` — replaced by the value (looked up innermost context first);
//!   newlines become line breaks.
//! - `{{#list}} … {{/list}}` — repeat. Both markers in one paragraph → that paragraph repeats; in
//!   one table row (different cells) → the row repeats; in different paragraphs/rows → everything
//!   from the start element to the end element repeats. A missing or empty list removes the block
//!   (optional sections). Blocks nest.
//!
//! Word often splits `{{title}}` across runs; runs are merged per placeholder before filling, and
//! the placeholder's first run keeps its formatting. Untouched XML is written back byte for byte.

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};
use quick_xml::Reader;
use quick_xml::events::Event;
use serde_json::Value;

// ── A minimal DOM that keeps every tag's raw source text ─────────────────────────────────────

#[derive(Debug, Clone)]
enum Node {
    Element(Element),
    /// Raw (still escaped) text, comments, declarations, entity references.
    Raw(String),
}

#[derive(Debug, Clone)]
struct Element {
    name: String,
    /// The start tag exactly as written, e.g. `<w:p w14:paraId="1A">` or `<w:br/>`.
    open: String,
    children: Vec<Node>,
    empty: bool,
}

impl Element {
    fn new(name: &str, open: &str, empty: bool) -> Self {
        Self {
            name: name.to_owned(),
            open: open.to_owned(),
            children: Vec::new(),
            empty,
        }
    }
}

fn tag_name(raw: &str) -> String {
    raw.trim_start_matches(['<', '/'])
        .split(|c: char| c.is_whitespace() || c == '>' || c == '/')
        .next()
        .unwrap_or_default()
        .to_owned()
}

fn parse(xml: &str) -> Result<Vec<Node>> {
    let mut reader = Reader::from_str(xml);
    let mut stack: Vec<Element> = vec![Element::new("#root", "", false)];
    let mut last = 0usize;
    loop {
        let event = reader.read_event().context("reading Word XML")?;
        let pos = reader.buffer_position() as usize;
        let raw = &xml[last..pos];
        last = pos;
        match event {
            Event::Eof => break,
            Event::Start(_) => stack.push(Element::new(&tag_name(raw), raw, false)),
            Event::Empty(_) => {
                let el = Element::new(&tag_name(raw), raw, true);
                stack
                    .last_mut()
                    .expect("root")
                    .children
                    .push(Node::Element(el));
            }
            Event::End(_) => {
                let el = stack.pop().context("unbalanced Word XML")?;
                stack
                    .last_mut()
                    .context("unbalanced Word XML")?
                    .children
                    .push(Node::Element(el));
            }
            _ => stack
                .last_mut()
                .expect("root")
                .children
                .push(Node::Raw(raw.to_owned())),
        }
    }
    if stack.len() != 1 {
        bail!("unbalanced Word XML");
    }
    Ok(stack.pop().expect("root").children)
}

fn write_nodes(nodes: &[Node], out: &mut String) {
    for node in nodes {
        match node {
            Node::Raw(raw) => out.push_str(raw),
            Node::Element(el) => {
                out.push_str(&el.open);
                if !el.empty {
                    write_nodes(&el.children, out);
                    out.push_str("</");
                    out.push_str(&el.name);
                    out.push('>');
                }
            }
        }
    }
}

fn unescape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let Some(semi) = rest[amp..].find(';') else {
            out.push_str(&rest[amp..]);
            return out;
        };
        let entity = &rest[amp + 1..amp + semi];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ if entity.starts_with("#x") => u32::from_str_radix(&entity[2..], 16)
                .ok()
                .and_then(char::from_u32),
            _ if entity.starts_with('#') => entity[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        };
        match decoded {
            Some(c) => out.push(c),
            None => out.push_str(&rest[amp..=amp + semi]),
        }
        rest = &rest[amp + semi + 1..];
    }
    out.push_str(rest);
    out
}

/// XML-escapes text and drops characters XML 1.0 forbids (Word rejects the file otherwise).
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\t' | '\n' => out.push(c),
            c if (c as u32) < 0x20 || c == '\u{FFFE}' || c == '\u{FFFF}' => {}
            c => out.push(c),
        }
    }
    out
}

// ── Text access ───────────────────────────────────────────────────────────────────────────────

/// Unescaped text of a `<w:t>` element.
fn t_text(t: &Element) -> String {
    let mut raw = String::new();
    for c in &t.children {
        if let Node::Raw(r) = c {
            raw.push_str(r);
        }
    }
    unescape(&raw)
}

fn set_t_text(t: &mut Element, text: &str) {
    if t.empty {
        t.empty = false;
        t.open = format!(
            "{}>",
            t.open
                .trim_end_matches('>')
                .trim_end_matches('/')
                .trim_end()
        );
    }
    if !t.open.contains("xml:space") {
        t.open = format!("{} xml:space=\"preserve\">", t.open.trim_end_matches('>'));
    }
    t.children = vec![Node::Raw(escape(text))];
}

/// All `<w:t>` elements below `el`, in document order.
fn collect_t<'a>(el: &'a mut Element, out: &mut Vec<&'a mut Element>) {
    for child in &mut el.children {
        if let Node::Element(c) = child {
            if c.name == "w:t" {
                out.push(c);
            } else {
                collect_t(c, out);
            }
        }
    }
}

fn text_of(el: &Element) -> String {
    let mut s = String::new();
    for child in &el.children {
        if let Node::Element(c) = child {
            if c.name == "w:t" {
                s.push_str(&t_text(c));
            } else {
                s.push_str(&text_of(c));
            }
        }
    }
    s
}

/// Word splits text into runs freely; make every `{{…}}` live inside one `<w:t>` (the run where
/// it starts keeps the formatting).
fn merge_placeholder_runs(paragraph: &mut Element) {
    let mut ts = Vec::new();
    collect_t(paragraph, &mut ts);
    let texts: Vec<String> = ts.iter().map(|t| t_text(t)).collect();
    let joined: String = texts.concat();
    if !joined.contains("{{") {
        return;
    }
    // Owner (t index) of every byte of `joined`.
    let mut starts = Vec::with_capacity(texts.len());
    let mut offset = 0;
    for t in &texts {
        starts.push(offset);
        offset += t.len();
    }
    let owner = |pos: usize| starts.iter().rposition(|s| *s <= pos).unwrap_or(0);
    let mut new_texts = texts.clone();
    let mut tokens = Vec::new();
    let mut search = 0;
    while let Some(open) = joined[search..].find("{{").map(|i| i + search) {
        let Some(close) = joined[open..].find("}}").map(|i| i + open + 2) else {
            break;
        };
        tokens.push((open, close));
        search = close;
    }
    // Last to first, so earlier byte offsets stay valid.
    for &(open, close) in tokens.iter().rev() {
        let (i, j) = (owner(open), owner(close - 1));
        if i == j {
            continue;
        }
        let local_open = open - starts[i];
        let local_close = close - starts[j];
        let head = new_texts[i][..local_open].to_owned();
        let tail = new_texts[j][local_close..].to_owned();
        new_texts[i] = format!("{head}{}", &joined[open..close]);
        for text in &mut new_texts[i + 1..j] {
            text.clear();
        }
        new_texts[j] = tail;
    }
    // Only rewrite the runs that changed, so the rest stay byte-identical.
    for ((t, old), new) in ts.into_iter().zip(&texts).zip(new_texts) {
        if *old != new {
            set_t_text(t, &new);
        }
    }
}

// ── Filling ───────────────────────────────────────────────────────────────────────────────────

fn lookup<'a>(stack: &[&'a Value], path: &str) -> Option<&'a Value> {
    stack.iter().rev().find_map(|ctx| {
        let mut v = *ctx;
        for key in path.split('.') {
            v = v.get(key)?;
        }
        Some(v)
    })
}

fn display(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

/// Items a block repeats over: arrays as-is; `true`, an object, a non-empty string or a number
/// once (so `{{#venue}}Venue: {{venue}}{{/venue}}` works); false/null/""/missing never.
fn block_items(value: Option<&Value>) -> Vec<&Value> {
    match value {
        Some(Value::Array(items)) => items.iter().collect(),
        Some(v @ (Value::Bool(true) | Value::Object(_) | Value::Number(_))) => vec![v],
        Some(v @ Value::String(s)) if !s.trim().is_empty() => vec![v],
        _ => Vec::new(),
    }
}

fn first_open_marker(text: &str) -> Option<String> {
    let start = text.find("{{#")?;
    let end = text[start..].find("}}")? + start;
    Some(text[start + 3..end].trim().to_owned())
}

const REPEATABLE: [&str; 3] = ["w:p", "w:tr", "w:tbl"];

/// Fills `nodes` in place (blocks, then fields) with the context stack. Placeholder runs must
/// already be merged (`merge_all`), so markers are always found whole.
// ponytail: text boxes (w:txbxContent) are treated as part of their anchor paragraph; process them
// as separate paragraphs if a template ever puts placeholders in a text box.
fn process(nodes: &mut Vec<Node>, stack: &[&Value]) -> Result<()> {
    let mut i = 0;
    while i < nodes.len() {
        let Node::Element(el) = &mut nodes[i] else {
            i += 1;
            continue;
        };
        let text = text_of(el);
        let block = first_open_marker(&text).filter(|_| REPEATABLE.contains(&el.name.as_str()));
        if let Some(key) = block {
            let close = format!("{{{{/{key}}}}}");
            let end = if text.contains(&close) {
                // Both markers inside this element. A paragraph repeats itself; a row repeats
                // when the markers sit in different cells; a table never repeats as a unit (the
                // block is among its rows, or deeper in one cell).
                let open = format!("{{{{#{key}}}}}");
                let one_child_has_both = el
                    .children
                    .iter()
                    .any(|c| matches!(c, Node::Element(c) if { let t = text_of(c); t.contains(&open) && t.contains(&close) }));
                let deeper = match el.name.as_str() {
                    "w:p" => false,
                    "w:tbl" => true,
                    _ => one_child_has_both,
                };
                if deeper { None } else { Some(i) }
            } else {
                let found = nodes[i + 1..]
                    .iter()
                    .position(|n| matches!(n, Node::Element(e) if text_of(e).contains(&close)));
                match found {
                    Some(offset) => Some(i + 1 + offset),
                    None => bail!("template block {{{{#{key}}}}} has no matching {{{{/{key}}}}}"),
                }
            };
            if let Some(end) = end {
                let range: Vec<Node> = nodes.drain(i..=end).collect();
                let mut produced = Vec::new();
                for item in block_items(lookup(stack, &key)) {
                    let mut copy = range.clone();
                    strip_markers(&mut copy, &key)?;
                    let mut inner = stack.to_vec();
                    inner.push(item);
                    process(&mut copy, &inner)?;
                    produced.extend(copy);
                }
                let count = produced.len();
                nodes.splice(i..i, produced);
                i += count;
                continue;
            }
            // The block lives deeper (among this element's children): fall through.
        }
        let Node::Element(el) = &mut nodes[i] else {
            unreachable!("checked above")
        };
        if el.name == "w:p" {
            fill_runs(el, stack);
        } else {
            process(&mut el.children, stack)?;
            // Keep the structure Word requires after blocks were removed.
            if el.name == "w:tc" && !has_child(el, "w:p") {
                el.children
                    .push(Node::Element(Element::new("w:p", "<w:p/>", true)));
            }
            if el.name == "w:tbl" && !has_child(el, "w:tr") {
                nodes.remove(i);
                continue;
            }
        }
        i += 1;
    }
    Ok(())
}

fn has_child(el: &Element, name: &str) -> bool {
    el.children
        .iter()
        .any(|c| matches!(c, Node::Element(c) if c.name == name))
}

/// Any content besides text (drawings, breaks, section properties…) anywhere below `el`.
fn has_non_text_content(el: &Element) -> bool {
    el.children.iter().any(|c| match c {
        Node::Element(c) => {
            matches!(
                c.name.as_str(),
                "w:drawing" | "w:pict" | "w:br" | "w:sectPr" | "w:object" | "mc:AlternateContent"
            ) || has_non_text_content(c)
        }
        Node::Raw(_) => false,
    })
}

/// Merges placeholder runs in every paragraph of a part, once, before filling.
fn merge_all(nodes: &mut [Node]) {
    for node in nodes {
        if let Node::Element(el) = node {
            if el.name == "w:p" {
                merge_placeholder_runs(el);
            }
            merge_all(&mut el.children);
        }
    }
}

/// Removes the first `{{#key}}` and the last `{{/key}}` of a block copy; drops the first/last
/// paragraph when it held nothing but the marker.
fn strip_markers(range: &mut Vec<Node>, key: &str) -> Result<()> {
    let open = format!("{{{{#{key}}}}}");
    let close = format!("{{{{/{key}}}}}");
    fn replace_once(el: &mut Element, needle: &str, last: bool) -> bool {
        let mut ts = Vec::new();
        collect_t(el, &mut ts);
        let order: Vec<usize> = if last {
            (0..ts.len()).rev().collect()
        } else {
            (0..ts.len()).collect()
        };
        for k in order {
            let text = t_text(ts[k]);
            let found = if last {
                text.rfind(needle)
            } else {
                text.find(needle)
            };
            if let Some(at) = found {
                let new = format!("{}{}", &text[..at], &text[at + needle.len()..]);
                set_t_text(ts[k], &new);
                return true;
            }
        }
        false
    }
    let elements: Vec<usize> = range
        .iter()
        .enumerate()
        .filter(|(_, n)| matches!(n, Node::Element(_)))
        .map(|(k, _)| k)
        .collect();
    let (Some(&first), Some(&last)) = (elements.first(), elements.last()) else {
        return Ok(());
    };
    let Node::Element(el) = &mut range[first] else {
        unreachable!()
    };
    if !replace_once(el, &open, false) {
        // Would otherwise be found again by `process` and repeat forever.
        bail!("template block {open} could not be removed (is it split oddly in Word?)");
    }
    let Node::Element(el) = &mut range[last] else {
        unreachable!()
    };
    if !replace_once(el, &close, true) {
        bail!("template block {close} could not be removed (is it split oddly in Word?)");
    }
    // A paragraph that held only a marker disappears; designed blank paragraphs, breaks and
    // drawings elsewhere in the block are left alone.
    let mut drop = Vec::new();
    for k in [first, last] {
        if let Node::Element(el) = &range[k] {
            if el.name == "w:p"
                && text_of(el).trim().is_empty()
                && !has_non_text_content(el)
                && !drop.contains(&k)
            {
                drop.push(k);
            }
        }
    }
    drop.sort_unstable_by(|a, b| b.cmp(a));
    for k in drop {
        range.remove(k);
    }
    Ok(())
}

/// Replaces `{{field}}` in a paragraph's text runs; a value with newlines becomes line breaks.
fn fill_runs(el: &mut Element, stack: &[&Value]) {
    for child in &mut el.children {
        let Node::Element(c) = child else { continue };
        if c.name == "w:r" {
            fill_run(c, stack);
        } else {
            fill_runs(c, stack);
        }
    }
}

fn fill_run(run: &mut Element, stack: &[&Value]) {
    let mut new_children = Vec::with_capacity(run.children.len());
    for child in std::mem::take(&mut run.children) {
        match child {
            Node::Element(t) if t.name == "w:t" && !t_text(&t).contains("{{") => {
                new_children.push(Node::Element(t))
            }
            Node::Element(t) if t.name == "w:t" => {
                let filled = substitute(&t_text(&t), stack)
                    .replace("\r\n", "\n")
                    .replace('\r', "\n");
                let mut lines = filled.split('\n').peekable();
                while let Some(line) = lines.next() {
                    let mut piece = t.clone();
                    set_t_text(&mut piece, line);
                    new_children.push(Node::Element(piece));
                    if lines.peek().is_some() {
                        new_children.push(Node::Element(Element::new("w:br", "<w:br/>", true)));
                    }
                }
            }
            other => new_children.push(other),
        }
    }
    run.children = new_children;
}

fn substitute(text: &str, stack: &[&Value]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find("{{") {
        out.push_str(&rest[..open]);
        let Some(close) = rest[open..].find("}}") else {
            out.push_str(&rest[open..]);
            return out;
        };
        let name = rest[open + 2..open + close].trim();
        if !name.starts_with(['#', '/']) {
            out.push_str(&display(lookup(stack, name)));
        }
        rest = &rest[open + close + 2..];
    }
    out.push_str(rest);
    out
}

/// Fills one Word part (document, header or footer XML).
fn fill_part(xml: &str, data: &Value) -> Result<String> {
    // quick-xml skips a BOM without counting it, which would shift every raw slice.
    let xml = xml.strip_prefix('\u{feff}').unwrap_or(xml);
    let mut nodes = parse(xml)?;
    merge_all(&mut nodes);
    process(&mut nodes, &[data])?;
    let mut out = String::with_capacity(xml.len() + 1024);
    write_nodes(&nodes, &mut out);
    Ok(out)
}

fn is_fillable(name: &str) -> bool {
    name == "word/document.xml"
        || (name.starts_with("word/header") && name.ends_with(".xml"))
        || (name.starts_with("word/footer") && name.ends_with(".xml"))
}

/// Fills `template` with `data` and writes the result to `out` atomically (temp file + rename).
pub fn render(template: &Path, data: &Value, out: &Path) -> Result<()> {
    let mut archive = zip::ZipArchive::new(
        File::open(template).with_context(|| format!("opening {}", template.display()))?,
    )
    .context("the template is not a valid .docx")?;
    let tmp = out.with_extension("docx.tmp");
    let written = (|| -> Result<()> {
        let mut writer = zip::ZipWriter::new(
            File::create(&tmp).with_context(|| format!("creating {}", tmp.display()))?,
        );
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index)?;
            let name = entry.name()?.into_owned();
            if is_fillable(&name) {
                let mut xml = String::new();
                entry
                    .read_to_string(&mut xml)
                    .with_context(|| format!("reading {name}"))?;
                let filled = fill_part(&xml, data).with_context(|| format!("filling {name}"))?;
                let options = zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated);
                writer.start_file(name, options)?;
                writer.write_all(filled.as_bytes())?;
            } else {
                writer.raw_copy_file(entry)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn collect_paragraph_text(nodes: &mut [Node], out: &mut Vec<String>) {
        for node in nodes {
            if let Node::Element(el) = node {
                if el.name == "w:p" {
                    out.push(text_of(el));
                } else {
                    collect_paragraph_text(&mut el.children, out);
                }
            }
        }
    }
    use serde_json::json;

    const NS: &str = r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main""#;

    fn p(runs: &[&str]) -> String {
        let runs: String = runs
            .iter()
            .map(|t| {
                format!(r#"<w:r><w:rPr><w:b/></w:rPr><w:t xml:space="preserve">{t}</w:t></w:r>"#)
            })
            .collect();
        format!("<w:p><w:pPr><w:spacing w:after=\"100\"/></w:pPr>{runs}</w:p>")
    }

    fn cell(text: &str) -> String {
        format!("<w:tc><w:tcPr/>{}</w:tc>", p(&[text]))
    }

    fn doc(body: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document {NS}><w:body>{body}<w:sectPr/></w:body></w:document>"#
        )
    }

    fn texts(xml: &str) -> Vec<String> {
        let mut nodes = parse(xml).unwrap();
        let mut out = Vec::new();
        collect_paragraph_text(&mut nodes, &mut out);
        out
    }

    #[test]
    fn round_trips_untouched_xml_exactly() {
        let xml = doc(&format!(
            "{}<w:p/><!-- c --><w:p><w:r><w:t>a &amp; b</w:t></w:r></w:p>",
            p(&["plain"])
        ));
        assert_eq!(fill_part(&xml, &json!({})).unwrap(), xml);
    }

    #[test]
    fn merges_split_placeholders_and_fills_fields() {
        // Word split "{{title}}" over three runs, with text before and after.
        let xml = doc(&p(&["Minutes: {", "{ti", "tle}} — {{date}}!"]));
        let out = fill_part(
            &xml,
            &json!({ "title": "Partner call & co", "date": "24/10/2026" }),
        )
        .unwrap();
        assert_eq!(
            texts(&out),
            vec!["Minutes: Partner call & co — 24/10/2026!"]
        );
        assert!(out.contains("Partner call &amp; co"), "escaped in XML");
        assert!(out.contains("<w:rPr><w:b/></w:rPr>"), "formatting kept");
    }

    #[test]
    fn newlines_become_line_breaks() {
        let out = fill_part(
            &doc(&p(&["{{summary}}"])),
            &json!({ "summary": "one\ntwo" }),
        )
        .unwrap();
        assert!(
            out.contains(r#"one</w:t><w:br/><w:t xml:space="preserve">two"#),
            "{out}"
        );
    }

    #[test]
    fn repeats_table_rows_and_removes_empty_blocks() {
        let header = format!("<w:tr>{}{}</w:tr>", cell("Action"), cell("Owner"));
        let pattern = format!(
            "<w:tr>{}{}</w:tr>",
            cell("{{#actions}}{{item}}"),
            cell("{{owner}}{{/actions}}")
        );
        let xml = doc(&format!(
            "<w:tbl>{header}{pattern}</w:tbl>{}",
            p(&["{{#notes}}Note: {{text}}{{/notes}}"])
        ));
        let data = json!({ "actions": [
            { "item": "Send agenda", "owner": "Minh" },
            { "item": "Confirm venue", "owner": "Sarah" },
            { "item": "Brief authorities", "owner": "Tien" }
        ]});
        let out = fill_part(&xml, &data).unwrap();
        assert_eq!(
            texts(&out),
            vec![
                "Action",
                "Owner",
                "Send agenda",
                "Minh",
                "Confirm venue",
                "Sarah",
                "Brief authorities",
                "Tien"
            ]
        );
        assert_eq!(out.matches("<w:tr>").count(), 4, "header + 3 rows");
        assert!(
            !out.contains("Note:"),
            "missing list removes the paragraph block"
        );
    }

    /// A block over rows (group header row + repeated member row) repeats rows, never the table.
    #[test]
    fn grouped_rows_repeat_inside_one_table() {
        let header = format!(
            "<w:tr><w:trPr><w:tblHeader/></w:trPr>{}{}</w:tr>",
            cell("Name"),
            cell("Title")
        );
        let group = format!("<w:tr>{}</w:tr>", cell("{{#groups}}{{org}}"));
        let member = format!(
            "<w:tr>{}{}</w:tr>",
            cell("{{#members}}{{name}}"),
            cell("{{title}}{{/members}}{{/groups}}")
        );
        let xml = doc(&format!("<w:tbl>{header}{group}{member}</w:tbl>"));
        let data = json!({ "groups": [
            { "org": "VIFC", "members": [{ "name": "Tien", "title": "Head" }, { "name": "Minh", "title": "Lead" }] },
            { "org": "Acme", "members": [{ "name": "Sarah", "title": "Partner" }] }
        ]});
        let out = fill_part(&xml, &data).unwrap();
        assert_eq!(out.matches("<w:tbl>").count(), 1, "one table");
        assert_eq!(out.matches("<w:tblHeader/>").count(), 1, "header once");
        assert_eq!(
            texts(&out),
            vec![
                "Name", "Title", "VIFC", "Tien", "Head", "Minh", "Lead", "Acme", "Sarah", "Partner"
            ]
        );
    }

    #[test]
    fn nested_blocks_across_paragraphs() {
        let body = format!(
            "{}{}{}",
            p(&["{{#discussion}}{{number}}  {{topic}}"]),
            p(&["•  {{#points}}{{text}}{{/points}}{{/discussion}}"]),
            p(&["After"])
        );
        let data = json!({ "discussion": [
            { "number": "3.1", "topic": "Pipeline", "points": [{ "text": "12 deals" }, { "text": "4 at term sheet" }] },
            { "number": "3.2", "topic": "Regulation", "points": [{ "text": "Sandbox published" }] }
        ]});
        let out = fill_part(&doc(&body), &data).unwrap();
        assert_eq!(
            texts(&out),
            vec![
                "3.1  Pipeline",
                "•  12 deals",
                "•  4 at term sheet",
                "3.2  Regulation",
                "•  Sandbox published",
                "After"
            ]
        );
    }

    #[test]
    fn block_inside_a_table_cell_repeats_the_paragraph_not_the_row() {
        let row = format!(
            "<w:tr>{}<w:tc>{}</w:tc></w:tr>",
            cell("Agenda"),
            p(&["{{#agenda}}Session {{n}}: {{topic}}{{/agenda}}"])
        );
        let out = fill_part(
            &doc(&format!("<w:tbl>{row}</w:tbl>")),
            &json!({ "agenda": [{ "n": 1, "topic": "A" }, { "n": 2, "topic": "B" }] }),
        )
        .unwrap();
        assert_eq!(texts(&out), vec!["Agenda", "Session 1: A", "Session 2: B"]);
        assert_eq!(out.matches("<w:tr>").count(), 1);
    }

    /// Word split the row marker across runs inside a cell: merged first, no endless loop.
    #[test]
    fn split_block_marker_in_a_cell() {
        let row = format!(
            "<w:tr><w:tc>{}</w:tc><w:tc>{}</w:tc></w:tr>",
            p(&["{{#act", "ions}}{{item}}"]),
            p(&["{{owner}}{{/act", "ions}}"])
        );
        let out = fill_part(
            &doc(&format!("<w:tbl>{row}</w:tbl>")),
            &json!({ "actions": [{ "item": "A", "owner": "M" }, { "item": "B", "owner": "S" }] }),
        )
        .unwrap();
        assert_eq!(texts(&out), vec!["A", "M", "B", "S"]);
    }

    /// An empty block must not leave a cell without a paragraph (Word calls that unreadable).
    #[test]
    fn empty_block_keeps_cells_valid() {
        let row = format!(
            "<w:tr>{}<w:tc>{}</w:tc></w:tr>",
            cell("Agenda"),
            p(&["{{#agenda}}Session {{n}}{{/agenda}}"])
        );
        let out = fill_part(
            &doc(&format!("<w:tbl>{row}</w:tbl>")),
            &json!({ "agenda": [] }),
        )
        .unwrap();
        assert!(out.contains("<w:tc><w:p/></w:tc>"), "{out}");
        let gone = fill_part(
            &doc(&format!(
                "<w:tbl><w:tr>{}{}</w:tr></w:tbl>",
                cell("{{#x}}a"),
                cell("b{{/x}}")
            )),
            &json!({}),
        )
        .unwrap();
        assert!(
            !gone.contains("<w:tbl>"),
            "a table with no rows left is removed"
        );
    }

    #[test]
    fn bom_and_bad_characters() {
        let xml = format!("\u{feff}{}", doc(&p(&["{{v}}"])));
        let out = fill_part(&xml, &json!({ "v": "Tiếng\u{1}Việt\r\nok" })).unwrap();
        assert_eq!(texts(&out), vec!["TiếngViệtok"]);
        assert!(out.contains("<w:br/>"));
    }

    #[test]
    fn untouched_runs_stay_byte_identical_beside_a_placeholder() {
        let body = r#"<w:p><w:r><w:t>Plain &#233;</w:t></w:r><w:r><w:t>{{v}}</w:t></w:r></w:p>"#;
        let out = fill_part(&doc(body), &json!({ "v": "x" })).unwrap();
        assert!(out.contains("<w:r><w:t>Plain &#233;</w:t></w:r>"), "{out}");
    }

    #[test]
    fn string_values_open_optional_blocks() {
        let body = p(&["{{#venue}}Venue: {{venue}}{{/venue}}"]);
        assert_eq!(
            texts(&fill_part(&doc(&body), &json!({ "venue": "L30" })).unwrap()),
            vec!["Venue: L30"]
        );
        assert!(texts(&fill_part(&doc(&body), &json!({ "venue": "" })).unwrap()).is_empty());
    }

    #[test]
    fn unbalanced_block_is_an_error() {
        assert!(fill_part(&doc(&p(&["{{#a}} only start"])), &json!({ "a": [1] })).is_err());
        assert!(
            fill_part(
                &doc(&format!("{}{}", p(&["{{#a}}x"]), p(&["y"]))),
                &json!({ "a": [1] })
            )
            .is_err()
        );
    }

    /// The VIFC template in the repo fills cleanly: no `{{` left, sections present.
    #[test]
    fn fills_the_vifc_template() {
        let template = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../brands/vifc/templates/meeting-minutes/template.docx");
        let out = std::env::temp_dir().join(format!("newdawn-vifc-{}.docx", uuid::Uuid::now_v7()));
        let data = sample_minutes();
        render(&template, &data, &out).unwrap();
        let mut archive = zip::ZipArchive::new(File::open(&out).unwrap()).unwrap();
        let mut xml = String::new();
        archive
            .by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut xml)
            .unwrap();
        let all = texts(&xml).join("\n");
        assert!(!all.contains("{{"), "leftover placeholder:\n{all}");
        for expected in [
            "Quarterly partner review",
            "BB-24102026-01",
            "VIFC Fintech Hub",
            "Ms Sarah Lee",
            "3.1  Pipeline",
            "•  Twelve active deals",
            "QĐ-01",
            "CV-03",
            "Brief the authorities",
            "1.  Invite the central bank observer?",
        ] {
            assert!(all.contains(expected), "missing {expected:?}");
        }
        if let Ok(keep) = std::env::var("NEWDAWN_KEEP_SAMPLE") {
            std::fs::copy(&out, keep).unwrap();
        }
        std::fs::remove_file(out).unwrap();
    }

    fn sample_minutes() -> Value {
        json!({
            "title": "Quarterly partner review  ×  Acme Capital",
            "datetime": "Friday, 24/10/2026 — 10:00–11:30",
            "meeting_type": "External partner",
            "reference": "BB-24102026-01",
            "agenda": [{ "n": 1, "topic": "Pipeline" }, { "n": 2, "topic": "Delegation visit" }],
            "summary": "The Hub met Acme Capital to review the partner pipeline and prepare next month's delegation visit. Both sides agreed the roundtable venue; the budget split goes to management.",
            "next_focus": "Minh circulates the draft agenda by Friday.",
            "attendee_groups": [
                { "organisation": "VIFC Fintech Hub", "members": [
                    { "name": "Mr Tien Nguyen", "job_title": "Head of Hub", "attendance": "Present" },
                    { "name": "Mr Minh Tran", "job_title": "Programme lead", "attendance": "Present" } ] },
                { "organisation": "Acme Capital", "members": [
                    { "name": "Ms Sarah Lee", "job_title": "Partner", "attendance": "Present" } ] }
            ],
            "discussion": [
                { "n": 1, "topic": "Pipeline", "points": [{ "text": "Twelve active deals" }, { "text": "Four at term-sheet stage" }] },
                { "n": 2, "topic": "Delegation visit", "points": [{ "text": "About twenty participants from Singapore and Seoul" }] }
            ],
            "decisions": [
                { "nn": "01", "text": "Host the roundtable at city hall", "decided_by": "Chair, with Acme", "condition": "Subject to venue availability" }
            ],
            "actions": [
                { "nn": "01", "item": "Circulate the draft agenda", "owner": "Minh", "due": "Friday", "status": "Open" },
                { "nn": "02", "item": "Confirm the roundtable venue", "owner": "Sarah", "due": "Monday", "status": "Open" },
                { "nn": "03", "item": "Brief the authorities", "owner": "Tien", "due": "Wednesday", "status": "Open" }
            ],
            "open_issues": [{ "n": 1, "issue": "Invite the central bank observer?", "who": "Management", "by": "Not set" }]
        })
    }
}
