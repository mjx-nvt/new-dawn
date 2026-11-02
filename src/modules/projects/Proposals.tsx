import { Link } from "@tanstack/react-router";
import { useDecideUpdate, useProject, type ProjectUpdate } from "../../kernel/bridge/projects";
import { SparkleIcon } from "../../kernel/shell/icons";
import { formatWhen } from "../meetings/format";
import { FIELD_LABEL, formatDay } from "./format";
import { PRIORITIES } from "./items/items";
import styles from "./Projects.module.css";

/** What a proposal changes, in words: the dossier's value, a new work item's title, a work item
 *  moving to a state, or a comment on one (D-085). */
function headline(u: ProjectUpdate): string {
  if (u.field === "item_state") return `${u.item_key ?? "A work item"} ${u.item_title ?? ""} → ${u.value}`;
  if (u.field === "comment") return `On ${u.item_key ?? "a work item"}: ${u.value}`;
  return `${u.value}${u.due ? ` · by ${formatDay(u.due)}` : ""}`;
}

/** A new work item's fields as proposed: who, when, how urgent, its labels, parent and cycle. */
function details(u: ProjectUpdate): string {
  const w = u.work;
  if (u.field !== "work_item" || !w) return "";
  return [
    w.assignee_names?.length ? `For ${w.assignee_names.join(", ")}` : "",
    w.start_date ? `from ${formatDay(w.start_date)}` : "",
    w.target_date ? `due ${formatDay(w.target_date)}` : "",
    w.priority && w.priority !== "none" ? (PRIORITIES.find((p) => p.key === w.priority)?.label ?? "") : "",
    (w.label_names ?? []).join(", "),
    w.parent_key ? `under ${w.parent_key}` : "",
    w.cycle_name ? `in ${w.cycle_name}` : "",
  ]
    .filter(Boolean)
    .join(" · ");
}

/** The proposals, each with its evidence and where it came from: Apply or Dismiss (D-054, D-085). */
export default function Proposals({ slug, updates, readOnly = false }: { slug?: string; updates: ProjectUpdate[]; readOnly?: boolean }) {
  const decide = useDecideUpdate(slug);
  return (
    <div className={styles.list}>
      {decide.error && <p className="formError">{decide.error.message}</p>}
      {updates.map((u) => (
        <div key={u.id} className={`card ${styles.update}`}>
          <div className={styles.updateText}>
            <p className="eyebrow">{FIELD_LABEL[u.field]}</p>
            <p className={styles.updateValue}>{headline(u)}</p>
            {details(u) && <p className="fieldHint">{details(u)}</p>}
            {u.evidence && <p className={styles.evidence}>“{u.evidence}”</p>}
            <p className="fieldHint aiMark">
              <SparkleIcon /> Proposed from{" "}
              {u.email_item_id ? (
                <>
                  the email{" "}
                  <Link to="/email" className={styles.link}>
                    “{u.email_subject || "No subject"}”
                  </Link>
                </>
              ) : u.meeting_id ? (
                <Link to="/meetings/$meetingId" params={{ meetingId: u.meeting_id }} search={{ tab: "summary" }} className={styles.link}>
                  {u.meeting_title ?? "a meeting"}
                </Link>
              ) : (
                "a meeting"
              )}{" "}
              · {formatWhen(u.created_at)}
            </p>
          </div>
          <div className={styles.updateActions}>
            {/* An archived project or item takes no change: only Dismiss. */}
            {!readOnly && (
              <button type="button" className="btn btnSecondary btnCompact" disabled={decide.isPending} aria-label={`Apply: ${FIELD_LABEL[u.field]} ${headline(u)}`} onClick={() => decide.mutate({ id: u.id, apply: true })}>
                Apply
              </button>
            )}
            <button type="button" className="btn btnGhost btnCompact" disabled={decide.isPending} aria-label={`Dismiss: ${FIELD_LABEL[u.field]} ${headline(u)}`} onClick={() => decide.mutate({ id: u.id, apply: false })}>
              Dismiss
            </button>
          </div>
        </div>
      ))}
    </div>
  );
}

/** What the meetings proposed for a project's work: its new work items above the list (shown
 *  while some wait — a queue, not a section), or what they propose for one item (its state, a
 *  comment) in its view, a section like the others there (ghost text when nothing waits). */
export function MeetingProposals({ slug, projectId, itemId, readOnly = false }: { slug: string; projectId: string; itemId?: string; readOnly?: boolean }) {
  const waiting = (useProject(slug, projectId).data?.waiting ?? []).filter((u) => (itemId ? u.item_id === itemId : u.field === "work_item"));
  if (!waiting.length && !itemId) return null;
  return (
    <section className={styles.proposed} aria-label="Proposed from meetings">
      <h3 className="groupTitle">Proposed from meetings</h3>
      {waiting.length ? <Proposals slug={slug} updates={waiting} readOnly={readOnly} /> : <p className="ghostText">Nothing proposed from meetings</p>}
    </section>
  );
}
