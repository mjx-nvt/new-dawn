import { convertFileSrc } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { Link } from "@tanstack/react-router";
import { Fragment, useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { usePerson, type CommitmentStatus } from "../../../kernel/bridge/crm";
import { isImage, openAttachment, pickFiles, revealAttachment, saveAttachmentAs, useAddAttachments, useDeleteAttachment } from "../../../kernel/bridge/files";
import { useAddLinked, useAttachments, useLinked, useTimeline, type LinkedKind, type TimelineEvent, type Widget } from "../../../kernel/bridge/layouts";
import { draftEmail, openInMail, useAddPersonFromEmail, useBlockHandle, useCalendarEvents, useEmailMessages, useEmailThreads, useRefreshEmailThreads, type EmailThread } from "../../../kernel/bridge/mailbox";
import { asRecord, hasColumns, recordId, recordLabel, tableFields, useObject, useUpdateRecord, type FieldMeta, type ObjectDef, type Query, type RecordJson } from "../../../kernel/bridge/records";
import Editor from "../../../kernel/editor/Editor";
import { richExcerpt } from "../../../kernel/editor/plain";
import MenuButton from "../../../kernel/shell/MenuButton";
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
  { type: "EMAILS", label: "Emails" },
  { type: "CALENDAR", label: "Calendar" },
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
    case "EMAILS":
      return <EmailsWidget ctx={ctx} />;
    case "CALENDAR":
      return <CalendarWidget ctx={ctx} />;
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
          if (f.type === "RICH_TEXT") {
            // The body: the editor across both columns, saving as it goes (0.10.6).
            // Keyed by the record too: ‹ › to another record remounts the editor with its body.
            return (
              <div key={`${id}:${f.id}`} className={styles.fieldWide}>
                <span className={styles.fieldLabel}>{f.label}</span>
                <Editor value={record[f.name]} placeholder={`Write the ${f.label.toLowerCase()}…`} readOnly={frozen} home={{ slug, object: object.meta.name_plural, recordId: id }} onChange={(json) => update.mutate({ id, data: { [f.name]: json } })} />
              </div>
            );
          }
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
  // A system junction (a note's or task's targets) has only system fields: its relations are the columns.
  const base = t.meta.is_system ? t.fields.filter((f) => f.is_active && f.type === "RELATION" && hasColumns(f)) : tableFields(t);
  const shown = chosen.length ? chosen.flatMap((n) => base.filter((f) => f.name === n)) : base.filter((f) => f.name !== relation.inverse).slice(0, 6);
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
              {meetingOf(a) && (
                <span>
                  {" · from "}
                  <Link to="/meetings/$meetingId" params={{ meetingId: meetingOf(a)?.id ?? "" }} search={{ tab: "summary" }} className={styles.inlineLink}>
                    {meetingOf(a)?.title}
                  </Link>
                </span>
              )}
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
                  {richExcerpt(r.body, 70) && <span className={styles.itemExcerpt}> — {richExcerpt(r.body, 70)}</span>}
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

/** The record's files (0.10.6): dropped on the widget or picked in the dialog, copied into the
 *  workspace; a picture shows as a thumbnail; ⋯ opens, reveals in Finder, saves a copy or
 *  deletes (to Deleted records). */
function FilesWidget({ ctx }: { ctx: WidgetContext }) {
  const { slug, object, record } = ctx;
  const name = object.meta.name_singular;
  const id = recordId(record);
  const files = useAttachments(slug, name, id);
  const add = useAddAttachments(slug, name, id);
  const remove = useDeleteAttachment(slug, name, id);
  const [over, setOver] = useState(false);
  const [trouble, setTrouble] = useState<string | null>(null);
  const zone = useRef<HTMLDivElement>(null);
  const addFiles = add.mutate; // stable across renders (TanStack Query)
  const frozen = isDeleted(record) || !id;
  // Files dragged from the Finder: the window's drag-drop event, kept when it lands on this widget.
  useEffect(() => {
    if (frozen) return;
    let unlisten: (() => void) | undefined;
    let gone = false;
    // On macOS wry hands the drop point through in logical pixels (the NSPoint as it is), so
    // it compares with the widget's rect as it is — no device-pixel scaling.
    const inside = (x: number, y: number) => {
      const r = zone.current?.getBoundingClientRect();
      return !!r && x >= r.left && x <= r.right && y >= r.top && y <= r.bottom;
    };
    void getCurrentWebview()
      .onDragDropEvent((event) => {
        const p = event.payload;
        if (p.type === "over") setOver(inside(p.position.x, p.position.y));
        else if (p.type === "drop") {
          setOver(false);
          if (p.paths.length && inside(p.position.x, p.position.y)) addFiles(p.paths);
        } else setOver(false);
      })
      .then((fn) => {
        if (gone) fn();
        else unlisten = fn;
      });
    return () => {
      gone = true;
      unlisten?.();
    };
  }, [frozen, addFiles]);
  const act = (work: Promise<unknown>) => {
    setTrouble(null);
    work.catch((e: Error) => setTrouble(e.message));
  };
  const list = files.data ?? [];
  const error = trouble ?? add.error?.message ?? remove.error?.message ?? (files.isError ? files.error.message : null);
  return (
    <div ref={zone} className={over ? `${styles.dropZone} ${styles.dropOver}` : styles.dropZone}>
      {error && (
        <p className="formError" role="alert">
          {error}
        </p>
      )}
      {list.length ? (
        <ul className={styles.list} role="list">
          {list.map((f) => (
            <li key={f.id} className={styles.item}>
              {isImage(f.type) ? <img className={styles.thumb} src={convertFileSrc(f.full_path)} alt="" /> : <span className={styles.fileMark}>{(f.type || "file").slice(0, 4)}</span>}
              <button type="button" className={`${styles.itemLink} ${styles.itemButton}`} onClick={() => act(openAttachment(slug, f.id))}>
                {f.name || f.full_path}
              </button>
              <span className={styles.itemMeta}>{formatWhen(f.created_at)}</span>
              <MenuButton
                label={`Actions for ${f.name}`}
                className="btn btnGhost btnCompact"
                items={[
                  { label: "Open", onSelect: () => act(openAttachment(slug, f.id)) },
                  { label: "Reveal in Finder", onSelect: () => act(revealAttachment(slug, f.id)) },
                  { label: "Save as…", onSelect: () => act(saveAttachmentAs(slug, f.id, f.name)) },
                  { label: "Delete", danger: true, separator: true, disabled: frozen, onSelect: () => remove.mutate(f.id) },
                ]}
              >
                ⋯
              </MenuButton>
            </li>
          ))}
        </ul>
      ) : (
        <Ghost text={over ? "Drop to add" : "Not yet captured — drop files here, or add them"} />
      )}
      {!frozen && (
        <div className={styles.addRow}>
          <button type="button" className="btn btnSecondary btnCompact" disabled={add.isPending} onClick={() => act(pickFiles().then((paths) => paths.length && add.mutate(paths)))}>
            {add.isPending ? "Adding…" : "Add file…"}
          </button>
          {over && <span className="fieldHint"> Drop to add</span>}
        </div>
      )}
      {list.length > 0 && remove.isPending && <span className="fieldHint">Deleting…</span>}
    </div>
  );
}

/** A task made from a meeting's action item (0.10.6): the timeline names the meeting. */
function meetingOf(a: TimelineEvent): { id: string; title: string } | null {
  const m = asRecord(asRecord(a.properties).meeting);
  return typeof m.id === "string" && m.id ? { id: m.id, title: typeof m.title === "string" ? m.title : "a meeting" } : null;
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
const MAIL_OBJECTS = ["person", "company", "opportunity"];

/** Twenty's Emails tab (D-066): the record's threads from Apple Mail, newest first — a thread
 *  opens to its messages, each opens in Mail; people the CRM doesn't know yet are proposed. */
function EmailsWidget({ ctx }: { ctx: WidgetContext }) {
  const { slug, object, record } = ctx;
  const kind = object.meta.name_singular;
  const id = recordId(record);
  const applies = MAIL_OBJECTS.includes(kind);
  const threads = useEmailThreads(applies ? slug : undefined, kind, id);
  const refresh = useRefreshEmailThreads(slug, kind, id);
  const addPerson = useAddPersonFromEmail(slug);
  const block = useBlockHandle(slug);
  const [open, setOpen] = useState<string | null>(null);
  const [trouble, setTrouble] = useState<string | null>(null);
  if (!applies) return <Ghost text="Emails are kept on people, companies and opportunities" />;
  const emails = asRecord(record.emails);
  const address = kind === "person" && typeof emails.primaryEmail === "string" && emails.primaryEmail ? emails.primaryEmail : null;
  const data = threads.data;
  const fail = (e: Error) => setTrouble(e.message);
  return (
    <div className={styles.stack}>
      <div className={styles.mailHead}>
        {data?.refreshed_at && <span className={styles.itemMeta}>Read from Mail {formatWhen(data.refreshed_at)}</span>}
        <button type="button" className="btn btnGhost btnCompact" disabled={refresh.isPending || threads.isFetching} aria-busy={refresh.isPending || threads.isFetching} onClick={() => refresh.mutate(undefined, { onError: fail })}>
          Refresh
        </button>
        {address && (
          <button type="button" className="btn btnSecondary btnCompact" onClick={() => draftEmail(address).catch(fail)}>
            Send email
          </button>
        )}
      </div>
      {trouble && (
        <p className="formError" role="alert">
          {trouble}
        </p>
      )}
      {threads.error && (
        <p className="formError" role="alert">
          {threads.error.message}
        </p>
      )}
      {data && data.mail_state !== "ok" && <p className="fieldHint">{data.note}</p>}
      {!data ? (
        <Ghost text={threads.isPending ? "Reading from Mail…" : "Not yet captured"} />
      ) : data.threads.length === 0 ? (
        <Ghost text={data.mail_state === "ok" ? data.note || "Not yet captured — the threads with this record's addresses show here" : "Not yet captured"} />
      ) : (
        <ul className={styles.list} role="list">
          {data.threads.map((t) => (
            <li key={t.id} className={styles.mailThread}>
              <div className={styles.mailRow}>
                <button type="button" className={styles.itemButton} aria-expanded={open === t.id} onClick={() => setOpen(open === t.id ? null : t.id)}>
                  <span className={styles.mailSubject}>{t.subject || "(no subject)"}</span>
                </button>
                <span className={styles.itemMeta}>
                  {formatWhen(t.last_message_at)} · {t.count === 1 ? "1 message" : `${t.count} messages`}
                </span>
                <button type="button" className="btn btnGhost btnCompact" disabled={!t.latest_message_id} onClick={() => openInMail(t.latest_message_id).catch(fail)}>
                  Open in Mail
                </button>
              </div>
              <span className={styles.itemMeta}>
                <People thread={t} />
              </span>
              {t.snippet && <span className={styles.mailSnippet}>{t.snippet}</span>}
              {open === t.id && <ThreadMessages slug={slug} threadId={t.id} onTrouble={fail} />}
            </li>
          ))}
        </ul>
      )}
      {data && data.unknown.length > 0 && (
        <>
          <p className={styles.subTitle}>Not in the CRM</p>
          <ul className={styles.list} role="list">
            {data.unknown.map((u) => (
              <li key={u.handle} className={styles.item}>
                <span className={styles.itemExcerpt}>{u.name ? `${u.name} · ${u.handle}` : u.handle}</span>
                <span className={styles.mailActions}>
                  <button type="button" className="btn btnSecondary btnCompact" disabled={addPerson.isPending} onClick={() => addPerson.mutate({ handle: u.handle, name: u.name }, { onError: fail })}>
                    Add person
                  </button>
                  <button type="button" className="btn btnGhost btnCompact" disabled={block.isPending} onClick={() => block.mutate(u.handle, { onError: fail })}>
                    Block
                  </button>
                </span>
              </li>
            ))}
          </ul>
        </>
      )}
    </div>
  );
}

/** A thread's people: a name links to its record (a person, else the company of its domain). */
function People({ thread }: { thread: EmailThread }) {
  return (
    <>
      {thread.participants.map((p, i) => (
        <Fragment key={p.handle}>
          {i > 0 && ", "}
          {p.record ? (
            <Link to="/crm/$object/$recordId" params={{ object: p.record.object, recordId: p.record.id }} className={styles.inlineLink}>
              {p.name || p.handle}
            </Link>
          ) : (
            p.name || p.handle
          )}
        </Fragment>
      ))}
    </>
  );
}

function ThreadMessages({ slug, threadId, onTrouble }: { slug: string; threadId: string; onTrouble: (e: Error) => void }) {
  const messages = useEmailMessages(slug, threadId);
  const list = messages.data ?? [];
  return (
    <ul className={styles.mailMessages} role="list">
      {list.map((m) => (
        <li key={m.id} className={styles.mailRow}>
          <span className={styles.itemExcerpt}>
            <span className={styles.mailSubject}>{m.sender_name || m.sender_handle}</span> · {formatWhen(m.received_at)} · {m.direction === "out" ? "Sent" : "Received"}
            {m.snippet && <span className={styles.mailSnippet}> — {m.snippet}</span>}
          </span>
          <button type="button" className="btn btnGhost btnCompact" onClick={() => openInMail(m.message_id).catch(onTrouble)}>
            Open in Mail
          </button>
        </li>
      ))}
      {list.length === 0 && (
        <li>
          <span className="ghostText">{messages.isPending ? "Reading…" : "No messages kept"}</span>
        </li>
      )}
    </ul>
  );
}

/** Twenty's Calendar tab (D-066): the record's meetings as its events — a person's, a
 *  company's people's, an opportunity's company's — newest first, with their attendees and
 *  whether the recording is kept. */
function CalendarWidget({ ctx }: { ctx: WidgetContext }) {
  const { slug, object, record } = ctx;
  const kind = object.meta.name_singular;
  const applies = MAIL_OBJECTS.includes(kind);
  const events = useCalendarEvents(applies ? slug : undefined, kind, recordId(record));
  if (!applies) return <Ghost text="The calendar is kept on people, companies and opportunities" />;
  const list = events.data ?? [];
  if (!list.length) return <Ghost text={events.isPending ? "Reading…" : "Not yet captured — the summarised meetings with these people show here"} />;
  return (
    <ul className={styles.list} role="list">
      {list.map((e) => (
        <li key={e.meeting_id} className={styles.mailThread}>
          <div className={styles.mailRow}>
            <Link to="/meetings/$meetingId" params={{ meetingId: e.meeting_id }} search={{ tab: "summary" }} className={styles.itemLink}>
              {e.title}
            </Link>
            <span className={styles.itemMeta}>
              {formatWhen(e.held_on)}
              {e.duration_s ? ` · ${Math.max(1, Math.round(e.duration_s / 60))} min` : ""}
            </span>
            {e.has_recording && <span className="chip">Recording</span>}
          </div>
          <span className={styles.itemMeta}>
            {e.attendees.map((a, i) => (
              <Fragment key={`${a.name}-${i}`}>
                {i > 0 && ", "}
                {a.person_id ? (
                  <Link to="/crm/$object/$recordId" params={{ object: "people", recordId: a.person_id }} className={styles.inlineLink}>
                    {a.name}
                  </Link>
                ) : (
                  a.name
                )}
              </Fragment>
            ))}
          </span>
        </li>
      ))}
    </ul>
  );
}

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
