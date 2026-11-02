import { getRouteApi, useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import type { PmProject } from "../../../kernel/bridge/pm";
import { useCycleMutations, usePmCycleDetails, usePmCycles, type CycleStatus, type PmCycle } from "../../../kernel/bridge/pmCycles";
import { useUpdatePmItem, type ItemInput } from "../../../kernel/bridge/pmItems";
import { usePmQuery } from "../../../kernel/bridge/pmViews";
import { formatDay } from "../../../kernel/format/format";
import DangerButton from "../../../kernel/shell/DangerButton";
import MenuButton from "../../../kernel/shell/MenuButton";
import { inField } from "../../../kernel/shell/useShortcuts";
import Burndown from "./Burndown";
import CreateItem from "./CreateItem";
import CycleSheet from "./CycleSheet";
import ItemPeek, { type PeekMode } from "./ItemPeek";
import { Dot } from "./Props";
import styles from "./Items.module.css";

const route = getRouteApi("/shell/projects/$projectId");
const TABS: { key: CycleStatus; label: string }[] = [
  { key: "active", label: "Active" },
  { key: "upcoming", label: "Upcoming" },
  { key: "completed", label: "Completed" },
  { key: "draft", label: "Draft" },
];
const STATUS_LABEL: Record<CycleStatus, string> = { draft: "Draft", upcoming: "Upcoming", active: "Active", completed: "Completed" };

const dates = (c: PmCycle) => (c.start_date && c.end_date ? `${formatDay(c.start_date, { short: true })} – ${formatDay(c.end_date, { short: true })}` : "No dates");
/** Done of all but the cancelled. */
const share = (c: PmCycle) => (c.items - c.cancelled > 0 ? c.done / (c.items - c.cancelled) : 0);

/** A cycle's progress bar and words. */
export function Progress({ cycle: c }: { cycle: PmCycle }) {
  const p = Math.round(share(c) * 100);
  return (
    <span className={styles.progressWrap} aria-label={`${p} % done`}>
      <span className={styles.progress}>
        <span style={{ width: `${p}%` }} />
      </span>
      <span className="fieldHint">
        {c.done} of {c.items - c.cancelled} done · {p} %
      </span>
    </span>
  );
}

/** One cycle open (D-078): its dates and status, the actions its status allows, the burn-down
 *  and the breakdowns, and its work items — added from the project's, made in it, taken out. */
function CycleView({ slug, project, cycle: c, cycles, onPeek, onClose }: { slug: string; project: PmProject; cycle: PmCycle; cycles: PmCycle[]; onPeek: (id: string) => void; onClose: () => void }) {
  const m = useCycleMutations(slug);
  const update = useUpdatePmItem(slug);
  const details = usePmCycleDetails(slug, c.id);
  const inIt = usePmQuery(slug, project.id, [{ field: "cycle", op: "in", values: [c.id] }]);
  const [adding, setAdding] = useState(false);
  const [words, setWords] = useState("");
  const others = usePmQuery(slug, project.id, [{ field: "cycle", op: "not_in", values: [c.id] }], adding);
  const [picked, setPicked] = useState<string[]>([]);
  const [editing, setEditing] = useState(false);
  const [making, setMaking] = useState<Partial<ItemInput> | null>(null);
  const ro = !!project.archived_at;
  const next = cycles.filter((x) => x.id !== c.id && (x.status === "active" || x.status === "upcoming"));
  const error = m.update.error ?? m.start.error ?? m.complete.error ?? m.transfer.error ?? m.archive.error ?? m.remove.error ?? m.addItems.error ?? update.error;
  const list = inIt.data ?? [];
  const candidates = (others.data ?? []).filter((i) => !words.trim() || `${i.key} ${i.name}`.toLowerCase().includes(words.trim().toLowerCase()));
  const d = details.data;
  return (
    <section className={styles.cycleView} aria-label={c.name}>
      <div className={styles.toolbar}>
        <button type="button" className="btn btnGhost btnCompact" onClick={onClose}>
          ‹ Cycles
        </button>
        <h2 className="sectionTitle">{c.name}</h2>
        <span className="chip">{STATUS_LABEL[c.status]}</span>
        <span className="fieldHint">{dates(c)}</span>
        <span className={styles.spacer} />
        {c.status === "upcoming" && (
          <button type="button" className="btn btnSecondary btnCompact" disabled={ro || m.start.isPending} onClick={() => m.start.mutate(c.id)}>
            Start now
          </button>
        )}
        {c.status === "active" && (
          <button type="button" className="btn btnSecondary btnCompact" disabled={ro || m.complete.isPending} onClick={() => m.complete.mutate(c.id)}>
            Complete now
          </button>
        )}
        {c.status === "completed" && !c.transferred_at && c.items - c.done - c.cancelled > 0 && (
          <select className="input" aria-label="Move the open work to" value="" disabled={ro || !next.length || m.transfer.isPending} onChange={(e) => e.target.value && m.transfer.mutate({ id: c.id, to: e.target.value })}>
            <option value="">{next.length ? "Move open work to…" : "No active or upcoming cycle to move to"}</option>
            {next.map((x) => (
              <option key={x.id} value={x.id}>
                {x.name}
              </option>
            ))}
          </select>
        )}
        {c.transferred_at && <span className="fieldHint">Open work moved on</span>}
        <MenuButton
          label={`${c.name} actions`}
          items={[
            { label: "Edit", disabled: ro, onSelect: () => setEditing(true) },
            { label: c.is_favorite ? "Unfavorite" : "Favorite", disabled: ro, onSelect: () => m.update.mutate({ id: c.id, patch: { is_favorite: !c.is_favorite } }) },
            { label: c.archived_at ? "Restore" : "Archive", disabled: ro || (c.status !== "completed" && !c.archived_at), onSelect: () => m.archive.mutate({ id: c.id, archived: !c.archived_at }) },
          ]}
        />
        {!ro && (
          <DangerButton confirm={`Delete “${c.name}”? Its work items stay, in no cycle.`} onConfirm={() => m.remove.mutate(c.id, { onSuccess: onClose })}>
            Delete
          </DangerButton>
        )}
      </div>
      {error && (
        <p className="formError" role="alert">
          {error.message}
        </p>
      )}
      <p className={c.description ? "fieldHint" : "ghostText"}>{c.description || "Not yet captured"}</p>
      <Progress cycle={c} />
      <div className={styles.cycleBody}>
        <div className={styles.viewMain}>
          <div className={styles.subsHead}>
            <h3 className={styles.h3}>Work items · {list.length}</h3>
            <span className={styles.spacer} />
            {c.status !== "completed" && !ro && (
              <>
                <button type="button" className="btn btnGhost btnCompact" aria-expanded={adding} onClick={() => setAdding(!adding)}>
                  Add existing
                </button>
                <button type="button" className="btn btnSecondary btnCompact" onClick={() => setMaking({ cycle_id: c.id })}>
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
                      {i.cycle_name && <span className="fieldHint"> · in {i.cycle_name}</span>}
                    </label>
                  </li>
                ))}
                {!candidates.length && <li className="ghostText">{others.data ? "No other work items" : "Reading…"}</li>}
              </ul>
              <button
                type="button"
                className="btn btnSecondary btnCompact"
                disabled={!picked.length || m.addItems.isPending}
                onClick={() =>
                  m.addItems.mutate(
                    { id: c.id, itemIds: picked },
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
              {c.status !== "completed" && !ro && (
                <button type="button" className="btn btnGhost btnCompact" aria-label={`Take ${i.key} out of ${c.name}`} disabled={update.isPending} onClick={() => update.mutate({ id: i.id, patch: { cycle_id: "" } })}>
                  ×
                </button>
              )}
            </div>
          ))}
          {!list.length &&
            [0, 1].map((n) => (
              <div key={n} className={`${styles.ghostRow} ghostCard`}>
                {n === 0 && <span className="ghostText">{inIt.data ? "No work items in this cycle yet" : "Reading…"}</span>}
              </div>
            ))}
        </div>
        <aside className={styles.side} aria-label="Details">
          <h3 className={styles.h3}>Burn-down</h3>
          {d ? <Burndown details={d} /> : <p className="ghostText">{details.isError ? "Couldn't be read" : "Reading…"}</p>}
          {(
            [
              ["Assignees", d?.assignees],
              ["States", d?.states],
              ["Labels", d?.labels],
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
                    {s.done}/{s.total}
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
        </aside>
      </div>
      {editing && <CycleSheet slug={slug} projectId={project.id} cycle={c} onClose={() => setEditing(false)} />}
      {making && <CreateItem slug={slug} projectId={project.id} preset={making} onClose={() => setMaking(null)} />}
    </section>
  );
}

/** A project's Cycles tab (D-078, Plane's): Active, Upcoming, Completed and Draft (the tab in
 *  the URL), search and a date range, favorites first; each row its dates, status and progress;
 *  New cycle (`N` or `C`); a cycle opens with its burn-down, breakdowns and work items. */
export default function Cycles({ slug, project }: { slug: string; project: PmProject }) {
  const search = route.useSearch();
  const navigate = useNavigate();
  const cycles = usePmCycles(slug, project.id);
  const m = useCycleMutations(slug);
  const [words, setWords] = useState("");
  const [making, setMaking] = useState(false);
  // The date range and "Archived too" are URL state with the tab.
  const from = search.cfrom ?? "";
  const to = search.cto ?? "";
  const archived = search.carch === "1";
  const set = (patch: Record<string, string | undefined>) => void navigate({ to: "/projects/$projectId", params: { projectId: project.id }, search: { ...search, ...patch }, replace: true });
  const tab: CycleStatus = TABS.find((t) => t.key === search.ctab)?.key ?? "active";
  const all = cycles.data ?? [];
  const open = all.find((c) => c.id === search.cycle);
  const ro = !!project.archived_at;
  const shown = all.filter(
    (c) =>
      c.status === tab &&
      (archived || !c.archived_at) &&
      (!words.trim() || c.name.toLowerCase().includes(words.trim().toLowerCase())) &&
      (!from || !c.end_date || c.end_date >= from) &&
      (!to || !c.start_date || c.start_date <= to),
  );

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // Not over an open cycle or a peek (their own keys), nor while typing.
      if (e.defaultPrevented || e.isComposing || e.metaKey || e.ctrlKey || e.altKey || making || ro || open || search.peek || inField(e.target) || document.querySelector("dialog[open]")) return;
      if (e.key === "n" || e.key === "c") {
        e.preventDefault();
        setMaking(true);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [making, ro, open, search.peek]);

  return (
    <section className="section" aria-label="Cycles">
      {open ? (
        <CycleView slug={slug} project={project} cycle={open} cycles={all} onPeek={(id) => set({ peek: id })} onClose={() => set({ cycle: undefined, peek: undefined })} />
      ) : (
        <>
          <div className={styles.toolbar}>
            <div className="segmented" role="tablist" aria-label="Cycles">
              {TABS.map((t) => (
                <button key={t.key} type="button" role="tab" aria-selected={tab === t.key} onClick={() => set({ ctab: t.key === "active" ? undefined : t.key })}>
                  {t.label} · {all.filter((c) => c.status === t.key && !c.archived_at).length}
                </button>
              ))}
            </div>
            <input className={`input ${styles.search}`} aria-label="Search cycles" placeholder="Search" value={words} onChange={(e) => setWords(e.target.value)} />
            <label className={styles.date}>
              <span className="fieldLabel">From</span>
              <input className="input" type="date" value={from} onChange={(e) => set({ cfrom: e.target.value || undefined })} />
            </label>
            <label className={styles.date}>
              <span className="fieldLabel">To</span>
              <input className="input" type="date" value={to} onChange={(e) => set({ cto: e.target.value || undefined })} />
            </label>
            {tab === "completed" && (
              <label className={styles.check}>
                <input type="checkbox" checked={archived} onChange={(e) => set({ carch: e.target.checked ? "1" : undefined })} /> Archived too
              </label>
            )}
            <span className={styles.spacer} />
            <button type="button" className="btn btnPrimary" disabled={ro} onClick={() => setMaking(true)}>
              New cycle
            </button>
          </div>
          {(cycles.error ?? m.update.error) && <p className="formError">{(cycles.error ?? m.update.error)?.message}</p>}
          <div className={styles.group}>
            {shown.map((c) => (
              <div key={c.id} className={styles.row}>
                <button type="button" className={`${styles.star} ${c.is_favorite ? styles.starOn : ""}`} aria-pressed={c.is_favorite} aria-label={c.is_favorite ? `Unfavorite ${c.name}` : `Favorite ${c.name}`} disabled={ro || m.update.isPending} onClick={() => m.update.mutate({ id: c.id, patch: { is_favorite: !c.is_favorite } })}>
                  {c.is_favorite ? "★" : "☆"}
                </button>
                <button type="button" className={styles.rowTitle} onClick={() => set({ cycle: c.id })}>
                  {c.name}
                </button>
                <span className={styles.rowProps}>
                  <span className="fieldHint">{dates(c)}</span>
                  {c.archived_at && <span className="chip">Archived</span>}
                  <Progress cycle={c} />
                </span>
              </div>
            ))}
            {!shown.length &&
              [0, 1].map((n) => (
                <div key={n} className={`${styles.row} ghostCard`}>
                  {n === 0 && <span className="ghostText">{cycles.isError ? "Couldn't be read" : cycles.data ? `No ${TABS.find((t) => t.key === tab)?.label.toLowerCase()} cycles — press C to make one` : "Reading…"}</span>}
                </div>
              ))}
          </div>
        </>
      )}
      {making && <CycleSheet slug={slug} projectId={project.id} onClose={() => setMaking(false)} />}
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
