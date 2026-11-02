import type { Formats_Serialize as Formats } from "../../bindings";

/** How dates, times and numbers read everywhere (D-072, Settings → General → Formats). Plain
 *  functions on the formats this window last read: `FormatsRoot` sets them before anything draws
 *  and redraws the window when they change. No Tauri import, so scripts can run them. */

export type { Formats };

/** New Dawn's own, until the owner picks others. */
export const DEFAULT_FORMATS: Formats = { timeZone: "", date: "dayMonthName", time: "hours24", number: "commasAndDot", weekStart: "monday" };

let current: Formats = DEFAULT_FORMATS;

export function setFormats(formats: Formats) {
  // A zone this system doesn't know (kept from another Mac) would break every date: this Mac's.
  current = formats.timeZone && !knownZone(formats.timeZone) ? { ...formats, timeZone: "" } : formats;
}

export function currentFormats(): Formats {
  return current;
}

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const NUMBER_LOCALE: Record<Formats["number"], string> = { commasAndDot: "en-GB", dotsAndComma: "de-DE", spacesAndComma: "fr-FR", apostrophesAndDot: "de-CH" };
const DAY = /^(\d{4})-(\d{2})-(\d{2})$/;
const pad = (n: number) => String(n).padStart(2, "0");
const zone = () => current.timeZone || undefined;

/** Year, month, day: a "YYYY-MM-DD" day as written (a calendar day, no zone moves it), a time
 *  as it falls in the chosen zone. */
function ymd(value: string | Date): [number, number, number] | null {
  if (typeof value === "string") {
    const m = DAY.exec(value);
    if (m) return [Number(m[1]), Number(m[2]), Number(m[3])];
  }
  const at = typeof value === "string" ? new Date(value) : value;
  if (Number.isNaN(at.getTime())) return null;
  const parts = new Intl.DateTimeFormat("en-CA", { timeZone: zone(), year: "numeric", month: "2-digit", day: "2-digit" }).formatToParts(at);
  const get = (type: string) => Number(parts.find((p) => p.type === type)?.value);
  return [get("year"), get("month"), get("day")];
}

/** A date as the owner reads them: "24 Oct 2026", "24/10/2026", "10/24/2026" or "2026-10-24".
 *  `short` leaves this year's year out (New Dawn's own format only); `as` shows another format
 *  (the picker's examples). */
export function formatDay(value: string | Date, opts: { short?: boolean; now?: Date; as?: Formats["date"] } = {}): string {
  const d = ymd(value);
  if (!d) return typeof value === "string" ? value : "";
  const [y, m, day] = d;
  switch (opts.as ?? current.date) {
    case "dayFirst":
      return `${pad(day)}/${pad(m)}/${y}`;
    case "monthFirst":
      return `${pad(m)}/${pad(day)}/${y}`;
    case "yearFirst":
      return `${y}-${pad(m)}-${pad(day)}`;
    default:
      return opts.short && y === ymd(opts.now ?? new Date())?.[0] ? `${day} ${MONTHS[m - 1]}` : `${day} ${MONTHS[m - 1]} ${y}`;
  }
}

/** A time of day in the chosen zone: "14:05" or "2:05 PM". */
export function formatTime(value: string | Date, as?: Formats["time"]): string {
  const at = typeof value === "string" ? new Date(value) : value;
  if (Number.isNaN(at.getTime())) return "";
  return (as ?? current.time) === "hours12"
    ? at.toLocaleTimeString("en-US", { timeZone: zone(), hour: "numeric", minute: "2-digit", hour12: true })
    : at.toLocaleTimeString("en-GB", { timeZone: zone(), hour: "2-digit", minute: "2-digit", hourCycle: "h23" });
}

/** "24 Oct 2026, 14:05". */
export function formatDateTime(value: string | Date): string {
  const time = formatTime(value);
  return time ? `${formatDay(value)}, ${time}` : formatDay(value);
}

/** A number with the chosen separators: "1,234.56", "1.234,56", "1 234,56", "1’234.56". */
export function formatNumber(n: number, maxDecimals = 2, as?: Formats["number"]): string {
  return n.toLocaleString(NUMBER_LOCALE[as ?? current.number], { maximumFractionDigits: maxDecimals });
}

/** The day a value falls on in the chosen zone, as a local midnight (to group and place by). */
export function dayIn(value: string | Date): Date {
  const d = ymd(value);
  return d ? new Date(d[0], d[1] - 1, d[2]) : new Date(Number.NaN);
}

/** The same day as "YYYY-MM-DD". */
export function dayKeyIn(value: string | Date): string {
  const d = ymd(value);
  return d ? `${d[0]}-${pad(d[1])}-${pad(d[2])}` : "";
}

/** A local calendar day as "YYYY-MM-DD" (for a Date made at a local midnight). */
export function dayString(d: Date): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** The week's first day, as `Date.getDay()` numbers them (0 Sunday, 1 Monday, 6 Saturday). */
export function weekStartDay(): 0 | 1 | 6 {
  return current.weekStart === "sunday" ? 0 : current.weekStart === "saturday" ? 6 : 1;
}

/** Days from the week's first day to `day` (0 … 6). */
export function daysIntoWeek(day: Date): number {
  return (day.getDay() - weekStartDay() + 7) % 7;
}

/** The time zones this Mac knows, for the picker. */
export function timeZones(): string[] {
  const intl = Intl as unknown as { supportedValuesOf?: (key: string) => string[] };
  try {
    return intl.supportedValuesOf?.("timeZone") ?? [];
  } catch {
    return [];
  }
}

/** Whether the system knows a zone. */
export function knownZone(zoneName: string): boolean {
  try {
    new Intl.DateTimeFormat("en-GB", { timeZone: zoneName });
    return true;
  } catch {
    return false;
  }
}
