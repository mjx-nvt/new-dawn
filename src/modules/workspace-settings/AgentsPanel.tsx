import { useEffect, useState } from "react";
import { useAgents, useResetAgentGuide, useSetAgentGuide, type AgentGuide } from "../../kernel/bridge/agents";
import DangerButton from "../../kernel/shell/DangerButton";
import { SparkleIcon } from "../../kernel/shell/icons";
import { formatWhen } from "../meetings/format";
import styles from "./AgentsPanel.module.css";

/** Workspace → Agents: each AI task's guide (the human-tunable part of its prompt), editable
 *  in place, with its last run. The code keeps the data fence and the JSON shape (D-035). */
export default function AgentsPanel({ slug }: { slug: string }) {
  const agents = useAgents(slug);
  const save = useSetAgentGuide(slug);
  const reset = useResetAgentGuide(slug);
  const error = agents.error ?? save.error ?? reset.error;

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

function AgentCard({ guide, saving, onSave, onReset }: { guide: AgentGuide; saving: boolean; onSave: (text: string) => void; onReset: () => void }) {
  const [draft, setDraft] = useState(guide.guide);
  useEffect(() => setDraft(guide.guide), [guide.guide]);
  const dirty = draft.trim() !== guide.guide.trim();
  const run = guide.last_run;

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
