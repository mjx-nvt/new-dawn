//! Rich text (0.10.6): a RICH_TEXT column holds the editor's JSON (a ProseMirror document). The
//! words of it go to search, the agents and a table cell's excerpt; the 0.9.x plain bodies are
//! kept as they are.

use serde_json::Value;

/// The plain text of a stored body: the document's text runs, a mention as `@Label`, a newline
/// per block; text that isn't a document comes back unchanged.
pub fn plain(stored: &str) -> String {
    let Ok(doc) = serde_json::from_str::<Value>(stored) else { return stored.to_owned() };
    if !doc.is_object() {
        return stored.to_owned();
    }
    let mut out = String::new();
    walk(&doc, &mut out);
    out.trim().to_owned()
}

fn walk(node: &Value, out: &mut String) {
    let kind = node.get("type").and_then(Value::as_str).unwrap_or("");
    match kind {
        "text" => out.push_str(node.get("text").and_then(Value::as_str).unwrap_or("")),
        "mention" => {
            if let Some(label) = node.get("attrs").and_then(|a| a.get("label")).and_then(Value::as_str) {
                out.push('@');
                out.push_str(label);
            }
        }
        "hardBreak" => out.push('\n'),
        _ => {}
    }
    if let Some(children) = node.get("content").and_then(Value::as_array) {
        for (i, child) in children.iter().enumerate() {
            if i > 0 && matches!(kind, "tableRow") {
                out.push('\t');
            }
            walk(child, out);
        }
    }
    if matches!(kind, "paragraph" | "heading" | "listItem" | "taskItem" | "blockquote" | "codeBlock" | "tableRow" | "horizontalRule") && !out.ends_with('\n') {
        out.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::plain;
    use serde_json::json;

    #[test]
    fn a_document_reads_as_lines() {
        let doc = json!({ "type": "doc", "content": [
            { "type": "heading", "attrs": { "level": 2 }, "content": [{ "type": "text", "text": "Call notes" }] },
            { "type": "paragraph", "content": [{ "type": "text", "text": "With " }, { "type": "mention", "attrs": { "id": "p1", "label": "Harry Do" } }, { "type": "text", "text": " on pricing." }] },
            { "type": "table", "content": [{ "type": "tableRow", "content": [
                { "type": "tableCell", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "Item" }] }] },
                { "type": "tableCell", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "Price" }] }] }
            ] }] },
            { "type": "paragraph" }
        ] });
        assert_eq!(plain(&doc.to_string()), "Call notes\nWith @Harry Do on pricing.\nItem\n\tPrice");
        assert_eq!(plain("just a body from before"), "just a body from before");
        assert_eq!(plain(""), "");
        assert_eq!(plain("[1, 2]"), "[1, 2]");
    }
}
