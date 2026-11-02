import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { useUpdatePmItem, type PmItem } from "../../../kernel/bridge/pmItems";
import { colorOf } from "../../../kernel/charts/Charts";
import { MONTHS, dayIn, dayString, formatDay, weekStartDay } from "../../../kernel/format/format";
import type { LayoutProps } from "./Board";
import { Dot } from "./Props";
import styles from "./Items.module.css";

const DAY = 86_400_000;
/** Points a day at each zoom. */
const PX: Record<Zoom, number> = { week: 40, month: 16, quarter: 5 };
/** The axis reaches at most this many days either side of today. */
const REACH = 365;
/** Half the one-date mark's width (`.mark`, 12 pt). */
const MARK_HALF = 6;
type Zoom = "week" | "month" | "quarter";
type Edge = "move" | "start" | "end";

const plus = (day: string, n: number) => {
  const d = dayIn(day);
  return dayString(new Date(d.getFullYear(), d.getMonth(), d.getDate() + n));
};
const between = (a: string, b: string) => Math.round((dayIn(b).getTime() - dayIn(a).getTime()) / DAY);

/** Plane's Timeline (D-076): a bar an item, from its start date to its due date (one date: a
 *  mark on that day), in its state's colour, on a day axis zoomed by week, month or quarter
 *  (remembered) around today, today's line in orange. A bar dragged moves both dates; its ends
 *  move one; Escape or leaving the window drops a drag. The items without dates, or too far from
 *  today, are listed below — their peek sets the dates by keyboard. */
export default function Timeline({ slug, project, items, onPeek, zoom, onZoom }: LayoutProps & { zoom: Zoom; onZoom: (zoom: Zoom) => void }) {
  const update = useUpdatePmItem(slug);
  const [drag, setDrag] = useState<{ id: string; edge: Edge; x0: number; delta: number } | null>(null);
  const done = useRef<(() => void) | null>(null);
  const px = PX[zoom];
  const ro = !!project.archived_at;
  const today = dayString(new Date());
  const ends = (i: PmItem): [string, string] => [i.start_date ?? i.target_date ?? today, i.target_date ?? i.start_date ?? today];
  const dated = items.filter((i) => i.start_date || i.target_date);
  // The window: a week before the earliest date to four weeks after the latest, held within a
  // year of today either way (today always in it). Days as YYYY-MM-DD compare as text.
  const days = dated.flatMap(ends).sort();
  const min = (a: string, b: string) => (a < b ? a : b);
  const max = (a: string, b: string) => (a > b ? a : b);
  const from = min(plus(today, -7), max(plus(days[0] ?? today, -7), plus(today, -REACH)));
  const to = max(plus(today, 28), min(plus(days[days.length - 1] ?? today, 28), plus(today, REACH)));
  const span = between(from, to);
  const shown = dated.filter((i) => ends(i)[1] >= from && ends(i)[0] <= to);
  const away = [...items.filter((i) => !i.start_date && !i.target_date), ...dated.filter((i) => !shown.includes(i))];
  const x = (day: string) => between(from, day) * px;

  /** The dates a bar shows: its own, or as the drag has them. */
  const dates = (i: PmItem): [string, string] => {
    const [s, e] = ends(i);
    if (!drag || drag.id !== i.id) return [s, e];
    const d = drag.delta;
    if (drag.edge === "move") return [plus(s, d), plus(e, d)];
    if (drag.edge === "start") return [plus(s, Math.min(d, between(s, e))), e];
    return [s, plus(e, Math.max(d, -between(s, e)))];
  };

  // While dragging: follow the pointer (a redraw only when the day changes); release writes the
  // new dates; Escape, a cancelled pointer or leaving the window drops the drag.
  useEffect(() => {
    if (!drag) return;
    const onMove = (e: PointerEvent) =>
      setDrag((d) => {
        if (!d) return d;
        const delta = Math.round((e.clientX - d.x0) / px);
        return delta === d.delta ? d : { ...d, delta };
      });
    const onUp = () => done.current?.();
    const cancel = () => setDrag(null);
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && cancel();
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", cancel);
    window.addEventListener("blur", cancel);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("pointercancel", cancel);
      window.removeEventListener("blur", cancel);
      window.removeEventListener("keydown", onKey);
    };
  }, [drag?.id, px]);

  done.current = () => {
    const d = drag;
    setDrag(null);
    const item = d && items.find((i) => i.id === d.id);
    if (!d || !item || !d.delta) return;
    const [s, e] = dates(item);
    update.mutate({ id: item.id, patch: { ...(item.start_date ? { start_date: s } : {}), ...(item.target_date ? { target_date: e } : {}) } });
  };

  const grab = (i: PmItem, edge: Edge) => (e: ReactPointerEvent) => {
    if (ro || e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    setDrag({ id: i.id, edge, x0: e.clientX, delta: 0 });
  };

  // The axis: each month's first day (its name), and at Weeks each week's first day (its date).
  const weekStart = weekStartDay();
  const ticks = Array.from({ length: span + 1 }, (_, n) => plus(from, n)).filter((d) => d.endsWith("-01") || (zoom === "week" && dayIn(d).getDay() === weekStart));
  const tick = (d: string) => {
    const day = dayIn(d);
    return d.endsWith("-01") ? `${MONTHS[day.getMonth()]}${day.getMonth() === 0 || zoom === "quarter" ? ` ${day.getFullYear()}` : ""}` : String(day.getDate());
  };

  return (
    <section className={styles.timeline} aria-label="Timeline">
      <div className={styles.calHead}>
        <span className="fieldHint">{shown.length ? "Drag a bar to move its dates, or an end to change one." : "No work item has dates near today yet."}</span>
        <span className={styles.spacer} />
        <div className="segmented" role="group" aria-label="Zoom">
          {(["week", "month", "quarter"] as const).map((z) => (
            <button key={z} type="button" aria-pressed={zoom === z} onClick={() => onZoom(z)}>
              {z === "week" ? "Weeks" : z === "month" ? "Months" : "Quarters"}
            </button>
          ))}
        </div>
      </div>
      {update.error && (
        <p className="formError" role="alert">
          {update.error.message}
        </p>
      )}
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
          {shown.map((i) => {
            const [s, e] = dates(i);
            const one = !i.start_date || !i.target_date;
            const color = colorOf(0, i.state_color);
            return (
              <div key={i.id} className={styles.tlRow}>
                <button type="button" className={styles.tlLabel} onClick={() => onPeek(i.id)} title={i.name} aria-label={`${i.key} ${i.name}, ${one ? formatDay(s) : `${formatDay(s)} to ${formatDay(e)}`}`}>
                  <Dot color={i.state_color} /> <span className={styles.key}>{i.key}</span> {i.name}
                </button>
                <div className={styles.tlTrack} style={{ width: (span + 1) * px }}>
                  <span className={styles.todayLine} style={{ left: x(today) }} aria-hidden />
                  <span
                    className={`${one ? styles.mark : styles.bar} ${drag?.id === i.id ? styles.dragged : ""}`}
                    style={one ? { left: x(s) + px / 2 - MARK_HALF, background: color } : { left: x(s), width: (between(s, e) + 1) * px, background: color }}
                    aria-hidden
                    data-item={i.id}
                    onPointerDown={grab(i, "move")}
                  >
                    {!one && !ro && (
                      <>
                        <span className={`${styles.barEnd} ${styles.barStart}`} onPointerDown={grab(i, "start")} />
                        <span className={styles.barEnd} onPointerDown={grab(i, "end")} />
                      </>
                    )}
                  </span>
                </div>
              </div>
            );
          })}
          {!shown.length &&
            [0, 1].map((n) => (
              <div key={n} className={`${styles.tlRow} ghostCard`}>
                {n === 0 && <span className="ghostText">No work item has a start or due date near today — set one to place it here</span>}
              </div>
            ))}
        </div>
      </div>
      <section className={styles.subs} aria-label="Not on the timeline">
        <h3 className={styles.h3}>Without dates, or far from today · {away.length}</h3>
        {away.map((i) => (
          <button key={i.id} type="button" className={styles.subRow} onClick={() => onPeek(i.id)}>
            <Dot color={i.state_color} /> <span className={styles.key}>{i.key}</span> {i.name}
            {(i.start_date || i.target_date) && <span className="fieldHint">{formatDay(i.target_date ?? i.start_date ?? "")}</span>}
          </button>
        ))}
        {!away.length &&
          [0, 1].map((n) => (
            <div key={n} className={`${styles.ghostRow} ghostCard`}>
              {n === 0 && <span className="ghostText">Every work item is on the timeline</span>}
            </div>
          ))}
      </section>
    </section>
  );
}
