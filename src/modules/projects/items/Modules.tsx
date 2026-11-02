import { getRouteApi, useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import type { PmProject } from "../../../kernel/bridge/pm";
import { usePeople } from "../../../kernel/bridge/people";
import { openLink, useUpdatePmItem, type ItemInput } from "../../../kernel/bridge/pmItems";
import { useModuleMutations, usePmModuleDetails, usePmModules, type ModuleStatus, type PmModule } from "../../../kernel/bridge/pmModules";
import { usePmQuery } from "../../../kernel/bridge/pmViews";
import { colorOf } from "../../../kernel/charts/Charts";
import { formatDay } from "../../../kernel/format/format";
import DangerButton from "../../../kernel/shell/DangerButton";
import MenuButton from "../../../kernel/shell/MenuButton";
import { inField } from "../../../kernel/shell/useShortcuts";
import { whoName } from "../pm";
import CreateItem from "./CreateItem";
import Gantt, { frame, type GanttRow, type Zoom } from "./Gantt";
import ItemPeek, { type PeekMode } from "./ItemPeek";
import ModuleSheet from "./ModuleSheet";
import { Dot } from "./Props";
import { GROUP_LABEL, MODULE_STATUS, PRIORITIES } from "./items";
import styles from "./Items.module.css";

const route = getRouteApi("/shell/projects/$projectId");
const LAYOUTS = [
  { key: "list", label: "List" },
  { key: "gallery", label: "Gallery" },
  { key: "timeline", label: "Timeline" },
] as const;
type ModuleLayout = (typeof LAYOUTS)[number]["key"];
const SORTS = [
  { key: "name", label: "Name" },
  { key: "progress", label: "Progress" },
  { key: "start", label: "Start date" },
  { key: "target", label: "Target date" },
  { key: "newest", label: "Newest" },
] as const;

const statusOf = (s: ModuleStatus) => MODULE_STATUS.find((x) => x.key === s) ?? MODULE_STATUS[0];
/** Done of all but the cancelled. */
const share = (m: PmModule) => (m.items - m.cancelled > 0 ? m.done / (m.items - m.cancelled) : 0);
const dates = (m: PmModule) =>
  m.start_date || m.target_date ? `${m.start_date ? formatDay(m.start_date, { short: true }) : "…"} – ${m.target_date ? formatDay(m.target_date, { short: true }) : "…"}` : "No dates";

function sorted(modules: PmModule[], by: string): PmModule[] {
  const list = [...modules];
  // Days as YYYY-MM-DD compare as text; no date last.
  const day = (d: string | null) => d ?? "9999";
  switch (by) {
    case "progress":
      return list.sort((a, b) => share(b) - share(a));
    case "start":
      return list.sort((a, b) => day(a.start_date).localeCompare(day(b.start_date)));
    case "target":
      return list.sort((a, b) => day(a.target_date).localeCompare(day(b.target_date)));
    case "newest":
      return list.sort((a, b) => b.created_at.localeCompare(a.created_at));
    default:
      return list.sort((a, b) => a.name.localeCompare(b.name, undefined, { sensitivity: "base" }));
  }
}

function Status({ status }: { status: ModuleStatus }) {
  const s = statusOf(status);
  return (
    <span className={`chip ${styles.statusChip}`}>
      <Dot color={s.color} /> {s.label}
    </span>
  );
}

/** A module's progress bar and words. */
function Progress({ module: m }: { module: PmModule }) {
  const p = Math.round(share(m) * 100);
  return (
    <span className={styles.progressWrap} aria-label={`${p} % done`}>
      <span className={styles.progress}>
        <span style={{ width: `${p}%` }} />
      </span>
      <span className="fieldHint">
        {m.done} of {m.items - m.cancelled} done · {p} %
      </span>
    </span>
  );
}

function Star({ module: m, disabled, onToggle }: { module: PmModule; disabled: boolean; onToggle: () => void }) {
  return (
    <button type="button" className={`${styles.star} ${m.is_favorite ? styles.starOn : ""}`} aria-pressed={m.is_favorite} aria-label={m.is_favorite ? `Unfavorite ${m.name}` : `Favorite ${m.name}`} disabled={disabled} onClick={onToggle}>
      {m.is_favorite ? "★" : "☆"}
    </button>
  );
}

/** One module open (D-079): its status, dates, lead and members, the actions, its progress, its
 *  work items (added from the project's, made in it, taken out) and the details — Needs
 *  attention (open items overdue or blocked), the work by state group, breakdowns, links. */
function ModuleView({ slug, project, module: m, onPeek, onClose }: { slug: string; project: PmProject; module: PmModule; onPeek: (id: string) => void; onClose: () => void }) {
  const mutations = useModuleMutations(slug);
  const update = useUpdatePmItem(slug);
  const details = usePmModuleDetails(slug, m.id);
  const people = usePeople(slug).data ?? [];
  const inIt = usePmQuery(slug, project.id, [{ field: "module", op: "in", values: [m.id] }]);
  const [adding, setAdding] = useState(false);
  const [words, setWords] = useState("");
  const others = usePmQuery(slug, project.id, [{ field: "module", op: "not_in", values: [m.id] }], adding);
  const [picked, setPicked] = useState<string[]>([]);
  const [editing, setEditing] = useState(false);
  const [making, setMaking] = useState<Partial<ItemInput> | null>(null);
  const [link, setLink] = useState({ url: "", title: "" });
  const [failed, setFailed] = useState<string | null>(null);
  const ro = !!project.archived_at || !!m.archived_at;
  const error = mutations.update.error ?? mutations.favorite.error ?? mutations.archive.error ?? mutations.remove.error ?? mutations.addItems.error ?? mutations.addLink.error ?? mutations.removeLink.error ?? update.error ?? (failed ? new Error(failed) : null);
  const list = inIt.data ?? [];
  const candidates = (others.data ?? []).filter((i) => !i.archived_at && (!words.trim() || `${i.key} ${i.name}`.toLowerCase().includes(words.trim().toLowerCase())));
  const d = details.data;
  const who = (w: string) => whoName(w, people);
  return (
    <section className={styles.cycleView} aria-label={m.name}>
      <div className={styles.toolbar}>
        <button type="button" className="btn btnGhost btnCompact" onClick={onClose}>
          ‹ Modules
        </button>
        <Star module={m} disabled={!!project.archived_at || !!m.archived_at || mutations.favorite.isPending} onToggle={() => mutations.favorite.mutate({ id: m.id, on: !m.is_favorite })} />
        <h2 className="sectionTitle">{m.name}</h2>
        <select className="input" aria-label="Status" value={m.status} disabled={ro || mutations.update.isPending} onChange={(e) => mutations.update.mutate({ id: m.id, patch: { status: MODULE_STATUS.find((s) => s.key === e.target.value)?.key ?? m.status } })}>
          {MODULE_STATUS.map((s) => (
            <option key={s.key} value={s.key}>
              {s.label}
            </option>
          ))}
        </select>
        <span className="fieldHint">{dates(m)}</span>
        {m.archived_at && <span className="chip">Archived</span>}
        <span className={styles.spacer} />
        <MenuButton
          label={`${m.name} actions`}
          items={[
            { label: "Edit", disabled: ro, onSelect: () => setEditing(true) },
            {
              label: m.archived_at ? "Restore" : "Archive",
              disabled: !!project.archived_at || (!m.archived_at && m.status !== "completed" && m.status !== "cancelled"),
              onSelect: () => mutations.archive.mutate({ id: m.id, archived: !m.archived_at }),
            },
          ]}
        />
        {!project.archived_at && (
          <DangerButton confirm={`Delete “${m.name}”? Its work items stay, in their other modules.`} onConfirm={() => mutations.remove.mutateAsync(m.id).then(onClose, () => undefined)}>
            Delete
          </DangerButton>
        )}
      </div>
      {error && (
        <p className="formError" role="alert">
          {error.message}
        </p>
      )}
      <p className={m.description ? "fieldHint" : "ghostText"}>{m.description || "Not yet captured"}</p>
      <p className="fieldHint">
        Lead: {m.lead ? who(m.lead) : <span className="ghostText">No one</span>} · Members: {m.members.length ? m.members.map(who).join(", ") : <span className="ghostText">None yet</span>}
      </p>
      <Progress module={m} />
      <div className={styles.cycleBody}>
        <div className={styles.viewMain}>
          <div className={styles.subsHead}>
            <h3 className={styles.h3}>Work items · {list.length}</h3>
            <span className={styles.spacer} />
            {!ro && (
              <>
                <button type="button" className="btn btnGhost btnCompact" aria-expanded={adding} onClick={() => setAdding(!adding)}>
                  Add existing
                </button>
                <button type="button" className="btn btnSecondary btnCompact" onClick={() => setMaking({ modules: [m.id] })}>
                  New work item
                </button>
              </>
            )}
          </div>
          {adding && (
            <div className={`glass ${styles.finder}`}>
              <input className="input" autoFocus aria-label="Find work items to add" placeholder="A key or words of a title" value={words} onChange={(e) => setWords(e.target.value)} />
              <ul className={styles.hits}>
                {candidates.slice(0, 50).map((i) => (
                  <li key={i.id}>
                    <label className={styles.check}>
                      <input type="checkbox" checked={picked.includes(i.id)} onChange={(e) => setPicked(e.target.checked ? [...picked, i.id] : picked.filter((x) => x !== i.id))} /> <span className={styles.key}>{i.key}</span> {i.name}
                      {i.modules.length > 0 && <span className="fieldHint"> · in {i.modules.map((x) => x.name).join(", ")}</span>}
                    </label>
                  </li>
                ))}
                {!candidates.length && <li className="ghostText">{others.data ? "No other work items" : "Reading…"}</li>}
              </ul>
              <button
                type="button"
                className="btn btnSecondary btnCompact"
                disabled={!picked.length || mutations.addItems.isPending}
                onClick={() =>
                  mutations.addItems.mutate(
                    { id: m.id, itemIds: picked },
                    {
                      onSuccess: () => {
                        setPicked([]);
                        setAdding(false);
                      },
                    },
                  )
                }
              >
                Add {picked.length || ""}
              </button>
            </div>
          )}
          {list.map((i) => (
            <div key={i.id} className={styles.tieRow}>
              <span className={styles.key}>{i.key}</span>
              <button type="button" className={styles.rowTitle} onClick={() => onPeek(i.id)}>
                {i.name}
              </button>
              <span className={styles.prop}>
                <Dot color={i.state_color} /> {i.state_name}
              </span>
              {!ro && !i.archived_at && (
                <button type="button" className="btn btnGhost btnCompact" aria-label={`Take ${i.key} out of ${m.name}`} disabled={update.isPending} onClick={() => update.mutate({ id: i.id, patch: { modules: i.modules.map((x) => x.id).filter((x) => x !== m.id) } })}>
                  ×
                </button>
              )}
            </div>
          ))}
          {!list.length &&
            [0, 1].map((n) => (
              <div key={n} className={`${styles.ghostRow} ghostCard`}>
                {n === 0 && <span className="ghostText">{inIt.data ? "No work items in this module yet" : "Reading…"}</span>}
              </div>
            ))}
        </div>
        <aside className={styles.side} aria-label="Details">
          <div className={styles.breakdown}>
            <h3 className={styles.h3}>Needs attention</h3>
            {(d?.attention ?? []).map((a) => (
              <div key={a.item.id} className={styles.tieRow}>
                <span className={styles.key}>{a.item.key}</span>
                <button type="button" className={styles.rowTitle} onClick={() => onPeek(a.item.id)}>
                  {a.item.name}
                </button>
                {a.overdue && <span className={`chip ${styles.overdue}`}>Overdue</span>}
                {a.blocked && <span className="chip">Blocked</span>}
              </div>
            ))}
            {!d?.attention.length &&
              [0, 1].map((n) => (
                <div key={n} className={`${styles.ghostRow} ghostCard`}>
                  {n === 0 && <span className="ghostText">{d ? "Nothing overdue or blocked" : details.isError ? "Couldn't be read" : "Reading…"}</span>}
                </div>
              ))}
          </div>
          {(
            [
              ["Progress", d?.groups.map((g) => ({ ...g, label: GROUP_LABEL[g.key as keyof typeof GROUP_LABEL] ?? g.label }))],
              ["Assignees", d?.assignees.map((s) => ({ ...s, label: s.key === "none" ? s.label : who(s.key) }))],
              ["Labels", d?.labels],
              ["Priorities", d?.priorities.map((s) => ({ ...s, label: s.key === "none" ? "No priority" : (PRIORITIES.find((p) => p.key === s.key)?.label ?? s.label) }))],
              ["States", d?.states],
            ] as const
          ).map(([title, shares]) => (
            <div key={title} className={styles.breakdown}>
              <h3 className={styles.h3}>{title}</h3>
              {(shares ?? []).map((s) => (
                <div key={s.key} className={styles.shareRow}>
                  <span>{s.label}</span>
                  <span className={styles.progress}>
                    <span style={{ width: `${s.total ? Math.round((s.done / s.total) * 100) : 0}%` }} />
                  </span>
                  <span className="fieldHint">
                    {title === "Progress" ? s.total : `${s.done}/${s.total}`}
                  </span>
                </div>
              ))}
              {!shares?.length &&
                [0, 1].map((n) => (
                  <div key={n} className={`${styles.ghostRow} ghostCard`}>
                    {n === 0 && <span className="ghostText">{d ? "Nothing yet" : "Reading…"}</span>}
                  </div>
                ))}
            </div>
          ))}
          <div className={styles.breakdown}>
            <h3 className={styles.h3}>Links</h3>
            {(d?.links ?? []).map((l) => (
              <div key={l.id} className={styles.tieRow}>
                <button type="button" className={styles.rowTitle} title={l.url} onClick={() => openLink(l.url).then(() => setFailed(null), (e: unknown) => setFailed(`Couldn't open it: ${String(e)}`))}>
                  {l.title || l.url}
                </button>
                {!ro && (
                  <button type="button" className="btn btnGhost btnCompact" aria-label={`Remove the link ${l.title || l.url}`} onClick={() => mutations.removeLink.mutate(l.id)}>
                    ×
                  </button>
                )}
              </div>
            ))}
            {!d?.links.length &&
              [0, 1].map((n) => (
                <div key={n} className={`${styles.ghostRow} ghostCard`}>
                  {n === 0 && <span className="ghostText">{d ? "No links yet" : "Reading…"}</span>}
                </div>
              ))}
            {!ro && (
              <form
                className={styles.inline}
                onSubmit={(e) => {
                  e.preventDefault();
                  if (link.url.trim()) mutations.addLink.mutate({ id: m.id, ...link }, { onSuccess: () => setLink({ url: "", title: "" }) });
                }}
              >
                <input className="input" aria-label="Link address" placeholder="example.com/spec" value={link.url} onChange={(e) => setLink({ ...link, url: e.target.value })} />
                <input className="input" aria-label="Link title" placeholder="Title" value={link.title} onChange={(e) => setLink({ ...link, title: e.target.value })} />
                <button type="submit" className="btn btnSecondary btnCompact" disabled={!link.url.trim() || mutations.addLink.isPending}>
                  Add link
                </button>
              </form>
            )}
          </div>
        </aside>
      </div>
      {editing && <ModuleSheet slug={slug} projectId={project.id} module={m} onClose={() => setEditing(false)} />}
      {making && <CreateItem slug={slug} projectId={project.id} preset={making} onClose={() => setMaking(null)} />}
    </section>
  );
}

/** A project's Modules tab (D-079, Plane's): List, Gallery and Timeline (the layout in the URL),
 *  search, a sort, filters (status, lead, member, favorites, dates, archived too) — favorites
 *  first; New module (`N` or `M`); the timeline drags a module's dates; a module opens with its
 *  work items and details. */
export default function Modules({ slug, project }: { slug: string; project: PmProject }) {
  const search = route.useSearch();
  const navigate = useNavigate();
  const modules = usePmModules(slug, project.id);
  const mutations = useModuleMutations(slug);
  const people = usePeople(slug).data ?? [];
  const [words, setWords] = useState("");
  const [making, setMaking] = useState(false);
  const [zoom, setZoom] = useState<Zoom>("month");
  const set = (patch: Record<string, string | undefined>) => void navigate({ to: "/projects/$projectId", params: { projectId: project.id }, search: { ...search, ...patch }, replace: true });
  const layout: ModuleLayout = LAYOUTS.find((l) => l.key === search.mlayout)?.key ?? "list";
  const sort = SORTS.find((s) => s.key === search.msort)?.key ?? "name";
  const all = modules.data ?? [];
  const open = all.find((m) => m.id === search.module);
  const ro = !!project.archived_at;
  const from = search.mfrom ?? "";
  const to = search.mto ?? "";
  const shown = sorted(
    all.filter(
      (m) =>
        (search.march === "1" || !m.archived_at) &&
        (!search.mstatus || m.status === search.mstatus) &&
        (!search.mlead || m.lead === search.mlead) &&
        (!search.mmember || m.members.includes(search.mmember)) &&
        (search.mfav !== "1" || m.is_favorite) &&
        (!from || !m.target_date || m.target_date >= from) &&
        (!to || !m.start_date || m.start_date <= to) &&
        (!words.trim() || m.name.toLowerCase().includes(words.trim().toLowerCase())),
    ),
    sort,
  ).sort((a, b) => Number(b.is_favorite) - Number(a.is_favorite));
  const leads = [...new Set(all.flatMap((m) => (m.lead ? [m.lead] : [])))];
  const members = [...new Set(all.flatMap((m) => m.members))];
  const star = (m: PmModule) => <Star module={m} disabled={ro || !!m.archived_at || mutations.favorite.isPending} onToggle={() => mutations.favorite.mutate({ id: m.id, on: !m.is_favorite })} />;
  const openIt = (m: PmModule) => set({ module: m.id });

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // Not over an open module or a peek (their own keys), nor while typing.
      if (e.defaultPrevented || e.isComposing || e.metaKey || e.ctrlKey || e.altKey || making || ro || open || search.peek || inField(e.target) || document.querySelector("dialog[open]")) return;
      if (e.key === "n" || e.key === "m") {
        e.preventDefault();
        setMaking(true);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [making, ro, open, search.peek]);

  const rows: (GanttRow & { module: PmModule })[] = shown.map((m) => ({
    id: m.id,
    start: m.start_date,
    end: m.target_date,
    color: colorOf(0, statusOf(m.status).color),
    name: m.name,
    label: (
      <>
        <Dot color={statusOf(m.status).color} /> {m.name}
      </>
    ),
    onOpen: () => openIt(m),
    readOnly: !!m.archived_at,
    module: m,
  }));
  const framed = frame(rows);
  const ghosts = (text: string, className: string) =>
    [0, 1].map((n) => (
      <div key={n} className={`${className} ghostCard`}>
        {n === 0 && <span className="ghostText">{text}</span>}
      </div>
    ));
  const none = modules.isError ? "Couldn't be read" : modules.data ? (all.length ? "No module matches" : ro ? "No modules" : "No modules yet — press M to make one") : "Reading…";

  return (
    <section className="section" aria-label="Modules">
      {open ? (
        <ModuleView slug={slug} project={project} module={open} onPeek={(id) => set({ peek: id })} onClose={() => set({ module: undefined, peek: undefined })} />
      ) : (
        <>
          <div className={styles.toolbar}>
            <div className="segmented" role="tablist" aria-label="Layout">
              {LAYOUTS.map((l) => (
                <button key={l.key} type="button" role="tab" aria-selected={layout === l.key} onClick={() => set({ mlayout: l.key === "list" ? undefined : l.key })}>
                  {l.label}
                </button>
              ))}
            </div>
            <input className={`input ${styles.search}`} aria-label="Search modules" placeholder="Search" value={words} onChange={(e) => setWords(e.target.value)} />
            <select className="input" aria-label="Sort by" value={sort} onChange={(e) => set({ msort: e.target.value === "name" ? undefined : e.target.value })}>
              {SORTS.map((s) => (
                <option key={s.key} value={s.key}>
                  {s.label}
                </option>
              ))}
            </select>
            <span className={styles.spacer} />
            <button type="button" className="btn btnPrimary" disabled={ro} onClick={() => setMaking(true)}>
              New module
            </button>
          </div>
          <div className={styles.toolbar}>
            <select className="input" aria-label="Status" value={search.mstatus ?? ""} onChange={(e) => set({ mstatus: e.target.value || undefined })}>
              <option value="">Any status</option>
              {MODULE_STATUS.map((s) => (
                <option key={s.key} value={s.key}>
                  {s.label}
                </option>
              ))}
            </select>
            <select className="input" aria-label="Lead" value={search.mlead ?? ""} onChange={(e) => set({ mlead: e.target.value || undefined })}>
              <option value="">Any lead</option>
              {leads.map((w) => (
                <option key={w} value={w}>
                  {whoName(w, people)}
                </option>
              ))}
            </select>
            <select className="input" aria-label="Member" value={search.mmember ?? ""} onChange={(e) => set({ mmember: e.target.value || undefined })}>
              <option value="">Any member</option>
              {members.map((w) => (
                <option key={w} value={w}>
                  {whoName(w, people)}
                </option>
              ))}
            </select>
            <label className={styles.date}>
              <span className="fieldLabel">From</span>
              <input className="input" type="date" value={from} onChange={(e) => set({ mfrom: e.target.value || undefined })} />
            </label>
            <label className={styles.date}>
              <span className="fieldLabel">To</span>
              <input className="input" type="date" value={to} onChange={(e) => set({ mto: e.target.value || undefined })} />
            </label>
            <label className={styles.check}>
              <input type="checkbox" checked={search.mfav === "1"} onChange={(e) => set({ mfav: e.target.checked ? "1" : undefined })} /> Favorites
            </label>
            <label className={styles.check}>
              <input type="checkbox" checked={search.march === "1"} onChange={(e) => set({ march: e.target.checked ? "1" : undefined })} /> Archived too
            </label>
          </div>
          {(modules.error ?? mutations.update.error ?? mutations.favorite.error) && <p className="formError">{(modules.error ?? mutations.update.error ?? mutations.favorite.error)?.message}</p>}
          {layout === "list" && (
            <div className={styles.group}>
              {shown.map((m) => (
                <div key={m.id} className={styles.row}>
                  {star(m)}
                  <button type="button" className={styles.rowTitle} onClick={() => openIt(m)}>
                    {m.name}
                  </button>
                  <span className={styles.rowProps}>
                    <Status status={m.status} />
                    <span className="fieldHint">{dates(m)}</span>
                    {m.lead && <span className={styles.prop}>{whoName(m.lead, people)}</span>}
                    {m.archived_at && <span className="chip">Archived</span>}
                    <Progress module={m} />
                  </span>
                </div>
              ))}
              {!shown.length && ghosts(none, styles.row)}
            </div>
          )}
          {layout === "gallery" && (
            <div className={styles.gallery}>
              {shown.map((m) => (
                <article key={m.id} className={styles.moduleCard}>
                  <div className={styles.cardHead}>
                    <button type="button" className={styles.rowTitle} onClick={() => openIt(m)}>
                      {m.name}
                    </button>
                    {star(m)}
                  </div>
                  <p className={m.description ? "fieldHint" : "ghostText"}>{m.description || "Not yet captured"}</p>
                  <span className={styles.cardProps}>
                    <Status status={m.status} />
                    <span className="fieldHint">{dates(m)}</span>
                  </span>
                  <span className="fieldHint">{m.lead ? `Lead: ${whoName(m.lead, people)}` : "No lead"}</span>
                  <Progress module={m} />
                </article>
              ))}
              {!shown.length && ghosts(none, styles.moduleCard)}
            </div>
          )}
          {layout === "timeline" && (
            <section className={styles.timeline} aria-label="Timeline">
              <div className={styles.calHead}>
                <span className="fieldHint">{framed.shown.length ? "Drag a bar to move its dates, or an end to change one." : "No module has dates near today yet."}</span>
                <span className={styles.spacer} />
                <div className="segmented" role="group" aria-label="Zoom">
                  {(["week", "month", "quarter"] as const).map((z) => (
                    <button key={z} type="button" aria-pressed={zoom === z} onClick={() => setZoom(z)}>
                      {z === "week" ? "Weeks" : z === "month" ? "Months" : "Quarters"}
                    </button>
                  ))}
                </div>
              </div>
              <Gantt
                rows={framed.shown}
                from={framed.from}
                to={framed.to}
                zoom={zoom}
                readOnly={ro}
                empty="No module has a start or target date near today — set one to place it here"
                onMove={(row, s, e) => {
                  const m = all.find((x) => x.id === row.id);
                  if (m && !m.archived_at) mutations.update.mutate({ id: m.id, patch: { ...(m.start_date ? { start_date: s } : {}), ...(m.target_date ? { target_date: e } : {}) } });
                }}
              />
              <section className={styles.subs} aria-label="Not on the timeline">
                <h3 className={styles.h3}>Without dates, or far from today · {framed.away.length}</h3>
                {framed.away.map(({ module: m }) => (
                  <button key={m.id} type="button" className={styles.subRow} onClick={() => openIt(m)}>
                    <Dot color={statusOf(m.status).color} /> {m.name}
                  </button>
                ))}
                {!framed.away.length && ghosts("Every module is on the timeline", styles.ghostRow)}
              </section>
            </section>
          )}
        </>
      )}
      {making && <ModuleSheet slug={slug} projectId={project.id} onClose={() => setMaking(false)} />}
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
