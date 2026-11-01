import { useLocation, useNavigate } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";
import { ALL_MODULES, useModules } from "../bridge/modules";
import { useAllViews, useRecents } from "../bridge/navigation";
import { labelInput, useCreateRecord, useObject, useObjects, useSearchRecords } from "../bridge/records";
import { openSettings } from "../bridge/settings";
import { useCurrentWorkspace } from "../bridge/workspaces";
import styles from "./CommandMenu.module.css";
import { SHORTCUTS } from "./useShortcuts";

interface Entry {
  id: string;
  section: string;
  label: string;
  hint?: string;
  run: () => void;
}

interface Props {
  open: boolean;
  /** `search` starts in the records; `all` shows everything. */
  mode: "all" | "search";
  onClose: () => void;
  onToggleSidebar: () => void;
}

/** Twenty's command menu (D-064, 0.10.7), one for the whole app: recents, records that match,
 *  "Create a …" (then the name — two keystrokes to a new company), go to pages, objects and
 *  views, and the page's actions (edit the record page layout, deleted records, the sidebar,
 *  copy the link, the keyboard shortcuts). Arrows move, ↩ runs, ⌘↩ runs the first, Esc closes. */
export default function CommandMenu({ open, mode, onClose, onToggleSidebar }: Props) {
  const dialog = useRef<HTMLDialogElement>(null);
  const navigate = useNavigate();
  const location = useLocation();
  const { data: workspace } = useCurrentWorkspace();
  const slug = workspace?.slug;
  const [q, setQ] = useState("");
  const [selected, setSelected] = useState(0);
  const [creating, setCreating] = useState<string | null>(null); // an object's singular name
  const [keys, setKeys] = useState(false);
  const [trouble, setTrouble] = useState<string | null>(null);
  const objects = useObjects(slug);
  const views = useAllViews(slug, open);
  const recents = useRecents(slug, open && !q);
  const hits = useSearchRecords(slug, open ? q : "");
  const modules = useModules(slug).data ?? ALL_MODULES;
  const createDef = useObject(slug, creating ?? undefined);
  const create = useCreateRecord(slug, creating ?? undefined);

  useEffect(() => {
    if (open) {
      dialog.current?.showModal();
      setQ("");
      setSelected(0);
      setCreating(null);
      setKeys(false);
      setTrouble(null);
    } else dialog.current?.close();
  }, [open]);

  const recordObjects = (objects.data ?? []).filter((o) => !o.is_system && o.is_active);
  const path = location.pathname;
  const onRecord = /^\/crm\/[^/]+\/[^/]+$/.test(path);
  const crmTab = path === "/crm" ? (location.search as { tab?: string }).tab : undefined;
  const go = (fn: () => void) => () => {
    onClose();
    fn();
  };
  // A few dozen entries, built on every render: no memo, so no stale closure over the callers.
  const entries = ((): Entry[] => {
    const needle = q.trim().toLowerCase();
    const fits = (label: string) => !needle || label.toLowerCase().includes(needle);
    const out: Entry[] = [];
    if (!needle) for (const r of recents.data ?? []) out.push({ id: `recent:${r.record_id}`, section: "Recent", label: r.label, hint: r.object_label, run: go(() => void navigate({ to: "/crm/$object/$recordId", params: { object: r.object, recordId: r.record_id } })) });
    // "Create a company…" ranks above the records when what's typed starts the object's name
    // (⌘K, "co", ↩, the name, ↩ — Twenty's two keystrokes to a new company); else after them.
    const creates: Entry[] = [];
    if (modules.crm) for (const o of recordObjects) {
      const noun = o.label_singular.toLowerCase();
      const label = `Create ${/^[aeiou]/.test(noun) ? "an" : "a"} ${noun}…`;
      if (fits(label) || fits(o.label_singular)) creates.push({ id: `create:${o.id}`, section: "Create", label, run: () => setCreating(o.name_singular) });
    }
    const quick = !!needle && recordObjects.some((o) => o.label_singular.toLowerCase().startsWith(needle) || o.label_plural.toLowerCase().startsWith(needle));
    if (quick) out.push(...creates);
    if (needle) for (const h of hits.data ?? []) out.push({ id: `hit:${h.record_id}`, section: "Records", label: h.label || "Unnamed", hint: h.object_plural, run: go(() => void navigate({ to: "/crm/$object/$recordId", params: { object: h.object_plural, recordId: h.record_id } })) });
    if (!quick) out.push(...creates);
    const pages: [string, () => void][] = [
      ["Today", () => void navigate({ to: "/today" })],
      ["Meetings", () => void navigate({ to: "/meetings", search: {} })],
      ["Live", () => void navigate({ to: "/live", search: {} })],
      ["Tasks", () => void navigate({ to: "/tasks" })],
      ...(modules.projects ? [["Projects", () => void navigate({ to: "/projects" })] as [string, () => void]] : []),
      ["Email", () => void navigate({ to: "/email" })],
      ["Workspace", () => void navigate({ to: "/workspace", search: { tab: "overview" } })],
      ["Settings", () => void openSettings("general")],
    ];
    for (const [label, fn] of pages) if (fits(label)) out.push({ id: `page:${label}`, section: "Go to", label, run: go(fn) });
    if (modules.crm) {
      for (const o of recordObjects) if (fits(o.label_plural)) out.push({ id: `object:${o.id}`, section: "Go to", label: o.label_plural, hint: "CRM", run: go(() => void navigate({ to: "/crm", search: { tab: o.name_plural } })) });
      for (const v of views.data ?? []) if (needle && (fits(v.name) || fits(v.object_label))) out.push({ id: `view:${v.id}`, section: "Views", label: v.name, hint: v.object_label, run: go(() => void navigate({ to: "/crm", search: { tab: v.object, view: v.id } })) });
    }
    if (onRecord && fits("Edit record page layout")) out.push({ id: "action:layout", section: "Actions", label: "Edit record page layout", run: go(() => window.dispatchEvent(new CustomEvent("newdawn:edit-layout"))) });
    if (crmTab && crmTab !== "commitments" && fits("Deleted records")) out.push({ id: "action:deleted", section: "Actions", label: "Deleted records", hint: crmTab, run: go(() => window.dispatchEvent(new CustomEvent("newdawn:show-deleted"))) });
    // Import / export (D-065): the sheet and the export are the CRM page's, reached by URL state.
    if (crmTab && crmTab !== "commitments" && fits("Export view")) out.push({ id: "action:export", section: "Actions", label: "Export view", hint: "CSV", run: go(() => void navigate({ to: "/crm", search: { tab: crmTab, export: true } })) });
    if (modules.crm) for (const o of recordObjects) {
      const label = `Import ${o.label_plural.toLowerCase()} from a file…`;
      if (fits(label) || fits("Import records")) out.push({ id: `import:${o.id}`, section: "Actions", label, run: go(() => void navigate({ to: "/crm", search: { tab: o.name_plural, import: true } })) });
    }
    if (fits("Collapse sidebar") || fits("Expand sidebar")) out.push({ id: "action:sidebar", section: "Actions", label: "Collapse or expand the sidebar", hint: "⌃⌘S", run: go(onToggleSidebar) });
    if (fits("Copy link to page")) out.push({ id: "action:link", section: "Actions", label: "Copy link to page", run: go(() => void navigator.clipboard.writeText(window.location.href).catch(() => undefined)) });
    if (fits("Keyboard shortcuts")) out.push({ id: "action:keys", section: "Actions", label: "Keyboard shortcuts", run: () => setKeys(true) });
    return out;
  })();

  useEffect(() => setSelected(0), [q]);
  const run = (entry: Entry | undefined) => entry?.run();
  const submitCreate = () => {
    const def = createDef.data;
    const name = q.trim();
    if (!def || !name) return;
    setTrouble(null);
    create.mutate(labelInput(def, name), {
      onSuccess: (made) => {
        const id = made && typeof made === "object" && typeof (made as { id?: unknown }).id === "string" ? (made as { id: string }).id : "";
        onClose();
        if (id) void navigate({ to: "/crm/$object/$recordId", params: { object: def.meta.name_plural, recordId: id } });
      },
      onError: (e: Error) => setTrouble(e.message),
    });
  };
  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.nativeEvent.isComposing) return;
    if (e.key === "Escape") {
      e.preventDefault();
      if (creating || keys) {
        setCreating(null);
        setKeys(false);
        setQ("");
      } else onClose();
      return;
    }
    if (creating) {
      if (e.key === "Enter") {
        e.preventDefault();
        submitCreate();
      }
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelected((s) => Math.min(s + 1, Math.max(0, entries.length - 1)));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelected((s) => Math.max(0, s - 1));
    } else if (e.key === "Enter") {
      e.preventDefault();
      run(e.metaKey ? entries[0] : entries[selected]);
    }
  };
  const sections = [...new Set(entries.map((e) => e.section))];
  const picked = creating ? recordObjects.find((o) => o.name_singular === creating) : undefined;

  return (
    <dialog ref={dialog} className={`glass sheet ${styles.menu}`} aria-label="Command menu" onClose={onClose} onCancel={(e) => { e.preventDefault(); onClose(); }} onKeyDown={onKeyDown}>
      <div className={styles.inputRow}>
        {picked && <span className="chip chipPrimary">New {picked.label_singular.toLowerCase()}</span>}
        <input
          className={styles.input}
          autoFocus
          aria-label={picked ? `Name of the new ${picked.label_singular.toLowerCase()}` : "Search or type a command"}
          placeholder={picked ? `${picked.label_singular} name, then ↩` : mode === "search" ? "Search records…" : "Search, create, go to…"}
          value={q}
          onChange={(e) => setQ(e.target.value)}
        />
        {!creating && <kbd className={styles.kbd}>esc</kbd>}
      </div>
      {trouble && (
        <p className="formError" role="alert">
          {trouble}
        </p>
      )}
      {keys ? (
        <div className={styles.keys}>
          <p className={styles.section}>Keyboard shortcuts</p>
          {SHORTCUTS.map((s) => (
            <div key={s.keys} className={styles.keyRow}>
              <span className={styles.keyWhat}>{s.what}</span>
              <kbd className={styles.kbd}>{s.keys}</kbd>
            </div>
          ))}
        </div>
      ) : creating ? (
        <p className="fieldHint">Type the name and press ↩ — the record opens on its page. Esc goes back.</p>
      ) : (
        <div className={styles.list} role="listbox" aria-label="Results">
          {entries.length ? (
            sections.map((section) => (
              <div key={section}>
                <p className={styles.section}>{section}</p>
                {entries
                  .filter((e) => e.section === section)
                  .map((e) => {
                    const i = entries.indexOf(e);
                    return (
                      <button key={e.id} type="button" role="option" aria-selected={i === selected} className={i === selected ? `${styles.row} ${styles.rowSelected}` : styles.row} onMouseEnter={() => setSelected(i)} onClick={e.run}>
                        <span className={styles.rowLabel}>{e.label}</span>
                        {e.hint && <span className={styles.rowHint}>{e.hint}</span>}
                      </button>
                    );
                  })}
              </div>
            ))
          ) : (
            <p className="ghostText">{hits.isFetching ? "Searching…" : "Nothing matches"}</p>
          )}
        </div>
      )}
    </dialog>
  );
}
