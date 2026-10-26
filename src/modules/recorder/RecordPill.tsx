import { useRecorderLevel, useRecorderStatus, useSetPaused } from "../../kernel/bridge/recorder";
import { formatClock } from "../meetings/format";
import styles from "./RecordPill.module.css";
import { useStopAndOpen } from "./useStopAndOpen";

/** RMS 0–1 → meter height 0–1 on a 60 dB scale. */
function meterHeight(rms: number): number {
  if (rms <= 0) return 0;
  return Math.min(1, Math.max(0, (20 * Math.log10(rms) + 60) / 60));
}

/** Toolbar record control: ● Record when idle; dot · elapsed · you/them meters · pause · ■ while recording. */
export default function RecordPill({ onStart }: { onStart: () => void }) {
  const { data: status } = useRecorderStatus();
  const phase = status?.phase ?? "idle";
  const level = useRecorderLevel(phase === "recording" || phase === "paused");
  const setPaused = useSetPaused();
  const { stop, isPending: stopping, error: stopError } = useStopAndOpen();
  const error = stopError ?? setPaused.error;

  if (phase === "idle") {
    return (
      <button type="button" className="btn btnRecord btnCompact" onClick={onStart}>
        <span className="recordDot" />
        Record
      </button>
    );
  }

  const paused = phase === "paused";
  const elapsed = level?.elapsed_ms ?? status?.elapsed_ms ?? 0;
  // A quick translation (no meeting) records nothing: its controls don't say "recording".
  const what = status?.meeting_id ? " recording" : "";
  return (
    <div className={`${styles.pill} ${styles.live}`} role="group" aria-label={what ? "Recording" : "Quick translation"}>
      <span className={paused ? styles.dotPaused : `${styles.dot} ${styles.pulse}`} />
      {/* A quick translation (no meeting) saves nothing: it only finishes its last lines. */}
      <span className={styles.clock}>{phase === "stopping" ? (status?.meeting_id ? "Saving…" : "Finishing…") : formatClock(elapsed)}</span>
      <span className={styles.meters} aria-hidden>
        <span title="You" style={{ height: `${Math.max(12, meterHeight(level?.you ?? 0) * 100)}%` }} />
        <span title="Them" className={styles.them} style={{ height: `${Math.max(12, meterHeight(level?.them ?? 0) * 100)}%` }} />
      </span>
      <button
        type="button"
        className={styles.icon}
        aria-label={`${paused ? "Resume" : "Pause"}${what}`}
        disabled={phase === "stopping" || setPaused.isPending}
        onClick={() => setPaused.mutate(!paused)}
      >
        {paused ? "▶" : "❚❚"}
      </button>
      <button type="button" className={styles.icon} aria-label={`Stop${what}`} disabled={phase === "stopping" || stopping} onClick={stop}>
        ■
      </button>
      {error && (
        <span className={styles.error} role="alert" title={error.message}>
          !
          <span className="srOnly">{error.message}</span>
        </span>
      )}
    </div>
  );
}
