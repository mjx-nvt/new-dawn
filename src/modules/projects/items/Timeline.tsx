import { useUpdatePmItem, type PmItem } from "../../../kernel/bridge/pmItems";
import { colorOf } from "../../../kernel/charts/Charts";
import { formatDay } from "../../../kernel/format/format";
import type { LayoutProps } from "./Board";
import Gantt, { frame, type GanttRow, type Zoom } from "./Gantt";
import { Dot } from "./Props";
import styles from "./Items.module.css";

/** Plane's Timeline (D-076): a bar an item, from its start date to its due date (one date: a
 *  mark on that day), in its state's colour, on a day axis zoomed by week, month or quarter
 *  (remembered) around today, today's line in orange. A bar dragged moves both dates; its ends
 *  move one; Escape or leaving the window drops a drag. The items without dates, or too far from
 *  today, are listed below — their peek sets the dates by keyboard. */

export default function Timeline({ slug, project, items, onPeek, zoom, onZoom }: LayoutProps & { zoom: Zoom; onZoom: (zoom: Zoom) => void }) {
  const update = useUpdatePmItem(slug);
  const rows: (GanttRow & { item: PmItem })[] = items.map((i) => ({
    id: i.id,
    start: i.start_date,
    end: i.target_date,
    color: colorOf(0, i.state_color),
    name: `${i.key} ${i.name}`,
    label: (
      <>
        <Dot color={i.state_color} /> <span className={styles.key}>{i.key}</span> {i.name}
      </>
    ),
    onOpen: () => onPeek(i.id),
    item: i,
  }));
  const { from, to, shown, away } = frame(rows);
  // Only the dates it has: a one-date mark moves that date.
  const move = (row: GanttRow, s: string, e: string) => {
    const item = items.find((i) => i.id === row.id);
    if (item) update.mutate({ id: item.id, patch: { ...(item.start_date ? { start_date: s } : {}), ...(item.target_date ? { target_date: e } : {}) } });
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
      <Gantt rows={shown} from={from} to={to} zoom={zoom} readOnly={!!project.archived_at} empty="No work item has a start or due date near today — set one to place it here" onMove={move} />
      <section className={styles.subs} aria-label="Not on the timeline">
        <h3 className={styles.h3}>Without dates, or far from today · {away.length}</h3>
        {away.map(({ item: i }) => (
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
