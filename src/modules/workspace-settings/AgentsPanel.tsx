import { useEffect, useState } from "react";
import { useAgents, useResetAgentGuide, useSetAgentGuide, useSetAgentModel, type AgentGuide } from "../../kernel/bridge/agents";
import { useOllamaStatus } from "../../kernel/bridge/summary";
import DangerButton from "../../kernel/shell/DangerButton";
import { SparkleIcon } from "../../kernel/shell/icons";
import { formatWhen } from "../meetings/format";
import styles from "./AgentsPanel.module.css";

/** The agents whose work you correct (the others leave nothing to record). */
const LEARNS: AgentGuide["agent"][] = ["summary", "naming", "email", "tasks"];

/** What the agent learns from you (D-049): the feedback ledger's count, and for the summary, the
 *  examples it gets. */
function learning(guide: AgentGuide): string {
  const n = guide.feedback;
  const recorded = n ? `${n} of your edits and decisions recorded` : "None of your edits or decisions yet";
  return guide.agent === "summary"
    ? `${recorded}. Your newest rewritten sections of the same kind of meeting go with each new summary as examples.`
    : `${recorded}.`;
}

/** The Live agent's default when installed, before the app model (FAST_MODEL in kernel/ai/live.rs). */
const FAST_MODEL = "qwen3.5:4b";

/** Workspace → Agents: each AI task's guide (the human-tunable part of its prompt), editable
 *  in place, with its last run. The code keeps the data fence and the JSON shape (D-035). */
export default function AgentsPanel({ slug }: { slug: string }) {
  const agents = useAgents(slug);
  const save = useSetAgentGuide(slug);
  const reset = useResetAgentGuide(slug);
  const setModel = useSetAgentModel(slug);
  const ollama = useOllamaStatus();
  const installed = ollama.data?.installed.map((m) => m.name) ?? [];
  const error = agents.error ?? save.error ?? reset.error ?? setModel.error;

  return (
    <section className="section">
      <h2 className="sectionTitle">Agents</h2>
      <p className="sectionSubtitle">What each AI task is told. Edit the text, save, and the next run follows it.</p>
      {agents.data ? (
        agents.data.map((g) => (
          <AgentCard
            key={g.agent}
            guide={g}
            saving={save.isPending && save.variables?.agent === g.agent}
            onSave={(text) => save.mutate({ agent: g.agent, text })}
            onReset={() => reset.mutate(g.agent)}
            installed={installed}
            appModel={ollama.data?.model ?? ""}
            onModel={(model) => setModel.mutate({ agent: g.agent, model })}
          />
        ))
      ) : (
        <>
          {[0, 1, 2].map((i) => (
            <div key={i} className={`${styles.ghost} ghostCard`} />
          ))}
        </>
      )}
      {error && (
        <p className="formError" role="alert">
          {error.message}
        </p>
      )}
    </section>
  );
}

function AgentCard({
  guide,
  saving,
  onSave,
  onReset,
  installed,
  appModel,
  onModel,
}: {
  guide: AgentGuide;
  saving: boolean;
  onSave: (text: string) => void;
  onReset: () => void;
  installed: string[];
  appModel: string;
  onModel: (model: string | null) => void;
}) {
  const [draft, setDraft] = useState(guide.guide);
  useEffect(() => setDraft(guide.guide), [guide.guide]);
  const dirty = draft.trim() !== guide.guide.trim();
  const run = guide.last_run;
  const fast = guide.agent === "live" && installed.includes(FAST_MODEL);
  const fallback = fast ? FAST_MODEL : appModel;

  return (
    <div className={`card ${styles.card}`}>
      <div className={styles.head}>
        <div className={styles.titles}>
          <p className={`${styles.title} aiMark`}>
            <SparkleIcon />
            {guide.label} agent
            {guide.customised && <span className="chip chipPrimary">Customised</span>}
          </p>
          <p className="fieldHint">{guide.description}</p>
        </div>
        <p className={`fieldHint ${styles.run}`}>
          {run ? `Last run ${formatWhen(run.created_at)} · ${run.model} · ${run.status}${run.output_tokens ? ` · ${run.output_tokens} tokens` : ""}${run.duration_ms != null ? ` · ${(run.duration_ms / 1000).toFixed(0)} s` : ""}` : "Not run yet"}
        </p>
      </div>
      <div className={styles.modelRow}>
        {guide.agent === "naming" || guide.agent === "tasks" ? (
          <p className="fieldHint">Runs inside the summary, on the Summary agent's model (switching models mid-run costs seconds).</p>
        ) : (
          <label className={styles.model}>
            <span className="fieldLabel">Model</span>
            <select className="input" value={guide.model ?? ""} onChange={(e) => onModel(e.target.value || null)}>
              <option value="">{fast ? "Fast" : "App"} default{fallback ? ` (${fallback})` : ""}</option>
              {guide.model && !installed.includes(guide.model) && <option value={guide.model}>{guide.model} (not installed — uses the {fast ? "fast" : "app"} default)</option>}
              {installed.map((m) => (
                <option key={m} value={m}>
                  {m}
                </option>
              ))}
            </select>
          </label>
        )}
        {guide.agent === "email" && <p className="fieldHint">For Rewrite email. The cover note written after a summary uses the Summary agent's model.</p>}
        {guide.by_model.length > 0 && (
          <p className="fieldHint">
            {guide.by_model.map((s) => `${s.model}: ${s.runs} run${s.runs === 1 ? "" : "s"}${s.avg_ms != null ? `, ~${Math.round(s.avg_ms / 1000)} s` : ""}`).join(" · ")}
          </p>
        )}
        {LEARNS.includes(guide.agent) && <p className="fieldHint">{learning(guide)}</p>}
      </div>
      <textarea
        className={`input ${styles.editor}`}
        value={draft}
        rows={Math.min(24, Math.max(8, draft.split("\n").length + 1))}
        spellCheck={false}
        aria-label={`${guide.label} agent guide`}
        onChange={(e) => setDraft(e.target.value)}
      />
      <div className={styles.actions}>
        {guide.customised && (
          <DangerButton confirm="Back to the bundled guide?" onConfirm={onReset}>
            Reset to default
          </DangerButton>
        )}
        <span className={styles.spacer} />
        {dirty && (
          <button type="button" className="btn btnGhost btnCompact" onClick={() => setDraft(guide.guide)}>
            Discard
          </button>
        )}
        <button type="button" className="btn btnPrimary btnCompact" disabled={!dirty || saving} onClick={() => onSave(draft)}>
          {saving ? "Saving…" : "Save"}
        </button>
      </div>
    </div>
  );
}
