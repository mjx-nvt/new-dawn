/** Due days in Tasks. `due_on` is a local calendar day, "YYYY-MM-DD"; these read and make such
 *  days against the Mac's clock. Pure, so `scripts/check-tasks.ts` can run them. */

const pad = (n: number) => String(n).padStart(2, "0");

/** Midnight of a YYYY-MM-DD day, local time. */
function localDay(iso: string): Date {
  const [y, m, d] = iso.split("-").map(Number);
  return new Date(y, m - 1, d);
}

/** Calendar days from today to `iso` (negative: before today). UTC arithmetic, so a day the
 *  clocks change is still one day. */
function daysFromToday(iso: string, now: Date): number {
  const [y, m, d] = iso.split("-").map(Number);
  return Math.round((Date.UTC(y, m - 1, d) - Date.UTC(now.getFullYear(), now.getMonth(), now.getDate())) / 86_400_000);
}

/** "3 Nov", with the year when it isn't this one ("5 Jan 2027"). */
export function shortDate(day: Date, now = new Date()): string {
  return day.toLocaleDateString("en-GB", day.getFullYear() === now.getFullYear() ? { day: "numeric", month: "short" } : { day: "numeric", month: "short", year: "numeric" });
}

/** The due chip: "Today", "Tomorrow", "Yesterday", the weekday in the week ahead ("Fri"), else
 *  the date ("3 Nov"). */
export function dueLabel(iso: string, now = new Date()): string {
  const days = daysFromToday(iso, now);
  if (days === 0) return "Today";
  if (days === 1) return "Tomorrow";
  if (days === -1) return "Yesterday";
  if (days > 1 && days < 7) return localDay(iso).toLocaleDateString("en-GB", { weekday: "short" });
  return shortDate(localDay(iso), now);
}

/** Before today: an open task due then is overdue. */
export const isPast = (iso: string, now = new Date()) => daysFromToday(iso, now) < 0;

type QuickDue = "today" | "tomorrow" | "next_week";

/** The due chip menu's days. "Next week" is next week's Friday: a due date is a deadline, as
 *  when "next week" is said in a meeting (D-042), so it isn't overdue from Tuesday. */
export function quickDue(pick: QuickDue, now = new Date()): string {
  const weekday = (now.getDay() + 6) % 7; // Monday 0 … Sunday 6
  const ahead = pick === "today" ? 0 : pick === "tomorrow" ? 1 : 7 - weekday + 4;
  const day = new Date(now.getFullYear(), now.getMonth(), now.getDate() + ahead);
  return `${day.getFullYear()}-${pad(day.getMonth() + 1)}-${pad(day.getDate())}`;
}
