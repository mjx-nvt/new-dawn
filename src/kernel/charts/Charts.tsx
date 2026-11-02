import { useEffect, useRef, useState } from "react";
import type { ChartData } from "../bridge/dashboards";
import { formatNumber as numeral } from "../format/format";
import styles from "./Charts.module.css";

/** Charts (D-067, 0.10.10): bars, lines, a pie and a single value as inline SVG in the Majoris
 *  families (`--chart-1…8`); a select's option colours map to the same families. Each chart
 *  fills its card and redraws with it. */

const COLORS = ["var(--chart-1)", "var(--chart-2)", "var(--chart-3)", "var(--chart-4)", "var(--chart-5)", "var(--chart-6)", "var(--chart-7)", "var(--chart-8)"];
/** Twenty's option colours, by name. */
const OPTION_COLORS: Record<string, string> = {
  blue: COLORS[0],
  sky: COLORS[1],
  turquoise: COLORS[1],
  green: COLORS[2],
  yellow: COLORS[3],
  orange: COLORS[4],
  red: COLORS[5],
  pink: COLORS[5],
  purple: COLORS[6],
  gray: COLORS[7],
  grey: COLORS[7],
};

export function colorOf(index: number, name?: string | null): string {
  return (name && OPTION_COLORS[name.toLowerCase()]) || COLORS[index % COLORS.length];
}

/** A series' values as numbers (the bindings allow null). */
function valuesOf(values: (number | null)[]): number[] {
  return values.map((v) => v ?? 0);
}

export interface ChartStyle {
  horizontal?: boolean;
  stacked?: boolean;
  labels?: boolean;
  legend?: boolean;
  /** none · x · y · both */
  axisName?: string;
  xName?: string;
  yName?: string;
  prefix?: string;
  suffix?: string;
  showCenter?: boolean;
  hideEmpty?: boolean;
}

/** The size of a box, followed as it changes. */
function useSize<T extends HTMLElement>(): [React.RefObject<T | null>, { width: number; height: number }] {
  const ref = useRef<T>(null);
  const [size, setSize] = useState({ width: 0, height: 0 });
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const measure = () => setSize({ width: el.clientWidth, height: el.clientHeight });
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, []);
  return [ref, size];
}

/** 1234.5 → "1,234.5"; 1 250 000 → "1.25M"; a share → "33.3". */
export function formatNumber(n: number, prefix = "", suffix = ""): string {
  const abs = Math.abs(n);
  // The owner's separators (D-072); maximumFractionDigits drops trailing zeros.
  const text = abs >= 1_000_000 ? `${numeral(n / 1_000_000, abs >= 10_000_000 ? 1 : 2)}M` : abs >= 10_000 ? `${numeral(n / 1_000, 1)}k` : numeral(n, 2);
  return `${prefix}${text}${suffix}`;
}

/** Four round steps up to the largest value. */
function ticksFor(max: number): number[] {
  if (max <= 0) return [0];
  const raw = max / 4;
  const power = 10 ** Math.floor(Math.log10(raw));
  const step = [1, 2, 2.5, 5, 10].map((m) => m * power).find((s) => s >= raw) ?? power;
  const out: number[] = [];
  for (let v = 0; v <= max + step * 0.999; v += step) out.push(Number(v.toFixed(6)));
  return out;
}

function Legend({ items }: { items: { key: string; label: string; color: string }[] }) {
  if (!items.length) return null;
  return (
    <ul className={styles.legend} aria-label="Legend">
      {items.map((i) => (
        <li key={i.key}>
          <span className={styles.swatch} style={{ background: i.color }} />
          {i.label}
        </li>
      ))}
    </ul>
  );
}

function Empty({ text }: { text: string }) {
  return (
    <div className={styles.chart}>
      <p className="ghostText">{text}</p>
    </div>
  );
}

/** Bars by category — one series as coloured bars, several grouped or stacked. */
export function BarChart({ data, style }: { data: ChartData; style: ChartStyle }) {
  const [ref, size] = useSize<HTMLDivElement>();
  const categories = data.categories;
  const series = data.series.map((s) => ({ ...s, values: valuesOf(s.values) }));
  const multi = series.length > 1;
  const n = categories.length;
  if (!n || !series.length) return <Empty text="No data" />;
  const totals = categories.map((_, i) => (style.stacked ? series.reduce((s, x) => s + Math.max(0, x.values[i] ?? 0), 0) : Math.max(...series.map((x) => x.values[i] ?? 0))));
  const max = Math.max(0, ...totals);
  const ticks = ticksFor(max);
  const top = ticks[ticks.length - 1] || 1;
  const horizontal = !!style.horizontal;
  const w = Math.max(size.width, 200);
  const h = Math.max(size.height - (style.legend && multi ? 28 : 0), 140);
  const pad = { left: horizontal ? 96 : 44, right: 8, top: 10, bottom: horizontal ? 24 : 30 };
  const plotW = w - pad.left - pad.right;
  const plotH = h - pad.top - pad.bottom;
  const band = (horizontal ? plotH : plotW) / n;
  const inner = band * 0.72;
  const colorFor = (si: number, ci: number) => (multi ? colorOf(si) : colorOf(ci, categories[ci].color));
  const bars: React.ReactNode[] = [];
  categories.forEach((c, ci) => {
    let stackedSoFar = 0;
    series.forEach((s, si) => {
      const v = Math.max(0, s.values[ci] ?? 0);
      const share = style.stacked || !multi ? inner : inner / series.length;
      const along = (horizontal ? pad.top : pad.left) + ci * band + (band - inner) / 2 + (style.stacked || !multi ? 0 : si * share);
      const start = style.stacked ? stackedSoFar : 0;
      const len = (v / top) * (horizontal ? plotW : plotH);
      const from = (start / top) * (horizontal ? plotW : plotH);
      if (style.stacked) stackedSoFar += v;
      const rect = horizontal
        ? { x: pad.left + from, y: along, width: len, height: share }
        : { x: along, y: pad.top + plotH - from - len, width: share, height: len };
      bars.push(
        <g key={`${s.key}-${c.key}`}>
          <rect {...rect} rx={2} fill={colorFor(si, ci)}>
            <title>{`${c.label}${multi ? ` · ${s.label}` : ""}: ${formatNumber(v, style.prefix, style.suffix)}`}</title>
          </rect>
          {style.labels && v > 0 && (!style.stacked || si === series.length - 1) && (
            <text className={styles.value} x={horizontal ? rect.x + rect.width + 4 : rect.x + rect.width / 2} y={horizontal ? rect.y + rect.height / 2 + 4 : rect.y - 4} textAnchor={horizontal ? "start" : "middle"}>
              {formatNumber(style.stacked ? totals[ci] : v, style.prefix, style.suffix)}
            </text>
          )}
        </g>,
      );
    });
  });
  return (
    <div ref={ref} className={styles.chart}>
      <svg className={styles.svg} viewBox={`0 0 ${w} ${h}`} role="img" aria-label="Bar chart">
        {ticks.map((t) =>
          horizontal ? (
            <g key={t}>
              <line className={styles.grid} x1={pad.left + (t / top) * plotW} x2={pad.left + (t / top) * plotW} y1={pad.top} y2={pad.top + plotH} />
              <text className={styles.tick} x={pad.left + (t / top) * plotW} y={h - 8} textAnchor="middle">
                {formatNumber(t, style.prefix, style.suffix)}
              </text>
            </g>
          ) : (
            <g key={t}>
              <line className={styles.grid} x1={pad.left} x2={w - pad.right} y1={pad.top + plotH - (t / top) * plotH} y2={pad.top + plotH - (t / top) * plotH} />
              <text className={styles.tick} x={pad.left - 6} y={pad.top + plotH - (t / top) * plotH + 4} textAnchor="end">
                {formatNumber(t, style.prefix, style.suffix)}
              </text>
            </g>
          ),
        )}
        {bars}
        {categories.map((c, ci) =>
          horizontal ? (
            <text key={c.key} className={styles.tick} x={pad.left - 8} y={pad.top + ci * band + band / 2 + 4} textAnchor="end">
              {c.label.length > 14 ? `${c.label.slice(0, 13)}…` : c.label}
            </text>
          ) : (
            <text key={c.key} className={styles.tick} x={pad.left + ci * band + band / 2} y={h - 12} textAnchor="middle">
              {c.label.length > Math.max(4, band / 7) ? `${c.label.slice(0, Math.max(3, Math.floor(band / 7) - 1))}…` : c.label}
            </text>
          ),
        )}
        {(style.axisName === "x" || style.axisName === "both") && style.xName && (
          <text className={styles.axisName} x={pad.left + plotW / 2} y={h - 1} textAnchor="middle">
            {style.xName}
          </text>
        )}
        {(style.axisName === "y" || style.axisName === "both") && style.yName && (
          <text className={styles.axisName} x={10} y={pad.top + plotH / 2} textAnchor="middle" transform={`rotate(-90 10 ${pad.top + plotH / 2})`}>
            {style.yName}
          </text>
        )}
      </svg>
      {style.legend && multi && <Legend items={series.map((s, i) => ({ key: s.key, label: s.label || "Value", color: colorOf(i) }))} />}
    </div>
  );
}

/** Lines over the categories, one a series. */
export function LineChart({ data, style }: { data: ChartData; style: ChartStyle }) {
  const [ref, size] = useSize<HTMLDivElement>();
  const categories = data.categories;
  const series = data.series.map((s) => ({ ...s, values: valuesOf(s.values) }));
  const n = categories.length;
  if (!n || !series.length) return <Empty text="No data" />;
  const max = Math.max(0, ...series.flatMap((s) => s.values));
  const ticks = ticksFor(max);
  const top = ticks[ticks.length - 1] || 1;
  const w = Math.max(size.width, 200);
  const h = Math.max(size.height - (style.legend && series.length > 1 ? 28 : 0), 140);
  const pad = { left: 44, right: 12, top: 10, bottom: 30 };
  const plotW = w - pad.left - pad.right;
  const plotH = h - pad.top - pad.bottom;
  const x = (i: number) => pad.left + (n === 1 ? plotW / 2 : (i / (n - 1)) * plotW);
  const y = (v: number) => pad.top + plotH - (Math.max(0, v) / top) * plotH;
  return (
    <div ref={ref} className={styles.chart}>
      <svg className={styles.svg} viewBox={`0 0 ${w} ${h}`} role="img" aria-label="Line chart">
        {ticks.map((t) => (
          <g key={t}>
            <line className={styles.grid} x1={pad.left} x2={w - pad.right} y1={y(t)} y2={y(t)} />
            <text className={styles.tick} x={pad.left - 6} y={y(t) + 4} textAnchor="end">
              {formatNumber(t, style.prefix, style.suffix)}
            </text>
          </g>
        ))}
        {series.map((s, si) => (
          <g key={s.key}>
            <polyline className={styles.line} stroke={colorOf(si)} points={s.values.map((v, i) => `${x(i)},${y(v)}`).join(" ")} />
            {s.values.map((v, i) => (
              <circle key={i} className={styles.dot} cx={x(i)} cy={y(v)} r={3} fill={colorOf(si)}>
                <title>{`${categories[i]?.label ?? ""}${series.length > 1 ? ` · ${s.label}` : ""}: ${formatNumber(v, style.prefix, style.suffix)}`}</title>
              </circle>
            ))}
            {style.labels &&
              s.values.map((v, i) => (
                <text key={`l${i}`} className={styles.value} x={x(i)} y={y(v) - 7} textAnchor="middle">
                  {formatNumber(v, style.prefix, style.suffix)}
                </text>
              ))}
          </g>
        ))}
        {categories.map((c, i) => (
          <text key={c.key} className={styles.tick} x={x(i)} y={h - 12} textAnchor="middle">
            {n > 12 && i % Math.ceil(n / 12) !== 0 ? "" : c.label.length > 10 ? `${c.label.slice(0, 9)}…` : c.label}
          </text>
        ))}
        {(style.axisName === "x" || style.axisName === "both") && style.xName && (
          <text className={styles.axisName} x={pad.left + plotW / 2} y={h - 1} textAnchor="middle">
            {style.xName}
          </text>
        )}
        {(style.axisName === "y" || style.axisName === "both") && style.yName && (
          <text className={styles.axisName} x={10} y={pad.top + plotH / 2} textAnchor="middle" transform={`rotate(-90 10 ${pad.top + plotH / 2})`}>
            {style.yName}
          </text>
        )}
      </svg>
      {style.legend && series.length > 1 && <Legend items={series.map((s, i) => ({ key: s.key, label: s.label || "Value", color: colorOf(i) }))} />}
    </div>
  );
}

/** A ring of slices — each category's share of the first series — with the total in the centre. */
export function PieChart({ data, style }: { data: ChartData; style: ChartStyle }) {
  const [ref, size] = useSize<HTMLDivElement>();
  const first = data.series[0];
  if (!first || !data.categories.length) return <Empty text="No data" />;
  const firstValues = valuesOf(first.values);
  const slices = data.categories.map((c, i) => ({ key: c.key, label: c.label, value: Math.max(0, firstValues[i] ?? 0), color: colorOf(i, c.color) })).filter((s) => !style.hideEmpty || s.value > 0);
  const sum = slices.reduce((s, x) => s + x.value, 0);
  if (sum <= 0) return <Empty text="No data" />;
  const h = Math.max(size.height, 140);
  const d = Math.min(h, Math.max(size.width, 160) * 0.6);
  const r = d / 2 - 4;
  const inner = r * 0.58;
  let angle = -Math.PI / 2;
  const arcs = slices.map((s) => {
    const sweep = (s.value / sum) * Math.PI * 2;
    const a0 = angle;
    const a1 = angle + sweep;
    angle = a1;
    const large = sweep > Math.PI ? 1 : 0;
    const point = (rad: number, a: number) => `${(d / 2 + rad * Math.cos(a)).toFixed(2)} ${(d / 2 + rad * Math.sin(a)).toFixed(2)}`;
    const path = slices.length === 1 ? `M ${point(r, a0)} A ${r} ${r} 0 1 1 ${point(r, a0 + Math.PI)} A ${r} ${r} 0 1 1 ${point(r, a0)} M ${point(inner, a0)} A ${inner} ${inner} 0 1 0 ${point(inner, a0 + Math.PI)} A ${inner} ${inner} 0 1 0 ${point(inner, a0)}` : `M ${point(r, a0)} A ${r} ${r} 0 ${large} 1 ${point(r, a1)} L ${point(inner, a1)} A ${inner} ${inner} 0 ${large} 0 ${point(inner, a0)} Z`;
    return { ...s, path, share: s.value / sum };
  });
  return (
    <div ref={ref} className={`${styles.chart} ${styles.chartRow}`}>
      <svg width={d} height={d} viewBox={`0 0 ${d} ${d}`} role="img" aria-label="Pie chart">
        {arcs.map((a) => (
          <path key={a.key} className={styles.slice} d={a.path} fill={a.color} fillRule="evenodd">
            <title>{`${a.label}: ${formatNumber(a.value, style.prefix, style.suffix)} (${Math.round(a.share * 100)}%)`}</title>
          </path>
        ))}
        {style.showCenter !== false && (
          <text className={styles.centre} x={d / 2} y={d / 2 + 6} textAnchor="middle">
            {formatNumber(data.total ?? 0, style.prefix, style.suffix)}
          </text>
        )}
      </svg>
      <Legend items={arcs.map((a) => ({ key: a.key, label: `${a.label} · ${Math.round(a.share * 100)}%`, color: a.color }))} />
    </div>
  );
}

/** One number, large, with its prefix and suffix. */
export function AggregateValue({ value, prefix, suffix, hint }: { value: number; prefix?: string; suffix?: string; hint?: string }) {
  return (
    <div className={styles.big}>
      <span>{formatNumber(value, prefix, suffix)}</span>
      {hint && <span className={styles.bigHint}>{hint}</span>}
    </div>
  );
}

export function TruncatedNote({ data }: { data: ChartData }) {
  return data.truncated ? <p className={styles.note}>Undisplayed data: at most 100 categories and 50 series a chart — add a filter or a coarser granularity.</p> : null;
}
