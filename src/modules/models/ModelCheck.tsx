import { useState } from "react";
import {
  cancelModelCheck,
  runAiJobNow,
  useAiActivity,
  useModelChecks,
  usePickModelCheck,
  useReferences,
  useStartModelCheck,
  type CheckKind,
  type ModelCheck,
  type Pick,
} from "../../kernel/bridge/activity";
import { useAgents, useSetAgentModel } from "../../kernel/bridge/agents";
import { useModels, useSetSpeechModel, useSpeechChoices } from "../../kernel/bridge/models";
import { useOllamaStatus } from "../../kernel/bridge/summary";
import { useCurrentWorkspace } from "../../kernel/bridge/workspaces";
import { SparkleIcon } from "../../kernel/shell/icons";
import { formatWhen } from "../meetings/format";
import styles from "./ModelCheck.module.css";

const STATUS: Record<ModelCheck["status"], string> = {
  waiting: "Waiting",
  running: "Running",
  done: "Done",
  failed: "Failed",
  cancelled: "Cancelled",
};

/** Settings → Models: try a model on your own meetings before switching (D-050). */
export default function ModelCheckSection() {
  const { data: workspace } = useCurrentWorkspace();
  const slug = workspace?.slug;
  const references = useReferences(slug);
  const checks = useModelChecks(slug);
  const { data: models } = useModels();
  const { data: choices } = useSpeechChoices();
  const ollama = useOllamaStatus();
  const agents = useAgents(slug ?? "");
  const start = useStartModelCheck(slug);
  const meetingsChoice = choices?.find((c) => c.job === "meetings");
  const speechInUse = meetingsChoice?.chosen ?? meetingsChoice?.automatic ?? null;
  const textInUse = agents.data?.find((a) => a.agent === "summary")?.model ?? ollama.data?.model ?? null;
  const speechOptions = (models ?? []).filter((m) => m.status.state === "ready" && m.id !== speechInUse).map((m) => m.id);
  const textOptions = (ollama.data?.installed ?? []).map((m) => m.name).filter((name) => name !== textInUse);

  return (
    <section className="section">
      <h2 className="sectionTitle">Try a model on your meetings</h2>
      <p className="sectionSubtitle">Compare a model with the one in use on meetings you know, blind, before you switch</p>
      <p className="groupTitle">References{workspace ? ` in ${workspace.name}` : ""}</p>
      {references.data?.length ? (
        <ul className={styles.references}>
          {references.data.map((r) => (
            <li key={r.id} className="chip">
              {r.title} · {formatWhen(r.held_on)}
            </li>
          ))}
        </ul>
      ) : (
        <p className="fieldHint ghostText">No reference meetings yet: choose “Use as a reference” in a meeting's ⋯ menu.</p>
      )}
      <div className={`group ${styles.tryGroup}`}>
        <TryRow
          kind="speech"
          label="Speech for meetings"
          hint={`Two 90-second stretches of each reference meeting, against ${speechInUse ?? "the model in use"}.`}
          options={speechOptions}
          onTry={(candidate) => start.mutate({ kind: "speech", candidate })}
          busy={start.isPending}
        />
        <TryRow
          kind="text"
          label="Summaries"
          hint={`Each summarised reference meeting written again, against ${textInUse ?? "the model in use"} (a few minutes each).`}
          options={textOptions}
          onTry={(candidate) => start.mutate({ kind: "text", candidate })}
          busy={start.isPending}
        />
      </div>
      {start.error && (
        <p className="formError" role="alert">
          {start.error.message}
        </p>
      )}
      {checks.data?.map((check) => (
        <CheckCard key={check.id} check={check} slug={slug ?? ""} inUse={check.kind === "speech" ? speechInUse : textInUse} />
      ))}
    </section>
  );
}

function TryRow({
  kind,
  label,
  hint,
  options,
  onTry,
  busy,
}: {
  kind: CheckKind;
  label: string;
  hint: string;
  options: string[];
  onTry: (candidate: string) => void;
  busy: boolean;
}) {
  const [picked, setPicked] = useState("");
  const candidate = options.includes(picked) ? picked : (options[0] ?? "");
  return (
    <div className="groupRow">
      <div className="rowText">
        <p className="rowLabel">{label}</p>
        <p className="rowHint">{options.length ? hint : kind === "speech" ? "Download another Whisper model above to compare." : "Download another text model above to compare."}</p>
      </div>
      <div className={`rowControl ${styles.tryControl}`}>
        <select className="input" aria-label={`Model to try for ${label}`} value={candidate} disabled={!options.length} onChange={(e) => setPicked(e.target.value)}>
          {options.map((o) => (
            <option key={o} value={o}>
              {o}
            </option>
          ))}
        </select>
        <button type="button" className="btn btnAi btnCompact" aria-busy={busy || undefined} disabled={!candidate || busy} onClick={() => onTry(candidate)}>
          <SparkleIcon />
          Try
        </button>
      </div>
    </div>
  );
}

function CheckCard({ check, slug, inUse }: { check: ModelCheck; slug: string; inUse: string | null }) {
  const pick = usePickModelCheck(slug);
  const setSpeech = useSetSpeechModel();
  const setAgentModel = useSetAgentModel(slug);
  const switching = check.kind === "speech" ? setSpeech : setAgentModel;
  const { data: activity } = useAiActivity();
  const held = activity?.find((a) => a.task.id === check.id && a.waiting_for)?.waiting_for;
  const active = check.status === "waiting" || check.status === "running";
  const next = check.pairs.find((p) => !p.pick);
  const judged = check.pairs.length - check.tally.left;
  const { candidate: wins, baseline: losses, same } = check.tally;
  const what = check.kind === "speech" ? "meetings" : "summaries";

  return (
    <div className={`card ${styles.check}`}>
      <div className={styles.checkHead}>
        <p className={styles.checkTitle}>
          {check.candidate} <span className={styles.vs}>vs</span> {check.baseline}
        </p>
        <span className="chip">
          {held ? "Waiting" : STATUS[check.status]}
          {!held && check.status === "running" && check.progress > 0 ? ` · ${check.progress} %` : ""}
        </span>
        <span className="fieldHint">{formatWhen(check.created_at)}</span>
        {active && (
          <span className={styles.checkActions}>
            {held && (
              <button type="button" className="btn btnGhost btnCompact" onClick={() => void runAiJobNow(check.id)}>
                Run now
              </button>
            )}
            <button type="button" className="btn btnGhost btnCompact" onClick={() => void cancelModelCheck(slug, check.id)}>
              Cancel
            </button>
          </span>
        )}
      </div>
      {held && <p className="fieldHint">{held}. Run now starts it anyway.</p>}
      {check.error && <p className="formError">{check.error}</p>}
      {check.meetings.map((m) => (
        <table key={m.meeting} className={styles.figures}>
          <caption>{m.meeting}</caption>
          <thead>
            <tr>
              <th scope="col" />
              <th scope="col">{check.baseline} (in use)</th>
              <th scope="col">{check.candidate}</th>
            </tr>
          </thead>
          <tbody>
            {m.figures.map((f) => (
              <tr key={f.label}>
                <th scope="row">{f.label}</th>
                <td>{f.baseline}</td>
                <td>{f.candidate}</td>
              </tr>
            ))}
          </tbody>
        </table>
      ))}
      {check.note && <p className="fieldHint">{check.note}</p>}
      {check.status === "done" && check.pairs.length > 0 && (
        <>
          {next ? (
            <div className={styles.blind}>
              <p className="rowLabel">
                Which is better? {judged + 1} of {check.pairs.length} · {next.meeting} · {next.label}
              </p>
              <div className={styles.sides}>
                <div className={styles.side}>
                  <p className="eyebrow">A</p>
                  <pre className={styles.text}>{next.a || "(nothing)"}</pre>
                </div>
                <div className={styles.side}>
                  <p className="eyebrow">B</p>
                  <pre className={styles.text}>{next.b || "(nothing)"}</pre>
                </div>
              </div>
              <div className={styles.picks}>
                {(["a", "same", "b"] as Pick[]).map((choice) => (
                  <button
                    key={choice}
                    type="button"
                    className="btn btnSecondary btnCompact"
                    disabled={pick.isPending}
                    onClick={() => pick.mutate({ id: check.id, pairId: next.id, pick: choice })}
                  >
                    {choice === "same" ? "About the same" : `${choice.toUpperCase()} is better`}
                  </button>
                ))}
              </div>
            </div>
          ) : (
            <div className={styles.verdict}>
              <p className="rowLabel">
                Judged blind: {check.candidate} {wins}, {check.baseline} {losses}
                {same ? `, about the same ${same}` : ""}.
              </p>
              {wins > losses && inUse !== check.candidate && (
                <button
                  type="button"
                  className="btn btnPrimary btnCompact"
                  disabled={switching.isPending}
                  onClick={() => {
                    if (check.kind === "speech") setSpeech.mutate({ job: "meetings", model: check.candidate });
                    else setAgentModel.mutate({ agent: "summary", model: check.candidate });
                  }}
                >
                  Use {check.candidate} for {what}
                </button>
              )}
              {inUse === check.candidate && <p className="fieldHint">{check.candidate} writes your {what}.</p>}
              {switching.error && <p className="formError">{switching.error.message}</p>}
            </div>
          )}
        </>
      )}
    </div>
  );
}
