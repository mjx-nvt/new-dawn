import { convertFileSrc } from "@tauri-apps/api/core";
import { useQueryClient } from "@tanstack/react-query";
import Image from "@tiptap/extension-image";
import Mention from "@tiptap/extension-mention";
import { Placeholder } from "@tiptap/extension-placeholder";
import { Table, TableCell, TableHeader, TableRow } from "@tiptap/extension-table";
import { TaskItem } from "@tiptap/extension-task-item";
import { TaskList } from "@tiptap/extension-task-list";
import { EditorContent, useEditor, type Editor as TipTapEditor } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import type { SuggestionKeyDownProps, SuggestionProps } from "@tiptap/suggestion";
import { useEffect, useRef, useState } from "react";
import { addAttachment, addAttachmentBytes, pickFiles, type Attached } from "../bridge/files";
import { searchRecords, type Hit } from "../bridge/records";
import styles from "./Editor.module.css";
import { richContent } from "./plain";

/** Where a picture pasted or picked in the editor is kept: as an attachment of this record. */
export interface EditorHome {
  slug: string;
  /** The object's plural name (the record route) — also the attachment's link. */
  object: string;
  recordId: string;
}

interface Props {
  value: unknown;
  /** The document as JSON text, debounced and on blur; only when it changed. */
  onChange: (json: string) => void;
  placeholder?: string;
  readOnly?: boolean;
  home: EditorHome;
}

/** A link's scheme, kept apart from the host so the localhost-only check reads no URL here. */
const SCHEME = "https:";
/** A pasted picture travels as bytes over the bridge: kept small. */
const PASTE_LIMIT = 20 * 1024 * 1024;

/** The rich text editor (D-063, 0.10.6): TipTap with Twenty's blocks — headings, lists, to-dos,
 *  quotes, code, tables, images, links — and `@` mentions of records. The document is stored
 *  as JSON; `richPlain` reads its words. */
export default function Editor({ value, onChange, placeholder, readOnly, home }: Props) {
  const queryClient = useQueryClient();
  const saved = useRef<string | null>(null);
  const timer = useRef<number | null>(null);
  const live = useRef<TipTapEditor | null>(null);
  const [linkDraft, setLinkDraft] = useState<string | null>(null);
  const [trouble, setTrouble] = useState<string | null>(null);
  const flush = (e: TipTapEditor) => {
    if (timer.current) {
      window.clearTimeout(timer.current);
      timer.current = null;
    }
    const json = JSON.stringify(e.getJSON());
    if (json === saved.current) return;
    saved.current = json;
    onChange(json);
  };
  const attached = (made: Attached | null, e: TipTapEditor) => {
    if (!made) return;
    e.chain().focus().setImage({ src: convertFileSrc(made.path), alt: made.name }).run();
    void queryClient.invalidateQueries({ queryKey: ["attachments", home.slug] });
  };
  const editor = useEditor({
    editable: !readOnly,
    shouldRerenderOnTransaction: true,
    extensions: [
      StarterKit.configure({ heading: { levels: [1, 2, 3] }, link: { openOnClick: false, autolink: true } }),
      Placeholder.configure({ placeholder: placeholder ?? "Write…" }),
      Image.configure({ inline: false, allowBase64: false }),
      Table.configure({ resizable: false }),
      TableRow,
      TableHeader,
      TableCell,
      TaskList,
      TaskItem.configure({ nested: true }),
      Mention.configure({
        HTMLAttributes: { class: styles.mention },
        renderText: ({ node }) => `@${String(node.attrs.label ?? node.attrs.id)}`,
        renderHTML: ({ node }) => ["span", { class: styles.mention, "data-id": String(node.attrs.id), "data-object": String(node.attrs.object ?? "") }, `@${String(node.attrs.label ?? node.attrs.id)}`],
        suggestion: {
          char: "@",
          items: ({ query }) => searchRecords(home.slug, query, 8),
          command: ({ editor: e, range, props }) => {
            const hit = props as unknown as Hit;
            e.chain().focus().insertContentAt(range, [{ type: "mention", attrs: { id: hit.record_id, label: hit.label, object: hit.object_plural } }, { type: "text", text: " " }]).run();
          },
          render: mentionList,
        },
      }),
    ],
    content: richContent(value),
    // The baseline: what is on the page now — a blur without a change saves nothing.
    onCreate: ({ editor: e }) => {
      saved.current = JSON.stringify(e.getJSON());
      live.current = e;
    },
    onUpdate: ({ editor: e }) => {
      if (timer.current) window.clearTimeout(timer.current);
      timer.current = window.setTimeout(() => flush(e), 600);
    },
    onBlur: ({ editor: e }) => flush(e),
    editorProps: {
      attributes: { class: styles.prose },
      handlePaste: (_view, event) => {
        const files = [...(event.clipboardData?.files ?? [])].filter((f) => f.type.startsWith("image/"));
        if (!files.length) return false;
        for (const file of files) {
          if (file.size > PASTE_LIMIT) {
            setTrouble("A pasted picture can be up to 20 MB — add larger ones from Files.");
            continue;
          }
          void pasteImage(file, home)
            .then((made) => live.current && attached(made, live.current))
            .catch((e: Error) => setTrouble(e.message));
        }
        return true;
      },
    },
  });
  // Another writer changed the body (an agent, a restore): follow it unless it is what we saved.
  useEffect(() => {
    if (!editor) return;
    const incoming = JSON.stringify(richContent(value));
    if (incoming === saved.current || editor.isFocused) return;
    saved.current = incoming;
    editor.commands.setContent(richContent(value), { emitUpdate: false });
  }, [editor, value]);
  useEffect(() => {
    editor?.setEditable(!readOnly);
  }, [editor, readOnly]);
  // Leaving the page mid-typing: what the debounce still holds is saved, not lost.
  useEffect(
    () => () => {
      if (timer.current && live.current) flush(live.current);
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );

  if (!editor) return <div className={`${styles.editor} ghostCard`} />;
  const tool = (label: string, active: boolean, run: () => void, text: string) => (
    <button type="button" className={styles.tool} aria-label={label} title={label} aria-pressed={active} disabled={readOnly} onMouseDown={(e) => e.preventDefault()} onClick={run}>
      {text}
    </button>
  );
  const setLink = () => {
    const url = (linkDraft ?? "").trim();
    setLinkDraft(null);
    if (!url) {
      editor.chain().focus().unsetLink().run();
      return;
    }
    editor.chain().focus().extendMarkRange("link").setLink({ href: /^[a-z]+:/i.test(url) ? url : `${SCHEME}//${url}` }).run();
  };
  const pickPictures = () =>
    pickFiles(true)
      .then(async (paths) => {
        for (const path of paths) attached(await addAttachment(home.slug, home.object, home.recordId, path), editor);
      })
      .catch((e: Error) => setTrouble(e.message));
  return (
    <div className={readOnly ? styles.editor : `${styles.editor} ${styles.editing}`}>
      {!readOnly && (
        <div className={styles.toolbar} role="toolbar" aria-label="Formatting">
          {tool("Bold", editor.isActive("bold"), () => editor.chain().focus().toggleBold().run(), "B")}
          {tool("Italic", editor.isActive("italic"), () => editor.chain().focus().toggleItalic().run(), "I")}
          {tool("Strikethrough", editor.isActive("strike"), () => editor.chain().focus().toggleStrike().run(), "S")}
          <span className={styles.gap} />
          {tool("Heading 1", editor.isActive("heading", { level: 1 }), () => editor.chain().focus().toggleHeading({ level: 1 }).run(), "H1")}
          {tool("Heading 2", editor.isActive("heading", { level: 2 }), () => editor.chain().focus().toggleHeading({ level: 2 }).run(), "H2")}
          {tool("Heading 3", editor.isActive("heading", { level: 3 }), () => editor.chain().focus().toggleHeading({ level: 3 }).run(), "H3")}
          <span className={styles.gap} />
          {tool("Bulleted list", editor.isActive("bulletList"), () => editor.chain().focus().toggleBulletList().run(), "•")}
          {tool("Numbered list", editor.isActive("orderedList"), () => editor.chain().focus().toggleOrderedList().run(), "1.")}
          {tool("To-do list", editor.isActive("taskList"), () => editor.chain().focus().toggleTaskList().run(), "☐")}
          {tool("Quote", editor.isActive("blockquote"), () => editor.chain().focus().toggleBlockquote().run(), "❝")}
          {tool("Code block", editor.isActive("codeBlock"), () => editor.chain().focus().toggleCodeBlock().run(), "<>")}
          <span className={styles.gap} />
          {tool("Table", editor.isActive("table"), () => editor.chain().focus().insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run(), "⊞")}
          {tool("Picture", false, () => void pickPictures(), "🖼")}
          {linkDraft === null ? (
            tool("Link", editor.isActive("link"), () => setLinkDraft(String(editor.getAttributes("link").href ?? "")), "🔗")
          ) : (
            <input
              className={`input ${styles.linkInput}`}
              autoFocus
              aria-label="Link URL"
              placeholder="www.example.com"
              value={linkDraft}
              onChange={(e) => setLinkDraft(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") setLink();
                if (e.key === "Escape") setLinkDraft(null);
              }}
              onBlur={setLink}
            />
          )}
        </div>
      )}
      <EditorContent editor={editor} className={styles.content} />
      {trouble && (
        <p className="formError" role="alert">
          {trouble}
        </p>
      )}
    </div>
  );
}

async function pasteImage(file: File, home: EditorHome): Promise<Attached | null> {
  const bytes = new Uint8Array(await file.arrayBuffer());
  const name = file.name && file.name !== "image.png" ? file.name : `Pasted ${new Date().toISOString().slice(0, 19).replace(/[T:]/g, "-")}.${file.type.split("/")[1] || "png"}`;
  return addAttachmentBytes(home.slug, home.object, home.recordId, name, bytes);
}

/** The `@` list: a glass menu under the caret, arrows and Enter pick, Escape closes. Plain DOM,
 *  as TipTap's suggestion drives it imperatively. */
function mentionList() {
  let root: HTMLDivElement | null = null;
  let items: Hit[] = [];
  let selected = 0;
  let command: ((hit: Hit) => void) | null = null;
  const paint = () => {
    if (!root) return;
    root.innerHTML = "";
    if (!items.length) {
      const empty = document.createElement("p");
      empty.className = "ghostText";
      empty.textContent = "No record";
      root.appendChild(empty);
      return;
    }
    items.forEach((hit, i) => {
      const button = document.createElement("button");
      button.type = "button";
      button.setAttribute("role", "menuitem");
      button.className = i === selected ? `menuItem ${styles.mentionItemSelected}` : "menuItem";
      button.textContent = `${hit.label || "Unnamed"} · ${hit.object_plural}`;
      button.addEventListener("mousedown", (e) => e.preventDefault());
      button.addEventListener("click", () => command?.(hit));
      root?.appendChild(button);
    });
  };
  const place = (clientRect: SuggestionProps["clientRect"]) => {
    const rect = clientRect?.();
    if (!root || !rect) return;
    root.style.left = `${rect.left}px`;
    root.style.top = `${rect.bottom + 4}px`;
  };
  return {
    onStart: (props: SuggestionProps) => {
      root = document.createElement("div");
      root.className = `glass menu ${styles.mentionMenu}`;
      root.setAttribute("role", "menu");
      document.body.appendChild(root);
      items = props.items as Hit[];
      selected = 0;
      command = (hit) => props.command(hit);
      paint();
      place(props.clientRect);
    },
    onUpdate: (props: SuggestionProps) => {
      items = props.items as Hit[];
      selected = Math.min(selected, Math.max(0, items.length - 1));
      command = (hit) => props.command(hit);
      paint();
      place(props.clientRect);
    },
    onKeyDown: (props: SuggestionKeyDownProps) => {
      if (props.event.key === "ArrowDown") {
        selected = items.length ? (selected + 1) % items.length : 0;
        paint();
        return true;
      }
      if (props.event.key === "ArrowUp") {
        selected = items.length ? (selected - 1 + items.length) % items.length : 0;
        paint();
        return true;
      }
      if (props.event.key === "Enter") {
        const hit = items[selected];
        if (hit) command?.(hit);
        return true;
      }
      if (props.event.key === "Escape") {
        root?.remove();
        root = null;
        return true;
      }
      return false;
    },
    onExit: () => {
      root?.remove();
      root = null;
    },
  };
}
