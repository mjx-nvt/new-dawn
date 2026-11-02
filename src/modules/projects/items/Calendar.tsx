import { useState } from "react";
import { useUpdatePmItem, type PmItem } from "../../../kernel/bridge/pmItems";
import { MONTHS, dayIn, dayString, formatDay, weekStartDay } from "../../../kernel/format/format";
import type { LayoutProps } from "./Board";
import { Dot } from "./Props";
import styles from "./Items.module.css";

const WEEKDAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/** A day moved by `n` days, as "YYYY-MM-DD". */
function shift(day: string, n: number): string {
  const d = dayIn(day);
  return dayString(new Date(d.getFullYear(), d.getMonth(), d.getDate() + n));
}

/** Plane's Calendar (D-076): the items by due date, a month or a week (remembered), starting on
 *  the owner's first day of the week; an item dragged to another day takes it as its due date
 *  (its start moves with it, keeping its length); + on a day makes an item due then. Items
 *  without a due date aren't shown — the count says how many. */
export default function Calendar({ slug, project, items, onPeek, onCreate, span, onSpan }: LayoutProps & { span: "month" | "week"; onSpan: (span: "month" | "week") => void }) {
  const update = useUpdatePmItem(slug);
  const [cursor, setCursor] = useState(() => {
    const d = new Date();
    return new Date(d.getFullYear(), d.getMonth(), d.getDate());
  });
  const [drag, setDrag] = useState<string | null>(null);
  const start = weekStartDay();
  const first = span === "month" ? new Date(cursor.getFullYear(), cursor.getMonth(), 1) : cursor;
  const back = (first.getDay() - start + 7) % 7;
  const days = Array.from({ length: span === "month" ? 42 : 7 }, (_, n) => new Date(first.getFullYear(), first.getMonth(), first.getDate() - back + n));
  const byDay = new Map<string, PmItem[]>();
  for (const i of items) if (i.target_date) byDay.set(i.target_date, [...(byDay.get(i.target_date) ?? []), i]);
  const undated = items.filter((i) => !i.target_date).length;
  const today = dayString(new Date());
  const ro = !!project.archived_at;
  const move = (n: number) => setCursor(span === "month" ? new Date(cursor.getFullYear(), cursor.getMonth() + n, 1) : new Date(cursor.getFullYear(), cursor.getMonth(), cursor.getDate() + 7 * n));
  const title = span === "month" ? `${MONTHS[cursor.getMonth()]} ${cursor.getFullYear()}` : `${formatDay(dayString(days[0]), { short: true })} – ${formatDay(dayString(days[6]))}`;

  const dropOn = (day: string) => {
    const item = items.find((i) => i.id === drag);
    setDrag(null);
    if (!item || item.target_date === day) return;
    const by = item.target_date ? Math.round((dayIn(day).getTime() - dayIn(item.target_date).getTime()) / 86_400_000) : 0;
    update.mutate({ id: item.id, patch: { target_date: day, ...(item.start_date && item.target_date ? { start_date: shift(item.start_date, by) } : {}) } });
  };

  return (
    <section className={styles.calendar} aria-label="Calendar">
      <div className={styles.calHead}>
        <button type="button" className="btn btnGhost btnCompact" aria-label={span === "month" ? "The month before" : "The week before"} onClick={() => move(-1)}>
          ‹
        </button>
        <button type="button" className="btn btnGhost btnCompact" onClick={() => setCursor(dayIn(today))}>
          Today
        </button>
        <button type="button" className="btn btnGhost btnCompact" aria-label={span === "month" ? "The month after" : "The week after"} onClick={() => move(1)}>
          ›
        </button>
        <h3 className={styles.h3}>{title}</h3>
        <span className={styles.spacer} />
        {undated > 0 && (
          <span className="fieldHint">
            {undated} without a due date {undated === 1 ? "isn't" : "aren't"} shown
          </span>
        )}
        <div className="segmented" role="group" aria-label="Calendar span">
          {(["month", "week"] as const).map((s) => (
            <button key={s} type="button" aria-pressed={span === s} onClick={() => onSpan(s)}>
              {s === "month" ? "Month" : "Week"}
            </button>
          ))}
        </div>
      </div>
      {update.error && (
        <p className="formError" role="alert">
          {update.error.message}
        </p>
      )}
      <div className={styles.weekdays} aria-hidden>
        {days.slice(0, 7).map((d) => (
          <span key={d.getDay()}>{WEEKDAYS[d.getDay()]}</span>
        ))}
      </div>
      <div className={`${styles.days} ${span === "week" ? styles.weekSpan : ""}`}>
        {days.map((d) => {
          const key = dayString(d);
          const on = byDay.get(key) ?? [];
          const other = span === "month" && d.getMonth() !== cursor.getMonth();
          return (
            <div key={key} className={`${styles.day} ${other ? styles.otherMonth : ""}`} data-day={key} onDragOver={(e) => e.preventDefault()} onDrop={() => dropOn(key)}>
              <div className={styles.dayHead}>
                <span className={key === today ? styles.todayNum : styles.dayNum}>{d.getDate()}</span>
                {!ro && (
                  <button type="button" className={`btn btnGhost btnCompact ${styles.dayAdd}`} aria-label={`New work item due ${key}`} onClick={() => onCreate({ target_date: key })}>
                    +
                  </button>
                )}
              </div>
              {on.map((i) => (
                <button key={i.id} type="button" className={`${styles.dayItem} ${drag === i.id ? styles.dragged : ""}`} draggable={!ro} onDragStart={() => setDrag(i.id)} onDragEnd={() => setDrag(null)} onClick={() => onPeek(i.id)} title={`${i.key} ${i.name}`}>
                  <Dot color={i.state_color} /> <span className={styles.key}>{i.key}</span> {i.name}
                </button>
              ))}
            </div>
          );
        })}
      </div>
    </section>
  );
}
