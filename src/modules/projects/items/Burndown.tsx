import { colorOf } from "../../../kernel/charts/Charts";
import type { CycleDetails } from "../../../kernel/bridge/pmCycles";
import { formatDay, formatNumber } from "../../../kernel/format/format";
import styles from "./Items.module.css";

const W = 560;
const H = 200;
const PAD = { left: 36, right: 12, top: 12, bottom: 28 };

/** A cycle's burn-down (D-078), inline SVG as the app's charts (D-067): the ideal line from all
 *  the scope to none, dashed, and what remained open at each day's end up to today; work items,
 *  or points when the project estimates in numbers. */
export default function Burndown({ details }: { details: CycleDetails }) {
  const days = details.burndown;
  if (!days.length) {
    return <p className="ghostText">A burn-down needs the cycle's dates — a draft has none.</p>;
  }
  if (days.length === 1) {
    return <p className="fieldHint">A one-day cycle: {formatNumber(days[0].remaining ?? days[0].ideal ?? 0, 1)} {details.points ? "points" : "work items"} open.</p>;
  }
  const top = Math.max(1, ...days.map((d) => Math.max(d.ideal ?? 0, d.remaining ?? 0)));
  const x = (n: number) => PAD.left + (n / (days.length - 1)) * (W - PAD.left - PAD.right);
  const y = (v: number) => PAD.top + (1 - v / top) * (H - PAD.top - PAD.bottom);
  const ideal = days.map((d, n) => `${x(n)},${y(d.ideal ?? 0)}`).join(" ");
  const actual = days
    .map((d, n) => (d.remaining == null ? null : `${x(n)},${y(d.remaining)}`))
    .filter((p): p is string => !!p)
    .join(" ");
  const unit = details.points ? "points" : "work items";
  const last = [...days].reverse().find((d) => d.remaining != null);
  return (
    <figure className={styles.burndown}>
      <svg viewBox={`0 0 ${W} ${H}`} role="img" aria-label={`Burn-down: ${last ? `${formatNumber(last.remaining ?? 0, 1)} ${unit} open on ${formatDay(last.day, { short: true })}` : "not started"}, from ${formatNumber(top, 1)}`}>
        {[0, 0.5, 1].map((f) => (
          <g key={f}>
            <line className={styles.grid} x1={PAD.left} x2={W - PAD.right} y1={y(top * f)} y2={y(top * f)} />
            <text className={styles.axis} x={PAD.left - 6} y={y(top * f) + 4} textAnchor="end">
              {formatNumber(top * f, 0)}
            </text>
          </g>
        ))}
        <text className={styles.axis} x={x(0)} y={H - 8} textAnchor="start">
          {formatDay(days[0].day, { short: true })}
        </text>
        <text className={styles.axis} x={x(days.length - 1)} y={H - 8} textAnchor="end">
          {formatDay(days[days.length - 1].day, { short: true })}
        </text>
        <polyline className={styles.ideal} points={ideal} />
        {actual && <polyline points={actual} fill="none" stroke={colorOf(0)} strokeWidth={2} />}
      </svg>
      <figcaption className="fieldHint">
        Open {unit} each day — the dashed line is the ideal pace.
      </figcaption>
    </figure>
  );
}
