import {
  revealModelsFolder,
  useSetWhisperIdleUnload,
  useWhisperIdleUnload,
  useCancelModelDownload,
  useDeleteModel,
  useModelDownloads,
  useModels,
  useStartModelDownload,
  type DownloadProgress,
  type ModelInfo,
} from "../../kernel/bridge/models";
import type { AppError } from "../../kernel/bridge/unwrap";
import styles from "./ModelsPanel.module.css";

const MB = 1024 * 1024;

function formatSize(bytes: number): string {
  return bytes >= 1024 * MB ? `${(bytes / (1024 * MB)).toFixed(2)} GB` : `${Math.round(bytes / MB)} MB`;
}

/** The Whisper models with download / resume / delete (also shown in the welcome steps). */
export function SpeechModels() {
  const models = useModels();
  const { progress, errors } = useModelDownloads();
  return <ModelList models={models.data ?? []} progress={progress} errors={errors} loading={models.isPending} />;
}

/** Settings → Models: download, resume, cancel and delete the local Whisper and VAD models. */
export default function ModelsPanel() {
  return (
    <>
      <section className="section">
        <h2 className="sectionTitle">Speech model</h2>
        <p className="sectionSubtitle">Whisper transcribes recordings on this Mac; turbo is the default</p>
        <SpeechModels />
      </section>

      <section className="section">
        <h2 className="sectionTitle">Voice detection</h2>
        <p className="sectionSubtitle">Silero finds where people speak, so Whisper skips the silence</p>
        <ul className={styles.list}>
          <li className={`card ${styles.row}`}>
            <div className={styles.main}>
              <p className={styles.name}>
                Silero VAD v5.1.2
                <span className="fieldHint">0.9 MB</span>
                <span className="chip chipBrand">Built in</span>
              </p>
              <p className="fieldHint">Ships with New Dawn; nothing to download.</p>
            </div>
          </li>
        </ul>
      </section>

      <section className="section">
        <h2 className="sectionTitle">Memory</h2>
        <p className="sectionSubtitle">How long Whisper stays loaded after a transcription (it uses 1.5–3 GB)</p>
        <IdleUnload />
      </section>

      <section className="section">
        <h2 className="sectionTitle">Storage</h2>
        <p className="sectionSubtitle">Files come from Hugging Face at a pinned version and are checked (SHA-256) before use</p>
        <button type="button" className="btn btnGhost btnCompact" onClick={() => void revealModelsFolder()}>
          Reveal models folder
        </button>
      </section>
    </>
  );
}

const IDLE_CHOICES = [
  [60, "1 min"],
  [120, "2 min"],
  [300, "5 min"],
  [900, "15 min"],
] as const;

function IdleUnload() {
  const { data: secs } = useWhisperIdleUnload();
  const save = useSetWhisperIdleUnload();
  return (
    <div className="segmented" role="group" aria-label="Unload Whisper after">
      {IDLE_CHOICES.map(([value, label]) => (
        <button key={value} type="button" aria-pressed={secs === value} onClick={() => save.mutate(value)}>
          {label}
        </button>
      ))}
    </div>
  );
}

interface ListProps {
  models: ModelInfo[];
  progress: Record<string, DownloadProgress>;
  errors: Record<string, AppError | null>;
  loading: boolean;
}

function ModelList({ models, progress, errors, loading }: ListProps) {
  if (loading || !models.length) return <div className={`${styles.ghost} ghostCard`} />;
  return (
    <ul className={styles.list}>
      {models.map((model) => (
        <ModelRow key={model.id} model={model} progress={progress[model.id]} error={errors[model.id]} />
      ))}
    </ul>
  );
}

function ModelRow({ model, progress, error }: { model: ModelInfo; progress?: DownloadProgress; error?: AppError | null }) {
  const start = useStartModelDownload();
  const cancel = useCancelModelDownload();
  const remove = useDeleteModel();
  const status = model.status;
  const done = progress?.downloaded ?? (status.state === "partial" ? status.bytes : 0);
  const percent = Math.min(100, (done / model.size_bytes) * 100);
  const failure = start.error ?? remove.error ?? error;

  return (
    <li className={`card ${styles.row}`}>
      <div className={styles.main}>
        <p className={styles.name}>
          {model.label}
          <span className="fieldHint">{formatSize(model.size_bytes)}</span>
          {status.state === "ready" && <span className="chip chipBrand">Ready</span>}
          {status.state === "partial" && <span className="chip">Paused at {Math.round(percent)} %</span>}
        </p>
        <p className="fieldHint">{model.description}</p>

        {status.state === "downloading" && (
          <div className={styles.progress}>
            <div className={styles.bar}>
              <span style={{ width: `${percent}%` }} />
            </div>
            <span className="fieldHint">
              {formatSize(done)} of {formatSize(model.size_bytes)}
              {progress && ` · ${(progress.bytesPerSec / MB).toFixed(1)} MB/s`}
            </span>
          </div>
        )}
        {failure && failure.message && <p className={styles.error}>{failure.message}</p>}
      </div>

      <div className={styles.actions}>
        {status.state === "downloading" ? (
          <button type="button" className="btn btnGhost btnCompact" onClick={() => cancel.mutate(model.id)}>
            Cancel
          </button>
        ) : status.state === "ready" ? (
          <button type="button" className="btn btnGhost btnCompact" onClick={() => remove.mutate(model.id)}>
            Delete
          </button>
        ) : (
          <>
            {status.state === "partial" && (
              <button type="button" className="btn btnGhost btnCompact" onClick={() => remove.mutate(model.id)}>
                Discard
              </button>
            )}
            <button type="button" className="btn btnPrimary btnCompact" onClick={() => start.mutate(model.id)}>
              {status.state === "partial" ? "Resume" : "Download"}
            </button>
          </>
        )}
      </div>
    </li>
  );
}
