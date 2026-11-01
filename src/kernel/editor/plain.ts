/** The words of a stored rich text (the editor's JSON document, or a plain body from before),
 *  for a table cell's excerpt and a list row — the Rust `rich::plain` for the index. */
export function richPlain(stored: unknown): string {
  if (typeof stored !== "string") return "";
  const text = stored.trim();
  if (!text.startsWith("{")) return text;
  try {
    const out: string[] = [];
    walk(JSON.parse(text), out);
    return out.join("").trim();
  } catch {
    return text;
  }
}

function walk(node: unknown, out: string[]) {
  if (!node || typeof node !== "object") return;
  const n = node as { type?: unknown; text?: unknown; attrs?: { label?: unknown }; content?: unknown };
  const kind = typeof n.type === "string" ? n.type : "";
  if (kind === "text" && typeof n.text === "string") out.push(n.text);
  if (kind === "mention" && typeof n.attrs?.label === "string") out.push(`@${n.attrs.label}`);
  if (kind === "hardBreak") out.push("\n");
  if (Array.isArray(n.content)) {
    n.content.forEach((child, i) => {
      if (i > 0 && kind === "tableRow") out.push("\t");
      walk(child, out);
    });
  }
  if (["paragraph", "heading", "listItem", "taskItem", "blockquote", "codeBlock", "tableRow", "horizontalRule"].includes(kind) && !out[out.length - 1]?.endsWith("\n")) out.push("\n");
}

export function richExcerpt(stored: unknown, max = 80): string {
  const text = richPlain(stored).replace(/\s+/g, " ").trim();
  return text.length > max ? `${text.slice(0, max - 1)}…` : text;
}

/** A stored body as the editor's content: a document as it is, a plain body as one paragraph. */
export function richContent(stored: unknown): object {
  if (typeof stored === "string" && stored.trim().startsWith("{")) {
    try {
      const doc: unknown = JSON.parse(stored);
      if (doc && typeof doc === "object" && (doc as { type?: unknown }).type === "doc") return doc;
    } catch {
      /* a plain body */
    }
  }
  const text = typeof stored === "string" ? stored : "";
  return { type: "doc", content: [{ type: "paragraph", ...(text ? { content: [{ type: "text", text }] } : {}) }] };
}
