import {
  revealModelsFolder,
  useSetWhisperIdleUnload,
  useWhisperIdleUnload,
  useCancelModelDownload,
  useDeleteModel,
  useModelDownloads,
  useModels,
  useSetSpeechModel,
  useSpeechChoices,
  useStartModelDownload,
  type DownloadProgress,
  type ModelInfo,
  type SpeechJob,
} from "../../kernel/bridge/models";
import { useAiPolicy, useSetAiPolicy } from "../../kernel/bridge/activity";
import type { AppError } from "../../kernel/bridge/unwrap";
import MenuButton from "../../kernel/shell/MenuButton";
import styles from "./ModelsPanel.module.css";
import ModelCheckSection from "./ModelCheck";
import TextModels from "./TextModels";
import { useOllamaPull } from "../../kernel/bridge/onboarding";
import { useOllamaStatus, useRemoveOllamaModel } from "../../kernel/bridge/summary";

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

/** Settings → Models: the text model (Ollama) and the local Whisper and VAD models. */
export default function ModelsPanel() {
  return (
    <>
      <section className="section">
        <h2 className="sectionTitle">Text model</h2>
        <p className="sectionSubtitle">Writes the summaries, through Ollama on this Mac</p>
        <TextModels />
      </section>

      <section className="section">
        <h2 className="sectionTitle">Search by meaning</h2>
        <p className="sectionSubtitle">Finds meetings by what was meant, in Vietnamese or English, through Ollama on this Mac</p>
        <EmbedModel />
      </section>

      <section className="section">
        <h2 className="sectionTitle">Speech model</h2>
        <p className="sectionSubtitle">Whisper transcribes on this Mac: the most accurate model for meetings, the fastest for Live</p>
        <SpeechModels />
        <SpeechChoices />
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
                <span className="chip chipPrimary">Built in</span>
              </p>
              <p className="fieldHint">Ships with New Dawn; nothing to download.</p>
            </div>
          </li>
        </ul>
      </section>

      <section className="section">
        <h2 className="sectionTitle">While you work</h2>
        <p className="sectionSubtitle">How transcriptions and summaries share the Mac with you</p>
        <PolicyChoice />
      </section>

      <ModelCheckSection />

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

const JOBS: { job: SpeechJob; label: string; hint: string }[] = [
  { job: "meetings", label: "Meetings", hint: "Transcripts after Stop and imports. Automatic: large-v3, the most accurate." },
  { job: "live", label: "Live", hint: "Lines as people speak. Automatic: turbo, twice as fast." },
];

/** Which downloaded model each job uses; "Automatic" follows the job's order. */
function SpeechChoices() {
  const { data: models } = useModels();
  const { data: choices } = useSpeechChoices();
  const save = useSetSpeechModel();
  const ready = (models ?? []).filter((m) => m.status.state === "ready");
  const label = (id: string | null | undefined) => models?.find((m) => m.id === id)?.label ?? "no model downloaded";
  return (
    <>
      <p className="groupTitle">Used for</p>
      <div className="group">
        {JOBS.map(({ job, label: jobLabel, hint }) => {
          const choice = choices?.find((c) => c.job === job);
          return (
            <div key={job} className="groupRow">
              <div className="rowText">
                <p className="rowLabel">{jobLabel}</p>
                <p className="rowHint">{hint}</p>
              </div>
              <div className="rowControl">
                <select
                  className="input"
                  aria-label={`Speech model for ${jobLabel}`}
                  value={choice?.chosen ?? ""}
                  disabled={!choice}
                  onChange={(e) => save.mutate({ job, model: e.target.value || null })}
                >
                  <option value="">Automatic ({label(choice?.automatic)})</option>
                  {ready.map((m) => (
                    <option key={m.id} value={m.id}>
                      {m.label}
                    </option>
                  ))}
                  {choice?.chosen && !ready.some((m) => m.id === choice.chosen) && (
                    <option value={choice.chosen}>{label(choice.chosen)} (not downloaded: automatic runs)</option>
                  )}
                </select>
              </div>
            </div>
          );
        })}
      </div>
    </>
  );
}

const PACE_HINT = {
  gentle:
    "While you use the Mac, on battery or when it runs hot: half the processor threads at a lower priority, and the text model leaves memory a minute after a job. Jobs take a little longer.",
  full: "Every core at full priority, whatever else you're doing.",
} as const;

/** The pace of the work you wait for, and when catch-up work (model checks) may run. */
function PolicyChoice() {
  const { data: policy } = useAiPolicy();
  const save = useSetAiPolicy();
  return (
    <>
      <div className="segmented" role="group" aria-label="While you work">
        <button type="button" aria-pressed={policy?.pace === "gentle"} disabled={!policy} onClick={() => policy && save.mutate({ ...policy, pace: "gentle" })}>
          Gentle
        </button>
        <button type="button" aria-pressed={policy?.pace === "full"} disabled={!policy} onClick={() => policy && save.mutate({ ...policy, pace: "full" })}>
          Full speed
        </button>
      </div>
      {policy && <p className={`fieldHint ${styles.hint}`}>{PACE_HINT[policy.pace]}</p>}
      <p className="groupTitle">Catch-up work</p>
      <div className="group">
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">Model checks and other work nobody waits for</p>
            <p className="rowHint">Never on battery below 30 %, when the Mac runs hot or is short of memory.</p>
          </div>
          <div className="rowControl">
            <div className="segmented" role="group" aria-label="Catch-up work">
              <button
                type="button"
                aria-pressed={policy?.background_while_working === false}
                disabled={!policy}
                onClick={() => policy && save.mutate({ ...policy, background_while_working: false })}
              >
                When I'm away
              </button>
              <button
                type="button"
                aria-pressed={policy?.background_while_working === true}
                disabled={!policy}
                onClick={() => policy && save.mutate({ ...policy, background_while_working: true })}
              >
                Also while I work
              </button>
            </div>
          </div>
        </div>
      </div>
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
          {status.state === "ready" && <span className="chip chipPrimary">Ready</span>}
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
        ) : status.state !== "ready" ? (
          <button type="button" className="btn btnPrimary btnCompact" onClick={() => start.mutate(model.id)}>
            {status.state === "partial" ? "Resume" : "Download"}
          </button>
        ) : null}
        <MenuButton
          label={`More actions for ${model.label}`}
          items={[
            {
              label: status.state === "partial" ? "Discard download" : "Delete",
              onSelect: () => remove.mutate(model.id),
              danger: true,
              disabled: status.state !== "ready" && status.state !== "partial",
            },
          ]}
        />
      </div>
    </li>
  );
}


/** The embedding model behind search by meaning (D-053): download it through Ollama; once there,
 *  meetings are indexed in the background when the Mac is free. */
function EmbedModel() {
  const status = useOllamaStatus();
  const pull = useOllamaPull();
  const remove = useRemoveOllamaModel();
  const s = status.data;
  if (!s) return <div className={`${styles.ghost} ghostCard`} />;
  const pulling = pull.running && pull.model === "bge-m3";
  const percent = pull.totalMb > 0 ? Math.round((pull.completedMb / pull.totalMb) * 100) : 0;
  return (
    <ul className={styles.list}>
      <li className={`card ${styles.row}`}>
        <div className={styles.main}>
          <p className={styles.name}>
            bge-m3
            <span className="fieldHint">1.2 GB</span>
            {s.embed_installed && <span className="chip chipPrimary">Installed</span>}
          </p>
          <p className="fieldHint">
            {s.embed_installed
              ? "Your meetings are indexed in the background when the Mac is free; search on the Meetings page."
              : !s.running
                ? "Needs Ollama running on this Mac."
                : pulling
                  ? `Downloading… ${percent} %`
                  : "Not downloaded. Text search works without it."}
          </p>
          {pull.error && pull.model === "bge-m3" && <p className="formError">{pull.error.message}</p>}
          {remove.error && <p className="formError">{remove.error.message}</p>}
        </div>
        {s.embed_installed && (
          <div className={styles.actions}>
            <MenuButton
              label="More actions for bge-m3"
              items={[{ label: "Remove from Ollama", onSelect: () => remove.mutate("bge-m3"), danger: true, disabled: remove.isPending }]}
            />
          </div>
        )}
        {!s.embed_installed && s.running && (
          <div className={styles.actions}>
            <button type="button" className="btn btnPrimary btnCompact" disabled={pulling} aria-busy={pulling || undefined} onClick={() => pull.start("bge-m3")}>
              {pulling ? "Downloading…" : "Download"}
            </button>
          </div>
        )}
      </li>
    </ul>
  );
}
