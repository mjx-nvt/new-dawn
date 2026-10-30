import { useEffect, useState } from "react";
import { useAgents, useForgetLearned, useJudgeLesson, useLearnNow, useResetAgentGuide, useSetAgentGuide, useSetAgentModel, type AgentGuide, type Judged } from "../../kernel/bridge/agents";
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

/** How you judged its work on one model, in words (D-052). */
function verdict(agent: AgentGuide["agent"], n: Judged): string {
  const model = n.model || "Unknown model";
  const stood = n.judged - n.changed - n.dropped;
  switch (agent) {
    case "summary":
      return `${model}: ${stood} of ${n.judged} summaries kept without rewriting a section`;
    case "email":
      return `${model}: ${stood} of ${n.judged} emails kept as first written`;
    case "naming":
      return `${model}: of ${n.judged} names it doubted, you changed ${n.changed} and kept ${stood} as heard`;
    case "tasks":
      return `${model}: ${stood} of ${n.judged} tasks it added kept as written (${n.changed} reworded, ${n.dropped} deleted)`;
    default:
      return "";
  }
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
            slug={slug}
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
  slug,
  guide,
  saving,
  onSave,
  onReset,
  installed,
  appModel,
  onModel,
}: {
  slug: string;
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
        {guide.numbers.length > 0 && <p className="fieldHint">{guide.numbers.map((n) => verdict(guide.agent, n)).join(" · ")}</p>}
      </div>
      {guide.learns && <Learned slug={slug} guide={guide} dirty={dirty} />}
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

/** "What I learned" (D-052): lines proposed from your edits, each with its evidence — Approve adds
 *  it to what the agent is told after its guide, Dismiss sets it aside; never applied by itself.
 *  Below them, the lines in use, each with Remove. Approve waits while the guide has unsaved
 *  changes (saving the list reloads the editor). */
function Learned({ slug, guide, dirty }: { slug: string; guide: AgentGuide; dirty: boolean }) {
  const learn = useLearnNow(slug);
  const judge = useJudgeLesson(slug);
  const forget = useForgetLearned(slug);
  const checking = learn.isPending && learn.variables === guide.agent;
  const checked = learn.isSuccess && learn.variables === guide.agent ? learn.data : null;
  const error = (learn.variables === guide.agent ? learn.error : null) ?? judge.error ?? forget.error;

  return (
    <div className={styles.learned}>
      <div className={styles.learnedHead}>
        <p className="rowLabel">What I learned</p>
        <button type="button" className="btn btnAi btnCompact" aria-busy={checking || undefined} disabled={checking} onClick={() => learn.mutate(guide.agent)}>
          <SparkleIcon />
          {checking ? "Reading your edits…" : "Check now"}
        </button>
      </div>
      {guide.lessons.length ? (
        guide.lessons.map((l) => (
          <div key={l.id} className={styles.lesson}>
            <div className={styles.lessonText}>
              <p className={styles.lessonLine}>{l.line}</p>
              <p className="fieldHint">
                {l.evidence ? `${l.evidence} · ` : ""}From {l.edits} of your edits
              </p>
            </div>
            <button
              type="button"
              className="btn btnSecondary btnCompact"
              disabled={judge.isPending || dirty}
              title={dirty ? "Save or discard your changes to the guide first" : undefined}
              aria-label={`Approve “${l.line}”`}
              onClick={() => judge.mutate({ id: l.id, approve: true })}
            >
              Approve
            </button>
            <button type="button" className="btn btnGhost btnCompact" disabled={judge.isPending} aria-label={`Dismiss “${l.line}”`} onClick={() => judge.mutate({ id: l.id, approve: false })}>
              Dismiss
            </button>
          </div>
        ))
      ) : (
        <p className={`${styles.lesson} ${styles.lessonGhost} fieldHint ghostText`}>
          {checked === 0 ? "Nothing new to propose from your recent edits." : "After five of your edits to its work, what they have in common shows here for you to approve."}
        </p>
      )}
      {dirty && guide.lessons.length > 0 && <p className="fieldHint">Save or discard your changes to the guide first, then approve.</p>}
      {guide.learned.length > 0 && (
        <>
          <p className="fieldHint">In use — told to the agent after its guide, and they win over it:</p>
          <ul className={styles.learnedLines}>
            {guide.learned.map((line) => (
              <li key={line} className={styles.learnedLine}>
                <span>{line}</span>
                <button type="button" className="btn btnGhost btnCompact" disabled={forget.isPending} aria-label={`Stop using “${line}”`} onClick={() => forget.mutate({ agent: guide.agent, line })}>
                  Remove
                </button>
              </li>
            ))}
          </ul>
        </>
      )}
      {error && (
        <p className="formError" role="alert">
          {error.message}
        </p>
      )}
    </div>
  );
}
