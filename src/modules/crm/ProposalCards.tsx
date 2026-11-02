import { Link } from "@tanstack/react-router";
import { useApplyCrmProposal, useCrmProposals, useDismissCrmProposal, type CrmProposal, type CrmProposalKind } from "../../kernel/bridge/crmAgent";
import { asRecord, useObject, useObjects } from "../../kernel/bridge/records";
import { SparkleIcon } from "../../kernel/shell/icons";
import { formatWhen } from "../meetings/format";
import { CellView } from "./cells";
import styles from "./Proposals.module.css";

const KIND: Record<CrmProposalKind, string> = { company: "New company", person: "New person", opportunity: "New opportunity", note: "Note", task: "Task" };
/** Shown otherwise: the name is the card's title, a note's body its text, a task's status is To do. */
const SHOWN_APART = new Set(["name", "title", "body", "status"]);

function Card({ slug, p, plural }: { slug: string; p: CrmProposal; plural: (singular: string) => string | undefined }) {
  const def = useObject(slug, p.kind);
  const apply = useApplyCrmProposal(slug);
  const dismiss = useDismissCrmProposal(slug);
  const data = asRecord(p.data);
  const fields = (def.data?.fields ?? []).filter((f) => f.name in data && !SHOWN_APART.has(f.name));
  const aboutPlural = p.about_object ? plural(p.about_object) : undefined;
  const busy = apply.isPending || dismiss.isPending;
  const failed = apply.error ?? dismiss.error;
  return (
    <div className={`card ${styles.card}`}>
      <div className={styles.text}>
        <p className="eyebrow">{KIND[p.kind]}</p>
        <p className={styles.name}>{p.name}</p>
        {(fields.length > 0 || p.about_name) && (
          <p className={styles.details}>
            {p.about_name && (
              <span className={styles.detail}>
                <span className={styles.detailLabel}>{p.kind === "person" || p.kind === "opportunity" ? "Company" : "On"}</span>
                {p.about_id && aboutPlural ? (
                  <Link to="/crm/$object/$recordId" params={{ object: aboutPlural, recordId: p.about_id }} className={styles.link}>
                    {p.about_name}
                  </Link>
                ) : (
                  <span>
                    {p.about_name} <span className={styles.detailLabel}>· waiting too</span>
                  </span>
                )}
              </span>
            )}
            {fields.map((f) => (
              <span key={f.id} className={styles.detail}>
                <span className={styles.detailLabel}>{f.label}</span>
                <CellView field={f} record={data} />
              </span>
            ))}
          </p>
        )}
        {p.kind === "note" && typeof data.body === "string" && data.body && <p className={styles.body}>{data.body}</p>}
        <p className={styles.quote}>“{p.quote}”</p>
        <p className="fieldHint aiMark">
          <SparkleIcon /> Proposed by {p.model ?? "the CRM agent"} from{" "}
          {p.meeting_id ? (
            <Link to="/meetings/$meetingId" params={{ meetingId: p.meeting_id }} search={{ tab: "summary" }} className={styles.link}>
              {p.source_title || "a meeting"}
            </Link>
          ) : (
            <>
              the email{" "}
              <Link to="/email" className={styles.link}>
                “{p.source_title || "No subject"}”
              </Link>
            </>
          )}{" "}
          · {formatWhen(p.created_at)}
        </p>
        {failed && (
          <p className="formError" role="alert">
            {failed.message}
          </p>
        )}
      </div>
      <div className={styles.actions}>
        <button type="button" className="btn btnSecondary btnCompact" disabled={busy} aria-label={`Apply: ${KIND[p.kind]} ${p.name}`} onClick={() => apply.mutate(p.id)}>
          Apply
        </button>
        <button type="button" className="btn btnGhost btnCompact" disabled={busy} aria-label={`Dismiss: ${KIND[p.kind]} ${p.name}`} onClick={() => dismiss.mutate(p.id)}>
          Dismiss
        </button>
      </div>
    </div>
  );
}

/** The CRM agent's proposals (D-072): what it would write, the sentence that says so and where it
 *  was read. Apply writes it into the CRM as the agent; Dismiss sets it aside. All of them on the
 *  CRM's "Waiting for you"; on a company's, a person's or an opportunity's page, those about it.
 *  Ghost cards when none. */
export default function ProposalCards({ slug, about }: { slug: string; about?: string }) {
  const proposals = useCrmProposals(slug, about ?? null);
  const objects = useObjects(slug);
  const plural = (singular: string) => objects.data?.find((o) => o.name_singular === singular)?.name_plural;
  const list = proposals.data ?? [];
  const none = proposals.isPending
    ? "Reading…"
    : proposals.isError
      ? `Couldn't be read — ${proposals.error.message}`
      : about
        ? "Nothing waiting — what the CRM agent proposes about this record shows here"
        : "Nothing waiting — after each summary and each email you read, the CRM agent proposes the companies, people, opportunities, notes and tasks it found";
  return (
    <section className={about ? styles.onRecord : styles.onTab} aria-label="Waiting for you">
      {about && (
        <>
          <h2 className="sectionTitle">Waiting for you</h2>
          <p className="sectionSubtitle">The CRM agent's proposals about this record — nothing is written until you apply one</p>
        </>
      )}
      <div className={styles.list}>
        {list.map((p) => (
          <Card key={p.id} slug={slug} p={p} plural={plural} />
        ))}
        {!list.length &&
          [0, 1].map((i) => (
            <div key={i} className={`card ghostCard ${styles.card}`}>
              <span className="ghostText">{i === 0 ? none : " "}</span>
            </div>
          ))}
      </div>
    </section>
  );
}
