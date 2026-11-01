import { Link } from "@tanstack/react-router";
import { Fragment, useState, type KeyboardEvent, type ReactNode } from "react";
import { usePerson, type CommitmentStatus } from "../../../kernel/bridge/crm";
import { useAddLinked, useAttachments, useLinked, useTimeline, type LinkedKind, type TimelineEvent, type Widget } from "../../../kernel/bridge/layouts";
import { asRecord, hasColumns, recordId, recordLabel, tableFields, useObject, useUpdateRecord, type FieldMeta, type ObjectDef, type Query, type RecordJson } from "../../../kernel/bridge/records";
import { formatWhen } from "../../meetings/format";
import Commitments from "../../people/Commitments";
import { CellEditor, CellView, editsInPlace, readOnly } from "../cells";
import RecordTable, { type Column } from "../RecordTable";
import styles from "./Record.module.css";

/** What every widget of a record page reads: the workspace, the object and the record. */
export interface WidgetContext {
  slug: string;
  object: ObjectDef;
  record: RecordJson;
}

/** Twenty's widget types (D-062); the ones without `since` work now. */
export const WIDGET_TYPES: { type: string; label: string; since?: string }[] = [
  { type: "FIELDS", label: "Fields" },
  { type: "RELATED", label: "Related records" },
  { type: "TIMELINE", label: "Timeline" },
  { type: "TASKS", label: "Tasks" },
  { type: "NOTES", label: "Notes" },
  { type: "FILES", label: "Files" },
  { type: "RICH_TEXT", label: "Rich text" },
  { type: "COMMITMENTS", label: "Commitments" },
  { type: "MEETINGS", label: "Meetings" },
  { type: "CHART", label: "Chart", since: "the dashboards release" },
  { type: "EMAILS", label: "Emails", since: "the emails release" },
  { type: "CALENDAR", label: "Calendar", since: "the emails release" },
  { type: "IFRAME", label: "iFrame", since: "the dashboards release" },
];

export function typeLabel(type: string): string {
  return WIDGET_TYPES.find((t) => t.type === type)?.label ?? type;
}

export function configOf(widget: Widget): Record<string, unknown> {
  return asRecord(widget.config);
}

const names = (v: unknown): string[] => (Array.isArray(v) ? v.filter((x): x is string => typeof x === "string") : []);

export function isDeleted(record: RecordJson): boolean {
  return typeof record.deletedAt === "string" && record.deletedAt !== "";
}

/** The fields a Fields widget shows: the configured names in their order, else every field but
 *  the label (it is the page's title) and the system ones. */
export function fieldsOf(object: ObjectDef, config: Record<string, unknown>): FieldMeta[] {
  const all = object.fields.filter((f) => f.is_active && !f.is_system && f.type !== "POSITION" && hasColumns(f));
  const wanted = names(config.fields);
  if (wanted.length) return wanted.flatMap((n) => all.filter((f) => f.name === n));
  return all.filter((f) => f.id !== object.meta.label_identifier_field_id);
}

/** Nothing captured: null, an empty text, an empty list, a composite whose parts are all empty. */
export function blankValue(v: unknown): boolean {
  if (v == null || v === "") return true;
  if (Array.isArray(v)) return v.length === 0;
  if (typeof v === "object") return Object.values(v as Record<string, unknown>).every(blankValue);
  return false;
}

/** A value as the timeline says it. */
function say(v: unknown): string {
  if (v == null || v === "") return "—";
  if (Array.isArray(v)) return v.map(say).join(", ") || "—";
  if (typeof v === "object") {
    const r = v as Record<string, unknown>;
    if (typeof r.label === "string") return r.label || "—";
    const parts = Object.values(r)
      .map(say)
      .filter((x) => x !== "—");
    return parts.join(" ") || "—";
  }
  if (typeof v === "boolean") return v ? "Yes" : "No";
  return String(v);
}

/** A widget's card, placed on the grid by its columns and rows. */
export function WidgetCard({ widget, tools, after, className, children }: { widget: Widget; tools?: ReactNode; after?: ReactNode; className?: string; children: ReactNode }) {
  return (
    <section className={className ? `${styles.widget} ${className}` : styles.widget} style={{ gridColumn: `${widget.grid_x + 1} / span ${widget.grid_w}`, gridRow: `${widget.grid_y + 1} / span ${widget.grid_h}` }} aria-label={widget.title || typeLabel(widget.type)}>
      <div className={styles.widgetHead}>
        <h2 className={styles.widgetTitle}>{widget.title || typeLabel(widget.type)}</h2>
        {tools}
      </div>
      <div className={styles.widgetBody}>{children}</div>
      {after}
    </section>
  );
}

function Ghost({ text, rows = 2 }: { text: string; rows?: number }) {
  return (
    <div className={styles.list}>
      <p className="ghostText">{text}</p>
      {Array.from({ length: rows }, (_, i) => (
        <div key={i} className={`${styles.ghostRow} ghostCard`} />
      ))}
    </div>
  );
}

/** What a widget shows, by type. */
export function WidgetBody({ ctx, widget }: { ctx: WidgetContext; widget: Widget }) {
  const config = configOf(widget);
  switch (widget.type) {
    case "FIELDS":
      return <FieldsWidget ctx={ctx} fields={fieldsOf(ctx.object, config)} />;
    case "RELATED":
      return <RelatedWidget ctx={ctx} config={config} />;
    case "TIMELINE":
      return <TimelineWidget ctx={ctx} />;
    case "TASKS":
      return <LinkedWidget ctx={ctx} kind="task" />;
    case "NOTES":
      return <LinkedWidget ctx={ctx} kind="note" />;
    case "FILES":
      return <FilesWidget ctx={ctx} />;
    case "RICH_TEXT":
      return <p className={styles.rich}>{typeof config.text === "string" && config.text ? config.text : <span className="ghostText">Nothing written yet — the layout editor sets this text</span>}</p>;
    case "COMMITMENTS":
      return <CommitmentsWidget ctx={ctx} />;
    case "MEETINGS":
      return <MeetingsWidget ctx={ctx} />;
    default:
      return <Ghost text={`${typeLabel(widget.type)} widgets arrive with ${WIDGET_TYPES.find((t) => t.type === widget.type)?.since ?? "a later release"}`} />;
  }
}

/** Label · value rows; a value edits in place like a table cell; empty ones read
 *  "Not yet captured". */
export function FieldsWidget({ ctx, fields, locked }: { ctx: WidgetContext; fields: FieldMeta[]; locked?: boolean }) {
  const { slug, object, record } = ctx;
  const update = useUpdateRecord(slug, object.meta.name_singular);
  const [editing, setEditing] = useState<string | null>(null);
  const frozen = !!locked || isDeleted(record);
  const id = recordId(record);
  const save = (field: FieldMeta, value: unknown) => {
    setEditing(null);
    update.mutate({ id, data: { [field.name]: value } });
  };
  if (!fields.length) return <Ghost text="No fields chosen — the layout editor picks them" />;
  return (
    <>
      {update.error && (
        <p className="formError" role="alert">
          {update.error.message}
        </p>
      )}
      <div className={styles.fields}>
        {fields.map((f) => {
          const editable = !frozen && !readOnly(f);
          return (
            <Fragment key={f.id}>
              <span className={styles.fieldLabel}>{f.label}</span>
              {editing === f.name ? (
                <div className={`${styles.fieldValue} ${styles.fieldEditing}`}>
                  <CellEditor slug={slug} field={f} record={record} onSave={(v) => save(f, v)} onCancel={() => setEditing(null)} />
                </div>
              ) : editsInPlace(f) ? (
                <div className={styles.fieldValue}>
                  <input type="checkbox" aria-label={f.label} checked={record[f.name] === true} disabled={frozen} onChange={(e) => save(f, e.target.checked)} />
                </div>
              ) : (
                <div
                  className={editable ? `${styles.fieldValue} ${styles.fieldValueEditable}` : styles.fieldValue}
                  role={editable ? "button" : undefined}
                  tabIndex={editable ? 0 : undefined}
                  aria-label={editable ? `Edit ${f.label.toLowerCase()}` : undefined}
                  onClick={() => editable && setEditing(f.name)}
                  onKeyDown={(e) => e.key === "Enter" && editable && setEditing(f.name)}
                >
                  {blankValue(record[f.name]) ? <span className="ghostText">Not yet captured</span> : <CellView field={f} record={record} />}
                </div>
              )}
            </Fragment>
          );
        })}
      </div>
    </>
  );
}

/** The records on the other side of a one-to-many relation, as a table; "Allow editing" off
 *  makes it read-only; a record added there is linked to this one. */
function RelatedWidget({ ctx, config }: { ctx: WidgetContext; config: Record<string, unknown> }) {
  const { slug, object, record } = ctx;
  const field = typeof config.field === "string" ? object.fields.find((f) => f.name === config.field) : undefined;
  const relation = field?.options.relation;
  const target = useObject(slug, relation?.target);
  if (!field || !relation?.inverse) return <Ghost text="Pick the relation in the layout editor" />;
  if (!target.data) return <Ghost text={target.isError ? `Couldn't be read — ${target.error.message}` : "Not yet captured"} />;
  const t = target.data;
  const chosen = names(config.fields);
  const base = tableFields(t);
  const shown = chosen.length ? chosen.flatMap((n) => base.filter((f) => f.name === n)) : base.filter((f) => f.name !== relation.inverse).slice(0, 5);
  const columns: Column[] = shown.map((f) => ({ field: f, size: 180, aggregate: null }));
  const id = recordId(record);
  const query: Query = { filter: { field: relation.inverse, sub: null, op: "is", value: [id] }, sort: [], search: null, deleted: false, limit: null, offset: null };
  const none = () => undefined;
  return <RecordTable slug={slug} object={t} columns={columns} query={query} deleted={false} compact locked={config.editable === false || isDeleted(record)} defaults={{ [relation.inverse]: id }} onResize={none} onReorder={none} onSort={none} onHide={none} onFilter={none} onAggregate={none} />;
}

function describe(a: TimelineEvent, object: ObjectDef): string {
  const event = a.name.slice(a.name.indexOf(".") + 1);
  const label = object.meta.label_singular.toLowerCase();
  switch (event) {
    case "created":
      return `Made the ${label}`;
    case "updated":
      return "Changed";
    case "deleted":
      return "Moved to Deleted records";
    case "restored":
      return "Restored";
    default:
      return event.replace(/[-_.]/g, " ");
  }
}

function diffLines(a: TimelineEvent): string[] {
  const diff = asRecord(asRecord(a.properties).diff);
  return Object.entries(diff).map(([field, change]) => {
    const c = asRecord(change);
    return `${typeof c.label === "string" ? c.label : field}: ${say(c.before)} → ${say(c.after)}`;
  });
}

function TimelineWidget({ ctx }: { ctx: WidgetContext }) {
  const { slug, object, record } = ctx;
  const events = useTimeline(slug, object.meta.name_singular, recordId(record));
  const list = events.data ?? [];
  if (!list.length) return <Ghost text={events.isError ? `Couldn't be read — ${events.error.message}` : "Not yet captured"} />;
  return (
    <ol className={styles.timeline}>
      {list.map((a) => {
        const lines = diffLines(a);
        return (
          <li key={a.id} className={styles.event}>
            <div>
              <span className={styles.eventWhat}>{describe(a, object)}</span>
              <span> · {a.actor}</span>
              {lines.length > 0 && (
                <ul className={styles.diff}>
                  {lines.map((line, i) => (
                    <li key={i}>{line}</li>
                  ))}
                </ul>
              )}
            </div>
            <span className={styles.eventWhen}>{formatWhen(a.happens_at)}</span>
          </li>
        );
      })}
    </ol>
  );
}

/** The tasks or notes that target the record, and a field that adds one. */
function LinkedWidget({ ctx, kind }: { ctx: WidgetContext; kind: LinkedKind }) {
  const { slug, object, record } = ctx;
  const id = recordId(record);
  const items = useLinked(slug, object.meta.name_singular, id, kind);
  const target = useObject(slug, kind);
  const add = useAddLinked(slug, object.meta.name_singular, id);
  const [draft, setDraft] = useState("");
  const onKey = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key !== "Enter" || e.nativeEvent.isComposing || !draft.trim()) return;
    add.mutate({ kind, title: draft.trim() }, { onSuccess: () => setDraft("") });
  };
  const t = target.data;
  const extras = t ? ["status", "dueAt"].flatMap((n) => t.fields.filter((f) => f.name === n)) : [];
  const list = items.data ?? [];
  const what = kind === "task" ? "task" : "note";
  return (
    <>
      {list.length ? (
        <ul className={styles.list} role="list">
          {list.map((r) => (
            <li key={recordId(r)} className={styles.item}>
              {t ? (
                <Link to="/crm/$object/$recordId" params={{ object: t.meta.name_plural, recordId: recordId(r) }} className={styles.itemLink}>
                  {recordLabel(t, r) || "Untitled"}
                </Link>
              ) : (
                <span className={styles.itemLink}>{typeof r.title === "string" ? r.title : "Untitled"}</span>
              )}
              {extras.map((f) => (
                <span key={f.id} className={styles.itemMeta}>
                  <CellView field={f} record={r} />
                </span>
              ))}
            </li>
          ))}
        </ul>
      ) : (
        <Ghost text={items.isError ? `Couldn't be read — ${items.error.message}` : "Not yet captured"} />
      )}
      {add.error && (
        <p className="formError" role="alert">
          {add.error.message}
        </p>
      )}
      {!isDeleted(record) && (
        <div className={styles.addRow}>
          <input className={`input ${styles.addInput}`} placeholder={`Add a ${what}…`} aria-label={`Add a ${what}`} value={draft} disabled={add.isPending} onChange={(e) => setDraft(e.target.value)} onKeyDown={onKey} />
        </div>
      )}
    </>
  );
}

function FilesWidget({ ctx }: { ctx: WidgetContext }) {
  const { slug, object, record } = ctx;
  const files = useAttachments(slug, object.meta.name_singular, recordId(record));
  const list = files.data ?? [];
  if (!list.length) return <Ghost text={files.isError ? `Couldn't be read — ${files.error.message}` : "Not yet captured"} />;
  return (
    <ul className={styles.list} role="list">
      {list.map((f) => (
        <li key={f.id} className={styles.item}>
          <span className={styles.itemLink}>{f.name || f.full_path}</span>
          {f.type && <span className={styles.itemMeta}>{f.type}</span>}
          <span className={styles.itemMeta}>{formatWhen(f.created_at)}</span>
        </li>
      ))}
    </ul>
  );
}

/** A person's commitments (D-055): waiting for you, what they owe you, kept and dropped. */
function CommitmentsWidget({ ctx }: { ctx: WidgetContext }) {
  const { slug, object, record } = ctx;
  const person = object.meta.name_singular === "person";
  const dossier = usePerson(person ? slug : undefined, recordId(record));
  if (!person) return <Ghost text="Commitments are kept on a person" />;
  const of = (statuses: CommitmentStatus[]) => (dossier.data?.commitments ?? []).filter((c) => statuses.includes(c.status));
  return (
    <div className={styles.stack}>
      <h3 className={styles.subTitle}>Waiting for you</h3>
      <Commitments slug={slug} items={of(["proposed"])} showWho={false} empty="Not yet captured" />
      <h3 className={styles.subTitle}>What they owe you</h3>
      <Commitments slug={slug} items={of(["open"])} showWho={false} empty="Not yet captured — confirmed commitments show here until kept or dropped" />
      <h3 className={styles.subTitle}>Kept and dropped</h3>
      <Commitments slug={slug} items={of(["kept", "dropped"])} showWho={false} empty="Not yet captured" />
    </div>
  );
}

/** The summarised meetings that name a person. */
function MeetingsWidget({ ctx }: { ctx: WidgetContext }) {
  const { slug, object, record } = ctx;
  const person = object.meta.name_singular === "person";
  const dossier = usePerson(person ? slug : undefined, recordId(record));
  if (!person) return <Ghost text="Meetings are kept on a person" />;
  const meetings = dossier.data?.meetings ?? [];
  if (!meetings.length) return <Ghost text="Not yet captured — the summarised meetings that name them show here" />;
  return (
    <ul className={styles.list} role="list">
      {meetings.map((m) => (
        <li key={m.id} className={styles.item}>
          <Link to="/meetings/$meetingId" params={{ meetingId: m.id }} search={{ tab: "summary" }} className={styles.itemLink}>
            {m.title}
          </Link>
          <span className={styles.itemMeta}>
            {m.attended ? "Attended" : "Involved"} · {formatWhen(m.held_on)}
          </span>
        </li>
      ))}
    </ul>
  );
}
