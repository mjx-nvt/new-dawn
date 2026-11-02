import { getRouteApi, useNavigate } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";
import { usePmLabels, type PmProject } from "../../../kernel/bridge/pm";
import { exportPageMarkdown, pageFiles, printWindow, searchItemMentions, usePageMutations, usePmPage, usePmPageVersions, usePmPages, type PmPage } from "../../../kernel/bridge/pmPages";
import { usePeople } from "../../../kernel/bridge/people";
import Editor from "../../../kernel/editor/Editor";
import { formatDateTime } from "../../../kernel/format/format";
import DangerButton from "../../../kernel/shell/DangerButton";
import MenuButton from "../../../kernel/shell/MenuButton";
import { inField } from "../../../kernel/shell/useShortcuts";
import { whoName } from "../pm";
import ItemPeek, { type PeekMode } from "./ItemPeek";
import { Dot, LabelPick } from "./Props";
import styles from "./Items.module.css";

const route = getRouteApi("/shell/projects/$projectId");
const TABS = [
  { key: "all", label: "All" },
  { key: "public", label: "Public" },
  { key: "private", label: "Private" },
  { key: "archived", label: "Archived" },
] as const;
const SORTS = [
  { key: "updated", label: "Last updated" },
  { key: "created", label: "Last created" },
  { key: "name", label: "Name" },
] as const;

const titleOf = (p: PmPage) => p.name || "Untitled";

/** A read-only page editor (a version's preview): the page blocks, no sub-page to make or open. */
const READ_ONLY_PAGE = { newSubPage: async () => null, title: () => undefined, open: () => undefined };

/** The headings of a document, in order, for the outline. */
function headings(doc: unknown): { level: number; text: string }[] {
  const out: { level: number; text: string }[] = [];
  const text = (v: unknown): string => {
    const n = v as { text?: string; content?: unknown[] };
    return (n.text ?? "") + (n.content ?? []).map(text).join("");
  };
  const walk = (v: unknown) => {
    const n = v as { type?: string; attrs?: { level?: number }; content?: unknown[] };
    if (n.type === "heading") out.push({ level: n.attrs?.level ?? 1, text: text(n) });
    else (n.content ?? []).forEach(walk);
  };
  if (doc) walk(doc);
  return out;
}

/** A page's versions (D-080): newest first, one read beside the list, Restore makes it the page. */
function VersionsSheet({ slug, page, readOnly, onClose }: { slug: string; page: PmPage; readOnly: boolean; onClose: () => void }) {
  const versions = usePmPageVersions(slug, page.id, true);
  const m = usePageMutations(slug);
  const people = usePeople(slug).data ?? [];
  const dialog = useRef<HTMLDialogElement>(null);
  const [picked, setPicked] = useState<string | null>(null);
  useEffect(() => {
    dialog.current?.showModal();
  }, []);
  const list = versions.data ?? [];
  const shown = list.find((v) => v.id === picked) ?? list[0];
  return (
    <dialog
      ref={dialog}
      className={`glass sheet ${styles.pageVersionsSheet}`}
      aria-labelledby="page-versions-title"
      onCancel={(e) => {
        e.preventDefault();
        onClose();
      }}
      onPointerDown={(e) => e.target === e.currentTarget && onClose()}
    >
      <div className="sheetBody">
        <h2 id="page-versions-title" className="sheetHeading">
          Versions of {titleOf(page)}
        </h2>
        <div className={styles.pageVersions}>
          <ul className={styles.pageVersionList} aria-label="Versions">
            {list.map((v) => (
              <li key={v.id}>
                <button type="button" className={styles.pageVersionRow} aria-current={shown?.id === v.id} onClick={() => setPicked(v.id)}>
                  <span>{formatDateTime(v.updated_at)}</span>
                  <span className="fieldHint">{whoName(v.created_by, people) || v.created_by}</span>
                </button>
              </li>
            ))}
            {!list.length &&
              [0, 1].map((n) => (
                <li key={n} className={`${styles.ghostRow} ghostCard`}>
                  {n === 0 && <span className="ghostText">{versions.isError ? "Couldn't be read" : versions.data ? "No version yet — one is kept as you write" : "Reading…"}</span>}
                </li>
              ))}
          </ul>
          <div className={styles.pageVersionPreview}>
            {shown ? (
              <>
                <h3 className={styles.h3}>{shown.name || "Untitled"}</h3>
                <Editor key={shown.id} value={shown.description ?? null} readOnly page={READ_ONLY_PAGE} home={{ slug, object: "pm-pages", recordId: page.id }} onChange={() => undefined} />
              </>
            ) : (
              <p className="ghostText">Not yet captured</p>
            )}
          </div>
        </div>
        {readOnly && <p className="fieldHint">The page takes no edit now (locked or archived) — a version comes back once it does.</p>}
        {m.restore.error && (
          <p className="formError" role="alert">
            {m.restore.error.message}
          </p>
        )}
        <div className="sheetActions">
          <button type="button" className="btn btnGhost" onClick={onClose}>
            Close
          </button>
          {/* The newest version is the page as it is. */}
          <button type="button" className="btn btnPrimary" disabled={!shown || shown.id === list[0]?.id || readOnly || m.restore.isPending} onClick={() => shown && m.restore.mutate(shown.id, { onSuccess: onClose })}>
            Restore this version
          </button>
        </div>
      </div>
    </dialog>
  );
}

/** One page open (D-080): its emoji and name, the editor (the `/` menu, `@` work items that open
 *  from the page), the outline, labels, its sub-pages; ★, Lock, Private, Versions, Full screen,
 *  Export (Markdown, PDF through the print sheet), Duplicate, Archive, Delete. */
function PageView({ slug, project, page: p, pages, full, onOpen, onPeek, onFull, onClose }: { slug: string; project: PmProject; page: PmPage; pages: PmPage[]; full: boolean; onOpen: (id: string) => void; onPeek: (id: string) => void; onFull: (on: boolean) => void; onClose: () => void }) {
  const doc = usePmPage(slug, p.id);
  const m = usePageMutations(slug);
  const people = usePeople(slug).data ?? [];
  const [name, setName] = useState(p.name);
  const [emoji, setEmoji] = useState(p.emoji);
  const [live, setLive] = useState<unknown>(null);
  const [versions, setVersions] = useState(false);
  const [note, setNote] = useState<string | null>(null);
  const body = useRef<HTMLDivElement>(null);
  useEffect(() => {
    setName(p.name);
    setEmoji(p.emoji);
  }, [p.id, p.name, p.emoji]);
  useEffect(() => {
    setLive(null);
  }, [p.id, doc.data?.description]);
  const archivedProject = !!project.archived_at;
  const ro = archivedProject || !!p.archived_at || p.is_locked;
  const error = m.update.error ?? m.archive.error ?? m.remove.error ?? m.favorite.error ?? m.duplicate.error ?? m.create.error;
  // Its sub-pages: archived with it, or open with it.
  const subs = pages.filter((x) => x.parent_id === p.id && !!x.archived_at === !!p.archived_at);
  const outline = headings(live ?? doc.data?.description);
  const saveName = () => name !== p.name && m.update.mutate({ id: p.id, patch: { name } });
  const saveEmoji = () => emoji !== p.emoji && m.update.mutate({ id: p.id, patch: { emoji } });
  const newSub = () => m.create.mutateAsync({ project_id: project.id, parent_id: p.id, name: "", emoji: "", is_private: p.is_private, labels: [] });
  const jump = (n: number) => body.current?.querySelectorAll(".ProseMirror h1, .ProseMirror h2, .ProseMirror h3, .ProseMirror h4, .ProseMirror h5, .ProseMirror h6")[n]?.scrollIntoView({ behavior: "smooth", block: "start" });

  useEffect(() => {
    if (!full) return;
    // Not an Escape that closed something else (a menu, the link field, a peek).
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && !e.defaultPrevented && !document.querySelector("dialog[open]") && !new URLSearchParams(location.search).get("peek") && onFull(false);
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [full, onFull]);

  return (
    <section className={`${styles.cycleView} ${full ? styles.fullScreen : ""}`} aria-label={titleOf(p)}>
      <div className={styles.toolbar}>
        <button type="button" className="btn btnGhost btnCompact" onClick={full ? () => onFull(false) : onClose}>
          {full ? "Exit full screen" : "‹ Pages"}
        </button>
        <button type="button" className={`${styles.star} ${p.is_favorite ? styles.starOn : ""}`} aria-pressed={p.is_favorite} aria-label={p.is_favorite ? `Unfavorite ${titleOf(p)}` : `Favorite ${titleOf(p)}`} disabled={archivedProject || !!p.archived_at || m.favorite.isPending} onClick={() => m.favorite.mutate({ id: p.id, on: !p.is_favorite })}>
          {p.is_favorite ? "★" : "☆"}
        </button>
        {p.is_private && <span className="chip">Private</span>}
        {p.is_locked && <span className="chip">Locked</span>}
        {p.archived_at && <span className="chip">Archived</span>}
        <span className={styles.spacer} />
        <button type="button" className="btn btnGhost btnCompact" onClick={() => setVersions(true)}>
          Versions
        </button>
        {!full && (
          <button type="button" className="btn btnGhost btnCompact" onClick={() => onFull(true)}>
            Full screen
          </button>
        )}
        <MenuButton
          label={`${titleOf(p)} actions`}
          items={[
            { label: p.is_locked ? "Unlock" : "Lock", disabled: archivedProject || !!p.archived_at, onSelect: () => m.update.mutate({ id: p.id, patch: { is_locked: !p.is_locked } }) },
            { label: p.is_private ? "Make public" : "Make private", disabled: archivedProject || !!p.archived_at, onSelect: () => m.update.mutate({ id: p.id, patch: { is_private: !p.is_private } }) },
            { label: "Duplicate", disabled: archivedProject || !!p.archived_at, onSelect: () => m.duplicate.mutate(p.id, { onSuccess: (copy) => onOpen(copy.id) }) },
            {
              label: "Export as Markdown",
              separator: true,
              onSelect: () =>
                void exportPageMarkdown(slug, p.id, titleOf(p)).then(
                  (path) => setNote(path ? `Saved as ${path}` : null),
                  (e: unknown) => setNote(`Couldn't export it: ${String(e)}`),
                ),
            },
            { label: "Export as PDF…", onSelect: () => void printWindow().catch((e: unknown) => setNote(`Couldn't print it: ${String(e)}`)) },
            { label: p.archived_at ? "Restore" : "Archive", separator: true, disabled: archivedProject, onSelect: () => m.archive.mutate({ id: p.id, archived: !p.archived_at }) },
          ]}
        />
        {p.archived_at && !archivedProject && (
          <DangerButton confirm={`Delete “${titleOf(p)}”${subs.length ? ` and its ${subs.length === 1 ? "sub-page" : `${subs.length} sub-pages`}` : ""}? This can't be undone.`} onConfirm={() => m.remove.mutateAsync(p.id).then(onClose, () => undefined)}>
            Delete
          </DangerButton>
        )}
      </div>
      {error && (
        <p className="formError" role="alert">
          {error.message}
        </p>
      )}
      {note && <p className="fieldHint">{note}</p>}
      {p.is_locked && <p className="fieldHint">Locked — unlock it from ⋯ to edit.</p>}
      <div className={full ? undefined : styles.cycleBody}>
        <div className={`${styles.viewMain} ${styles.printable}`} ref={body}>
          <div className={styles.pageTitle}>
            <input className={`input ${styles.pageEmoji}`} aria-label="Emoji" placeholder="📄" value={emoji} disabled={ro} onChange={(e) => setEmoji(e.target.value)} onBlur={saveEmoji} onKeyDown={(e) => e.key === "Enter" && e.currentTarget.blur()} />
            <input className={`input ${styles.pageName}`} aria-label="Page name" placeholder="Untitled" maxLength={255} value={name} disabled={ro} onChange={(e) => setName(e.target.value)} onBlur={saveName} onKeyDown={(e) => e.key === "Enter" && !e.nativeEvent.isComposing && e.currentTarget.blur()} />
          </div>
          {doc.data ? (
            <Editor
              key={`${p.id}-${ro}`}
              value={doc.data.description ?? null}
              readOnly={ro}
              placeholder="Write, or press / for blocks and @ for work items…"
              home={{ slug, object: "pm-pages", recordId: p.id, files: pageFiles(slug, p.id) }}
              page={{
                newSubPage: () => newSub().then((made) => ({ id: made.id, title: "" })),
                title: (id) => {
                  const sub = pages.find((x) => x.id === id);
                  return sub ? titleOf(sub) : undefined;
                },
                open: onOpen,
              }}
              mentions={{ search: (q) => searchItemMentions(slug, q), open: (object, id) => object === "pm-items" && onPeek(id) }}
              onChange={(json) => {
                setLive(JSON.parse(json));
                m.update.mutate({ id: p.id, patch: { description: json } });
              }}
            />
          ) : (
            <div className="ghostCard">{doc.isError ? <span className="ghostText">Couldn't be read</span> : null}</div>
          )}
        </div>
        {!full && (
          <aside className={styles.side} aria-label="About this page">
            <div className={styles.breakdown}>
              <h3 className={styles.h3}>Outline</h3>
              {outline.map((h, n) => (
                <button key={n} type="button" className={styles.outlineRow} style={{ paddingInlineStart: (h.level - 1) * 12 }} onClick={() => jump(n)}>
                  {h.text || "Untitled heading"}
                </button>
              ))}
              {!outline.length &&
                [0, 1].map((n) => (
                  <div key={n} className={`${styles.ghostRow} ghostCard`}>
                    {n === 0 && <span className="ghostText">Headings show here</span>}
                  </div>
                ))}
            </div>
            <div className={styles.breakdown}>
              <h3 className={styles.h3}>Labels</h3>
              <LabelPick slug={slug} projectId={project.id} value={p.labels} disabled={ro} onChange={(labels) => m.update.mutate({ id: p.id, patch: { labels } })} />
            </div>
            <div className={styles.breakdown}>
              <div className={styles.subsHead}>
                <h3 className={styles.h3}>Pages inside · {subs.length}</h3>
                <span className={styles.spacer} />
                {!archivedProject && !p.archived_at && (
                  <button type="button" className="btn btnGhost btnCompact" disabled={m.create.isPending} onClick={() => void newSub().then((made) => onOpen(made.id), () => undefined)}>
                    New sub-page
                  </button>
                )}
              </div>
              {subs.map((s) => (
                <button key={s.id} type="button" className={styles.subRow} onClick={() => onOpen(s.id)}>
                  {s.emoji || "📄"} {titleOf(s)}
                </button>
              ))}
              {!subs.length &&
                [0, 1].map((n) => (
                  <div key={n} className={`${styles.ghostRow} ghostCard`}>
                    {n === 0 && <span className="ghostText">No sub-pages yet</span>}
                  </div>
                ))}
            </div>
            <p className="fieldHint">
              Made by {whoName(p.created_by, people) || p.created_by} · {formatDateTime(p.created_at)}
              <br />
              Updated {formatDateTime(p.updated_at)}
            </p>
          </aside>
        )}
      </div>
      {versions && <VersionsSheet slug={slug} page={p} readOnly={ro} onClose={() => setVersions(false)} />}
    </section>
  );
}

/** A project's Pages tab (D-080, Plane's): All · Public · Private · Archived (in the URL),
 *  search, a sort, Favorites and a label as filters, ★ first; New page (`N` or `D`) opens it. */
export default function Pages({ slug, project }: { slug: string; project: PmProject }) {
  const search = route.useSearch();
  const navigate = useNavigate();
  const pages = usePmPages(slug, project.id);
  const labels = usePmLabels(slug, project.id).data ?? [];
  const m = usePageMutations(slug);
  const [words, setWords] = useState("");
  const set = (patch: Record<string, string | undefined>) => void navigate({ to: "/projects/$projectId", params: { projectId: project.id }, search: { ...search, ...patch }, replace: true });
  const tab = TABS.find((t) => t.key === search.ptab)?.key ?? "all";
  const sort = SORTS.find((s) => s.key === search.psort)?.key ?? "updated";
  const all = pages.data ?? [];
  const open = all.find((p) => p.id === search.page);
  const ro = !!project.archived_at;
  const make = () => m.create.mutate({ project_id: project.id, parent_id: null, name: "", emoji: "", is_private: tab === "private", labels: [] }, { onSuccess: (p) => set({ page: p.id }) });
  // Top-level pages, and sub-pages whose page is archived (archived ones) or gone.
  const top = (p: PmPage) => !p.parent_id || !all.some((x) => x.id === p.parent_id && !!x.archived_at === !!p.archived_at);
  const shown = all
    .filter(
      (p) =>
        top(p) &&
        (tab === "archived" ? !!p.archived_at : !p.archived_at && (tab === "all" || (tab === "private") === p.is_private)) &&
        (search.pfav !== "1" || p.is_favorite) &&
        (!search.plabel || p.labels.includes(search.plabel)) &&
        (!words.trim() || titleOf(p).toLowerCase().includes(words.trim().toLowerCase())),
    )
    .sort((a, b) => (sort === "name" ? titleOf(a).localeCompare(titleOf(b), undefined, { sensitivity: "base" }) : sort === "created" ? b.created_at.localeCompare(a.created_at) : b.updated_at.localeCompare(a.updated_at)))
    .sort((a, b) => Number(b.is_favorite) - Number(a.is_favorite));

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // Not over an open page or a peek (their own keys), nor while typing.
      if (e.defaultPrevented || e.isComposing || e.metaKey || e.ctrlKey || e.altKey || ro || open || search.peek || inField(e.target) || document.querySelector("dialog[open]")) return;
      if (e.key === "n" || e.key === "d") {
        e.preventDefault();
        make();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  const none = pages.isError ? "Couldn't be read" : pages.data ? (all.length ? "No page matches" : ro ? "No pages" : "No pages yet — press D to write one") : "Reading…";
  return (
    <section className="section" aria-label="Pages">
      {open ? (
        <PageView
          slug={slug}
          project={project}
          page={open}
          pages={all}
          full={search.pfull === "1"}
          onOpen={(id) => set({ page: id })}
          onPeek={(id) => set({ peek: id })}
          onFull={(on) => set({ pfull: on ? "1" : undefined })}
          onClose={() => set({ page: undefined, peek: undefined, pfull: undefined })}
        />
      ) : (
        <>
          <div className={styles.toolbar}>
            <div className="segmented" role="tablist" aria-label="Pages">
              {TABS.map((t) => (
                <button key={t.key} type="button" role="tab" aria-selected={tab === t.key} onClick={() => set({ ptab: t.key === "all" ? undefined : t.key })}>
                  {t.label}
                </button>
              ))}
            </div>
            <input className={`input ${styles.search}`} aria-label="Search pages" placeholder="Search" value={words} onChange={(e) => setWords(e.target.value)} />
            <select className="input" aria-label="Sort by" value={sort} onChange={(e) => set({ psort: e.target.value === "updated" ? undefined : e.target.value })}>
              {SORTS.map((s) => (
                <option key={s.key} value={s.key}>
                  {s.label}
                </option>
              ))}
            </select>
            <select className="input" aria-label="Label" value={search.plabel ?? ""} onChange={(e) => set({ plabel: e.target.value || undefined })}>
              <option value="">Any label</option>
              {labels.map((l) => (
                <option key={l.id} value={l.id}>
                  {l.name}
                </option>
              ))}
            </select>
            <label className={styles.check}>
              <input type="checkbox" checked={search.pfav === "1"} onChange={(e) => set({ pfav: e.target.checked ? "1" : undefined })} /> Favorites
            </label>
            <span className={styles.spacer} />
            <button type="button" className="btn btnPrimary" disabled={ro || m.create.isPending} onClick={make}>
              New page
            </button>
          </div>
          {(pages.error ?? m.create.error ?? m.favorite.error) && <p className="formError">{(pages.error ?? m.create.error ?? m.favorite.error)?.message}</p>}
          <div className={styles.group}>
            {shown.map((p) => (
              <div key={p.id} className={styles.row}>
                <button type="button" className={`${styles.star} ${p.is_favorite ? styles.starOn : ""}`} aria-pressed={p.is_favorite} aria-label={p.is_favorite ? `Unfavorite ${titleOf(p)}` : `Favorite ${titleOf(p)}`} disabled={ro || !!p.archived_at || m.favorite.isPending} onClick={() => m.favorite.mutate({ id: p.id, on: !p.is_favorite })}>
                  {p.is_favorite ? "★" : "☆"}
                </button>
                <button type="button" className={styles.rowTitle} onClick={() => set({ page: p.id })}>
                  {p.emoji || "📄"} {titleOf(p)}
                </button>
                <span className={styles.rowProps}>
                  {p.labels.map((id) => {
                    const l = labels.find((x) => x.id === id);
                    return l ? (
                      <span key={id} className={`chip ${styles.statusChip}`}>
                        <Dot color={l.color} /> {l.name}
                      </span>
                    ) : null;
                  })}
                  {p.is_private && <span className="chip">Private</span>}
                  {p.is_locked && <span className="chip">Locked</span>}
                  {p.sub_pages > 0 && <span className="fieldHint">{p.sub_pages} inside</span>}
                  <span className="fieldHint">Updated {formatDateTime(p.updated_at)}</span>
                </span>
              </div>
            ))}
            {!shown.length &&
              [0, 1].map((n) => (
                <div key={n} className={`${styles.row} ghostCard`}>
                  {n === 0 && <span className="ghostText">{none}</span>}
                </div>
              ))}
          </div>
        </>
      )}
      {search.peek && (
        <ItemPeek
          slug={slug}
          itemId={search.peek}
          mode={(["side", "modal", "full"] as PeekMode[]).find((x) => x === search.mode) ?? "side"}
          tab={search.activity}
          onTab={(activity) => set({ activity })}
          onMode={(x) => set({ mode: x === "side" ? undefined : x })}
          onOpen={(id) => set({ peek: id, activity: undefined })}
          onClose={() => set({ peek: undefined, mode: undefined, activity: undefined })}
        />
      )}
    </section>
  );
}
