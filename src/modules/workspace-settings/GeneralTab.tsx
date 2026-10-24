import { useGlassStatus } from "../../kernel/bridge/glass";
import { openPrivacySettings, useMicrophonePermission, useRequestMicrophone, type PermissionState } from "../../kernel/bridge/recorder";
import { useCurrentWorkspace } from "../../kernel/bridge/workspaces";
import styles from "./Settings.module.css";

const MIC_LABEL: Record<PermissionState, string> = {
  granted: "Allowed",
  denied: "Denied",
  not_determined: "Not asked yet",
  restricted: "Restricted",
};

const GLASS_LABELS = { liquid_glass: "Liquid Glass", vibrancy: "Vibrancy (older macOS)", none: "None" } as const;

function Field({ label, value }: { label: string; value: string | undefined }) {
  return (
    <div>
      <p className={styles.label}>{label}</p>
      {value ? <p className={styles.value}>{value}</p> : <p className="ghostText">Not yet captured</p>}
    </div>
  );
}

export default function GeneralTab() {
  const { data: workspace } = useCurrentWorkspace();
  const { data: glass } = useGlassStatus();
  const mic = useMicrophonePermission();
  const requestMic = useRequestMicrophone();

  return (
    <>
      <section className={styles.section}>
        <h2 className={styles.sectionTitle}>Workspace</h2>
        <p className={styles.sectionSubtitle}>The workspace New Dawn opens on launch</p>
        <div className={styles.fields}>
          <Field label="Current workspace" value={workspace?.name} />
          <Field label="Folder" value={workspace?.folder_path} />
        </div>
      </section>

      <section className={styles.section}>
        <h2 className={styles.sectionTitle}>Permissions</h2>
        <p className={styles.sectionSubtitle}>Recording needs your microphone and the call's audio from this Mac</p>
        <div className={styles.fields}>
          <div>
            <p className={styles.label}>Microphone</p>
            <div className={styles.permission}>
              {mic.data ? (
                <span className={mic.data === "granted" ? "chip chipBrand" : "chip"}>{MIC_LABEL[mic.data]}</span>
              ) : (
                <span className="ghostText">Not yet captured</span>
              )}
              {mic.data === "not_determined" && (
                <button type="button" className="btn btnSecondary btnCompact" onClick={() => requestMic.mutate()}>
                  Allow…
                </button>
              )}
              {(mic.data === "denied" || mic.data === "restricted") && (
                <button type="button" className="btn btnGhost btnCompact" onClick={() => openPrivacySettings(false)}>
                  Open Settings
                </button>
              )}
            </div>
          </div>
          <div>
            <p className={styles.label}>System audio</p>
            <div className={styles.permission}>
              <span className="chip">Checked while recording</span>
              <button type="button" className="btn btnGhost btnCompact" onClick={() => openPrivacySettings(true)}>
                Open Settings
              </button>
            </div>
            <p className="fieldHint">macOS has no way to ask in advance; a banner appears if the other side is silent.</p>
          </div>
        </div>
      </section>

      <section className={styles.section}>
        <h2 className={styles.sectionTitle}>Window</h2>
        <p className={styles.sectionSubtitle}>Appearance follows macOS: light or dark, accent colour, Reduce Transparency</p>
        <div className={styles.fields}>
          <Field label="Window glass" value={glass && GLASS_LABELS[glass]} />
        </div>
      </section>
    </>
  );
}
