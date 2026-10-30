import { getRouteApi, Link } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { usePerson, useSetPersonNotes, type CommitmentStatus } from "../../kernel/bridge/crm";
import { useCurrentWorkspace } from "../../kernel/bridge/workspaces";
import { formatWhen } from "../meetings/format";
import Commitments from "./Commitments";
import styles from "./People.module.css";

const route = getRouteApi("/shell/people/$personId");

/** A person's page (D-055): what they owe you (waiting, open, done), the meetings they attended,
 *  and your notes. Every section shows even when empty. */
export default function PersonPage() {
  const { personId } = route.useParams();
  const { data: workspace } = useCurrentWorkspace();
  const slug = workspace?.slug;
  const dossier = usePerson(slug, personId);
  const saveNotes = useSetPersonNotes(slug);
  const d = dossier.data;
  const [notes, setNotes] = useState("");
  useEffect(() => setNotes(d?.notes ?? ""), [d?.notes]);
  const dirty = !!d && notes.trim() !== d.notes.trim();
  const of = (statuses: CommitmentStatus[]) => (d?.commitments ?? []).filter((c) => statuses.includes(c.status));

  return (
    <div className="page">
      <p className="eyebrow">Person</p>
      <h1 className="pageTitle">{d?.person.name ?? "…"}</h1>
      {d && <p className={styles.meta}>{[d.person.title, d.person.organisation].filter(Boolean).join(" · ") || "No title or organisation yet"}</p>}
      {dossier.error && (
        <p className="formError" role="alert">
          {dossier.error.message}
        </p>
      )}

      <section className="section">
        <h2 className="sectionTitle">Waiting for you</h2>
        <p className="sectionSubtitle">From your meetings' action items — confirm the ones you'll follow up</p>
        <Commitments slug={slug} items={of(["proposed"])} showWho={false} empty="Not yet captured" />
      </section>

      <section className="section">
        <h2 className="sectionTitle">What they owe you</h2>
        <Commitments slug={slug} items={of(["open"])} showWho={false} empty="Not yet captured — confirmed commitments show here until kept or dropped" />
      </section>

      <section className="section">
        <h2 className="sectionTitle">Meetings</h2>
        {d?.meetings.length ? (
          <ul className={styles.items} role="list">
            {d.meetings.map((m) => (
              <li key={m.id} className={styles.item}>
                <Link to="/meetings/$meetingId" params={{ meetingId: m.id }} search={{ tab: "summary" }} className={styles.link}>
                  {m.title}
                </Link>
                <span className="fieldHint">
                  {m.attended ? "Attended" : "Involved"} · {formatWhen(m.held_on)}
                </span>
              </li>
            ))}
          </ul>
        ) : (
          <div className={styles.list}>
            <p className="ghostText">Not yet captured — the summarised meetings that name them show here</p>
            {[0, 1].map((i) => (
              <div key={i} className={`${styles.ghostRow} ghostCard`} />
            ))}
          </div>
        )}
      </section>

      <section className="section">
        <h2 className="sectionTitle">Notes</h2>
        <textarea className={`input ${styles.notes}`} rows={5} value={notes} placeholder="What to remember about them" aria-label="Notes" onChange={(e) => setNotes(e.target.value)} />
        <div className={styles.saveRow}>
          {saveNotes.error && (
            <p className="formError" role="alert">
              {saveNotes.error.message}
            </p>
          )}
          <button type="button" className="btn btnPrimary btnCompact" disabled={!dirty || saveNotes.isPending} onClick={() => d && saveNotes.mutate({ id: d.person.id, notes })}>
            {saveNotes.isPending ? "Saving…" : "Save notes"}
          </button>
        </div>
      </section>

      <section className="section">
        <h2 className="sectionTitle">Kept and dropped</h2>
        <Commitments slug={slug} items={of(["kept", "dropped"])} showWho={false} empty="Not yet captured" />
      </section>
    </div>
  );
}
