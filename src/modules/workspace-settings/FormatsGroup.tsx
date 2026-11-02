import type { ReactNode } from "react";
import { useFormats, useSetFormats } from "../../kernel/bridge/formats";
import { DEFAULT_FORMATS, formatDay, formatNumber, formatTime, knownZone, timeZones, type Formats } from "../../kernel/format/format";

const DATES: Formats["date"][] = ["dayMonthName", "dayFirst", "monthFirst", "yearFirst"];
const TIMES: [Formats["time"], string][] = [
  ["hours24", "24-hour"],
  ["hours12", "12-hour"],
];
const NUMBERS: Formats["number"][] = ["commasAndDot", "dotsAndComma", "spacesAndComma", "apostrophesAndDot"];
const WEEKS: [Formats["weekStart"], string][] = [
  ["monday", "Monday"],
  ["sunday", "Sunday"],
  ["saturday", "Saturday"],
];

function Row({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <div className="groupRow">
      <div className="rowText">
        <p className="rowLabel">{label}</p>
        {hint && <p className="rowHint">{hint}</p>}
      </div>
      <div className="rowControl">{children}</div>
    </div>
  );
}

/** Settings → General → Formats (D-072, Twenty's Experience settings): the time zone, how a date,
 *  a time and a number read everywhere in New Dawn, and the week's first day. Each choice shows
 *  itself on today's date. */
export default function FormatsGroup() {
  const formats = useFormats();
  const save = useSetFormats();
  const f = formats.data ?? DEFAULT_FORMATS;
  const set = (patch: Partial<Formats>) => save.mutate({ ...f, ...patch });
  const busy = !formats.data || save.isPending;
  const now = new Date();
  const mac = Intl.DateTimeFormat().resolvedOptions().timeZone;
  const zones = timeZones();
  if (f.timeZone && !zones.includes(f.timeZone)) zones.unshift(f.timeZone);
  return (
    <>
      <p className="groupTitle">Formats</p>
      <div className="group">
        <Row label="Time zone" hint="Times are shown in it; a date stays the day it was written. A time you type is this Mac's.">
          <select className="input" aria-label="Time zone" value={f.timeZone} disabled={busy} onChange={(e) => (e.target.value === "" || knownZone(e.target.value)) && set({ timeZone: e.target.value })}>
            <option value="">This Mac's ({mac.replace(/_/g, " ")})</option>
            {zones.map((z) => (
              <option key={z} value={z}>
                {z.replace(/_/g, " ")}
              </option>
            ))}
          </select>
        </Row>
        <Row label="Date">
          <select className="input" aria-label="Date format" value={f.date} disabled={busy} onChange={(e) => set({ date: DATES.find((d) => d === e.target.value) ?? f.date })}>
            {DATES.map((d) => (
              <option key={d} value={d}>
                {formatDay(now, { as: d })}
              </option>
            ))}
          </select>
        </Row>
        <Row label="Time">
          <select className="input" aria-label="Time format" value={f.time} disabled={busy} onChange={(e) => set({ time: TIMES.find(([t]) => t === e.target.value)?.[0] ?? f.time })}>
            {TIMES.map(([t, label]) => (
              <option key={t} value={t}>
                {label} ({formatTime(now, t)})
              </option>
            ))}
          </select>
        </Row>
        <Row label="Numbers">
          <select className="input" aria-label="Number format" value={f.number} disabled={busy} onChange={(e) => set({ number: NUMBERS.find((n) => n === e.target.value) ?? f.number })}>
            {NUMBERS.map((n) => (
              <option key={n} value={n}>
                {formatNumber(1234567.89, 2, n)}
              </option>
            ))}
          </select>
        </Row>
        <Row label="Week starts on">
          <select className="input" aria-label="Week starts on" value={f.weekStart} disabled={busy} onChange={(e) => set({ weekStart: WEEKS.find(([w]) => w === e.target.value)?.[0] ?? f.weekStart })}>
            {WEEKS.map(([w, label]) => (
              <option key={w} value={w}>
                {label}
              </option>
            ))}
          </select>
        </Row>
      </div>
      {save.error && (
        <p className="formError" role="alert">
          {save.error.message}
        </p>
      )}
    </>
  );
}
