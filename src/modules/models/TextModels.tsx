import { useEffect, useState } from "react";
import { useOllamaPull } from "../../kernel/bridge/onboarding";
import DangerButton from "../../kernel/shell/DangerButton";
import {
  openOllamaApp,
  unloadOllamaModel,
  useOllamaStatus,
  useRemoveOllamaModel,
  useSelectOllamaModel,
} from "../../kernel/bridge/summary";
import styles from "./ModelsPanel.module.css";

function formatMb(mb: number): string {
  return mb >= 1000 ? `${(mb / 1000).toFixed(1)} GB` : `${mb} MB`;
}

/** Settings → Models → Text model: the Ollama models on this Mac, which one writes summaries,
 *  download / update / remove, and free its memory. */
export default function TextModels() {
  const status = useOllamaStatus();
  const pull = useOllamaPull();
  const select = useSelectOllamaModel();
  const remove = useRemoveOllamaModel();
  const [name, setName] = useState("");
  const [freed, setFreed] = useState(false);
  const s = status.data;
  const error = select.error ?? remove.error ?? pull.error;

  useEffect(() => {
    if (!freed) return;
    const t = setTimeout(() => setFreed(false), 4000);
    return () => clearTimeout(t);
  }, [freed]);

  if (!s) return <div className={`${styles.ghost} ghostCard`} />;
  if (!s.running) {
    return (
      <ul className={styles.list}>
        <li className={`card ${styles.row}`}>
          <div className={styles.main}>
            <p className={styles.name}>Ollama isn't running</p>
            <p className="fieldHint">Summaries are written by a model running in Ollama on this Mac. Open it to manage models.</p>
          </div>
          <div className={styles.actions}>
            <button type="button" className="btn btnSecondary btnCompact" onClick={() => void openOllamaApp()}>
              Open Ollama
            </button>
          </div>
        </li>
      </ul>
    );
  }

  const rows = s.model_installed ? s.installed : [{ name: s.model, size_mb: 0 }, ...s.installed];
  const percent = pull.totalMb > 0 ? Math.round((pull.completedMb / pull.totalMb) * 100) : 0;

  return (
    <>
      <ul className={styles.list}>
        {rows.map((m) => {
          const inUse = m.name === s.model;
          const pulling = pull.running && (pull.model === m.name || (!pull.model && inUse));
          const installed = s.installed.some((i) => i.name === m.name);
          return (
            <li key={m.name} className={`card ${styles.row}`}>
              <div className={styles.main}>
                <p className={styles.name}>
                  {m.name}
                  {installed && <span className="fieldHint">{formatMb(m.size_mb)}</span>}
                  {inUse && <span className="chip chipBrand">Writes summaries</span>}
                  {!installed && <span className="chip">Not downloaded</span>}
                </p>
                {pulling && (
                  <div className={styles.progress}>
                    <div className={styles.bar}>
                      <span style={{ width: `${percent}%` }} />
                    </div>
                    <span className="fieldHint">
                      {pull.status}
                      {pull.totalMb > 0 && ` · ${formatMb(pull.completedMb)} of ${formatMb(pull.totalMb)}`}
                    </span>
                  </div>
                )}
              </div>
              <div className={styles.actions}>
                {installed && !inUse && (
                  <button type="button" className="btn btnSecondary btnCompact" disabled={select.isPending} onClick={() => select.mutate(m.name)}>
                    Use for summaries
                  </button>
                )}
                <button type="button" className="btn btnGhost btnCompact" disabled={pull.running} onClick={() => pull.start(m.name)}>
                  {pulling ? "Downloading…" : installed ? "Update" : "Download"}
                </button>
                {installed && !inUse && (
                  <DangerButton confirm="Remove from Ollama?" onConfirm={() => remove.mutate(m.name)} disabled={remove.isPending}>
                    Remove
                  </DangerButton>
                )}
              </div>
            </li>
          );
        })}
      </ul>

      <form
        className={styles.addForm}
        onSubmit={(e) => {
          e.preventDefault();
          if (name.trim()) pull.start(name.trim());
        }}
      >
        <label className={`field ${styles.addField}`}>
          <span className="fieldLabel">Download another model</span>
          <input
            className="input"
            value={name}
            placeholder="qwen3.5:4b"
            spellCheck={false}
            autoCapitalize="off"
            onChange={(e) => setName(e.target.value)}
          />
        </label>
        <button type="submit" className="btn btnSecondary" disabled={!name.trim() || pull.running}>
          Download
        </button>
      </form>
      <p className="fieldHint">
        Names as listed in Ollama's model library. A smaller model (qwen3.5:4b) is faster on 8–16 GB Macs; a bigger one needs
        more memory. Deep re-run needs a model that can think (Qwen 3.x can).
      </p>
      {pull.running && !rows.some((m) => m.name === pull.model) && (
        <p className="fieldHint">
          Downloading {pull.model}: {pull.status}
          {pull.totalMb > 0 && ` · ${percent} %`}
        </p>
      )}
      {error && <p className={styles.error}>{error.message}</p>}

      <div className={styles.addForm}>
        <button
          type="button"
          className="btn btnGhost btnCompact"
          onClick={() => void unloadOllamaModel().then(() => setFreed(true))}
        >
          {freed ? "Memory freed" : "Free memory now"}
        </button>
        <span className="fieldHint">Ollama keeps the model loaded for 5 minutes after a summary; this unloads it now.</span>
      </div>
    </>
  );
}
