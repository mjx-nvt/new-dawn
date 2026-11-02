import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent, type ReactNode } from "react";
import { MONTHS, dayIn, dayString, formatDay, weekStartDay } from "../../../kernel/format/format";
import styles from "./Items.module.css";

const DAY = 86_400_000;
/** Points a day at each zoom. */
const PX: Record<Zoom, number> = { week: 40, month: 16, quarter: 5 };
/** The axis reaches at most this many days either side of today. */
const REACH = 365;
/** Half the one-date mark's width (`.mark`, 12 pt). */
const MARK_HALF = 6;
export type Zoom = "week" | "month" | "quarter";
type Edge = "move" | "start" | "end";

const plus = (day: string, n: number) => {
  const d = dayIn(day);
  return dayString(new Date(d.getFullYear(), d.getMonth(), d.getDate() + n));
};
const between = (a: string, b: string) => Math.round((dayIn(b).getTime() - dayIn(a).getTime()) / DAY);

/** A bar: from its start to its end; one date, a mark on that day. */
export interface GanttRow {
  id: string;
  start: string | null;
  end: string | null;
  /** The bar's colour, as CSS. */
  color: string;
  label: ReactNode;
  /** Its name, for the label's tooltip and the bar's description. */
  name: string;
  onOpen: () => void;
  /** Not draggable (an archived module). */
  readOnly?: boolean;
}

const ends = (r: GanttRow, today: string): [string, string] => [r.start ?? r.end ?? today, r.end ?? r.start ?? today];

/** The days the rows need — a week before the earliest date to four weeks after the latest,
 *  held within a year of today either way (today always in) — the rows in them, and the rest
 *  (no dates, or too far from today). Days as YYYY-MM-DD compare as text. */
export function frame<T extends GanttRow>(rows: T[]): { from: string; to: string; shown: T[]; away: T[] } {
  const today = dayString(new Date());
  const dated = rows.filter((r) => r.start || r.end);
  const days = dated.flatMap((r) => ends(r, today)).sort();
  const min = (a: string, b: string) => (a < b ? a : b);
  const max = (a: string, b: string) => (a > b ? a : b);
  const from = min(plus(today, -7), max(plus(days[0] ?? today, -7), plus(today, -REACH)));
  const to = max(plus(today, 28), min(plus(days[days.length - 1] ?? today, 28), plus(today, REACH)));
  const shown = dated.filter((r) => ends(r, today)[1] >= from && ends(r, today)[0] <= to);
  return { from, to, shown, away: [...rows.filter((r) => !r.start && !r.end), ...dated.filter((r) => !shown.includes(r))] };
}

/** The timeline's day axis and bars (D-076; modules', D-079): today's line in orange; a bar
 *  dragged moves both dates, its ends move one; Escape or leaving the window drops a drag; a
 *  release reports the new days. */
export default function Gantt({ rows, from, to, zoom, readOnly, empty, onMove }: { rows: GanttRow[]; from: string; to: string; zoom: Zoom; readOnly: boolean; empty: string; onMove: (row: GanttRow, start: string, end: string) => void }) {
  const [drag, setDrag] = useState<{ id: string; edge: Edge; x0: number; delta: number } | null>(null);
  const done = useRef<(() => void) | null>(null);
  const px = PX[zoom];
  const today = dayString(new Date());
  const span = between(from, to);
  const x = (day: string) => between(from, day) * px;

  /** The dates a bar shows: its own, or as the drag has them. */
  const dates = (r: GanttRow): [string, string] => {
    const [s, e] = ends(r, today);
    if (!drag || drag.id !== r.id) return [s, e];
    const d = drag.delta;
    if (drag.edge === "move") return [plus(s, d), plus(e, d)];
    if (drag.edge === "start") return [plus(s, Math.min(d, between(s, e))), e];
    return [s, plus(e, Math.max(d, -between(s, e)))];
  };

  // While dragging: follow the pointer (a redraw only when the day changes); release reports the
  // new dates; Escape, a cancelled pointer or leaving the window drops the drag.
  useEffect(() => {
    if (!drag) return;
    const onPointer = (e: PointerEvent) =>
      setDrag((d) => {
        if (!d) return d;
        const delta = Math.round((e.clientX - d.x0) / px);
        return delta === d.delta ? d : { ...d, delta };
      });
    const onUp = () => done.current?.();
    const cancel = () => setDrag(null);
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && cancel();
    window.addEventListener("pointermove", onPointer);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", cancel);
    window.addEventListener("blur", cancel);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("pointermove", onPointer);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("pointercancel", cancel);
      window.removeEventListener("blur", cancel);
      window.removeEventListener("keydown", onKey);
    };
  }, [drag?.id, px]);

  done.current = () => {
    const d = drag;
    setDrag(null);
    const row = d && rows.find((r) => r.id === d.id);
    if (!d || !row || !d.delta) return;
    const [s, e] = dates(row);
    onMove(row, s, e);
  };

  const grab = (r: GanttRow, edge: Edge) => (e: ReactPointerEvent) => {
    if (readOnly || r.readOnly || e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    setDrag({ id: r.id, edge, x0: e.clientX, delta: 0 });
  };

  // The axis: each month's first day (its name), and at Weeks each week's first day (its date).
  const weekStart = weekStartDay();
  const ticks = Array.from({ length: span + 1 }, (_, n) => plus(from, n)).filter((d) => d.endsWith("-01") || (zoom === "week" && dayIn(d).getDay() === weekStart));
  const tick = (d: string) => {
    const day = dayIn(d);
    return d.endsWith("-01") ? `${MONTHS[day.getMonth()]}${day.getMonth() === 0 || zoom === "quarter" ? ` ${day.getFullYear()}` : ""}` : String(day.getDate());
  };

  return (
    <div className={styles.tlScroll}>
      <div className={styles.tlGrid}>
        <div className={styles.tlAxis} style={{ width: (span + 1) * px }} aria-hidden>
          {ticks.map((d) => (
            <span key={d} className={styles.tick} style={{ left: x(d) }}>
              {tick(d)}
            </span>
          ))}
          <span className={styles.todayLine} style={{ left: x(today) }} />
        </div>
        {rows.map((r) => {
          const [s, e] = dates(r);
          const one = !r.start || !r.end;
          return (
            <div key={r.id} className={styles.tlRow}>
              <button type="button" className={styles.tlLabel} onClick={r.onOpen} title={r.name} aria-label={`${r.name}, ${one ? formatDay(s) : `${formatDay(s)} to ${formatDay(e)}`}`}>
                {r.label}
              </button>
              <div className={styles.tlTrack} style={{ width: (span + 1) * px }}>
                <span className={styles.todayLine} style={{ left: x(today) }} aria-hidden />
                <span
                  className={`${one ? styles.mark : styles.bar} ${drag?.id === r.id ? styles.dragged : ""}`}
                  style={one ? { left: x(s) + px / 2 - MARK_HALF, background: r.color } : { left: x(s), width: (between(s, e) + 1) * px, background: r.color }}
                  aria-hidden
                  data-row={r.id}
                  onPointerDown={grab(r, "move")}
                >
                  {!one && !readOnly && !r.readOnly && (
                    <>
                      <span className={`${styles.barEnd} ${styles.barStart}`} onPointerDown={grab(r, "start")} />
                      <span className={styles.barEnd} onPointerDown={grab(r, "end")} />
                    </>
                  )}
                </span>
              </div>
            </div>
          );
        })}
        {!rows.length &&
          [0, 1].map((n) => (
            <div key={n} className={`${styles.tlRow} ghostCard`}>
              {n === 0 && <span className="ghostText">{empty}</span>}
            </div>
          ))}
      </div>
    </div>
  );
}
