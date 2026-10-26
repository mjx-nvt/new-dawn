// Self-check for the Live words in src/modules/meetings/format.ts and the line layout and Copy all
// in src/modules/live/lines.ts: `node --experimental-strip-types scripts/check-live.ts` (no test
// runner in the frontend).
import assert from "node:assert/strict";
import { columnTitles, copyText, DIRECTION_LABEL, directionOf, formatLabel, layoutLines, remembered } from "../src/modules/live/lines.ts";
import { liveStatusWord, pendingEnglish } from "../src/modules/meetings/format.ts";

const live = {
  phase: "listening",
  behind: 0,
  dropped: 0,
  translation_off_reason: null,
  model: "qwen3.5:4b",
  quick: false,
  direction: "vi_en",
  format: "discussion",
  presenter: null,
  overheard: 0,
} as const;
const off = { ...live, phase: "ready", model: null, translation_off_reason: "Ollama isn't running" } as const;

assert.equal(liveStatusWord(live, false), "Listening");
assert.equal(liveStatusWord({ ...live, phase: "translating" }, false), "Translating");
assert.equal(liveStatusWord({ ...live, phase: "transcribing", behind: 1 }, false), "Behind by 1 line");
assert.equal(liveStatusWord({ ...live, behind: 3 }, false), "Behind by 3 lines");
assert.equal(liveStatusWord({ ...live, behind: 3 }, true), "Paused", "paused wins");
assert.equal(liveStatusWord(off, false), "Vietnamese only — Ollama isn't running");
assert.equal(liveStatusWord({ ...off, direction: "en_vi" }, false), "English only — Ollama isn't running");
assert.equal(liveStatusWord({ ...off, direction: "both" }, false), "Transcript only — Ollama isn't running");
assert.equal(liveStatusWord({ ...off, phase: "transcribing" }, false), "Transcribing");

// Lines are translated in order: after the last translated one they're on their way…
assert.equal(pendingEnglish(5, 3, live), "Translating…");
assert.equal(pendingEnglish(0, -1, live), "Translating…", "nothing translated yet");
// …before it, a line was given up on.
assert.equal(pendingEnglish(2, 3, live), "Not translated");
// Translation off: the column head says why, once.
assert.equal(pendingEnglish(5, 3, off), null);
// After the session.
assert.equal(pendingEnglish(5, 3, null), "Not translated");

// Which way: the chip, the column heads, and a saved meeting's direction from its language.
assert.deepEqual(columnTitles("vi_en"), ["Tiếng Việt", "English"]);
assert.deepEqual(columnTitles("en_vi"), ["English", "Tiếng Việt"]);
assert.deepEqual(columnTitles("both"), ["Said", "Translation"]);
assert.equal(directionOf("vi"), "vi_en");
assert.equal(directionOf("en"), "en_vi");
assert.equal(directionOf("auto"), "both");
assert.equal(DIRECTION_LABEL.en_vi, "English → Tiếng Việt");
assert.equal(formatLabel("discussion", "Minh"), "Discussion");
assert.equal(formatLabel("presentation", "Minh"), "One presenter · Minh");
assert.equal(formatLabel("presentation", null), "One presenter");

const line = (start_ms: number, end_ms: number, text: string, more: object = {}) => ({ start_ms, end_ms, text, translation: null, speaker: null, clarity: null, ...more });

// A discussion: a label at each change of speaker, never a paragraph.
const call = [
  line(0, 2_000, "Chào anh.", { speaker: "you" }),
  line(2_500, 4_000, "Chào em.", { speaker: "them" }),
  line(4_200, 6_000, "Anh khoẻ không?", { speaker: "them" }),
  line(6_500, 8_000, "Dạ khoẻ.", { speaker: "you" }),
];
assert.deepEqual(
  layoutLines(call, "discussion").map((l) => l.label),
  ["You", "Them", null, "You"],
);
assert.ok(
  layoutLines(call, "discussion").every((l) => !l.continues),
  "a discussion has no paragraphs",
);
assert.deepEqual(
  layoutLines([line(0, 1_000, "Một."), line(1_200, 2_000, "Hai.")], "discussion").map((l) => l.label),
  [null, null],
  "before the call's first line, or in a room: no labels",
);

// A presenter: a line that starts under 1.5 s after the last one ended carries on its paragraph.
const talk = [
  line(0, 9_000, "Một."),
  line(9_800, 15_000, "Hai."), // 0.8 s later
  line(17_000, 20_000, "Ba."), // 2 s later: a new paragraph
  line(21_400, 25_000, "Bốn."), // 1.4 s later
  line(26_500, 30_000, "Năm."), // exactly 1.5 s later: a new one
];
assert.deepEqual(
  layoutLines(talk, "presentation").map((l) => l.continues),
  [false, true, false, true, false],
);
assert.ok(
  layoutLines(talk, "presentation").every((l) => !l.label),
  "a presenter's lines have no speaker",
);
assert.deepEqual(
  layoutLines([line(0, 1_000, "a", { speaker: "them" }), line(1_200, 2_000, "b", { speaker: "you" })], "presentation"),
  [
    { label: "Them", continues: false },
    { label: "You", continues: false },
  ],
  "a change of speaker starts a new block, even in a presentation",
);

// Copy all: time and who starts speaking, what was said, its translation — with the markers in words.
assert.equal(
  copyText(
    [
      line(5_000, 8_000, "Chào anh.", { speaker: "them", translation: "Hello." }),
      line(8_500, 9_000, "Ờ…", { speaker: "them", clarity: "unclear" }),
      line(65_000, 70_000, "Em gửi bảng tính nhé.", { speaker: "you", translation: "I'll send the spreadsheet.", clarity: "uncertain" }),
    ],
    "discussion",
  ),
  "0:05 · Them\nChào anh.\nHello.\n\n0:08\nỜ… [unclear]\n[not translated]\n\n1:05 · You\nEm gửi bảng tính nhé.\nI'll send the spreadsheet. [may be inaccurate]",
);
assert.equal(
  copyText(
    [line(0, 9_000, "Một.", { translation: "One." }), line(9_800, 15_000, "Hai.", { translation: "Two." }), line(17_000, 20_000, "Ba.")],
    "presentation",
  ),
  "0:00\nMột. Hai.\nOne. Two.\n\n0:17\nBa.\n[not translated]",
  "a presenter's paragraph is copied as one",
);
assert.equal(copyText([line(0, 1_000, "Một."), line(3_000, 4_000, "Hai.")], "discussion"), "0:00\nMột.\n\n0:03\nHai.", "translation off: what was said only");
assert.equal(copyText([], "discussion"), "");

// No storage (as here, under node): the first option.
assert.equal(remembered("newdawn.liveKeep", ["save", "quick"]), "save");

console.log("✅ live words check passed");
