import { Link } from "@tanstack/react-router";
import { useCurrentWorkspace } from "../../kernel/bridge/workspaces";
import { useAddEmailTask, useDismissEmailTask, useEmails, useForgetEmail, useReadEmailFile, useReadMail, type EmailItem, type EmailTask } from "../../kernel/bridge/inbox";
import DangerButton from "../../kernel/shell/DangerButton";
import { SparkleIcon } from "../../kernel/shell/icons";
import { formatWhen } from "../meetings/format";
import styles from "./Email.module.css";

/** Email (D-056): give New Dawn one email — the one selected in Mail, or an .eml file — and the
 *  Email reading agent proposes your tasks (under the email: Add to Tasks or Dismiss), the projects'
 *  updates and others' commitments, each quoting the email. Nothing is fetched or sent: New Dawn
 *  reads only what you give it. */
export default function EmailPage() {
  const { data: workspace } = useCurrentWorkspace();
  const slug = workspace?.slug;
  const emails = useEmails(slug);
  const readMail = useReadMail(slug);
  const readFile = useReadEmailFile(slug);
  const reading = readMail.isPending || readFile.isPending;
  const read = readMail.data ?? readFile.data ?? null;
  const error = readMail.error ?? readFile.error ?? emails.error;

  return (
    <div className="page">
      <p className="eyebrow">{workspace?.name ?? "No workspace"}</p>
      <h1 className="pageTitle">Email</h1>
      <p className={`fieldHint ${styles.lead}`}>
        New Dawn reads the email you give it, on this Mac, and proposes your tasks, the projects' updates and others' commitments — each quoting the email. Nothing is fetched or sent.
      </p>
      <div className={styles.actions}>
        <button type="button" className="btn btnPrimary btnAi" disabled={!slug || reading} aria-busy={readMail.isPending || undefined} onClick={() => (readFile.reset(), readMail.mutate())}>
          <SparkleIcon />
          {readMail.isPending ? "Reading…" : "Read the email selected in Mail"}
        </button>
        <button type="button" className="btn btnSecondary btnAi" disabled={!slug || reading} aria-busy={readFile.isPending || undefined} onClick={() => (readMail.reset(), readFile.mutate())}>
          <SparkleIcon />
          {readFile.isPending ? "Reading…" : "Open an .eml file…"}
        </button>
      </div>
      <div role="status">{read && <p className={`fieldHint aiMark ${styles.result}`}><SparkleIcon /> {outcome(read)}</p>}</div>
      {error && (
        <p className="formError" role="alert">
          {error.message}
        </p>
      )}

      <section className="section">
        <h2 className="sectionTitle">Emails read</h2>
        <p className="sectionSubtitle">What came of each: your tasks wait under the email, updates on their projects, commitments in People</p>
        {emails.data?.length ? (
          <ul className={styles.list} role="list">
            {emails.data.map((e) => (
              <EmailRow key={e.id} slug={slug} email={e} />
            ))}
          </ul>
        ) : (
          <div className={styles.list}>
            <p className="ghostText">{emails.isPending ? "Loading…" : "Not yet captured — read an email above"}</p>
            {[0, 1, 2].map((i) => (
              <div key={i} className={`${styles.slot} ghostCard`} />
            ))}
          </div>
        )}
      </section>
    </div>
  );
}

function outcome(e: EmailItem): string {
  const parts = [
    e.tasks ? `${e.tasks} task${e.tasks === 1 ? "" : "s"}` : "",
    e.updates ? `${e.updates} project update${e.updates === 1 ? "" : "s"}` : "",
    e.commitments ? `${e.commitments} commitment${e.commitments === 1 ? "" : "s"}` : "",
  ].filter(Boolean);
  return parts.length ? `“${e.subject || "No subject"}”: ${parts.join(", ")}` : `“${e.subject || "No subject"}”: nothing for you to do`;
}

function EmailRow({ slug, email: e }: { slug?: string; email: EmailItem }) {
  const forget = useForgetEmail(slug);
  return (
    <li className={`card ${styles.row}`}>
      <div className={styles.main}>
        <p className={styles.subject}>{e.subject || "No subject"}</p>
        <p className="fieldHint">
          {[e.sender, e.received_text || null, `read ${formatWhen(e.created_at)}`, e.source === "mail" ? "from Mail" : "from a file"].filter(Boolean).join(" · ")}
        </p>
        <p className={`fieldHint aiMark ${styles.links}`}>
          <SparkleIcon />
          {e.tasks + e.updates + e.commitments === 0 ? (
            "Nothing for you to do"
          ) : (
            <>
              {e.tasks > 0 && (
                <span>
                  {e.tasks} task{e.tasks === 1 ? "" : "s"}
                </span>
              )}
              {e.updates > 0 && (
                <Link to="/projects" search={{}} className={styles.link}>
                  {e.updates} project update{e.updates === 1 ? "" : "s"}
                </Link>
              )}
              {e.commitments > 0 && (
                <Link to="/people" className={styles.link}>
                  {e.commitments} commitment{e.commitments === 1 ? "" : "s"}
                </Link>
              )}
            </>
          )}
        </p>
        {e.proposed.length > 0 && (
          <ul className={styles.proposals} role="list">
            {e.proposed.map((t) => (
              <Proposal key={t.id} slug={slug} task={t} />
            ))}
          </ul>
        )}
        {forget.error && (
          <p className="formError" role="alert">
            {forget.error.message}
          </p>
        )}
      </div>
      <DangerButton confirm="Forget this email?" label={`Forget “${e.subject || "No subject"}”`} disabled={forget.isPending} onConfirm={() => forget.mutate(e.id)}>
        Forget
      </DangerButton>
    </li>
  );
}

/** A task the email asks of you: Add to Tasks or Dismiss (experts propose, you decide, D-054). */
function Proposal({ slug, task: t }: { slug?: string; task: EmailTask }) {
  const add = useAddEmailTask(slug);
  const dismiss = useDismissEmailTask(slug);
  const busy = add.isPending || dismiss.isPending;
  const error = add.error ?? dismiss.error;
  return (
    <li className={styles.proposal}>
      <div className={styles.proposalText}>
        <p className={styles.item}>
          {t.item}
          {t.due_text && <span className={styles.due}> · {t.due_text}</span>}
        </p>
        {t.evidence && <p className={styles.evidence}>“{t.evidence}”</p>}
        {error && (
          <p className="formError" role="alert">
            {error.message}
          </p>
        )}
      </div>
      {t.status === "added" ? (
        <Link to="/tasks" className={`fieldHint ${styles.link}`}>
          In Tasks
        </Link>
      ) : (
        <div className={styles.proposalActions}>
          <button type="button" className="btn btnSecondary btnCompact" disabled={busy} aria-label={`Add to Tasks: ${t.item}`} onClick={() => add.mutate(t.id)}>
            Add to Tasks
          </button>
          <button type="button" className="btn btnGhost btnCompact" disabled={busy} aria-label={`Dismiss: ${t.item}`} onClick={() => dismiss.mutate(t.id)}>
            Dismiss
          </button>
        </div>
      )}
    </li>
  );
}
