import { Link } from "@tanstack/react-router";
import { useRef } from "react";
import { useSetCommitmentStatus, type Commitment } from "../../kernel/bridge/crm";
import { SparkleIcon } from "../../kernel/shell/icons";
import { formatWhen } from "../meetings/format";
import styles from "./People.module.css";

/** Commitments as cards: who owes what (and by when), the meeting whose action items said it (the
 *  summary agent's words, so the AI marker), and the actions its status allows — Confirm /
 *  Dismiss while waiting, Kept / Dropped while open, Undo once dismissed. A decision moves the
 *  card away, so focus goes to the list. */
export default function Commitments({ slug, items, showWho, empty }: { slug?: string; items: Commitment[]; showWho: boolean; empty: string }) {
  const set = useSetCommitmentStatus(slug);
  const list = useRef<HTMLDivElement>(null);
  const decide = (id: string, status: Commitment["status"]) => set.mutate({ id, status }, { onSuccess: () => list.current?.focus() });
  if (!items.length) {
    return (
      <div className={styles.list}>
        <p className="ghostText">{empty}</p>
        {[0, 1].map((i) => (
          <div key={i} className={`${styles.ghostRow} ghostCard`} />
        ))}
      </div>
    );
  }
  return (
    <div className={styles.list} ref={list} tabIndex={-1} aria-label="Commitments">
      {set.error && (
        <p className="formError" role="alert">
          {set.error.message}
        </p>
      )}
      {items.map((c) => (
        <div key={c.id} className={`card ${styles.commitment}`}>
          <div className={styles.text}>
            <p className={styles.what}>
              {showWho && (c.person_id ? <Link to="/people/$personId" params={{ personId: c.person_id }} className={styles.link}>{c.person_name}</Link> : <span>{c.person_name}</span>)}
              {showWho && " · "}
              {c.what}
            </p>
            <p className="fieldHint aiMark">
              <SparkleIcon /> {c.due_text ? `By ${c.due_text} · ` : ""}
              {c.email_item_id ? (
                <>
                  from the email{" "}
                  <Link to="/email" className={styles.link}>
                    “{c.email_subject || "No subject"}”
                  </Link>
                </>
              ) : c.meeting_id ? (
                <>
                  from the action items of{" "}
                  <Link to="/meetings/$meetingId" params={{ meetingId: c.meeting_id }} search={{ tab: "summary" }} className={styles.link}>
                    {c.meeting_title ?? "a meeting"}
                  </Link>
                </>
              ) : (
                "from a meeting or an email since deleted"
              )}
              {c.held_on ? ` · ${formatWhen(c.held_on)}` : ""}
            </p>
          </div>
          <div className={styles.actions}>
            {c.status === "proposed" && (
              <>
                <button type="button" className="btn btnSecondary btnCompact" disabled={set.isPending} aria-label={`Confirm: ${c.person_name} — ${c.what}`} onClick={() => decide(c.id, "open")}>
                  Confirm
                </button>
                <button type="button" className="btn btnGhost btnCompact" disabled={set.isPending} aria-label={`Dismiss: ${c.person_name} — ${c.what}`} onClick={() => decide(c.id, "dismissed")}>
                  Dismiss
                </button>
              </>
            )}
            {c.status === "open" && (
              <>
                <button type="button" className="btn btnSecondary btnCompact" disabled={set.isPending} aria-label={`Kept: ${c.what}`} onClick={() => decide(c.id, "kept")}>
                  Kept
                </button>
                <button type="button" className="btn btnGhost btnCompact" disabled={set.isPending} aria-label={`Dropped: ${c.what}`} onClick={() => decide(c.id, "dropped")}>
                  Dropped
                </button>
              </>
            )}
            {(c.status === "kept" || c.status === "dropped") && <span className="chip">{c.status === "kept" ? "Kept" : "Dropped"}</span>}
            {c.status === "dismissed" && (
              <button type="button" className="btn btnGhost btnCompact" disabled={set.isPending} aria-label={`Undo dismiss: ${c.person_name} — ${c.what}`} onClick={() => decide(c.id, "proposed")}>
                Undo
              </button>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}
