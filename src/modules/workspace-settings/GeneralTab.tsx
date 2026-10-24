import { useGlassStatus } from "../../kernel/bridge/glass";
import { useBackupNow, useBackupStatus, useChooseBackupFolder } from "../../kernel/bridge/onboarding";
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

      <BackupSection />

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

/** Daily copy of every database into a folder you choose (newest 7 kept). */
function BackupSection() {
  const status = useBackupStatus();
  const choose = useChooseBackupFolder();
  const now = useBackupNow();
  const b = status.data;
  const error = choose.error ?? now.error ?? (b?.last_error ? { message: b.last_error } : null);
  return (
    <section className={styles.section}>
      <h2 className={styles.sectionTitle}>Backup</h2>
      <p className={styles.sectionSubtitle}>
        Once a day while New Dawn runs: the workspace databases (meetings, transcripts, summaries), newest 7 kept. Recordings
        and exported files are not included.
      </p>
      <div className={styles.fields}>
        <Field label="Folder" value={b?.folder ?? undefined} />
        <Field label="Last backup" value={b?.last_at ? new Date(b.last_at).toLocaleString("en-GB") : undefined} />
      </div>
      <div className={styles.permission}>
        <button type="button" className="btn btnSecondary btnCompact" disabled={choose.isPending} onClick={() => choose.mutate(false)}>
          {b?.folder ? "Change folder…" : "Choose folder…"}
        </button>
        {b?.folder && (
          <>
            <button type="button" className="btn btnSecondary btnCompact" disabled={now.isPending} onClick={() => now.mutate()}>
              {now.isPending ? "Backing up…" : "Back up now"}
            </button>
            <button type="button" className="btn btnGhost btnCompact" disabled={choose.isPending} onClick={() => choose.mutate(true)}>
              Turn off
            </button>
          </>
        )}
      </div>
      {error && (
        <p className="formError" role="alert">
          {error.message}
        </p>
      )}
    </section>
  );
}
