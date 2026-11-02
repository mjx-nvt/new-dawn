import { useRef, type KeyboardEvent as ReactKeyboardEvent } from "react";
import { useUpdatePmItem, type ItemPatch, type PmItem } from "../../../kernel/bridge/pmItems";
import { formatDateTime } from "../../../kernel/format/format";
import type { LayoutProps } from "./Board";
import { PROPERTY_LABEL, type Group, type Property } from "./items";
import { CyclePick, DateField, EstimatePick, LabelPick, ModulePick, PeoplePick, PriorityPick, StatePick } from "./Props";
import styles from "./Items.module.css";

const COLUMNS: Property[] = ["state", "priority", "assignees", "labels", "start", "due", "estimate", "cycle", "modules", "sub_items", "attachments", "links", "created", "created_by", "updated"];
const composing = (e: ReactKeyboardEvent) => e.nativeEvent.isComposing || e.keyCode === 229;

/** Plane's spreadsheet (D-076): a row an item, a column a property the display shows (Created on
 *  and by, Updated on too); each cell edited where it is. The arrow keys move between cells,
 *  Enter goes into one and Escape comes back out; a checkbox a row for the bulk bar. */
export default function Table({ slug, project, has, who, onPeek, groups, selected, onPick }: Omit<LayoutProps, "project"> & { project?: LayoutProps["project"]; groups: Group[]; selected?: string[]; onPick?: (ids: string[], on: boolean) => void }) {
  const update = useUpdatePmItem(slug);
  const table = useRef<HTMLTableElement>(null);
  // Across projects (a workspace view): no project to be archived, and no bulk bar.
  const ro = !!project?.archived_at;
  const picking = !!onPick && !!selected;
  const cols = COLUMNS.filter(has);
  // The table lists the items in the groups' order (the grouping orders, the table doesn't head).
  const rows: PmItem[] = groups.flatMap((g) => g.items).filter((i, n, all) => all.findIndex((x) => x.id === i.id) === n);
  const save = (i: PmItem, patch: ItemPatch) => update.mutate({ id: i.id, patch });
  const all = picking && rows.length > 0 && rows.every((i) => selected?.includes(i.id));

  const focusCell = (row: number, col: number) => table.current?.querySelector<HTMLElement>(`td[data-row="${row}"][data-col="${col}"]`)?.focus();
  const onKey = (e: ReactKeyboardEvent<HTMLTableElement>) => {
    const td = (e.target as HTMLElement).closest<HTMLElement>("td[data-row]");
    if (!td) return;
    const [row, col] = [Number(td.dataset.row), Number(td.dataset.col)];
    if (e.target === td) {
      const to: Record<string, [number, number]> = { ArrowUp: [row - 1, col], ArrowDown: [row + 1, col], ArrowLeft: [row, col - 1], ArrowRight: [row, col + 1] };
      if (to[e.key]) {
        e.preventDefault();
        focusCell(...to[e.key]);
      } else if (e.key === "Enter") {
        // Into the cell's field — not a chip's ✕, which a second Enter would press.
        e.preventDefault();
        (td.querySelector<HTMLElement>("input, select") ?? td.querySelector<HTMLElement>("button"))?.focus();
      }
    } else if (e.key === "Escape" && !composing(e)) {
      // A spreadsheet's Escape cancels: the field gets its value back before it is left.
      e.preventDefault();
      const field = e.target as HTMLInputElement;
      if (field.tagName === "INPUT" && field.type !== "checkbox") field.value = field.defaultValue;
      td.focus();
    }
  };

  const cell = (i: PmItem, p: Property) => {
    switch (p) {
      case "state":
        return <StatePick slug={slug} projectId={i.project_id} value={i.state_id} disabled={ro} onChange={(state_id) => save(i, { state_id })} />;
      case "priority":
        return <PriorityPick value={i.priority} disabled={ro} onChange={(priority) => save(i, { priority })} />;
      case "assignees":
        return <PeoplePick slug={slug} label="Assignees" value={i.assignees} disabled={ro} onChange={(assignees) => save(i, { assignees })} />;
      case "labels":
        return <LabelPick slug={slug} projectId={i.project_id} value={i.labels} disabled={ro} onChange={(l) => save(i, { labels: l })} />;
      case "start":
        return <DateField label={`Start date of ${i.key}`} value={i.start_date} disabled={ro} onSave={(start_date) => save(i, { start_date })} />;
      case "due":
        return <DateField label={`Due date of ${i.key}`} value={i.target_date} disabled={ro} onSave={(target_date) => save(i, { target_date })} />;
      case "estimate":
        return <EstimatePick slug={slug} projectId={i.project_id} value={i.estimate_point_id} disabled={ro} onChange={(estimate_point_id) => save(i, { estimate_point_id })} />;
      case "cycle":
        return <CyclePick slug={slug} projectId={i.project_id} value={i.cycle_id} disabled={ro} onChange={(cycle_id) => save(i, { cycle_id })} />;
      case "modules":
        return <ModulePick slug={slug} projectId={i.project_id} value={i.modules.map((m) => m.id)} disabled={ro} onChange={(modules) => save(i, { modules })} />;
      case "sub_items":
        return i.sub_items ? `${i.sub_done}/${i.sub_items - i.sub_cancelled}` : "";
      case "attachments":
        return i.attachments || "";
      case "links":
        return i.links || "";
      case "created":
        return formatDateTime(i.created_at);
      case "created_by":
        return who(i.created_by);
      case "updated":
        return formatDateTime(i.updated_at);
      default:
        return null;
    }
  };

  return (
    <div className={styles.tableWrap}>
      {update.error && (
        <p className="formError" role="alert">
          {update.error.message}
        </p>
      )}
      <table ref={table} className={styles.table} onKeyDown={onKey} aria-label="Work items">
        <thead>
          <tr>
            {picking && (
              <th scope="col">
                <input type="checkbox" aria-label="Select every work item" checked={all} disabled={!rows.length || ro} onChange={(e) => onPick?.(rows.map((i) => i.id), e.target.checked)} />
              </th>
            )}
            <th scope="col">ID</th>
            <th scope="col">Title</th>
            {cols.map((c) => (
              <th key={c} scope="col">
                {PROPERTY_LABEL[c]}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((i, r) => {
            let c = 0;
            const at = () => c++;
            return (
              <tr key={i.id}>
                {picking && (
                  <td data-row={r} data-col={at()} tabIndex={r === 0 ? 0 : -1}>
                    <input type="checkbox" aria-label={`Select ${i.key}`} checked={!!selected?.includes(i.id)} disabled={ro} onChange={(e) => onPick?.([i.id], e.target.checked)} />
                  </td>
                )}
                {/* The key opens the item: always shown here, whatever the display says. */}
                <td data-row={r} data-col={at()} tabIndex={!picking && r === 0 ? 0 : -1}>
                  <button type="button" className={styles.keyButton} onClick={() => onPeek(i.id)}>
                    {i.key}
                  </button>
                </td>
                <td data-row={r} data-col={at()} tabIndex={-1} className={styles.titleCell}>
                  <input
                    key={i.name}
                    className={styles.cellInput}
                    aria-label={`Title of ${i.key}`}
                    defaultValue={i.name}
                    readOnly={ro}
                    maxLength={255}
                    onBlur={(e) => e.target.value.trim() && e.target.value !== i.name && save(i, { name: e.target.value })}
                    onKeyDown={(e) => e.key === "Enter" && !composing(e) && e.currentTarget.blur()}
                  />
                </td>
                {cols.map((p) => (
                  <td key={p} data-row={r} data-col={at()} tabIndex={-1}>
                    {cell(i, p)}
                  </td>
                ))}
              </tr>
            );
          })}
          {!rows.length &&
            [0, 1].map((n) => (
              <tr key={n} className="ghostCard">
                <td colSpan={cols.length + (picking ? 3 : 2)}>{n === 0 && <span className="ghostText">No work items match</span>}</td>
              </tr>
            ))}
        </tbody>
      </table>
    </div>
  );
}
