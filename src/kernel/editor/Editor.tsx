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
import { useEffect, useRef, useState, type MouseEvent as ReactMouseEvent } from "react";
import { addAttachment, addAttachmentBytes, pickFiles, type Attached } from "../bridge/files";
import { searchRecords, type Hit } from "../bridge/records";
import styles from "./Editor.module.css";
import { menuList } from "./menu";
import { BACK_COLORS, BackColor, CALLOUTS, Callout, LANGUAGES, PageLink, PageLinks, SlashMenu, TEXT_COLORS, TextColor, type SlashItem } from "./pageBlocks";
import { richContent } from "./plain";

/** Where a picture pasted or picked in the editor is kept: as an attachment of this record —
 *  or, when `files` is given, where those say (a work item keeps its own, D-074). */
export interface EditorHome {
  slug: string;
  /** The object's plural name (the record route) — also the attachment's link. */
  object: string;
  recordId: string;
  files?: { add: (path: string) => Promise<Attached | null>; addBytes: (name: string, bytes: Uint8Array) => Promise<Attached | null> };
}

/** A page's editor (D-080): its sub-pages — made from the `/` menu, named and opened. */
export interface PageExtras {
  newSubPage: () => Promise<{ id: string; title: string } | null>;
  title: (id: string) => string | undefined;
  open: (id: string) => void;
}

/** `@` mentions from elsewhere than the CRM's records (a page's: work items), and what a click
 *  on one opens. */
export interface MentionSource {
  search: (query: string) => Promise<Hit[]>;
  open: (object: string, id: string) => void;
}

interface Props {
  value: unknown;
  /** The document as JSON text, debounced and on blur; only when it changed. */
  onChange: (json: string) => void;
  placeholder?: string;
  readOnly?: boolean;
  home: EditorHome;
  /** The page blocks: headings 1–6, callouts, colours, a code block's language, the `/` menu,
   *  sub-pages. */
  page?: PageExtras;
  mentions?: MentionSource;
}

/** A link's scheme, kept apart from the host so the localhost-only check reads no URL here. */
const SCHEME = "https:";
/** A pasted picture travels as bytes over the bridge: kept small. */
const PASTE_LIMIT = 20 * 1024 * 1024;

/** The rich text editor (D-063, 0.10.6): TipTap with Twenty's blocks — headings, lists, to-dos,
 *  quotes, code, tables, images, links — and `@` mentions of records. The document is stored
 *  as JSON; `richPlain` reads its words. */
export default function Editor({ value, onChange, placeholder, readOnly, home, page, mentions }: Props) {
  const queryClient = useQueryClient();
  const saved = useRef<string | null>(null);
  const timer = useRef<number | null>(null);
  const live = useRef<TipTapEditor | null>(null);
  const [linkDraft, setLinkDraft] = useState<string | null>(null);
  const [trouble, setTrouble] = useState<string | null>(null);
  // The `/` menu's blocks, read when it opens (they close over the latest props).
  const slash = useRef<SlashItem[]>([]);
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
      StarterKit.configure({ heading: { levels: page ? [1, 2, 3, 4, 5, 6] : [1, 2, 3] }, link: { openOnClick: false, autolink: true } }),
      Placeholder.configure({ placeholder: placeholder ?? "Write…" }),
      Image.configure({ inline: false, allowBase64: false }),
      Table.configure({ resizable: false }),
      TableRow,
      TableHeader,
      TableCell,
      TaskList,
      TaskItem.configure({ nested: true }),
      // A mention keeps what it names (a record's object, or "pm-items"), so a click can open it.
      Mention.extend({
        addAttributes() {
          return { ...this.parent?.(), object: { default: null, parseHTML: (el: HTMLElement) => el.getAttribute("data-object"), renderHTML: () => ({}) } };
        },
      }).configure({
        HTMLAttributes: { class: styles.mention },
        renderText: ({ node }) => `@${String(node.attrs.label ?? node.attrs.id)}`,
        renderHTML: ({ node }) => ["span", { class: styles.mention, "data-id": String(node.attrs.id), "data-object": String(node.attrs.object ?? "") }, `@${String(node.attrs.label ?? node.attrs.id)}`],
        suggestion: {
          char: "@",
          items: ({ query }) => (mentions ? mentions.search(query) : searchRecords(home.slug, query, 8)),
          command: ({ editor: e, range, props }) => {
            const hit = props as unknown as Hit;
            e.chain().focus().insertContentAt(range, [{ type: "mention", attrs: { id: hit.record_id, label: hit.label, object: hit.object_plural } }, { type: "text", text: " " }]).run();
          },
          render: menuList<Hit>((hit) => (hit.object_plural === "pm-items" ? hit.label : `${hit.label || "Unnamed"} · ${hit.object_plural}`), mentions ? "No work item" : "No record"),
        },
      }),
      ...(page ? [Callout, TextColor, BackColor, PageLink, SlashMenu.configure({ items: () => slash.current })] : []),
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
    // In the editor's own key order and defaults (the app hands JSON with its keys sorted), so a
    // refetch of what was just saved is no change.
    let incoming: string;
    try {
      incoming = JSON.stringify(editor.schema.nodeFromJSON(richContent(value)).toJSON());
    } catch {
      incoming = JSON.stringify(richContent(value));
    }
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
        for (const path of paths) attached(await (home.files ? home.files.add : (path: string) => addAttachment(home.slug, home.object, home.recordId, path))(path), editor);
      })
      .catch((e: Error) => setTrouble(e.message));
  if (page) {
    const block = (label: string, run: SlashItem["run"]): SlashItem => ({ label, run });
    slash.current = [
      block("Text", (e, r) => e.chain().focus().deleteRange(r).setParagraph().run()),
      ...([1, 2, 3, 4, 5, 6] as const).map((level) => block(`Heading ${level}`, (e, r) => e.chain().focus().deleteRange(r).setHeading({ level }).run())),
      block("Bulleted list", (e, r) => e.chain().focus().deleteRange(r).toggleBulletList().run()),
      block("Numbered list", (e, r) => e.chain().focus().deleteRange(r).toggleOrderedList().run()),
      block("To-do list", (e, r) => e.chain().focus().deleteRange(r).toggleTaskList().run()),
      block("Quote", (e, r) => e.chain().focus().deleteRange(r).toggleBlockquote().run()),
      ...CALLOUTS.map((c) => block(`Callout · ${c.label}`, (e, r) => e.chain().focus().deleteRange(r).wrapIn("callout", { emoji: c.emoji, color: c.color }).run())),
      block("Divider", (e, r) => e.chain().focus().deleteRange(r).setHorizontalRule().run()),
      block("Table", (e, r) => e.chain().focus().deleteRange(r).insertTable({ rows: 3, cols: 3, withHeaderRow: true }).run()),
      block("Code", (e, r) => e.chain().focus().deleteRange(r).setCodeBlock().run()),
      block("Mermaid diagram (code)", (e, r) => e.chain().focus().deleteRange(r).setCodeBlock({ language: "mermaid" }).run()),
      block("Picture", (e, r) => {
        e.chain().focus().deleteRange(r).run();
        void pickPictures();
      }),
      block("Page (a sub-page)", (e, r) => {
        e.chain().focus().deleteRange(r).run();
        void page
          .newSubPage()
          .then((made) => made && e.chain().focus().insertContent({ type: "pageLink", attrs: { id: made.id, title: made.title } }).run())
          .catch((err: Error) => setTrouble(err.message));
      }),
      block("Mention a work item", (e, r) => e.chain().focus().deleteRange(r).insertContent("@").run()),
    ];
  }
  const inCallout = page && editor.isActive("callout");
  const inCode = page && editor.isActive("codeBlock");
  const pick = (label: string, value: string, options: [string, string][], apply: (v: string) => void) => (
    <select className={`input ${styles.pick}`} aria-label={label} title={label} value={value} disabled={readOnly} onMouseDown={(e) => e.stopPropagation()} onChange={(e) => apply(e.target.value)}>
      {options.map(([v, text]) => (
        <option key={v} value={v}>
          {text}
        </option>
      ))}
    </select>
  );
  const named = (c: string) => c.charAt(0).toUpperCase() + c.slice(1);
  // A click on a mention opens what it names (a page's work items).
  const openMention = (e: ReactMouseEvent) => {
    const hit = (e.target as HTMLElement).closest<HTMLElement>("[data-id][data-object]");
    // A page's mentions are work items (ones written before they kept their object too).
    if (mentions && hit?.dataset.id) mentions.open(hit.dataset.object || "pm-items", hit.dataset.id);
  };
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
          {page && (
            <>
              <span className={styles.gap} />
              {pick("Text colour", String(editor.getAttributes("textColor").color ?? ""), [["", "Text colour"], ...TEXT_COLORS.map((c): [string, string] => [c, named(c)])], (c) => (c ? editor.chain().focus().setMark("textColor", { color: c }).run() : editor.chain().focus().unsetMark("textColor").run()))}
              {pick("Background", String(editor.getAttributes("backColor").color ?? ""), [["", "Background"], ...BACK_COLORS.map((c): [string, string] => [c, named(c)])], (c) => (c ? editor.chain().focus().setMark("backColor", { color: c }).run() : editor.chain().focus().unsetMark("backColor").run()))}
              {inCallout &&
                pick(
                  "Callout",
                  CALLOUTS.find((c) => c.emoji === editor.getAttributes("callout").emoji)?.label ?? "Info",
                  CALLOUTS.map((c): [string, string] => [c.label, `${c.emoji} ${c.label}`]),
                  (label) => {
                    const c = CALLOUTS.find((x) => x.label === label);
                    if (c) editor.chain().focus().updateAttributes("callout", { emoji: c.emoji, color: c.color }).run();
                  },
                )}
              {inCode && pick("Language", String(editor.getAttributes("codeBlock").language ?? ""), LANGUAGES.map((l): [string, string] => [l, l ? named(l) : "Plain text"]), (language) => editor.chain().focus().updateAttributes("codeBlock", { language: language || null }).run())}
            </>
          )}
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
                if (e.key === "Escape") {
                  e.preventDefault();
                  setLinkDraft(null);
                }
              }}
              onBlur={setLink}
            />
          )}
        </div>
      )}
      <PageLinks.Provider value={page ? { title: page.title, open: page.open } : null}>
        {/* A click on a mention opens what it names. */}
        <EditorContent editor={editor} className={styles.content} onClick={openMention} />
      </PageLinks.Provider>
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
  return (home.files ? home.files.addBytes : (name: string, bytes: Uint8Array) => addAttachmentBytes(home.slug, home.object, home.recordId, name, bytes))(name, bytes);
}
