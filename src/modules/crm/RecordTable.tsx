import { Link } from "@tanstack/react-router";
import { useState, type KeyboardEvent } from "react";
import {
  asRecord,
  labelInput,
  recordId,
  recordLabel,
  tableFields,
  useCreateRecord,
  useDeleteRecord,
  useDestroyRecord,
  useRecords,
  useRestoreRecord,
  useUpdateRecord,
  type FieldMeta,
  type ObjectDef,
  type RecordJson,
} from "../../kernel/bridge/records";
import DangerButton from "../../kernel/shell/DangerButton";
import { CellEditor, CellView, editsInPlace, readOnly } from "./cells";
import styles from "./Crm.module.css";

const PAGE = 60;

interface Props {
  slug: string;
  object: ObjectDef;
  search: string;
  /** Show the deleted records (Restore · Delete forever) instead of the live ones. */
  deleted: boolean;
}

/** Twenty's table of an object's records (D-060): one column per field, the label first, a cell
 *  that edits in place, rows you select for a delete, an Add row at the bottom, and the deleted
 *  records behind a switch. Views — filters, sorts, groups, saved — come with 0.10.2. */
export default function RecordTable({ slug, object, search, deleted }: Props) {
  const name = object.meta.name_singular;
  const fields = tableFields(object);
  const [limit, setLimit] = useState(PAGE);
  const page = useRecords(slug, name, { search: search || null, deleted, limit });
  const rows = (page.data?.records ?? []).map(asRecord);
  const total = page.data?.total ?? 0;
  const update = useUpdateRecord(slug, name);
  const create = useCreateRecord(slug, name);
  const remove = useDeleteRecord(slug, name);
  const restore = useRestoreRecord(slug, name);
  const destroy = useDestroyRecord(slug, name);
  const [editing, setEditing] = useState<{ id: string; field: string } | null>(null);
  const [selected, setSelected] = useState<string[]>([]);
  const [draft, setDraft] = useState("");
  const error = update.error ?? create.error ?? remove.error ?? restore.error ?? destroy.error;
  const chosen = selected.filter((id) => rows.some((r) => recordId(r) === id));
  const allChosen = rows.length > 0 && chosen.length === rows.length;

  const save = (record: RecordJson, field: FieldMeta, value: unknown) => {
    setEditing(null);
    update.mutate({ id: recordId(record), data: { [field.name]: value } });
  };
  const add = () => {
    const text = draft.trim();
    if (!text) return;
    create.mutate(labelInput(object, text), { onSuccess: () => setDraft("") });
  };
  const onAddKey = (e: KeyboardEvent<HTMLInputElement>) => e.key === "Enter" && !e.nativeEvent.isComposing && add();
  const act = (fn: (id: string) => void) => {
    chosen.forEach(fn);
    setSelected([]);
  };

  return (
    <div className={styles.tableWrap}>
      {error && (
        <p className="formError" role="alert">
          {error.message}
        </p>
      )}
      {chosen.length > 0 && (
        <div className={`glass ${styles.bar}`} role="toolbar" aria-label="Selected records">
          <span>{chosen.length === 1 ? "1 selected" : `${chosen.length} selected`}</span>
          {deleted ? (
            <>
              <button type="button" className="btn btnSecondary btnCompact" onClick={() => act((id) => restore.mutate(id))}>
                Restore
              </button>
              <DangerButton confirm={`Delete ${chosen.length === 1 ? "this record" : `these ${chosen.length} records`} forever`} onConfirm={() => act((id) => destroy.mutate(id))}>
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
        <table className={styles.table}>
          <thead>
            <tr>
              <th className={styles.check}>
                <input type="checkbox" aria-label="Select all" checked={allChosen} onChange={(e) => setSelected(e.target.checked ? rows.map(recordId) : [])} />
              </th>
              {fields.map((f) => (
                <th key={f.id} className={styles.head} style={{ width: widthOf(f) }} title={f.description || undefined}>
                  {f.label}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.map((record) => {
              const id = recordId(record);
              return (
                <tr key={id} className={chosen.includes(id) ? styles.rowSelected : styles.row}>
                  <td className={styles.check}>
                    <input type="checkbox" aria-label={`Select ${recordLabel(object, record) || "record"}`} checked={chosen.includes(id)} onChange={(e) => setSelected(e.target.checked ? [...chosen, id] : chosen.filter((x) => x !== id))} />
                  </td>
                  {fields.map((f, i) => (
                    <Cell key={f.id} slug={slug} object={object} field={f} record={record} first={i === 0} deleted={deleted} editing={editing?.id === id && editing.field === f.name} onEdit={() => !deleted && !readOnly(f) && setEditing({ id, field: f.name })} onSave={(v) => save(record, f, v)} onCancel={() => setEditing(null)} />
                  ))}
                </tr>
              );
            })}
            {rows.length === 0 &&
              [0, 1, 2].map((i) => (
                <tr key={i} className={styles.ghostRow}>
                  <td className={styles.check} />
                  <td colSpan={fields.length}>
                    {i === 0 && (
                      <span className="ghostText">
                        {page.isPending ? "Loading…" : page.isError ? "Couldn't be read" : deleted ? "No deleted records" : search ? "Nothing matches" : `No ${object.meta.label_plural.toLowerCase()} yet`}
                      </span>
                    )}
                  </td>
                </tr>
              ))}
            {!deleted && (
              <tr className={styles.addRow}>
                <td className={styles.check} />
                <td colSpan={fields.length}>
                  <input className={`input ${styles.addInput}`} placeholder={`Add ${article(object.meta.label_singular)} ${object.meta.label_singular.toLowerCase()}…`} aria-label={`Add ${object.meta.label_singular.toLowerCase()}`} value={draft} disabled={create.isPending} onChange={(e) => setDraft(e.target.value)} onKeyDown={onAddKey} />
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
      <p className={`fieldHint ${styles.count}`}>
        {total === 1 ? `1 ${object.meta.label_singular.toLowerCase()}` : `${total} ${object.meta.label_plural.toLowerCase()}`}
        {total > rows.length && (
          <>
            {" · "}
            <button type="button" className="btn btnGhost btnCompact" onClick={() => setLimit(limit + PAGE)}>
              Show more
            </button>
          </>
        )}
      </p>
    </div>
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

function widthOf(field: FieldMeta): number {
  switch (field.type) {
    case "FULL_NAME":
    case "LINKS":
    case "DOMAIN":
    case "EMAILS":
    case "PHONES":
      return 200;
    case "LONG_TEXT":
    case "RICH_TEXT":
    case "ADDRESS":
      return 240;
    case "NUMBER":
    case "CURRENCY":
    case "RATING":
    case "BOOLEAN":
    case "DATE":
      return 130;
    case "DATE_TIME":
    case "SELECT":
    case "ACTOR":
      return 150;
    default:
      return 180;
  }
}

function article(label: string): string {
  return /^[aeiou]/i.test(label) ? "an" : "a";
}
