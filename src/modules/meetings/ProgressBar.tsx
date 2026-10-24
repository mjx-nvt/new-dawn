import type { TranscriptionProgress } from "../../kernel/bridge/meetings";
import { STAGE_LABEL } from "./format";
import styles from "./ProgressBar.module.css";

export default function ProgressBar({ progress, label }: { progress: TranscriptionProgress; label?: string }) {
  return (
    <div className={styles.progress}>
      <div
        className={styles.bar}
        role="progressbar"
        aria-label={label ? `Transcription progress, ${label}` : "Transcription progress"}
        aria-valuenow={progress.percent}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuetext={`${STAGE_LABEL[progress.stage]}, ${progress.percent} %`}
      >
        <span style={{ width: `${progress.percent}%` }} />
      </div>
      <span className="fieldHint">
        {STAGE_LABEL[progress.stage]} · {progress.percent} %
      </span>
    </div>
  );
}
