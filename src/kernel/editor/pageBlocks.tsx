import { Extension, Mark, Node, NodeViewWrapper, ReactNodeViewRenderer, type Editor, type NodeViewProps, type Range } from "@tiptap/react";
import { PluginKey } from "@tiptap/pm/state";
import { Suggestion } from "@tiptap/suggestion";
import { createContext, useContext } from "react";
import { menuList } from "./menu";
import styles from "./Editor.module.css";

/** A page's blocks beyond the shared editor's (D-080, Plane's): callouts, text and background
 *  colours, sub-page links and the `/` menu. Colours are names the stylesheet turns into tokens. */

export const TEXT_COLORS = ["gray", "red", "orange", "yellow", "green", "blue", "purple"] as const;
export const BACK_COLORS = ["gray", "red", "orange", "yellow", "green"] as const;

export const CALLOUTS = [
  { label: "Info", emoji: "💡", color: "blue" },
  { label: "Note", emoji: "📝", color: "gray" },
  { label: "Warning", emoji: "⚠️", color: "yellow" },
  { label: "Success", emoji: "✅", color: "green" },
  { label: "Danger", emoji: "🛑", color: "red" },
] as const;

/** A callout: an emoji beside its blocks, on a colour. */
export const Callout = Node.create({
  name: "callout",
  group: "block",
  content: "block+",
  defining: true,
  addAttributes() {
    return {
      emoji: { default: "💡", rendered: false, parseHTML: (el: HTMLElement) => el.getAttribute("data-emoji") ?? "💡" },
      color: { default: "blue", rendered: false, parseHTML: (el: HTMLElement) => el.getAttribute("data-color") ?? "blue" },
    };
  },
  parseHTML() {
    return [{ tag: "div[data-callout]" }];
  },
  renderHTML({ node }) {
    return ["div", { "data-callout": "", "data-emoji": String(node.attrs.emoji), "data-color": String(node.attrs.color), class: styles.callout }, ["span", { class: styles.calloutEmoji, contenteditable: "false" }, String(node.attrs.emoji)], ["div", { class: styles.calloutBody }, 0]];
  },
});

function colorMark(name: "textColor" | "backColor", attribute: string, className: string) {
  return Mark.create({
    name,
    addAttributes() {
      return { color: { default: "gray", rendered: false, parseHTML: (el: HTMLElement) => el.getAttribute(attribute) } };
    },
    parseHTML() {
      return [{ tag: `span[${attribute}]` }];
    },
    renderHTML({ mark }) {
      return ["span", { [attribute]: String(mark.attrs.color), class: className }, 0];
    },
  });
}

export const TextColor = colorMark("textColor", "data-text-color", styles.textColor);
export const BackColor = colorMark("backColor", "data-back-color", styles.backColor);

/** What a page editor knows of its pages: a sub-page's name now, and opening one. */
export const PageLinks = createContext<{ title: (id: string) => string | undefined; open: (id: string) => void } | null>(null);

function PageLinkView({ node }: NodeViewProps) {
  const links = useContext(PageLinks);
  const id = String(node.attrs.id ?? "");
  const title = links?.title(id) || String(node.attrs.title ?? "") || "Untitled";
  return (
    <NodeViewWrapper className={styles.pageLink} data-page-link={id}>
      <button type="button" contentEditable={false} className={styles.pageLinkButton} onClick={() => links?.open(id)}>
        <span aria-hidden>📄</span> {title}
      </button>
    </NodeViewWrapper>
  );
}

/** A sub-page, as a block that opens it (its name as it is now). */
export const PageLink = Node.create({
  name: "pageLink",
  group: "block",
  atom: true,
  selectable: true,
  draggable: true,
  addAttributes() {
    return {
      id: { default: "", rendered: false, parseHTML: (el: HTMLElement) => el.getAttribute("data-page-link") ?? "" },
      title: { default: "", rendered: false, parseHTML: (el: HTMLElement) => el.getAttribute("data-title") ?? "" },
    };
  },
  parseHTML() {
    return [{ tag: "div[data-page-link]" }];
  },
  renderHTML({ node }) {
    return ["div", { "data-page-link": String(node.attrs.id), "data-title": String(node.attrs.title) }];
  },
  addNodeView() {
    return ReactNodeViewRenderer(PageLinkView);
  },
});

/** A block the `/` menu makes, in place of the `/` and what was typed after it. */
export interface SlashItem {
  label: string;
  run: (editor: Editor, range: Range) => void;
}

export const SlashMenu = Extension.create<{ items: () => SlashItem[] }>({
  name: "slashMenu",
  addOptions() {
    return { items: () => [] };
  },
  addProseMirrorPlugins() {
    return [
      Suggestion<SlashItem>({
        editor: this.editor,
        char: "/",
        pluginKey: new PluginKey("slashMenu"),
        // Not inside code, where a slash is code.
        allow: ({ state, range }) => !state.doc.resolve(range.from).parent.type.spec.code,
        items: ({ query }) =>
          this.options
            .items()
            .filter((i) => i.label.toLowerCase().includes(query.toLowerCase()))
            .slice(0, 24),
        command: ({ editor, range, props }) => props.run(editor, range),
        render: menuList<SlashItem>((i) => i.label, "No block by that name"),
      }),
    ];
  },
});

/** Plane's code block languages (Mermaid diagrams stay code: no diagram library here). */
export const LANGUAGES = ["", "bash", "css", "html", "javascript", "json", "markdown", "mermaid", "python", "rust", "sql", "typescript", "yaml"] as const;
