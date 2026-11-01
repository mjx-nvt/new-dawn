import { Link } from "@tanstack/react-router";
import { useState, type KeyboardEvent } from "react";
import { asRecord, labelInput, recordId, recordLabel, useCreateRecord, useRecords, useUpdateRecord, type FieldMeta, type Json, type ObjectDef, type Query, type RecordJson } from "../../../kernel/bridge/records";
import styles from "./Boards.module.css";

const WEEKDAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const EVENTS = 1_000;

interface Props {
  slug: string;
  object: ObjectDef;
  /** The date or date-time field the records are placed by. */
  field: FieldMeta;
  query: Query;
  deleted: boolean;
  /** Opens an event's record in the side panel instead of on its page (D-062). */
  onOpen?: (id: string) => void;
}

/** Twenty's calendar view (D-061): records placed by a date field, a month (Monday to Sunday) or
 *  a week, dragged to another day to move them, "+" on a day to add one. */
export default function CalendarView({ slug, object, field, query, deleted, onOpen }: Props) {
  const name = object.meta.name_singular;
  const [anchor, setAnchor] = useState(() => startOfDay(new Date()));
  const [mode, setMode] = useState<"month" | "week">("month");
  const [dragging, setDragging] = useState<string | null>(null);
  const update = useUpdateRecord(slug, name);
  const create = useCreateRecord(slug, name);
  const first = mode === "month" ? mondayOnOrBefore(new Date(anchor.getFullYear(), anchor.getMonth(), 1)) : mondayOnOrBefore(anchor);
  const days = Array.from({ length: mode === "month" ? 42 : 7 }, (_, i) => addDays(first, i));
  // A date-time is an instant: the grid's bounds are the Mac's midnights, sent as instants.
  const from = field.type === "DATE" ? dayKey(first) : first.toISOString();
  const to = field.type === "DATE" ? dayKey(addDays(first, days.length)) : addDays(first, days.length).toISOString();
  const base = query.filter && typeof query.filter === "object" && query.filter !== null && "and" in query.filter ? (query.filter as { and: Json[] }).and : query.filter ? [query.filter] : [];
  const rangeQuery: Query = {
    ...query,
    filter: { and: [...base, { field: field.name, op: "gte", value: from }, { field: field.name, op: "lt", value: to }] },
    limit: EVENTS,
  };
  const page = useRecords(slug, name, rangeQuery);
  const records = (page.data?.records ?? []).map(asRecord);
  const byDay = new Map<string, RecordJson[]>();
  for (const r of records) {
    const value = r[field.name];
    if (typeof value !== "string" || !value) continue;
    const key = field.type === "DATE" ? value.slice(0, 10) : dayKey(new Date(value));
    byDay.set(key, [...(byDay.get(key) ?? []), r]);
  }
  const today = dayKey(new Date());
  const title = mode === "month" ? anchor.toLocaleDateString("en-GB", { month: "long", year: "numeric" }) : `${first.toLocaleDateString("en-GB", { day: "numeric", month: "short" })} – ${addDays(first, 6).toLocaleDateString("en-GB", { day: "numeric", month: "short", year: "numeric" })}`;
  const step = (n: number) => setAnchor(mode === "month" ? new Date(anchor.getFullYear(), anchor.getMonth() + n, 1) : addDays(anchor, 7 * n));

  /** Moves a record to a day: a date field takes the day; a date-time keeps its time of day. */
  const moveTo = (record: RecordJson, day: Date) => {
    const current = record[field.name];
    let value: string = dayKey(day);
    if (field.type === "DATE_TIME") {
      const was = typeof current === "string" && current ? new Date(current) : new Date(day.getFullYear(), day.getMonth(), day.getDate(), 9);
      const at = new Date(day.getFullYear(), day.getMonth(), day.getDate(), was.getHours(), was.getMinutes());
      value = at.toISOString();
    }
    update.mutate({ id: recordId(record), data: { [field.name]: value } });
  };
  const error = update.error ?? create.error;

  return (
    <>
      <div className={styles.calendarBar}>
        <button type="button" className="btn btnGhost btnCompact" aria-label="Previous" onClick={() => step(-1)}>
          ‹
        </button>
        <button type="button" className="btn btnGhost btnCompact" aria-label="Next" onClick={() => step(1)}>
          ›
        </button>
        <button type="button" className="btn btnSecondary btnCompact" onClick={() => setAnchor(startOfDay(new Date()))}>
          Today
        </button>
        <span className={styles.calendarTitle}>{title}</span>
        <div className="segmented" role="group" aria-label="Calendar range">
          <button type="button" aria-pressed={mode === "month"} onClick={() => setMode("month")}>
            Month
          </button>
          <button type="button" aria-pressed={mode === "week"} onClick={() => setMode("week")}>
            Week
          </button>
        </div>
        <span className="fieldHint">By {field.label.toLowerCase()}</span>
      </div>
      {error && (
        <p className="formError" role="alert">
          {error.message}
        </p>
      )}
      <div className={styles.grid} aria-label={`${object.meta.label_plural} by ${field.label}`}>
        {WEEKDAYS.map((d) => (
          <div key={d} className={styles.weekday}>
            {d}
          </div>
        ))}
        {days.map((day) => {
          const key = dayKey(day);
          const events = byDay.get(key) ?? [];
          const other = mode === "month" && day.getMonth() !== anchor.getMonth();
          return (
            <Day
            onOpen={onOpen}
              key={key}
              day={day}
              isToday={key === today}
              other={other}
              week={mode === "week"}
              events={events}
              object={object}
              deleted={deleted}
              dragging={dragging}
              timeOf={(r) => (field.type === "DATE_TIME" && typeof r[field.name] === "string" ? new Date(r[field.name] as string).toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" }) : "")}
              onDragStart={setDragging}
              onDragEnd={() => setDragging(null)}
              onDrop={() => {
                const record = records.find((r) => recordId(r) === dragging);
                if (record && !deleted && !events.some((r) => recordId(r) === dragging)) moveTo(record, day);
                setDragging(null);
              }}
              onAdd={(text, done) => create.mutate({ ...labelInput(object, text), [field.name]: field.type === "DATE" ? key : new Date(day.getFullYear(), day.getMonth(), day.getDate(), 9).toISOString() }, { onSuccess: done })}
            />
          );
        })}
      </div>
      {page.isError && <p className="ghostText">Couldn't be read — {page.error.message}</p>}
    </>
  );
}

interface DayProps {
  onOpen?: (id: string) => void;
  day: Date;
  isToday: boolean;
  other: boolean;
  week: boolean;
  events: RecordJson[];
  object: ObjectDef;
  deleted: boolean;
  dragging: string | null;
  timeOf: (r: RecordJson) => string;
  onDragStart: (id: string) => void;
  onDragEnd: () => void;
  onDrop: () => void;
  onAdd: (text: string, done: () => void) => void;
}

function Day({ day, isToday, other, week, events, object, deleted, dragging, timeOf, onDragStart, onDragEnd, onDrop, onAdd, onOpen }: DayProps) {
  const [over, setOver] = useState(false);
  const [adding, setAdding] = useState(false);
  const [draft, setDraft] = useState("");
  const shown = week ? events : events.slice(0, 4);
  const onKey = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Escape") setAdding(false);
    if (e.key === "Enter" && !e.nativeEvent.isComposing && draft.trim()) {
      onAdd(draft.trim(), () => {
        setDraft("");
        setAdding(false);
      });
    }
  };
  return (
    <div
      className={`${styles.day} ${week ? styles.dayWeek : ""} ${other ? styles.otherMonth : ""} ${over && dragging ? styles.dayOver : ""}`}
      aria-label={day.toLocaleDateString("en-GB", { weekday: "long", day: "numeric", month: "long" })}
      onDragOver={(e) => {
        e.preventDefault();
        setOver(true);
      }}
      onDragLeave={() => setOver(false)}
      onDrop={(e) => {
        e.preventDefault();
        setOver(false);
        onDrop();
      }}
    >
      <div className={styles.dayHead}>
        <span className={isToday ? styles.today : undefined}>{day.getDate()}</span>
        {!deleted && (
          <button type="button" className={styles.addDay} aria-label={`Add on ${day.toLocaleDateString("en-GB", { day: "numeric", month: "short" })}`} onClick={() => setAdding(true)}>
            +
          </button>
        )}
      </div>
      {shown.map((r) => {
        const id = recordId(r);
        const label = recordLabel(object, r) || "Unnamed";
        const time = timeOf(r);
        const inner = (
          <>
            {time && <span className={styles.eventTime}>{time}</span>}
            {label}
          </>
        );
        const common = { className: styles.event, draggable: !deleted, onDragStart: () => onDragStart(id), onDragEnd, title: label };
        return !deleted && onOpen ? (
          <div key={id} {...common} role="button" tabIndex={0} aria-label={label} onClick={() => onOpen(id)} onKeyDown={(e) => e.key === "Enter" && onOpen(id)}>
            {inner}
          </div>
        ) : !deleted ? (
          <Link key={id} to="/crm/$object/$recordId" params={{ object: object.meta.name_plural, recordId: id }} {...common}>
            {inner}
          </Link>
        ) : (
          <div key={id} {...common} aria-label={label}>
            {inner}
          </div>
        );
      })}
      {events.length > shown.length && <span className={styles.more}>+{events.length - shown.length} more</span>}
      {adding && <input className={`input ${styles.addEvent}`} autoFocus placeholder={`New ${object.meta.label_singular.toLowerCase()}…`} aria-label={`New ${object.meta.label_singular.toLowerCase()}`} value={draft} onChange={(e) => setDraft(e.target.value)} onKeyDown={onKey} onBlur={() => !draft && setAdding(false)} />}
    </div>
  );
}

function startOfDay(d: Date): Date {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate());
}

function addDays(d: Date, n: number): Date {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate() + n);
}

/** The Monday on or before the day. */
function mondayOnOrBefore(d: Date): Date {
  const day = startOfDay(d);
  const back = (day.getDay() + 6) % 7;
  return addDays(day, -back);
}

/** YYYY-MM-DD in the Mac's time zone. */
function dayKey(d: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}
