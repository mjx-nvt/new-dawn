// Self-check for the Live words in src/modules/meetings/format.ts:
// `node --experimental-strip-types scripts/check-live.ts` (no test runner in the frontend).
import assert from "node:assert/strict";
import { liveStatusWord, pendingEnglish } from "../src/modules/meetings/format.ts";

const live = { phase: "listening", behind: 0, dropped: 0, translation_off_reason: null, model: "qwen3.5:4b" } as const;
const off = { ...live, phase: "ready", model: null, translation_off_reason: "Ollama isn't running" } as const;

assert.equal(liveStatusWord(live, false), "Listening");
assert.equal(liveStatusWord({ ...live, phase: "translating" }, false), "Translating");
assert.equal(liveStatusWord({ ...live, phase: "transcribing", behind: 1 }, false), "Behind by 1 line");
assert.equal(liveStatusWord({ ...live, behind: 3 }, false), "Behind by 3 lines");
assert.equal(liveStatusWord({ ...live, behind: 3 }, true), "Paused", "paused wins");
assert.equal(liveStatusWord(off, false), "Vietnamese only — Ollama isn't running");
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

console.log("✅ live words check passed");
