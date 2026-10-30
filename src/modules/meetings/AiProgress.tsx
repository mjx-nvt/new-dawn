import { useEffect, useState } from "react";
import type { TranscriptionProgress } from "../../kernel/bridge/meetings";
import type { SummaryProgress } from "../../kernel/bridge/summary";
import { STAGE_LABEL } from "./format";
import styles from "./AiProgress.module.css";

/** Re-renders every second while mounted (elapsed / time left). */
export function useNow(): number {
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(t);
  }, []);
  return now;
}

const clock = (ms: number) => {
  const s = Math.max(0, Math.round(ms / 1000));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
};

/** "about 2 min left" / "about 40 s left" / "almost done". */
export function timeLeft(ms: number | null): string | null {
  if (ms == null || !Number.isFinite(ms)) return null;
  if (ms < 5_000) return "almost done";
  if (ms < 60_000) return `about ${Math.round(ms / 5_000) * 5} s left`;
  return `about ${Math.round(ms / 60_000)} min left`;
}

/** One AI progress bar: label, bar (null fraction = indeterminate), elapsed and time left. */
export default function AiProgress({ label, fraction, startedAt, etaMs, name }: { label: string; fraction: number | null; startedAt: number; etaMs: number | null; name: string }) {
  const now = useNow();
  const left = timeLeft(etaMs);
  const pct = fraction == null ? null : Math.round(Math.min(1, Math.max(0, fraction)) * 100);
  return (
    <div className={styles.progress}>
      <div
        className={pct == null ? `${styles.bar} ${styles.indeterminate}` : styles.bar}
        role="progressbar"
        aria-label={name}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={pct ?? undefined}
        aria-valuetext={[label, pct != null ? `${pct} %` : null, left].filter(Boolean).join(", ")}
      >
        <span style={pct == null ? undefined : { width: `${pct}%` }} />
      </div>
      <span className={`fieldHint ${styles.meta}`}>
        {label} · {clock(now - startedAt)}
        {left ? ` · ${left}` : ""}
      </span>
    </div>
  );
}

/** Whisper reports a percentage: time left = elapsed × remaining / done, once it's moving. */
export function TranscriptionBar({ progress, title }: { progress: TranscriptionProgress; title?: string }) {
  const now = useNow();
  const elapsed = now - progress.startedAt;
  const eta = progress.percent >= 5 && progress.stage !== "queued" ? (elapsed * (100 - progress.percent)) / progress.percent : null;
  return (
    <AiProgress
      name={title ? `Transcription progress, ${title}` : "Transcription progress"}
      label={`${STAGE_LABEL[progress.stage]} · ${progress.percent} %`}
      fraction={progress.percent / 100}
      startedAt={progress.startedAt}
      etaMs={eta}
    />
  );
}

const SUMMARY_LABEL: Record<SummaryProgress["stage"], string> = {
  queued: "Waiting for another AI job",
  writing: "Writing the summary",
  merging: "Merging the parts",
  naming: "Checking the names",
  email: "Writing the cover note",
  tasks: "Finding your tasks",
};

/** Summary, naming, email and tasks stages: time left from the live token rate against what past runs
 *  of the same agent wrote (`expected_tokens`), or from their usual duration (`expected_ms`)
 *  before the first token. No history yet: elapsed only. */
export function SummaryBar({ progress, title }: { progress: SummaryProgress; title?: string }) {
  const now = useNow();
  const { stage, part, parts, tokens, expected_ms, expected_tokens, later_ms, stageStartedAt, firstTokenAt } = progress;
  const elapsed = now - stageStartedAt;
  let fraction: number | null = null;
  let eta: number | null = null;
  if (stage !== "queued") {
    if (firstTokenAt && tokens > 20 && expected_tokens > 0) {
      const rate = tokens / Math.max(1, now - firstTokenAt); // tokens per ms
      fraction = Math.min(0.97, tokens / expected_tokens);
      eta = Math.max(0, expected_tokens - tokens) / rate;
    } else if (expected_ms > 0) {
      fraction = Math.min(0.95, elapsed / expected_ms);
      eta = Math.max(0, expected_ms - elapsed);
    }
    // More parts to come (a long transcript is summarised in parts, then merged).
    if (parts > 1 && stage === "writing" && expected_ms > 0) {
      eta = (eta ?? 0) + (parts - part + 1) * expected_ms;
      fraction = fraction == null ? null : (part - 1 + fraction) / (parts + 1);
    }
    // Then the names check, the cover note and finding your tasks.
    if (eta != null) eta += later_ms;
  }
  const step = parts > 1 && stage !== "naming" && stage !== "email" ? ` · part ${part} of ${parts}` : "";
  return (
    <AiProgress
      name={title ? `Summary progress, ${title}` : "Summary progress"}
      label={`${SUMMARY_LABEL[stage]}${step}${tokens > 0 ? ` · ${tokens} tokens` : ""}`}
      fraction={fraction}
      startedAt={stageStartedAt}
      etaMs={eta}
    />
  );
}
