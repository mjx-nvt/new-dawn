import { getRouteApi, useNavigate } from "@tanstack/react-router";
import { useEffect, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";
import { usePmLabels, usePmStates, type PmProject } from "../../../kernel/bridge/pm";
import { useCreatePmItem, usePmItems, useUpdatePmItem, type ItemInput, type ItemPatch, type PmItem } from "../../../kernel/bridge/pmItems";
import { usePeople } from "../../../kernel/bridge/people";
import { formatDay } from "../../../kernel/format/format";
import { inField } from "../../../kernel/shell/useShortcuts";
import { whoName } from "../pm";
import CreateItem from "./CreateItem";
import ItemPeek, { type PeekMode } from "./ItemPeek";
import { DEFAULT_DISPLAY, GROUP_BY, ORDER_BY, PROPERTIES, PROPERTY_LABEL, grouped, loadDisplay, ordered, saveDisplay, type Display, type Group, type GroupBy, type OrderBy } from "./items";
import { Dot, PriorityMark } from "./Props";
import styles from "./Items.module.css";

const route = getRouteApi("/shell/projects/$projectId");
const composing = (e: ReactKeyboardEvent) => e.nativeEvent.isComposing || e.keyCode === 229;

function QuickAdd({ slug, projectId, group }: { slug: string; projectId: string; group: Group }) {
  const create = useCreatePmItem(slug);
  const [name, setName] = useState("");
  const add = () =>
    name.trim() &&
    create.mutate(
      { project_id: projectId, name, state_id: group.value.state_id ?? null, priority: group.value.priority ?? null, assignees: group.value.assignees ?? [], labels: group.value.labels ?? [] },
      { onSuccess: () => setName("") },
    );
  return (
    <div className={styles.quick}>
      <input className="input" aria-label={`New work item in ${group.label}`} placeholder="+ New work item" value={name} onChange={(e) => setName(e.target.value)} onKeyDown={(e) => e.key === "Enter" && !composing(e) && add()} />
      {create.error && <span className="formError">{create.error.message}</span>}
    </div>
  );
}

/** A project's work items, Plane's List layout (D-074): grouped (state, state group, priority,
 *  labels, assignees, created by, none) and ordered (manual — dragged —, created, updated,
 *  start, due, priority) as the URL says; the properties a row shows, sub-items and empty groups
 *  as the display says (remembered per project on this Mac); quick add at each group's end; a
 *  row opens its peek. `N` or `I` makes a work item. */
export default function WorkItems({ slug, project }: { slug: string; project: PmProject }) {
  const search = route.useSearch();
  const navigate = useNavigate();
  const items = usePmItems(slug, project.id);
  const states = usePmStates(slug, project.id).data ?? [];
  const labels = usePmLabels(slug, project.id).data ?? [];
  const people = usePeople(slug).data ?? [];
  const update = useUpdatePmItem(slug);
  const [display, setDisplay] = useState<Display>(() => loadDisplay(project.id));
  const [creating, setCreating] = useState<Partial<ItemInput> | null>(null);
  const [dragging, setDragging] = useState<string | null>(null);
  const group: GroupBy = GROUP_BY.find((g) => g.key === search.group)?.key ?? "state";
  const order: OrderBy = ORDER_BY.find((o) => o.key === search.order)?.key ?? "manual";
  const set = (patch: Record<string, string | undefined>) => void navigate({ to: "/projects/$projectId", params: { projectId: project.id }, search: { ...search, ...patch }, replace: true });
  const show = (d: Display) => {
    setDisplay(d);
    saveDisplay(project.id, d);
  };
  const who = (w: string) => whoName(w, people) || w;
  const all = items.data ?? [];
  const visible = display.subItems ? all : all.filter((i) => !i.parent_id);
  const groups = grouped(ordered(visible, order), group, states, labels, who).filter((g) => display.emptyGroups || g.items.length);
  const has = (p: (typeof PROPERTIES)[number]) => display.properties.includes(p);

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

  /** A dragged item lands before the row it's dropped on (or at the group's end), in that group:
   *  its order, and its state or priority when the groups say so. */
  const drop = (target: PmItem | null, g: Group) => {
    const id = dragging;
    setDragging(null);
    if (!id || id === target?.id) return;
    const moved = all.find((i) => i.id === id);
    if (!moved) return;
    const list = g.items.filter((i) => i.id !== id);
    const at = target ? list.findIndex((i) => i.id === target.id) : list.length;
    const before = list[at - 1]?.sort_order ?? null;
    const after = list[at]?.sort_order ?? null;
    const patch: ItemPatch = {};
    if (order === "manual") patch.sort_order = before != null && after != null ? (before + after) / 2 : after != null ? after - 1000 : (before ?? 0) + 1000;
    if ((group === "state" || group === "group") && g.value.state_id && (group === "state" ? moved.state_id !== g.value.state_id : moved.state_group !== g.key)) patch.state_id = g.value.state_id;
    if (group === "priority" && g.value.priority && moved.priority !== g.value.priority) patch.priority = g.value.priority;
    if (Object.keys(patch).length) update.mutate({ id, patch });
  };

  return (
    <section className={styles.list} aria-label="Work items">
      <div className={styles.toolbar}>
        <label className={styles.tool}>
          <span className="fieldLabel">Group by</span>
          <select className="input" value={group} onChange={(e) => set({ group: e.target.value === "state" ? undefined : e.target.value })}>
            {GROUP_BY.map((g) => (
              <option key={g.key} value={g.key}>
                {g.label}
              </option>
            ))}
          </select>
        </label>
        <label className={styles.tool}>
          <span className="fieldLabel">Order by</span>
          <select className="input" value={order} onChange={(e) => set({ order: e.target.value === "manual" ? undefined : e.target.value })}>
            {ORDER_BY.map((o) => (
              <option key={o.key} value={o.key}>
                {o.label}
              </option>
            ))}
          </select>
        </label>
        <details className={styles.display}>
          <summary className="btn btnGhost btnCompact">Display</summary>
          <div className={`glass ${styles.displayPanel}`}>
            <p className="fieldLabel">Properties</p>
            {PROPERTIES.map((p) => (
              <label key={p} className={styles.check}>
                <input type="checkbox" checked={has(p)} onChange={(e) => show({ ...display, properties: e.target.checked ? [...display.properties, p] : display.properties.filter((x) => x !== p) })} /> {PROPERTY_LABEL[p]}
              </label>
            ))}
            <p className="fieldLabel">Show</p>
            <label className={styles.check}>
              <input type="checkbox" checked={display.subItems} onChange={(e) => show({ ...display, subItems: e.target.checked })} /> Sub-work items
            </label>
            <label className={styles.check}>
              <input type="checkbox" checked={display.emptyGroups} onChange={(e) => show({ ...display, emptyGroups: e.target.checked })} /> Empty groups
            </label>
            <button type="button" className="btn btnGhost btnCompact" onClick={() => show(DEFAULT_DISPLAY)}>
              Reset
            </button>
          </div>
        </details>
        <span className={styles.spacer} />
        <button type="button" className="btn btnPrimary" disabled={!!project.archived_at} onClick={() => setCreating({})}>
          New work item
        </button>
      </div>
      {(items.error ?? update.error) && (
        <p className="formError" role="alert">
          {(items.error ?? update.error)?.message}
        </p>
      )}
      {groups.map((g) => (
        <div key={g.key} className={styles.group} onDragOver={(e) => e.preventDefault()} onDrop={() => drop(null, g)}>
          <div className={styles.groupHead}>
            {g.color && <Dot color={g.color} />}
            <span className={styles.groupName}>{g.label}</span>
            <span className="fieldHint">{g.items.length}</span>
            <button type="button" className="btn btnGhost btnCompact" aria-label={`New work item in ${g.label}`} disabled={!!project.archived_at} onClick={() => setCreating({ state_id: g.value.state_id ?? null, priority: g.value.priority ?? null, assignees: g.value.assignees ?? [], labels: g.value.labels ?? [] })}>
              +
            </button>
          </div>
          {g.items.map((i) => (
            <div
              key={i.id}
              className={`${styles.row} ${dragging === i.id ? styles.dragged : ""}`}
              draggable
              onDragStart={() => setDragging(i.id)}
              onDragEnd={() => setDragging(null)}
              onDragOver={(e) => e.preventDefault()}
              onDrop={(e) => {
                e.stopPropagation();
                drop(i, g);
              }}
            >
              {has("id") && <span className={styles.key}>{i.key}</span>}
              <button type="button" className={styles.rowTitle} onClick={() => set({ peek: i.id })}>
                {i.parent_id && display.subItems && <span className={styles.childMark}>↳ </span>}
                {i.name}
              </button>
              <span className={styles.rowProps}>
                {has("state") && (
                  <span className={styles.prop}>
                    <Dot color={i.state_color} /> {i.state_name}
                  </span>
                )}
                {has("priority") && <PriorityMark value={i.priority} />}
                {has("labels") &&
                  i.labels.map((l) => {
                    const label = labels.find((x) => x.id === l);
                    return label ? (
                      <span key={l} className="chip">
                        <Dot color={label.color} /> {label.name}
                      </span>
                    ) : null;
                  })}
                {has("assignees") && i.assignees.length > 0 && <span className={styles.prop}>{i.assignees.map(who).join(", ")}</span>}
                {has("start") && i.start_date && <span className={styles.prop}>From {formatDay(i.start_date, { short: true })}</span>}
                {has("due") && i.target_date && <span className={styles.prop}>Due {formatDay(i.target_date, { short: true })}</span>}
                {has("estimate") && i.estimate_value && <span className="chip">{i.estimate_value}</span>}
                {has("sub_items") && i.sub_items > 0 && (
                  <span className={styles.prop}>
                    {i.sub_done}/{i.sub_items - i.sub_cancelled} done
                  </span>
                )}
              </span>
            </div>
          ))}
          {!g.items.length && (
            <div className={`${styles.row} ghostCard`}>
              <span className="ghostText">{all.length ? "Nothing here" : "No work items yet — press N, or type one below"}</span>
            </div>
          )}
          {!project.archived_at && <QuickAdd slug={slug} projectId={project.id} group={g} />}
        </div>
      ))}
      {!groups.length &&
        [0, 1].map((n) => (
          <div key={n} className={`${styles.row} ghostCard`}>
            {n === 0 && <span className="ghostText">{items.isError ? "Couldn't be read" : items.data ? "No work items to show — Display hides sub-work items and empty groups" : "Reading…"}</span>}
          </div>
        ))}
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
