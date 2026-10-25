// Self-check for src/modules/meetings/listing.ts: `node --experimental-strip-types scripts/check-listing.ts`
// (no test runner in the frontend; this is the one runnable check for the date parsing).
import assert from "node:assert/strict";
import { applyFilters, groupLabel, parseDateQuery } from "../src/modules/meetings/listing.ts";

const now = new Date(2026, 9, 24, 15, 0); // Sat 24 Oct 2026, local

const p = (q: string) => {
  const { range, rest } = parseDateQuery(q, now);
  return range ? [range.from.toDateString(), range.to.toDateString(), rest] : [null, null, rest];
};

assert.deepEqual(p("24 Oct"), ["Sat Oct 24 2026", "Sun Oct 25 2026", ""]);
assert.deepEqual(p("24/10/2026 Wired Score"), ["Sat Oct 24 2026", "Sun Oct 25 2026", "Wired Score"]);
assert.deepEqual(p("Oct 2026 kickoff"), ["Thu Oct 01 2026", "Sun Nov 01 2026", "kickoff"]);
assert.deepEqual(p("2026-09"), ["Tue Sep 01 2026", "Thu Oct 01 2026", ""]);
assert.deepEqual(p("yesterday"), ["Fri Oct 23 2026", "Sat Oct 24 2026", ""]);
assert.deepEqual(p("last week harry"), ["Mon Oct 12 2026", "Mon Oct 19 2026", "harry"]);
assert.deepEqual(p("31/02/2026"), [null, null, "31/02/2026"], "an impossible date is just text");
assert.deepEqual(p("may the budget"), ["Fri May 01 2026", "Mon Jun 01 2026", "the budget"]);
assert.deepEqual(p("Wired Score"), [null, null, "Wired Score"]);
assert.deepEqual(p("Market review"), [null, null, "Market review"], "not March");
assert.deepEqual(p("decision log"), [null, null, "decision log"], "not December");
assert.deepEqual(p("Octo"), ["Thu Oct 01 2026", "Sun Nov 01 2026", ""]);

const m = (title: string, iso: string, status = "summarised", duration_ms: number | null = 60_000, meeting_type_id: string | null = null) => ({
  title,
  held_on: iso,
  status,
  duration_ms,
  meeting_type_id,
});
const list = [m("B", "2026-10-24T03:00:00.000Z"), m("A", "2026-10-01T03:00:00.000Z", "transcribed", 120_000, "t1"), m("C", "2025-12-05T03:00:00.000Z", "failed")];
const base = { status: "all" as const, type: "", when: "all" as const, sort: "newest" as const };
assert.deepEqual(applyFilters(list, base, null, now).map((x) => x.title), ["B", "A", "C"]);
assert.deepEqual(applyFilters(list, { ...base, sort: "title" }, null, now).map((x) => x.title), ["A", "B", "C"]);
assert.deepEqual(applyFilters(list, { ...base, sort: "longest" }, null, now).map((x) => x.title), ["A", "B", "C"]);
assert.deepEqual(applyFilters(list, { ...base, status: "needs_summary" }, null, now).map((x) => x.title), ["A"]);
assert.deepEqual(applyFilters(list, { ...base, type: "none" }, null, now).map((x) => x.title), ["B", "C"]);
assert.deepEqual(applyFilters(list, { ...base, when: "30d" }, null, now).map((x) => x.title), ["B", "A"]);
assert.deepEqual(applyFilters(list, base, parseDateQuery("Dec 2025", now).range, now).map((x) => x.title), ["C"]);

assert.equal(groupLabel("2026-10-24T03:00:00.000Z", now), "Today");
assert.equal(groupLabel("2026-10-23T03:00:00.000Z", now), "Yesterday");
assert.equal(groupLabel("2026-10-20T03:00:00.000Z", now), "This week");
assert.equal(groupLabel("2026-10-15T03:00:00.000Z", now), "Last week");
assert.equal(groupLabel("2026-10-02T03:00:00.000Z", now), "Earlier this month");
assert.equal(groupLabel("2026-09-02T03:00:00.000Z", now), "September 2026");

console.log("listing.check: ok");
