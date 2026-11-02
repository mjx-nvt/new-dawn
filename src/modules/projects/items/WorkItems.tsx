import { getRouteApi, useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { usePmLabels, usePmStates, type PmProject } from "../../../kernel/bridge/pm";
import { usePmDisplay, usePmItems, useSetPmDisplay, useUpdatePmItem, type ItemInput, type ItemPatch, type PmDisplay, type PmItem } from "../../../kernel/bridge/pmItems";
import { usePeople } from "../../../kernel/bridge/people";
import { inField } from "../../../kernel/shell/useShortcuts";
import { whoName } from "../pm";
import Board from "./Board";
import BulkBar from "./BulkBar";
import Calendar from "./Calendar";
import CreateItem from "./CreateItem";
import ItemPeek, { type PeekMode } from "./ItemPeek";
import { DEFAULT_DISPLAY, GROUP_BY, LAYOUTS, ORDER_BY, PROPERTIES, PROPERTY_LABEL, grouped, moveTo, ordered, type Group, type GroupBy, type Layout, type OrderBy, type Property } from "./items";
import { Dot, ItemProps, QuickAdd } from "./Props";
import Table from "./Table";
import Timeline from "./Timeline";
import styles from "./Items.module.css";

const route = getRouteApi("/shell/projects/$projectId");

/** A project's work items in Plane's layouts (D-074, D-076): List, Board, Calendar, Table,
 *  Timeline — the layout, grouping, sub-grouping and order in the URL; what the URL leaves out
 *  comes from the project's display, remembered with every change (with the properties shown,
 *  sub-items and empty groups, the calendar's span, the timeline's zoom, the folded columns). A
 *  row, card or bar opens its peek. `N` or `I` makes a work item. */
export default function WorkItems({ slug, project }: { slug: string; project: PmProject }) {
  const search = route.useSearch();
  const navigate = useNavigate();
  const items = usePmItems(slug, project.id);
  const states = usePmStates(slug, project.id).data ?? [];
  const labels = usePmLabels(slug, project.id).data ?? [];
  const people = usePeople(slug).data ?? [];
  const update = useUpdatePmItem(slug);
  const stored = usePmDisplay(slug, project.id);
  const keep = useSetPmDisplay(slug, project.id);
  const display: PmDisplay = stored.data ?? DEFAULT_DISPLAY;
  // Nothing drawn or changed until the items and the kept display are read (no flash of the
  // defaults, no defaults written over the kept display).
  const ready = !!items.data && (!!stored.data || stored.isError);
  const [creating, setCreating] = useState<Partial<ItemInput> | null>(null);
  // The row dragged and the group it left (an item can sit in several: labels, assignees).
  const [dragging, setDragging] = useState<{ id: string; from: string } | null>(null);
  // Bulk changes (D-075): the selected items, in the order picked.
  const [selected, setSelected] = useState<string[]>([]);
  const pick = (ids: string[], on: boolean) => setSelected((now) => (on ? [...now, ...ids.filter((id) => !now.includes(id))] : now.filter((id) => !ids.includes(id))));
  const layout: Layout = LAYOUTS.find((l) => l.key === (search.layout ?? display.layout))?.key ?? "list";
  const group: GroupBy = GROUP_BY.find((g) => g.key === (search.group ?? display.group))?.key ?? "state";
  const clash = (a: GroupBy, b: GroupBy) => a === b || (["state", "group"].includes(a) && ["state", "group"].includes(b));
  const wantedSub: GroupBy = GROUP_BY.find((g) => g.key === (search.sub ?? display.sub_group))?.key ?? "none";
  // A sub-group that repeats the grouping (or state with state group) is none.
  const sub: GroupBy = wantedSub !== "none" && clash(wantedSub, group) ? "none" : wantedSub;
  const order: OrderBy = ORDER_BY.find((o) => o.key === (search.order ?? display.order))?.key ?? "manual";
  const set = (patch: Record<string, string | undefined>) => void navigate({ to: "/projects/$projectId", params: { projectId: project.id }, search: { ...search, ...patch }, replace: true });
  /** Kept with the project (and taken out of the URL, which now says the same). */
  const remember = (patch: Partial<PmDisplay>) => keep.mutate({ ...display, ...patch });
  const choose = (key: "layout" | "group" | "sub" | "order", value: string) => {
    set({ [key]: undefined });
    if (key === "layout") setSelected([]);
    // Folded columns are a grouping's keys: a new grouping starts unfolded.
    remember(key === "sub" ? { sub_group: value } : key === "group" ? { group: value, collapsed: [] } : { [key]: value });
  };
  const who = (w: string) => (w === "automation" ? "Automation" : whoName(w, people) || w);
  const all = items.data ?? [];
  const visible = ordered(display.sub_items ? all : all.filter((i) => !i.parent_id), order);
  const groups = grouped(visible, group, states, labels, who).filter((g) => display.empty_groups || g.items.length);
  const has = (p: Property) => display.properties.includes(p);
  const chosen = selected.filter((id) => all.some((i) => i.id === id)); // the selected still here
  const peek = (id: string) => set({ peek: id, activity: undefined });
  const ro = !!project.archived_at;

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // Not while typing, composing, after a `g` chord took the key, or over an open sheet.
      if (e.defaultPrevented || e.isComposing || e.metaKey || e.ctrlKey || e.altKey || creating || inField(e.target) || document.querySelector("dialog[open]")) return;
      if (e.key === "n" || e.key === "i") {
        e.preventDefault();
        setCreating({});
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [creating]);

  /** A dragged row lands before the row it's dropped on (or at the group's end), in that group:
   *  its order (manual), and what the group gives it (state, priority, a label, an assignee). */
  const drop = (target: PmItem | null, g: Group) => {
    const d = dragging;
    setDragging(null);
    const id = d?.id;
    if (!d || !id || id === target?.id) return;
    const moved = all.find((i) => i.id === id);
    if (!moved) return;
    const list = g.items.filter((i) => i.id !== id);
    const at = target ? list.findIndex((i) => i.id === target.id) : list.length;
    const before = list[at - 1]?.sort_order ?? null;
    const after = list[at]?.sort_order ?? null;
    const from = groups.find((x) => x.key === d.from);
    const patch: ItemPatch = from?.key === g.key ? {} : moveTo(moved, group, from, g);
    if (order === "manual") patch.sort_order = before != null && after != null ? (before + after) / 2 : after != null ? after - 1000 : (before ?? 0) + 1000;
    if (Object.keys(patch).length) update.mutate({ id, patch });
  };

  const create = (preset: Partial<ItemInput>) => setCreating(preset);
  const layoutProps = { slug, project, items: visible, labels, states, display, has, who, onPeek: peek, onCreate: create };

  return (
    <section className={styles.list} aria-label="Work items">
      <div className={styles.toolbar}>
        <div className="segmented" role="tablist" aria-label="Layout">
          {LAYOUTS.map((l) => (
            <button key={l.key} type="button" role="tab" aria-selected={layout === l.key} disabled={!ready} onClick={() => choose("layout", l.key)}>
              {l.label}
            </button>
          ))}
        </div>
        {layout !== "calendar" && layout !== "timeline" && (
          <label className={styles.tool}>
            <span className="fieldLabel">Group by</span>
            <select className="input" value={group} disabled={!ready} onChange={(e) => choose("group", e.target.value)}>
              {GROUP_BY.map((g) => (
                <option key={g.key} value={g.key}>
                  {g.label}
                </option>
              ))}
            </select>
          </label>
        )}
        {layout === "board" && (
          <label className={styles.tool}>
            <span className="fieldLabel">Sub-group by</span>
            <select className="input" value={sub} disabled={!ready} onChange={(e) => choose("sub", e.target.value)}>
              {GROUP_BY.filter((g) => g.key === "none" || !clash(g.key, group)).map((g) => (
                <option key={g.key} value={g.key}>
                  {g.key === "none" ? "None" : g.label}
                </option>
              ))}
            </select>
          </label>
        )}
        {layout !== "calendar" && layout !== "timeline" && (
          <label className={styles.tool}>
            <span className="fieldLabel">Order by</span>
            <select className="input" value={order} disabled={!ready} onChange={(e) => choose("order", e.target.value)}>
              {ORDER_BY.map((o) => (
                <option key={o.key} value={o.key}>
                  {o.label}
                </option>
              ))}
            </select>
          </label>
        )}
        <details className={styles.display}>
          <summary className="btn btnGhost btnCompact">Display</summary>
          <div className={`glass ${styles.displayPanel}`}>
            <p className="fieldLabel">Properties</p>
            {PROPERTIES.map((p) => (
              <label key={p} className={styles.check}>
                <input type="checkbox" checked={has(p)} onChange={(e) => remember({ properties: e.target.checked ? [...display.properties, p] : display.properties.filter((x) => x !== p) })} /> {PROPERTY_LABEL[p]}
              </label>
            ))}
            <p className="fieldLabel">Show</p>
            <label className={styles.check}>
              <input type="checkbox" checked={display.sub_items} onChange={(e) => remember({ sub_items: e.target.checked })} /> Sub-work items
            </label>
            <label className={styles.check}>
              <input type="checkbox" checked={display.empty_groups} onChange={(e) => remember({ empty_groups: e.target.checked })} /> Empty groups
            </label>
            <button
              type="button"
              className="btn btnGhost btnCompact"
              onClick={() => {
                set({ group: undefined, sub: undefined, order: undefined });
                remember({ ...DEFAULT_DISPLAY, layout: display.layout });
              }}
            >
              Reset
            </button>
          </div>
        </details>
        <span className={styles.spacer} />
        <button type="button" className={chosen.length ? "btn btnSecondary" : "btn btnPrimary"} disabled={ro} onClick={() => setCreating({})}>
          New work item
        </button>
      </div>
      {(items.error ?? update.error ?? keep.error) && (
        <p className="formError" role="alert">
          {(items.error ?? update.error ?? keep.error)?.message}
        </p>
      )}
      {!ready &&
        [0, 1].map((n) => (
          <div key={n} className={`${styles.row} ghostCard`}>
            {n === 0 && <span className="ghostText">{items.isError ? "Couldn't be read" : "Reading…"}</span>}
          </div>
        ))}
      {ready && layout === "board" && <Board {...layoutProps} groups={groups} group={group} sub={sub} onCollapse={(collapsed) => remember({ collapsed })} />}
      {ready && layout === "calendar" && <Calendar {...layoutProps} span={display.calendar === "week" ? "week" : "month"} onSpan={(calendar) => remember({ calendar })} />}
      {ready && layout === "table" && <Table {...layoutProps} groups={groups} selected={selected} onPick={pick} />}
      {ready && layout === "timeline" && <Timeline {...layoutProps} zoom={display.zoom === "week" || display.zoom === "quarter" ? display.zoom : "month"} onZoom={(zoom) => remember({ zoom })} />}
      {ready &&
        layout === "list" &&
        groups.map((g) => (
          <div key={g.key} className={styles.group} onDragOver={(e) => e.preventDefault()} onDrop={() => drop(null, g)}>
            <div className={styles.groupHead}>
              <input
                type="checkbox"
                aria-label={`Select every work item in ${g.label}`}
                checked={g.items.length > 0 && g.items.every((i) => selected.includes(i.id))}
                disabled={!g.items.length || ro}
                ref={(el) => {
                  if (el) el.indeterminate = g.items.some((i) => selected.includes(i.id)) && !g.items.every((i) => selected.includes(i.id));
                }}
                onChange={(e) => pick(g.items.map((i) => i.id), e.target.checked)}
              />
              {g.color && <Dot color={g.color} />}
              <span className={styles.groupName}>{g.label}</span>
              <span className="fieldHint">{g.items.length}</span>
              <button type="button" className="btn btnGhost btnCompact" aria-label={`New work item in ${g.label}`} disabled={ro} onClick={() => setCreating({ state_id: g.value.state_id ?? null, priority: g.value.priority ?? null, assignees: g.value.assignees ?? [], labels: g.value.labels ?? [] })}>
                +
              </button>
            </div>
            {g.items.map((i) => (
              <div
                key={i.id}
                className={`${styles.row} ${dragging?.id === i.id ? styles.dragged : ""}`}
                draggable={!ro}
                onDragStart={() => setDragging({ id: i.id, from: g.key })}
                onDragEnd={() => setDragging(null)}
                onDragOver={(e) => e.preventDefault()}
                onDrop={(e) => {
                  e.stopPropagation();
                  drop(i, g);
                }}
              >
                <input type="checkbox" aria-label={`Select ${i.key}`} checked={selected.includes(i.id)} disabled={ro} onChange={(e) => pick([i.id], e.target.checked)} />
                {has("id") && <span className={styles.key}>{i.key}</span>}
                <button type="button" className={styles.rowTitle} onClick={() => peek(i.id)}>
                  {i.parent_id && display.sub_items && <span className={styles.childMark}>↳ </span>}
                  {i.name}
                </button>
                <ItemProps item={i} has={has} labels={labels} who={who} />
              </div>
            ))}
            {!g.items.length && (
              <div className={`${styles.row} ghostCard`}>
                <span className="ghostText">{all.length ? "Nothing here" : "No work items yet — press N, or type one below"}</span>
              </div>
            )}
            {!ro && <QuickAdd slug={slug} projectId={project.id} label={g.label} value={g.value} />}
          </div>
        ))}
      {ready &&
        (layout === "list" || layout === "board") &&
        !groups.length &&
        [0, 1].map((n) => (
          <div key={n} className={`${styles.row} ghostCard`}>
            {n === 0 && <span className="ghostText">{items.isError ? "Couldn't be read" : items.data ? "No work items to show — Display hides sub-work items and empty groups" : "Reading…"}</span>}
          </div>
        ))}
      {chosen.length > 0 && (layout === "list" || layout === "table") && <BulkBar slug={slug} project={project} ids={chosen} onClear={() => setSelected([])} />}
      {creating && <CreateItem slug={slug} projectId={project.id} preset={creating} onClose={() => setCreating(null)} />}
      {search.peek && (
        <ItemPeek
          slug={slug}
          itemId={search.peek}
          mode={(["side", "modal", "full"] as PeekMode[]).find((m) => m === search.mode) ?? "side"}
          tab={search.activity}
          onTab={(activity) => set({ activity })}
          onMode={(m) => set({ mode: m === "side" ? undefined : m })}
          onOpen={(id) => set({ peek: id, activity: undefined })}
          onClose={() => set({ peek: undefined, mode: undefined, activity: undefined })}
        />
      )}
    </section>
  );
}
