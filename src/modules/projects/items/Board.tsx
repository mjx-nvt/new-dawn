import { useState } from "react";
import type { PmLabel, PmProject, PmState } from "../../../kernel/bridge/pm";
import type { PmCycle } from "../../../kernel/bridge/pmCycles";
import type { PmModule } from "../../../kernel/bridge/pmModules";
import { useUpdatePmItem, type ItemInput, type PmDisplay, type PmItem } from "../../../kernel/bridge/pmItems";
import { grouped, moveTo, type Group, type GroupBy, type Property } from "./items";
import { Dot, ItemProps, QuickAdd } from "./Props";
import styles from "./Items.module.css";

export interface LayoutProps {
  slug: string;
  project: PmProject;
  /** The items shown (sub-items as the display says), in their order. */
  items: PmItem[];
  labels: PmLabel[];
  states: PmState[];
  cycles?: PmCycle[];
  modules?: PmModule[];
  display: PmDisplay;
  has: (p: Property) => boolean;
  who: (w: string) => string;
  onPeek: (id: string) => void;
  onCreate: (preset: Partial<ItemInput>) => void;
}

/** A column's and a row's values together: what an item made in that cell takes. */
function both(a: Group["value"], b: Group["value"]): Group["value"] {
  return { state_id: b.state_id ?? a.state_id, priority: b.priority ?? a.priority, labels: [...(a.labels ?? []), ...(b.labels ?? [])], assignees: [...(a.assignees ?? []), ...(b.assignees ?? [])], cycle_id: b.cycle_id ?? a.cycle_id, modules: [...(a.modules ?? []), ...(b.modules ?? [])] };
}

/** Plane's Board (D-076): a column a group, a row a sub-group when there is one; a card dragged
 *  to another column and row takes both at once (a state and an assignee, say) — one change;
 *  columns fold (remembered); each cell ends with a quick add. */
export default function Board({ slug, project, items, labels, states, cycles = [], modules = [], display, has, who, onPeek, onCreate, groups, group, sub, onCollapse }: LayoutProps & { groups: Group[]; group: GroupBy; sub: GroupBy; onCollapse: (collapsed: string[]) => void }) {
  const update = useUpdatePmItem(slug);
  const [drag, setDrag] = useState<{ id: string; column: string; lane: string } | null>(null);
  const lanes: Group[] = sub === "none" ? [{ key: "all", label: "", value: {}, items }] : grouped(items, sub, states, labels, who, cycles, modules).filter((l) => display.empty_groups || l.items.length);
  const folded = display.collapsed;
  const fold = (key: string) => onCollapse(folded.includes(key) ? folded.filter((k) => k !== key) : [...folded, key]);
  const ro = !!project.archived_at;

  const drop = (column: Group, lane: Group) => {
    const d = drag;
    setDrag(null);
    if (column.closed || lane.closed) return;
    const item = d && items.find((i) => i.id === d.id);
    if (!d || !item) return;
    // Nothing leaves an archived module's group (D-079).
    const left = (by: GroupBy, all: Group[], key: string, to: Group) => by === "module" && key !== to.key && !!all.find((g) => g.key === key)?.closed;
    if (left(group, groups, d.column, column) || left(sub, lanes, d.lane, lane)) return;
    const patch = {
      ...(d.column === column.key ? {} : moveTo(item, group, groups.find((g) => g.key === d.column), column)),
      ...(sub === "none" || d.lane === lane.key ? {} : moveTo(item, sub, lanes.find((l) => l.key === d.lane), lane)),
    };
    if (Object.keys(patch).length) update.mutate({ id: item.id, patch });
  };

  return (
    <section className={styles.board} aria-label="Board">
      {update.error && (
        <p className="formError" role="alert">
          {update.error.message}
        </p>
      )}
      {lanes.map((lane) => (
        <section key={lane.key} className={styles.lane} aria-label={lane.label || "Work items"}>
          {sub !== "none" && (
            <div className={styles.laneHead}>
              {lane.color && <Dot color={lane.color} />}
              <span className={styles.groupName}>{lane.label}</span>
              <span className="fieldHint">{lane.items.length}</span>
            </div>
          )}
          <div className={styles.columns}>
            {groups.map((column) => {
              const cell = column.items.filter((i) => lane.items.some((x) => x.id === i.id));
              const value = both(column.value, lane.value);
              if (folded.includes(column.key)) {
                return (
                  <button key={column.key} type="button" className={styles.folded} aria-label={`Unfold ${column.label}`} onClick={() => fold(column.key)}>
                    <span className={styles.foldedName}>{column.label}</span>
                    <span className="fieldHint">{cell.length}</span>
                  </button>
                );
              }
              return (
                <div key={column.key} className={styles.column} data-column={column.key} data-lane={lane.key} onDragOver={(e) => e.preventDefault()} onDrop={() => drop(column, lane)}>
                  <div className={styles.columnHead}>
                    {column.color && <Dot color={column.color} />}
                    <span className={styles.groupName}>{column.label}</span>
                    <span className="fieldHint">{cell.length}</span>
                    <span className={styles.spacer} />
                    <button
                      type="button"
                      className="btn btnGhost btnCompact"
                      aria-label={`New work item in ${column.label}${lane.label ? ` · ${lane.label}` : ""}`}
                      disabled={ro || column.closed || lane.closed}
                      onClick={() => onCreate({ state_id: value.state_id ?? null, priority: value.priority ?? null, assignees: value.assignees ?? [], labels: value.labels ?? [], cycle_id: value.cycle_id ?? null, modules: value.modules ?? [] })}
                    >
                      +
                    </button>
                    <button type="button" className="btn btnGhost btnCompact" aria-label={`Fold ${column.label}`} onClick={() => fold(column.key)}>
                      ‹
                    </button>
                  </div>
                  {cell.map((i) => (
                    <div key={i.id} className={`${styles.card} ${drag?.id === i.id ? styles.dragged : ""}`} draggable={!ro} onDragStart={() => setDrag({ id: i.id, column: column.key, lane: lane.key })} onDragEnd={() => setDrag(null)}>
                      {has("id") && <span className={styles.key}>{i.key}</span>}
                      <button type="button" className={styles.rowTitle} onClick={() => onPeek(i.id)}>
                        {i.parent_id && display.sub_items && <span className={styles.childMark}>↳ </span>}
                        {i.name}
                      </button>
                      <ItemProps item={i} has={(p) => p !== "id" && has(p)} labels={labels} who={who} />
                    </div>
                  ))}
                  {!cell.length && (
                    <div className={`${styles.card} ghostCard`}>
                      <span className="ghostText">Nothing here</span>
                    </div>
                  )}
                  {!ro && !column.closed && !lane.closed && <QuickAdd slug={slug} projectId={project.id} label={`${column.label}${lane.label ? ` · ${lane.label}` : ""}`} value={value} />}
                </div>
              );
            })}
          </div>
        </section>
      ))}
    </section>
  );
}
