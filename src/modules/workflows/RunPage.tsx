import { Link } from "@tanstack/react-router";
import { useState } from "react";
import { asRecord, recordId } from "../../kernel/bridge/records";
import { definitionOf, stepLabel, useRunAction, useWorkflowRun, type Step } from "../../kernel/bridge/workflows";
import type { WidgetContext } from "../crm/record/widgets";
import { formatClock, formatWhen } from "../meetings/format";
import { formFields, summarize, triggerSummary, triggerTitle } from "./StepSettings";
import s from "./Workflows.module.css";

function flatten(steps: Step[]): Step[] {
  return steps.flatMap((st) => [st, ...(st.branches ?? []).flatMap((b) => flatten(b.steps)), ...flatten(st.else_steps ?? []), ...flatten(st.steps ?? [])]);
}

function pretty(v: unknown): string {
  return JSON.stringify(v, null, 2) ?? "";
}

export function runStatusClass(status: string): string {
  return status === "COMPLETED" ? `chip ${s.statusOk}` : status === "FAILED" ? `chip ${s.statusBad}` : status === "WAITING" || status === "RUNNING" ? `chip ${s.statusWarn}` : "chip";
}

export function runStatusLabel(status: string): string {
  return (
    {
      NOT_STARTED: "Not started",
      RUNNING: "Running",
      COMPLETED: "Completed",
      FAILED: "Failed",
      WAITING: "Waiting",
    }[status] ?? status
  );
}

/** A run's page (D-068): the facts, the error, the steps with their outputs — done, waiting,
 *  failed, or still to come — a form the run waits for, Retry and Cancel. */
export default function RunBody({ ctx }: { ctx: WidgetContext }) {
  const { slug, record } = ctx;
  const id = recordId(record);
  const run = useWorkflowRun(slug, id);
  const act = useRunAction(slug, id);
  const [answers, setAnswers] = useState<Record<string, unknown>>({});
  const r = run.data;
  if (!r) {
    return (
      <div className={s.steps} style={{ marginTop: "var(--space-5)" }}>
        <div className={`${s.step} ghostCard`}>{run.error ? <span className="ghostText">Couldn't be read — {run.error.message}</span> : <span className="ghostText">Reading…</span>}</div>
      </div>
    );
  }
  const def = definitionOf(r.definition);
  const context = asRecord(r.context);
  const all = flatten(def.steps);
  // The step that failed: the first one not done, in document order (a waiting form or delay
  // is the same step while the run waits).
  const current = all.find((st) => !r.done.includes(st.id));
  const form = r.status === "WAITING" && r.waiting_for ? all.find((st) => st.id === r.waiting_for) : undefined;
  const started = r.started_at ? new Date(r.started_at).getTime() : 0;
  const ended = r.ended_at ? new Date(r.ended_at).getTime() : 0;
  const stateOf = (st: Step): { cls: string; mark: string; text: string } => {
    if (r.done.includes(st.id)) return { cls: s.stepDone, mark: "✓", text: "Done" };
    if (form && form.id === st.id) return { cls: s.stepWaiting, mark: "?", text: "Waiting for you" };
    if (r.status === "WAITING" && current?.id === st.id)
      return {
        cls: s.stepWaiting,
        mark: "…",
        text: r.wait_until ? `Until ${formatWhen(r.wait_until)}` : "Waiting",
      };
    if (r.status === "FAILED" && current?.id === st.id) return { cls: s.stepFailed, mark: "✕", text: "Failed" };
    if (r.status === "RUNNING" && current?.id === st.id) return { cls: s.stepWaiting, mark: "…", text: "Running" };
    return { cls: "", mark: "·", text: "" };
  };
  // Rendered by calling, not as a component: an open Output stays open while the run refreshes.
  const renderSteps = (list: Step[], depth: number) => (
    <>
      {list.map((st) => {
        const state = stateOf(st);
        const output = context[st.id];
        return (
          <div key={st.id} className={s.step} style={depth ? { marginLeft: `calc(${depth} * var(--space-6))` } : undefined}>
            <div className={s.stepHead}>
              <span className={`${s.stepState} ${state.cls}`} aria-hidden>
                {state.mark}
              </span>
              <div className={s.cardText}>
                <span className={s.cardKind}>{stepLabel(st.type)}</span>
                <span className={s.cardTitle}>{st.name || stepLabel(st.type)}</span>
                <span className={s.cardSummary}>{state.text || summarize(st)}</span>
              </div>
            </div>
            {output !== undefined && (
              <details className={s.output}>
                <summary>Output</summary>
                <pre className={s.json}>{pretty(output)}</pre>
              </details>
            )}
            {(st.branches ?? []).map((b, i) => (
              <div key={i}>
                <p className={s.routeLabel}>
                  {i === 0 ? "If" : "Else if"} · {b.name}
                </p>
                <div className={s.steps}>{renderSteps(b.steps, 1)}</div>
              </div>
            ))}
            {st.else_steps && st.else_steps.length > 0 && (
              <div>
                <p className={s.routeLabel}>Else</p>
                <div className={s.steps}>{renderSteps(st.else_steps, 1)}</div>
              </div>
            )}
            {st.steps && st.steps.length > 0 && (
              <div className={s.steps} style={{ marginTop: "var(--space-2)" }}>
                {renderSteps(st.steps, 1)}
              </div>
            )}
          </div>
        );
      })}
    </>
  );
  const fields = form ? formFields(form) : [];
  const title = form ? (typeof form.settings.title === "string" && form.settings.title) || form.name || "A form" : "";
  return (
    <>
      <div className={s.facts}>
        <div className={s.fact}>
          <span className={s.factLabel}>Status</span>
          <span>
            <span className={runStatusClass(r.status)}>{runStatusLabel(r.status)}</span>
          </span>
        </div>
        <div className={s.fact}>
          <span className={s.factLabel}>Workflow</span>
          <span className={s.factValue}>
            {r.workflow_id ? (
              <Link to="/crm/$object/$recordId" params={{ object: "workflows", recordId: r.workflow_id }} search={{}} className={s.itemLink}>
                {r.workflow_name || "Workflow"}
              </Link>
            ) : (
              <span className="ghostText">Gone</span>
            )}
          </span>
        </div>
        <div className={s.fact}>
          <span className={s.factLabel}>Trigger</span>
          <span className={s.factValue}>{triggerTitle(def.trigger)}</span>
          <span className="fieldHint">{triggerSummary(def.trigger)}</span>
        </div>
        <div className={s.fact}>
          <span className={s.factLabel}>Started</span>
          <span className={s.factValue}>{r.started_at ? formatWhen(r.started_at) : <span className="ghostText">Not yet</span>}</span>
        </div>
        <div className={s.fact}>
          <span className={s.factLabel}>Duration</span>
          <span className={s.factValue}>{started && ended ? formatClock(Math.max(0, ended - started)) : r.wait_until ? `Waiting until ${formatWhen(r.wait_until)}` : <span className="ghostText">Not yet</span>}</span>
        </div>
      </div>
      {r.error && (
        <p className="formError" role="alert">
          {r.error}
        </p>
      )}
      {["FAILED", "NOT_STARTED", "WAITING"].includes(r.status) && (
        <div className={s.bar}>
          <span className={s.spacer} />
          {r.status === "FAILED" && (
            <button type="button" className="btn btnSecondary btnCompact" disabled={act.isPending} onClick={() => act.mutate({ action: "retry" })}>
              Retry from the failed step
            </button>
          )}
          {["NOT_STARTED", "WAITING"].includes(r.status) && (
            <button type="button" className="btn btnDanger btnCompact" disabled={act.isPending} onClick={() => act.mutate({ action: "cancel" })}>
              Cancel run
            </button>
          )}
        </div>
      )}
      {act.error && (
        <p className="formError" role="alert">
          {act.error.message}
        </p>
      )}
      {form && (
        <form
          className={s.form}
          aria-label={title}
          onSubmit={(e) => {
            e.preventDefault();
            act.mutate({ action: "answer", stepId: form.id, answers });
          }}
        >
          <div>
            <p className="eyebrow">The run waits for you</p>
            <h2 className="sectionTitle">{title}</h2>
            {typeof form.settings.instructions === "string" && form.settings.instructions && <p className="fieldHint">{form.settings.instructions}</p>}
          </div>
          <div className="fields">
            {fields.map((f) => {
              const fid = `wf-form-${f.name}`;
              const v = answers[f.name];
              const set = (value: unknown) => setAnswers({ ...answers, [f.name]: value });
              return (
                <div key={f.name} className="field">
                  <label className="fieldLabel" htmlFor={fid}>
                    {f.label || f.name}
                  </label>
                  {f.type === "select" ? (
                    <select id={fid} className="input" value={typeof v === "string" ? v : ""} onChange={(e) => set(e.target.value)}>
                      <option value="">—</option>
                      {(f.options ?? "")
                        .split(",")
                        .map((o) => o.trim())
                        .filter(Boolean)
                        .map((o) => (
                          <option key={o} value={o}>
                            {o}
                          </option>
                        ))}
                    </select>
                  ) : (
                    <input
                      id={fid}
                      className="input"
                      type={f.type === "number" ? "number" : f.type === "date" ? "date" : "text"}
                      value={typeof v === "string" || typeof v === "number" ? v : ""}
                      onChange={(e) => set(f.type === "number" ? (e.target.value === "" ? "" : Number(e.target.value)) : e.target.value)}
                    />
                  )}
                </div>
              );
            })}
            {!fields.length && <p className="fieldHint">Nothing to fill in — Continue goes on.</p>}
          </div>
          <div className="sheetActions">
            <button type="submit" className="btn btnPrimary" disabled={act.isPending}>
              Continue
            </button>
          </div>
        </form>
      )}
      <section className="section">
        <h2 className="sectionTitle">Steps</h2>
        <div className={s.steps}>
          {def.steps.length ? (
            renderSteps(def.steps, 0)
          ) : (
            <div className={`${s.step} ghostCard`}>
              <span className="ghostText">This version has no steps.</span>
            </div>
          )}
        </div>
        <details className={s.output}>
          <summary>Trigger data</summary>
          <pre className={s.json}>{pretty(r.trigger)}</pre>
        </details>
      </section>
    </>
  );
}
