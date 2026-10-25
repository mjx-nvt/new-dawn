import { useBackupNow, useBackupStatus, useChooseBackupFolder } from "../../kernel/bridge/onboarding";

/** Settings → Backup: a daily copy of every database and brand folder into a folder you choose. */
export default function BackupPanel() {
  const status = useBackupStatus();
  const choose = useChooseBackupFolder();
  const now = useBackupNow();
  const b = status.data;
  const error = choose.error ?? now.error ?? (b?.last_error ? { message: b.last_error } : null);

  return (
    <>
      <div className="group">
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">Backup folder</p>
            <p className="rowHint">{b?.folder ?? "Off until you choose a folder."}</p>
          </div>
          <div className="rowControl">
            <button type="button" className="btn btnSecondary btnCompact" disabled={choose.isPending} onClick={() => choose.mutate(false)}>
              {b?.folder ? "Change…" : "Choose…"}
            </button>
            {b?.folder && (
              <button type="button" className="btn btnGhost btnCompact" disabled={choose.isPending} onClick={() => choose.mutate(true)}>
                Turn off
              </button>
            )}
          </div>
        </div>
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">Last backup</p>
            <p className="rowHint">{b?.last_at ? new Date(b.last_at).toLocaleString("en-GB") : "Not yet captured"}</p>
          </div>
          <div className="rowControl">
            <button type="button" className="btn btnSecondary btnCompact" disabled={!b?.folder || now.isPending} onClick={() => now.mutate()}>
              {now.isPending ? "Backing up…" : "Back up now"}
            </button>
          </div>
        </div>
      </div>
      <p className="fieldHint" style={{ marginTop: "var(--space-3)" }}>
        Once a day while New Dawn runs. Keeps the newest 7. Recordings and exported files are not included.
      </p>
      {error && (
        <p className="formError" role="alert">
          {error.message}
        </p>
      )}
    </>
  );
}
