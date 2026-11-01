import { Link } from "@tanstack/react-router";
import { useState, type KeyboardEvent } from "react";
import { asRecord, labelInput, recordId, recordLabel, useCreateRecord, useRecords, useUpdateRecord, type FieldMeta, type Json, type ObjectDef, type Query, type RecordJson } from "../../../kernel/bridge/records";
import { useAggregates, useGroupCounts } from "../../../kernel/bridge/views";
import { CellView, chipClass } from "../cells";
import crm from "../Crm.module.css";
import type { Column } from "../RecordTable";
import styles from "./Boards.module.css";

const CARDS = 200;

interface Props {
  slug: string;
  object: ObjectDef;
  /** The select field whose options are the columns. */
  field: FieldMeta;
  /** The view's visible fields: shown on each card (none in compact mode). */
  columns: Column[];
  query: Query;
  deleted: boolean;
  compact: boolean;
  /** The columns in the view's order, hidden ones left out. */
  groups: { value: string; label: string; color: string }[];
  /** The number at a column's head: an aggregate of a numeric field (Twenty's expected amount). */
  aggregate: { field: FieldMeta; op: string } | null;
}

interface Drag {
  id: string;
  from: string;
}

/** Twenty's kanban (D-061): a column per option of a select field, cards you drag between
 *  columns (the field changes, the timeline records it) and reorder within one (the position),
 *  a count and an aggregate per column, "+ New" at each column's foot, compact mode. */
export default function KanbanBoard({ slug, object, field, columns, query, deleted, compact, groups, aggregate }: Props) {
  const name = object.meta.name_singular;
  const update = useUpdateRecord(slug, name);
  const create = useCreateRecord(slug, name);
  const counts = useGroupCounts(slug, name, query, field.name);
  const [drag, setDrag] = useState<Drag | null>(null);
  const error = update.error ?? create.error;
  const cardFields = columns.filter((c) => c.field.id !== object.meta.label_identifier_field_id);
  return (
    <>
      {error && (
        <p className="formError" role="alert">
          {error.message}
        </p>
      )}
      <div className={styles.board} role="list" aria-label={`${object.meta.label_plural} by ${field.label}`}>
        {groups.map((g) => (
          <KanbanColumn
            key={g.value || "__none"}
            slug={slug}
            object={object}
            field={field}
            group={g}
            count={counts.data?.find((c) => c.value === g.value)?.count ?? 0}
            cardFields={compact ? [] : cardFields}
            query={query}
            deleted={deleted}
            aggregate={aggregate}
            drag={drag}
            setDrag={setDrag}
            onMove={(id, value, position) => update.mutate({ id, data: { [field.name]: value || null, ...(position == null ? {} : { position }) } })}
            onAdd={(text, done) => create.mutate({ ...labelInput(object, text), [field.name]: g.value || null }, { onSuccess: done })}
          />
        ))}
      </div>
    </>
  );
}

interface ColumnProps {
  slug: string;
  object: ObjectDef;
  field: FieldMeta;
  group: { value: string; label: string; color: string };
  count: number;
  cardFields: Column[];
  query: Query;
  deleted: boolean;
  aggregate: { field: FieldMeta; op: string } | null;
  drag: Drag | null;
  setDrag: (d: Drag | null) => void;
  onMove: (id: string, value: string, position: number | null) => void;
  onAdd: (text: string, done: () => void) => void;
}

function KanbanColumn({ slug, object, field, group, count, cardFields, query, deleted, aggregate, drag, setDrag, onMove, onAdd }: ColumnProps) {
  const name = object.meta.name_singular;
  const own: Json = group.value ? { field: field.name, op: "is", value: group.value } : { field: field.name, op: "isEmpty" };
  const base = query.filter && typeof query.filter === "object" && query.filter !== null && "and" in query.filter ? (query.filter as { and: Json[] }).and : query.filter ? [query.filter] : [];
  const columnQuery: Query = { ...query, filter: { and: [...base, own] }, limit: CARDS };
  const page = useRecords(slug, name, columnQuery);
  const cards = (page.data?.records ?? []).map(asRecord);
  const totals = useAggregates(slug, name, columnQuery, aggregate ? [{ field: aggregate.field.name, sub: null, op: aggregate.op }] : []);
  const total = totals.data?.[0]?.value;
  const [over, setOver] = useState(false);
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const positionOf = (r: RecordJson) => (typeof r.position === "number" ? r.position : 0);
  // With a sort on, the engine orders by it, not by position: no manual reorder (as Twenty).
  const manual = !(query.sort && query.sort.length);

  /** A card dropped on the column's empty part goes last; on a card, just before it. */
  const drop = (before?: RecordJson) => {
    setOver(false);
    if (!drag || deleted) return;
    if (before && recordId(before) === drag.id) return;
    const others = cards.filter((c) => recordId(c) !== drag.id);
    let position: number | null = null;
    if (!manual) {
      position = null;
    } else if (before) {
      const i = others.findIndex((c) => recordId(c) === recordId(before));
      const prev = i > 0 ? positionOf(others[i - 1]) : positionOf(before) - 1;
      position = (prev + positionOf(before)) / 2;
    } else if (others.length) {
      position = positionOf(others[others.length - 1]) + 1;
    }
    if (drag.from === group.value && position == null) return;
    onMove(drag.id, group.value, manual ? position : null);
    setDrag(null);
  };
  const onAddKey = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key !== "Enter" || e.nativeEvent.isComposing || !draft.trim()) return;
    setBusy(true);
    onAdd(draft.trim(), () => {
      setDraft("");
      setBusy(false);
    });
  };

  return (
    <section
      className={`${styles.column} ${over ? styles.columnOver : ""}`}
      role="listitem"
      aria-label={group.label}
      onDragOver={(e) => {
        e.preventDefault();
        setOver(true);
      }}
      onDragLeave={() => setOver(false)}
      onDrop={(e) => {
        e.preventDefault();
        drop();
      }}
    >
      <header className={styles.columnHead}>
        <span className={`${crm.chip} ${chipClass(group.color)}`}>{group.label}</span>
        <span className={styles.columnCount}>{count}</span>
        {aggregate && <span className={styles.columnTotal}>{formatTotal(aggregate, total)}</span>}
      </header>
      {cards.map((record) => (
        <KanbanCard key={recordId(record)} object={object} record={record} fields={cardFields} deleted={deleted} dragging={drag?.id === recordId(record)} onDragStart={() => setDrag({ id: recordId(record), from: group.value })} onDragEnd={() => setDrag(null)} onDropBefore={() => drop(record)} />
      ))}
      {cards.length === 0 && [0, 1].map((i) => <div key={i} className={`ghostCard ${styles.ghostCard}`} />)}
      {page.isError && <p className="ghostText">Couldn't be read — {page.error.message}</p>}
      {!deleted && <input className={`input ${styles.addCard}`} placeholder="+ New" aria-label={`New ${object.meta.label_singular.toLowerCase()} in ${group.label}`} value={draft} disabled={busy} onChange={(e) => setDraft(e.target.value)} onKeyDown={onAddKey} />}
    </section>
  );
}

function KanbanCard({ object, record, fields, deleted, dragging, onDragStart, onDragEnd, onDropBefore }: { object: ObjectDef; record: RecordJson; fields: Column[]; deleted: boolean; dragging: boolean; onDragStart: () => void; onDragEnd: () => void; onDropBefore: () => void }) {
  const id = recordId(record);
  const label = recordLabel(object, record) || "Unnamed";
  const body = (
    <>
      <span className={styles.cardTitle}>{label}</span>
      {fields
        .filter((c) => !isEmptyValue(record[c.field.name]))
        .map((c) => (
          <span key={c.field.id} className={styles.cardField}>
            <span className={styles.cardLabel}>{c.field.label}</span>
            <span className={styles.cardValue}>
              <CellView field={c.field} record={record} />
            </span>
          </span>
        ))}
    </>
  );
  const common = {
    className: `${styles.card} ${dragging ? styles.cardDragging : ""}`,
    draggable: !deleted,
    onDragStart,
    onDragEnd,
    onDragOver: (e: React.DragEvent) => e.preventDefault(),
    onDrop: (e: React.DragEvent) => {
      e.preventDefault();
      e.stopPropagation();
      onDropBefore();
    },
  };
  // A person's card opens their page (the record pages of every object come with 0.10.4).
  return object.meta.name_singular === "person" && !deleted ? (
    <Link to="/crm/people/$personId" params={{ personId: id }} {...common}>
      {body}
    </Link>
  ) : (
    <div {...common} role="article" aria-label={label} tabIndex={0}>
      {body}
    </div>
  );
}

/** Nothing to show for the field: no value, an empty text, an empty list or composite. */
function isEmptyValue(v: unknown): boolean {
  if (v == null || v === "" || v === false) return true;
  if (Array.isArray(v)) return v.length === 0;
  if (typeof v === "object") return Object.values(v as Record<string, unknown>).every((x) => x == null || x === "" || (Array.isArray(x) && x.length === 0));
  return false;
}

function formatTotal(aggregate: { field: FieldMeta; op: string }, value: Json | undefined): string {
  if (value == null) return "—";
  if (typeof value !== "number") return String(value);
  const n = aggregate.field.type === "CURRENCY" && ["SUM", "AVG", "MIN", "MAX"].includes(aggregate.op) ? value / 1_000_000 : value;
  return n.toLocaleString("en-GB", { maximumFractionDigits: 2 });
}
