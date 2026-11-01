import { Link } from "@tanstack/react-router";
import { useEffect, useRef, useState, type KeyboardEvent, type PointerEvent as ReactPointerEvent } from "react";
import {
  asRecord,
  labelInput,
  recordId,
  recordLabel,
  selectOptions,
  useCreateRecord,
  useDeleteRecord,
  useDestroyRecord,
  useRecords,
  useRestoreRecord,
  useUpdateRecord,
  type FieldMeta,
  type Json,
  type ObjectDef,
  type Query,
  type RecordJson,
} from "../../kernel/bridge/records";
import { useAggregates, useGroupCounts } from "../../kernel/bridge/views";
import DangerButton from "../../kernel/shell/DangerButton";
import { CellEditor, CellView, chipClass, editsInPlace, readOnly } from "./cells";
import styles from "./Crm.module.css";

const PAGE = 60;
const AGGREGATES: [string, string][] = [
  ["COUNT", "Count"],
  ["COUNT_EMPTY", "Count empty"],
  ["COUNT_NOT_EMPTY", "Count not empty"],
  ["COUNT_UNIQUE", "Count unique"],
  ["PERCENT_EMPTY", "Percent empty"],
  ["PERCENT_NOT_EMPTY", "Percent not empty"],
  ["SUM", "Sum"],
  ["AVG", "Average"],
  ["MIN", "Min"],
  ["MAX", "Max"],
];

/** A column of the table: a field, its width and its footer aggregate (from the view, D-061). */
export interface Column {
  field: FieldMeta;
  size: number;
  aggregate: string | null;
}

interface Props {
  slug: string;
  object: ObjectDef;
  columns: Column[];
  /** The working view's query, with the page's search and the deleted switch. */
  query: Query;
  deleted: boolean;
  /** Twenty's compact view: shorter rows. */
  compact?: boolean;
  /** Grouped by this select field, in this order (hidden groups left out). */
  groupBy?: FieldMeta;
  groups?: { value: string; label: string; color: string }[];
  onResize: (fieldId: string, size: number) => void;
  onReorder: (fieldId: string, overFieldId: string) => void;
  onSort: (field: FieldMeta, direction: "ASC" | "DESC") => void;
  onHide: (fieldId: string) => void;
  onFilter: (field: FieldMeta) => void;
  onAggregate: (fieldId: string, op: string | null) => void;
}

/** Twenty's table of an object's records (D-060, D-061): one column per visible field of the
 *  view, resized by dragging a border and reordered by dragging a header; a cell edits in place;
 *  rows select for a delete; an Add row at the end of the table or of each group; group rows
 *  with their counts and aggregates; an aggregate footer; the deleted records behind a switch. */
export default function RecordTable({ slug, object, columns, query, deleted, compact, groupBy, groups, onResize, onReorder, onSort, onHide, onFilter, onAggregate }: Props) {
  const name = object.meta.name_singular;
  const update = useUpdateRecord(slug, name);
  const create = useCreateRecord(slug, name);
  const remove = useDeleteRecord(slug, name);
  const restore = useRestoreRecord(slug, name);
  const destroy = useDestroyRecord(slug, name);
  const [editing, setEditing] = useState<{ id: string; field: string } | null>(null);
  const [selected, setSelected] = useState<string[]>([]);
  const [live, setLive] = useState<{ id: string; size: number } | null>(null);
  const [dragging, setDragging] = useState<string | null>(null);
  const [menu, setMenu] = useState<string | null>(null);
  const [footerMenu, setFooterMenu] = useState<string | null>(null);
  const openHead = useDismiss<HTMLTableCellElement>(menu !== null, () => setMenu(null));
  const openFoot = useDismiss<HTMLTableCellElement>(footerMenu !== null, () => setFooterMenu(null));
  const error = update.error ?? create.error ?? remove.error ?? restore.error ?? destroy.error;
  const ops = columns.filter((c) => c.aggregate).map((c) => ({ field: c.field.name, sub: null, op: c.aggregate as string }));
  const totals = useAggregates(slug, name, query, ops);

  const save = (record: RecordJson, field: FieldMeta, value: unknown) => {
    setEditing(null);
    update.mutate({ id: recordId(record), data: { [field.name]: value } });
  };
  const add = (text: string, extra: RecordJson, done: () => void) => create.mutate({ ...labelInput(object, text), ...extra }, { onSuccess: done });
  const act = (fn: (id: string) => void) => {
    selected.forEach(fn);
    setSelected([]);
  };
  const startResize = (e: ReactPointerEvent<HTMLSpanElement>, column: Column) => {
    e.preventDefault();
    const startX = e.clientX;
    const startSize = column.size;
    let size = startSize;
    const move = (ev: PointerEvent) => {
      size = Math.max(40, Math.min(1200, startSize + ev.clientX - startX));
      setLive({ id: column.field.id, size });
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      setLive(null);
      if (size !== startSize) onResize(column.field.id, size);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };
  const widthOf = (c: Column) => (live?.id === c.field.id ? live.size : c.size);
  const rowsProps = { slug, object, columns, deleted, editing, setEditing, selected, setSelected, save, add };

  return (
    <div className={styles.tableWrap}>
      {error && (
        <p className="formError" role="alert">
          {error.message}
        </p>
      )}
      {selected.length > 0 && (
        <div className={`glass ${styles.bar}`} role="toolbar" aria-label="Selected records">
          <span>{selected.length === 1 ? "1 selected" : `${selected.length} selected`}</span>
          {deleted ? (
            <>
              <button type="button" className="btn btnSecondary btnCompact" onClick={() => act((id) => restore.mutate(id))}>
                Restore
              </button>
              <DangerButton confirm={`Delete ${selected.length === 1 ? "this record" : `these ${selected.length} records`} forever`} onConfirm={() => act((id) => destroy.mutate(id))}>
                Delete forever
              </DangerButton>
            </>
          ) : (
            <button type="button" className="btn btnDanger btnCompact" onClick={() => act((id) => remove.mutate(id))}>
              Delete
            </button>
          )}
          <button type="button" className="btn btnGhost btnCompact" onClick={() => setSelected([])}>
            Clear
          </button>
        </div>
      )}
      <div className={styles.scroll}>
        <table className={compact ? `${styles.table} ${styles.tableCompact}` : styles.table}>
          <colgroup>
            <col style={{ width: 36 }} />
            {columns.map((c) => (
              <col key={c.field.id} style={{ width: widthOf(c) }} />
            ))}
          </colgroup>
          <thead>
            <tr>
              <th className={styles.check} />
              {columns.map((c, i) => (
                <th
                  key={c.field.id}
                  ref={menu === c.field.id ? openHead : undefined}
                  className={`${styles.head} ${dragging === c.field.id ? styles.headDragging : ""}`}
                  title={c.field.description || undefined}
                  draggable={i > 0}
                  onDragStart={() => setDragging(c.field.id)}
                  onDragOver={(e) => e.preventDefault()}
                  onDrop={() => dragging && dragging !== c.field.id && i > 0 && onReorder(dragging, c.field.id)}
                  onDragEnd={() => setDragging(null)}
                >
                  <span className={styles.headInner}>
                    <span className={styles.headLabel}>{c.field.label}</span>
                    <button type="button" className={styles.headMenu} aria-label={`Column ${c.field.label}`} aria-haspopup="menu" onClick={() => setMenu(menu === c.field.id ? null : c.field.id)}>
                      ⋮
                    </button>
                  </span>
                  {menu === c.field.id && (
                    <span className="glass menu" role="menu">
                      <button type="button" role="menuitem" className="menuItem" onClick={() => (onSort(c.field, "ASC"), setMenu(null))}>
                        Sort ascending
                      </button>
                      <button type="button" role="menuitem" className="menuItem" onClick={() => (onSort(c.field, "DESC"), setMenu(null))}>
                        Sort descending
                      </button>
                      <button type="button" role="menuitem" className="menuItem" onClick={() => (onFilter(c.field), setMenu(null))}>
                        Filter
                      </button>
                      {i > 0 && (
                        <button type="button" role="menuitem" className="menuItem" onClick={() => (onHide(c.field.id), setMenu(null))}>
                          Hide
                        </button>
                      )}
                    </span>
                  )}
                  <span
                    className={styles.resize}
                    role="separator"
                    aria-orientation="vertical"
                    aria-label={`Resize ${c.field.label}`}
                    tabIndex={0}
                    onPointerDown={(e) => startResize(e, c)}
                    onKeyDown={(e) => {
                      if (e.key === "ArrowLeft") onResize(c.field.id, Math.max(40, c.size - 8));
                      if (e.key === "ArrowRight") onResize(c.field.id, Math.min(1200, c.size + 8));
                    }}
                  />
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {groupBy && groups ? (
              groups.map((g) => <GroupSection key={g.value || "__none"} {...rowsProps} query={query} groupBy={groupBy} group={g} />)
            ) : (
              <Rows {...rowsProps} query={query} />
            )}
          </tbody>
          <tfoot>
            <tr className={styles.footer}>
              <td className={styles.check} />
              {columns.map((c) => {
                const value = totals.data?.find((a) => a.field === c.field.name && a.op === c.aggregate)?.value;
                return (
                  <td key={c.field.id} className={styles.footerCell} ref={footerMenu === c.field.id ? openFoot : undefined}>
                    <button type="button" className={styles.footerButton} aria-label={`Aggregate for ${c.field.label}`} aria-haspopup="menu" onClick={() => setFooterMenu(footerMenu === c.field.id ? null : c.field.id)}>
                      {c.aggregate ? (
                        <>
                          <span className={styles.muted}>{AGGREGATES.find(([op]) => op === c.aggregate)?.[1]}</span> {formatAggregate(c, value)}
                        </>
                      ) : (
                        <span className={styles.footerGhost}>Calculate</span>
                      )}
                    </button>
                    {footerMenu === c.field.id && (
                      <span className={`glass menu ${styles.menuUp}`} role="menu">
                        <button type="button" role="menuitem" className="menuItem" onClick={() => (onAggregate(c.field.id, null), setFooterMenu(null))}>
                          None
                        </button>
                        {AGGREGATES.filter(([op]) => aggregateFits(c.field, op)).map(([op, label]) => (
                          <button key={op} type="button" role="menuitem" className="menuItem" onClick={() => (onAggregate(c.field.id, op), setFooterMenu(null))}>
                            {label}
                          </button>
                        ))}
                      </span>
                    )}
                  </td>
                );
              })}
            </tr>
          </tfoot>
        </table>
      </div>
    </div>
  );
}

function aggregateFits(field: FieldMeta, op: string): boolean {
  const numeric = ["NUMBER", "CURRENCY", "RATING"].includes(field.type);
  const ordered = numeric || ["DATE", "DATE_TIME"].includes(field.type);
  if (["SUM", "AVG"].includes(op)) return numeric;
  if (["MIN", "MAX"].includes(op)) return ordered;
  return true;
}

/** A menu's container: a click outside it or Escape closes it (the header and footer menus). */
function useDismiss<T extends HTMLElement>(open: boolean, onClose: () => void) {
  const ref = useRef<T>(null);
  useEffect(() => {
    if (!open) return;
    const down = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    const key = (e: globalThis.KeyboardEvent) => e.key === "Escape" && onClose();
    document.addEventListener("mousedown", down);
    document.addEventListener("keydown", key);
    return () => {
      document.removeEventListener("mousedown", down);
      document.removeEventListener("keydown", key);
    };
  }, [open, onClose]);
  return ref;
}

function formatAggregate(column: Column, value: Json | undefined): string {
  if (value == null) return "—";
  if (typeof value === "number") {
    const n = column.field.type === "CURRENCY" && ["SUM", "AVG", "MIN", "MAX"].includes(column.aggregate ?? "") ? value / 1_000_000 : value;
    const percent = column.aggregate?.startsWith("PERCENT");
    return `${n.toLocaleString("en-GB", { maximumFractionDigits: percent ? 0 : 2 })}${percent ? " %" : ""}`;
  }
  return String(value);
}

interface RowsProps {
  slug: string;
  object: ObjectDef;
  columns: Column[];
  query: Query;
  deleted: boolean;
  editing: { id: string; field: string } | null;
  setEditing: (e: { id: string; field: string } | null) => void;
  selected: string[];
  setSelected: (ids: string[]) => void;
  save: (record: RecordJson, field: FieldMeta, value: unknown) => void;
  add: (text: string, extra: RecordJson, done: () => void) => void;
  /** Extra fields a new record of this section gets (a group's value). */
  extra?: RecordJson;
  /** Ghost rows when there are none: two for a table, one for a group. */
  ghosts?: number;
  /** What an empty section says (a group names itself). */
  emptyText?: string;
}

/** The rows a query lists, the ghost rows when there are none, and the Add row. */
function Rows({ slug, object, columns, query, deleted, editing, setEditing, selected, setSelected, save, add, extra = {}, ghosts = 2, emptyText }: RowsProps) {
  const [limit, setLimit] = useState(PAGE);
  const page = useRecords(slug, object.meta.name_singular, { ...query, limit });
  const rows = (page.data?.records ?? []).map(asRecord);
  const total = page.data?.total ?? 0;
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const onAddKey = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key !== "Enter" || e.nativeEvent.isComposing || !draft.trim()) return;
    setBusy(true);
    add(draft.trim(), extra, () => {
      setDraft("");
      setBusy(false);
    });
  };
  const empty = page.isPending ? "Loading…" : page.isError ? `Couldn't be read — ${page.error.message}` : deleted ? "No deleted records" : query.search ? "Nothing matches" : (emptyText ?? `No ${object.meta.label_plural.toLowerCase()} yet`);
  return (
    <>
      {rows.map((record) => {
        const id = recordId(record);
        return (
          <tr key={id} className={selected.includes(id) ? styles.rowSelected : styles.row}>
            <td className={styles.check}>
              <input type="checkbox" aria-label={`Select ${recordLabel(object, record) || "record"}`} checked={selected.includes(id)} onChange={(e) => setSelected(e.target.checked ? [...selected, id] : selected.filter((x) => x !== id))} />
            </td>
            {columns.map((c, i) => (
              <Cell key={c.field.id} slug={slug} object={object} field={c.field} record={record} first={i === 0} deleted={deleted} editing={editing?.id === id && editing.field === c.field.name} onEdit={() => !deleted && !readOnly(c.field) && setEditing({ id, field: c.field.name })} onSave={(v) => save(record, c.field, v)} onCancel={() => setEditing(null)} />
            ))}
          </tr>
        );
      })}
      {rows.length === 0 &&
        Array.from({ length: ghosts }, (_, i) => i).map((i) => (
          <tr key={`ghost-${i}`} className={styles.ghostRow}>
            <td className={styles.check} />
            <td colSpan={columns.length}>{i === 0 && <span className="ghostText">{empty}</span>}</td>
          </tr>
        ))}
      {total > rows.length && (
        <tr className={styles.ghostRow}>
          <td className={styles.check} />
          <td colSpan={columns.length}>
            <button type="button" className="btn btnGhost btnCompact" onClick={() => setLimit(limit + PAGE)}>
              Show more · {total - rows.length} left
            </button>
          </td>
        </tr>
      )}
      {!deleted && (
        <tr className={styles.addRow}>
          <td className={styles.check} />
          <td colSpan={columns.length}>
            <input className={`input ${styles.addInput}`} placeholder={`Add ${article(object.meta.label_singular)} ${object.meta.label_singular.toLowerCase()}…`} aria-label={`Add ${object.meta.label_singular.toLowerCase()}`} value={draft} disabled={busy} onChange={(e) => setDraft(e.target.value)} onKeyDown={onAddKey} />
          </td>
        </tr>
      )}
    </>
  );
}

/** One group of a grouped table: its header (chip, count, the columns' aggregates), collapsible,
 *  then its rows and its own Add row. */
function GroupSection({ groupBy, group, ...props }: RowsProps & { groupBy: FieldMeta; group: { value: string; label: string; color: string } }) {
  const { slug, object, columns, query } = props;
  const [collapsed, setCollapsed] = useState(false);
  const own: Json = group.value ? { field: groupBy.name, op: "is", value: group.value } : { field: groupBy.name, op: "isEmpty" };
  const base = query.filter && typeof query.filter === "object" && query.filter !== null && "and" in query.filter ? (query.filter as { and: Json[] }).and : query.filter ? [query.filter] : [];
  const groupQuery: Query = { ...query, filter: { and: [...base, own] } };
  const counts = useGroupCounts(slug, object.meta.name_singular, query, groupBy.name);
  const count = counts.data?.find((c) => c.value === group.value)?.count ?? 0;
  const ops = columns.filter((c) => c.aggregate).map((c) => ({ field: c.field.name, sub: null, op: c.aggregate as string }));
  const totals = useAggregates(slug, object.meta.name_singular, groupQuery, ops);
  return (
    <>
      <tr className={styles.groupRow}>
        <td className={styles.check}>
          <button type="button" className={styles.collapse} aria-label={collapsed ? `Expand ${group.label}` : `Collapse ${group.label}`} aria-expanded={!collapsed} onClick={() => setCollapsed(!collapsed)}>
            {collapsed ? "▸" : "▾"}
          </button>
        </td>
        <td className={styles.groupHead}>
          <span className={`${styles.chip} ${chipClass(group.color)}`}>{group.label}</span>
          <span className={styles.muted}>{count}</span>
        </td>
        {columns.slice(1).map((c) => {
          const value = totals.data?.find((a) => a.field === c.field.name && a.op === c.aggregate)?.value;
          return (
            <td key={c.field.id} className={styles.groupAggregate}>
              {c.aggregate ? formatAggregate(c, value) : ""}
            </td>
          );
        })}
      </tr>
      {!collapsed && <Rows {...props} query={groupQuery} extra={{ [groupBy.name]: group.value || null }} ghosts={1} emptyText={`No ${object.meta.label_plural.toLowerCase()} in ${group.label}`} />}
    </>
  );
}

interface CellProps {
  slug: string;
  object: ObjectDef;
  field: FieldMeta;
  record: RecordJson;
  first: boolean;
  deleted: boolean;
  editing: boolean;
  onEdit: () => void;
  onSave: (value: unknown) => void;
  onCancel: () => void;
}

function Cell({ slug, object, field, record, first, deleted, editing, onEdit, onSave, onCancel }: CellProps) {
  if (editing) {
    return (
      <td className={styles.cellEditing}>
        <CellEditor slug={slug} field={field} record={record} onSave={onSave} onCancel={onCancel} />
      </td>
    );
  }
  if (editsInPlace(field)) {
    return (
      <td className={styles.cell}>
        <input type="checkbox" aria-label={field.label} checked={record[field.name] === true} disabled={deleted} onChange={(e) => onSave(e.target.checked)} />
      </td>
    );
  }
  const view = <CellView field={field} record={record} />;
  // The label of a person opens their page (the record pages of every object come with 0.10.4).
  const link = first && object.meta.name_singular === "person" && !deleted;
  return (
    <td className={readOnly(field) || deleted ? styles.cell : styles.cellEditable} onClick={link ? undefined : onEdit} onKeyDown={(e) => e.key === "Enter" && !link && onEdit()} tabIndex={readOnly(field) || deleted ? undefined : 0}>
      {link ? (
        <Link to="/crm/people/$personId" params={{ personId: recordId(record) }} className={styles.labelLink}>
          {view ?? <span className="ghostText">Unnamed</span>}
        </Link>
      ) : (
        (view ?? (first ? <span className="ghostText">Unnamed</span> : null))
      )}
    </td>
  );
}

/** The groups of a select field in the view's order, hidden ones left out, "No value" last. */
export function groupsOf(field: FieldMeta, order: { field_value: string; is_visible: boolean; position: number }[], sort: string): { value: string; label: string; color: string }[] {
  const options = selectOptions(field);
  const all = [...options.map((o) => ({ value: o.value, label: o.label, color: o.color })), { value: "", label: "No value", color: "" }];
  const visible = all.filter((g) => order.find((o) => o.field_value === g.value)?.is_visible ?? true);
  if (sort === "ASC") return [...visible].sort((a, b) => a.label.localeCompare(b.label));
  if (sort === "DESC") return [...visible].sort((a, b) => b.label.localeCompare(a.label));
  const position = (value: string) => order.find((o) => o.field_value === value)?.position ?? 1_000 + all.findIndex((g) => g.value === value);
  return [...visible].sort((a, b) => position(a.value) - position(b.value));
}

function article(label: string): string {
  return /^[aeiou]/i.test(label) ? "an" : "a";
}
