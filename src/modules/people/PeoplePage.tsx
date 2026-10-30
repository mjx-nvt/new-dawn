import { Link } from "@tanstack/react-router";
import { useCollectCommitments, useDismissedCommitments, usePeopleCards, useUnlinkedCommitments, useWaitingCommitments, type PersonCard } from "../../kernel/bridge/crm";
import { useCurrentWorkspace } from "../../kernel/bridge/workspaces";
import { formatWhen } from "../meetings/format";
import Commitments from "./Commitments";
import styles from "./People.module.css";

/** People (D-055): the commitments others made in your meetings, waiting for you to confirm, then
 *  everyone you meet — by organisation, with when you last met them. Names and spellings are
 *  managed in Workspace → People. */
export default function PeoplePage() {
  const { data: workspace } = useCurrentWorkspace();
  const slug = workspace?.slug;
  const cards = usePeopleCards(slug);
  const waiting = useWaitingCommitments(slug);
  const collect = useCollectCommitments(slug);
  const others = useUnlinkedCommitments(slug);
  const dismissed = useDismissedCommitments(slug);
  const groups = byOrganisation(cards.data ?? []);

  return (
    <div className="page">
      <div className={styles.header}>
        <div>
          <p className="eyebrow">{workspace?.name ?? "No workspace"}</p>
          <h1 className="pageTitle">People</h1>
        </div>
        <Link to="/workspace" search={{ tab: "people" }} className="btn btnGhost">
          Names and spellings
        </Link>
      </div>

      <section className="section">
        <h2 className="sectionTitle">Commitments waiting for you</h2>
        <p className="sectionSubtitle">What someone said they would do, in your meetings and the emails you read — confirm the ones you'll follow up</p>
        <p className={`fieldHint ${styles.collect}`}>
          <button type="button" className="btn btnGhost btnCompact" disabled={collect.isPending} onClick={() => collect.mutate()}>
            {collect.isPending ? "Looking…" : "Look through earlier meetings"}
          </button>
          <span role="status">{collect.data != null && (collect.data ? `${collect.data} found` : "Nothing new")}</span>
        </p>
        {(waiting.error ?? collect.error) && (
          <p className="formError" role="alert">
            {(waiting.error ?? collect.error)?.message}
          </p>
        )}
        <Commitments slug={slug} items={waiting.data ?? []} showWho empty="Not yet captured — after each summary, others' action items show here" />
      </section>

      <section className="section">
        <h2 className="sectionTitle">Everyone</h2>
        <p className="sectionSubtitle">The people your meetings named, by organisation</p>
        {cards.error && (
          <p className="formError" role="alert">
            {cards.error.message}
          </p>
        )}
        {groups.length ? (
          groups.map(([org, people]) => (
            <div key={org} className={styles.group}>
              <h3 className="groupTitle">{org}</h3>
              <div className={styles.list}>
                {people.map((c) => (
                  <PersonRow key={c.person.id} card={c} />
                ))}
              </div>
            </div>
          ))
        ) : (
          <div className={styles.list}>
            <p className="ghostText">{cards.data ? "Not yet captured — people are remembered as you confirm names in summaries" : "Loading…"}</p>
            {[0, 1, 2].map((i) => (
              <div key={i} className={`${styles.slot} ghostCard`} />
            ))}
          </div>
        )}
      </section>

      <section className="section">
        <h2 className="sectionTitle">Others</h2>
        <p className="sectionSubtitle">Confirmed commitments of organisations and people not in the people memory</p>
        <Commitments slug={slug} items={others.data ?? []} showWho empty="Not yet captured" />
      </section>

      <section className="section">
        <h2 className="sectionTitle">Dismissed</h2>
        <p className="sectionSubtitle">Set aside as not commitments — Undo brings one back to waiting</p>
        <Commitments slug={slug} items={dismissed.data ?? []} showWho empty="Not yet captured" />
      </section>
    </div>
  );
}

function byOrganisation(cards: PersonCard[]): [string, PersonCard[]][] {
  const groups = new Map<string, PersonCard[]>();
  for (const c of cards) {
    const org = c.person.organisation.trim() || "No organisation";
    groups.set(org, [...(groups.get(org) ?? []), c]);
  }
  return [...groups.entries()].sort(([a], [b]) => (a === "No organisation" ? 1 : b === "No organisation" ? -1 : a.localeCompare(b)));
}

function PersonRow({ card: c }: { card: PersonCard }) {
  return (
    <Link to="/people/$personId" params={{ personId: c.person.id }} className={`card ${styles.row}`}>
      <div className={styles.main}>
        <p className={styles.name}>{c.person.name}</p>
        <p className={styles.meta}>
          {[c.person.title, c.last_contact ? `last met ${formatWhen(c.last_contact.held_on)}` : "not met in a summarised meeting yet"].filter(Boolean).join(" · ")}
        </p>
      </div>
      {c.waiting > 0 && <span className="chip chipPrimary">{c.waiting} waiting</span>}
      {c.open > 0 && <span className="chip">{c.open} open</span>}
    </Link>
  );
}
