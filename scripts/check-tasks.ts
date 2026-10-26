// Self-check for the due days in src/modules/tasks/due.ts:
// `node --experimental-strip-types scripts/check-tasks.ts` (no test runner in the frontend).
// Run it under a time zone with daylight saving too: `TZ=Europe/London node …`.
import assert from "node:assert/strict";
import { dueLabel, isPast, quickDue, shortDate } from "../src/modules/tasks/due.ts";

const now = new Date(2026, 9, 26, 15, 0); // Mon 26 Oct 2026, 15:00 local (London's clocks went back the day before)

assert.equal(dueLabel("2026-10-26", now), "Today");
assert.equal(dueLabel("2026-10-27", now), "Tomorrow");
assert.equal(dueLabel("2026-10-25", now), "Yesterday");
assert.equal(dueLabel("2026-10-28", now), "Wed");
assert.equal(dueLabel("2026-11-01", now), "Sun", "six days ahead: still the weekday");
assert.equal(dueLabel("2026-11-02", now), "2 Nov", "a week ahead: the date");
assert.equal(dueLabel("2026-10-20", now), "20 Oct");
assert.equal(dueLabel("2027-01-05", now), "5 Jan 2027", "another year shows it");
assert.equal(dueLabel("2026-10-26", new Date(2026, 9, 26, 0, 0)), "Today", "just after midnight");
assert.equal(dueLabel("2026-10-27", new Date(2026, 9, 26, 23, 59)), "Tomorrow", "just before midnight");

assert.ok(isPast("2026-10-25", now));
assert.ok(!isPast("2026-10-26", now), "due today isn't overdue");
assert.ok(isPast("2026-10-25", new Date(2026, 9, 26, 0, 30)), "overdue from the next day");

assert.equal(quickDue("today", now), "2026-10-26");
assert.equal(quickDue("tomorrow", now), "2026-10-27");
assert.equal(quickDue("next_week", now), "2026-11-06", "next week's Friday");
assert.equal(quickDue("next_week", new Date(2026, 9, 30)), "2026-11-06", "on a Friday: next week's");
assert.equal(quickDue("next_week", new Date(2026, 10, 1)), "2026-11-06", "on a Sunday: the week from Monday");
assert.equal(quickDue("tomorrow", new Date(2026, 11, 31, 23, 0)), "2027-01-01");
assert.equal(quickDue("tomorrow", new Date(2026, 9, 24, 23, 30)), "2026-10-25", "the night the clocks change");

assert.equal(shortDate(new Date(2026, 9, 24, 9), now), "24 Oct");
assert.equal(shortDate(new Date(2025, 11, 5, 9), now), "5 Dec 2025");

console.log("tasks.check: ok");
