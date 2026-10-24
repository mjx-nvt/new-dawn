import { openPrivacySettings, useRecorderStatus } from "../../kernel/bridge/recorder";
import styles from "./RecorderBanner.module.css";

/** Shown above the page while recording if the other side of the call isn't being captured.
 *  The live region is always mounted so screen readers announce the message when it appears. */
export default function RecorderBanner() {
  const { data: status } = useRecorderStatus();
  const live = status?.phase === "recording" || status?.phase === "paused";
  const problem = live && status.system_audio !== "ok" ? status.system_audio : null;

  return (
    <div role={problem === "silent" ? "alert" : "status"}>
      {problem && (
        <div className={problem === "silent" ? `${styles.banner} ${styles.warning}` : styles.banner}>
          <p>
            {problem === "silent"
              ? "Can't hear the other side of the call. Allow New Dawn under System Audio Recording, then start a new recording."
              : "Recording your microphone only — system audio isn't available on this Mac."}
          </p>
          {problem === "silent" && (
            <button type="button" className="btn btnSecondary btnCompact" onClick={() => openPrivacySettings(true)}>
              Open System Settings
            </button>
          )}
        </div>
      )}
    </div>
  );
}
