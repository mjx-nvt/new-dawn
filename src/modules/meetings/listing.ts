/** Filtering, sorting, date grouping and date-search parsing for the Meetings list. Pure
 *  functions over the meetings the list already has (personal use: hundreds at most), so no
 *  backend involvement. Dates are local time. */

export const STATUS_FILTERS = ["all", "needs_transcript", "needs_summary", "done", "failed"] as const;
export type StatusFilter = (typeof STATUS_FILTERS)[number];
export const STATUS_FILTER_LABEL: Record<StatusFilter, string> = {
  all: "All statuses",
  needs_transcript: "Needs transcript",
  needs_summary: "Needs summary",
  done: "Summarised",
  failed: "Failed",
};

export const WHEN_FILTERS = ["all", "7d", "30d", "year"] as const;
export type WhenFilter = (typeof WHEN_FILTERS)[number];
export const WHEN_FILTER_LABEL: Record<WhenFilter, string> = { all: "All time", "7d": "Last 7 days", "30d": "Last 30 days", year: "This year" };

export const SORTS = ["newest", "oldest", "longest", "title"] as const;
export type Sort = (typeof SORTS)[number];
export const SORT_LABEL: Record<Sort, string> = { newest: "Newest first", oldest: "Oldest first", longest: "Longest first", title: "Title A–Z" };

/** The slice of a meeting these functions need. */
export interface Listed {
  title: string;
  held_on: string; // ISO-8601 UTC
  duration_ms: number | null;
  status: string;
  meeting_type_id: string | null;
}

export interface Filters {
  status: StatusFilter;
  /** "" = all, "none" = meetings without a type, else a type id. */
  type: string;
  when: WhenFilter;
  sort: Sort;
}

export interface DateRange {
  from: Date;
  /** Exclusive. */
  to: Date;
  label: string;
}

const DAY = 86_400_000;
const startOfDay = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate());
const addDays = (d: Date, n: number) => new Date(d.getTime() + n * DAY);
/** Monday-first week start. */
const startOfWeek = (d: Date) => addDays(startOfDay(d), -((d.getDay() + 6) % 7));
const startOfMonth = (d: Date) => new Date(d.getFullYear(), d.getMonth(), 1);
const monthName = (d: Date) => d.toLocaleDateString("en-GB", { month: "long", year: "numeric" });

const MONTHS = ["january", "february", "march", "april", "may", "june", "july", "august", "september", "october", "november", "december"];
/** "oct", "octo", "october" → 9; "octopus", "market" → -1 (the word must be a prefix of the month). */
const monthIndex = (word: string) => {
  const w = word.toLowerCase();
  return w.length >= 3 ? MONTHS.findIndex((m) => m.startsWith(w)) : -1;
};

/** Pulls a date (or a period) out of a search query. Returns the range and the words left over
 *  for text search. Understands: 2026-10-24, 2026-10, 24/10/2026, 24/10, 24 Oct (2026),
 *  Oct 2026, October, today, yesterday, this week, last week, this month, last month. */
export function parseDateQuery(query: string, now = new Date()): { range: DateRange | null; rest: string } {
  const q = query.trim();
  const lower = q.toLowerCase();
  const today = startOfDay(now);
  const day = (d: Date, label: string): DateRange => ({ from: d, to: addDays(d, 1), label });
  const phrases: [string, () => DateRange][] = [
    ["today", () => day(today, "today")],
    ["yesterday", () => day(addDays(today, -1), "yesterday")],
    ["this week", () => ({ from: startOfWeek(today), to: addDays(startOfWeek(today), 7), label: "this week" })],
    ["last week", () => ({ from: addDays(startOfWeek(today), -7), to: startOfWeek(today), label: "last week" })],
    ["this month", () => ({ from: startOfMonth(today), to: new Date(today.getFullYear(), today.getMonth() + 1, 1), label: "this month" })],
    ["last month", () => ({ from: new Date(today.getFullYear(), today.getMonth() - 1, 1), to: startOfMonth(today), label: "last month" })],
  ];
  for (const [phrase, make] of phrases) {
    const at = lower.indexOf(phrase);
    if (at >= 0) return { range: make(), rest: (q.slice(0, at) + q.slice(at + phrase.length)).trim() };
  }

  const tokens = q.split(/\s+/).filter(Boolean);
  const take = (count: number, at: number) => tokens.filter((_, i) => i < at || i >= at + count).join(" ");
  const valid = (y: number, m: number, d: number) => m >= 0 && m < 12 && d >= 1 && d <= new Date(y, m + 1, 0).getDate();
  const year = (y: number) => (y < 100 ? 2000 + y : y);

  for (let i = 0; i < tokens.length; i++) {
    const t = tokens[i];
    let m: RegExpMatchArray | null;
    // 2026-10-24 · 2026-10
    if ((m = t.match(/^(\d{4})-(\d{1,2})(?:-(\d{1,2}))?$/))) {
      const [y, mo] = [Number(m[1]), Number(m[2]) - 1];
      if (m[3]) {
        const d = new Date(y, mo, Number(m[3]));
        if (valid(y, mo, Number(m[3]))) return { range: day(d, d.toLocaleDateString("en-GB", { day: "numeric", month: "short", year: "numeric" })), rest: take(1, i) };
      } else if (mo >= 0 && mo < 12) {
        return { range: { from: new Date(y, mo, 1), to: new Date(y, mo + 1, 1), label: monthName(new Date(y, mo, 1)) }, rest: take(1, i) };
      }
    }
    // 24/10/2026 · 24/10
    if ((m = t.match(/^(\d{1,2})\/(\d{1,2})(?:\/(\d{2,4}))?$/))) {
      const [d, mo] = [Number(m[1]), Number(m[2]) - 1];
      const y = m[3] ? year(Number(m[3])) : today.getFullYear();
      if (valid(y, mo, d)) {
        const date = new Date(y, mo, d);
        return { range: day(date, date.toLocaleDateString("en-GB", { day: "numeric", month: "short", year: "numeric" })), rest: take(1, i) };
      }
    }
    // 24 Oct (2026)
    if (/^\d{1,2}$/.test(t) && i + 1 < tokens.length && monthIndex(tokens[i + 1]) >= 0) {
      const mo = monthIndex(tokens[i + 1]);
      const next = tokens[i + 2];
      const hasYear = !!next && /^\d{4}$/.test(next);
      const y = hasYear ? Number(next) : today.getFullYear();
      if (valid(y, mo, Number(t))) {
        const date = new Date(y, mo, Number(t));
        return { range: day(date, date.toLocaleDateString("en-GB", { day: "numeric", month: "short", year: "numeric" })), rest: take(hasYear ? 3 : 2, i) };
      }
    }
    // Oct 2026 · October
    const mo = monthIndex(t);
    if (mo >= 0 && /^[a-z]+$/i.test(t)) {
      const next = tokens[i + 1];
      const hasYear = !!next && /^\d{4}$/.test(next);
      const y = hasYear ? Number(next) : today.getFullYear();
      const from = new Date(y, mo, 1);
      return { range: { from, to: new Date(y, mo + 1, 1), label: monthName(from) }, rest: take(hasYear ? 2 : 1, i) };
    }
  }
  return { range: null, rest: q };
}

function matchesStatus(status: string, filter: StatusFilter): boolean {
  switch (filter) {
    case "all":
      return true;
    case "needs_transcript":
      return status === "recorded" || status === "recording" || status === "transcribing";
    case "needs_summary":
      return status === "transcribed" || status === "summarising";
    case "done":
      return status === "summarised";
    case "failed":
      return status === "failed";
  }
}

function whenRange(when: WhenFilter, now: Date): DateRange | null {
  const today = startOfDay(now);
  switch (when) {
    case "all":
      return null;
    case "7d":
      return { from: addDays(today, -6), to: addDays(today, 1), label: "last 7 days" };
    case "30d":
      return { from: addDays(today, -29), to: addDays(today, 1), label: "last 30 days" };
    case "year":
      return { from: new Date(today.getFullYear(), 0, 1), to: new Date(today.getFullYear() + 1, 0, 1), label: "this year" };
  }
}

/** The meetings that pass the filters (and the searched date, if any), in the chosen order. */
export function applyFilters<M extends Listed>(meetings: M[], filters: Filters, dateRange: DateRange | null, now = new Date()): M[] {
  const when = whenRange(filters.when, now);
  const inRange = (m: M, r: DateRange | null) => {
    if (!r) return true;
    const t = new Date(m.held_on).getTime();
    return t >= r.from.getTime() && t < r.to.getTime();
  };
  const kept = meetings.filter(
    (m) =>
      matchesStatus(m.status, filters.status) &&
      (filters.type === "" || (filters.type === "none" ? !m.meeting_type_id : m.meeting_type_id === filters.type)) &&
      inRange(m, when) &&
      inRange(m, dateRange),
  );
  const byDate = (a: M, b: M) => b.held_on.localeCompare(a.held_on);
  switch (filters.sort) {
    case "newest":
      return kept.sort(byDate);
    case "oldest":
      return kept.sort((a, b) => -byDate(a, b));
    case "longest":
      return kept.sort((a, b) => (b.duration_ms ?? 0) - (a.duration_ms ?? 0) || byDate(a, b));
    case "title":
      return kept.sort((a, b) => a.title.localeCompare(b.title, undefined, { sensitivity: "base" }) || byDate(a, b));
  }
}

/** "Today", "Yesterday", "This week", "Last week", "Earlier this month", then "September 2026"… */
export function groupLabel(heldOn: string, now = new Date()): string {
  const d = new Date(heldOn);
  const today = startOfDay(now);
  const day = startOfDay(d);
  if (day.getTime() === today.getTime()) return "Today";
  if (day.getTime() === addDays(today, -1).getTime()) return "Yesterday";
  if (day > today) return "Upcoming";
  const week = startOfWeek(today);
  if (day >= week) return "This week";
  if (day >= addDays(week, -7)) return "Last week";
  if (day >= startOfMonth(today)) return "Earlier this month";
  return monthName(d);
}

/** Consecutive runs of the same group, in list order. */
export function groupMeetings<M extends Listed>(meetings: M[], now = new Date()): { label: string; items: M[] }[] {
  const groups: { label: string; items: M[] }[] = [];
  for (const m of meetings) {
    const label = groupLabel(m.held_on, now);
    const last = groups[groups.length - 1];
    if (last && last.label === label) last.items.push(m);
    else groups.push({ label, items: [m] });
  }
  return groups;
}
