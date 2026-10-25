import { useState } from "react";
import { formatBytes, useCompressRecordings, useSetRetentionDays, useStorageReport, useTrashOldRecordings } from "../../kernel/bridge/storage";
import DangerButton from "../../kernel/shell/DangerButton";

const RETENTION = [
  [30, "30 days"],
  [90, "90 days"],
  [180, "6 months"],
  [365, "1 year"],
  [0, "Forever"],
] as const;

/** Settings → Storage: what takes space, and the two levers — compress originals, and let old
 *  recordings go (the transcript, summary and minutes always stay). */
export default function StoragePanel() {
  const report = useStorageReport();
  const setRetention = useSetRetentionDays();
  const compress = useCompressRecordings();
  const trash = useTrashOldRecordings();
  const [done, setDone] = useState<string | null>(null);
  const r = report.data;
  const error = report.error ?? setRetention.error ?? compress.error ?? trash.error;
  const total = r ? r.workspaces.reduce((sum, w) => sum + (w.recordings_bytes ?? 0) + (w.exports_bytes ?? 0) + (w.database_bytes ?? 0), 0) + (r.models_bytes ?? 0) + (r.logs_bytes ?? 0) : 0;

  return (
    <>
      <p className="groupTitle">Recordings</p>
      <div className="group">
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">Keep recordings for</p>
            <p className="rowHint">Counted from the day the summary was written; checked daily. The audio goes to the Trash; transcript, summary and minutes stay.</p>
          </div>
          <div className="rowControl">
            <div className="segmented" role="group" aria-label="Keep recordings for">
              {RETENTION.map(([days, label]) => (
                <button key={days} type="button" aria-pressed={r?.retention_days === days} disabled={!r || setRetention.isPending} onClick={() => setRetention.mutate(days)}>
                  {label}
                </button>
              ))}
            </div>
          </div>
        </div>
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">Compression</p>
            <p className="rowHint">After transcription, recordings are compressed to about 20 MB an hour.</p>
          </div>
        </div>
      </div>

      <p className="groupTitle">Space used{r ? ` · ${formatBytes(total)}` : ""}</p>
      <div className="group">
        {r?.workspaces.length === 0 && (
          <div className="groupRow">
            <span className="ghostText">Not yet captured — no workspace yet</span>
          </div>
        )}
        {r ? (
          r.workspaces.map((w) => (
            <div key={w.slug} className="groupRow">
              <div className="rowText">
                <p className="rowLabel">{w.name}</p>
                <p className="rowHint">
                  Recordings {formatBytes(w.recordings_bytes)}
                  {w.originals > 0 && ` (${w.originals} not yet compressed)`} · exports {formatBytes(w.exports_bytes)} · database {formatBytes(w.database_bytes)}
                </p>
              </div>
              <div className="rowControl">
                {w.originals > 0 && (
                  <button
                    type="button"
                    className="btn btnSecondary btnCompact"
                    disabled={compress.isPending}
                    onClick={() => {
                      setDone(null);
                      compress.mutate(w.slug, { onSuccess: (n) => setDone(`${n} recording${n === 1 ? "" : "s"} compressed`) });
                    }}
                  >
                    {compress.isPending ? "Compressing…" : "Compress now"}
                  </button>
                )}
                {r.retention_days > 0 ? (
                  <DangerButton
                    confirm={`Trash recordings older than ${r.retention_days} days?`}
                    disabled={trash.isPending}
                    onConfirm={() => {
                      setDone(null);
                      trash.mutate({ slug: w.slug, days: r.retention_days }, { onSuccess: (n) => setDone(`${n} recording${n === 1 ? "" : "s"} moved to the Trash`) });
                    }}
                  >
                    Trash old recordings now
                  </DangerButton>
                ) : (
                  <span className="fieldHint">Kept forever</span>
                )}
              </div>
            </div>
          ))
        ) : (
          <div className="groupRow">
            <span className="ghostText">Not yet captured</span>
          </div>
        )}
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">Speech model</p>
            <p className="rowHint">Whisper, downloaded once. Manage it under Models.</p>
          </div>
          <div className="rowControl">{r ? <span className="chip">{formatBytes(r.models_bytes)}</span> : <span className="ghostText">Not yet captured</span>}</div>
        </div>
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">Logs</p>
            <p className="rowHint">Seven days are kept.</p>
          </div>
          <div className="rowControl">{r ? <span className="chip">{formatBytes(r.logs_bytes)}</span> : <span className="ghostText">Not yet captured</span>}</div>
        </div>
      </div>
      {done && (
        <p className="fieldHint" role="status" style={{ marginTop: "var(--space-3)" }}>
          {done}
        </p>
      )}
      {error && (
        <p className="formError" role="alert">
          {error.message}
        </p>
      )}
    </>
  );
}
