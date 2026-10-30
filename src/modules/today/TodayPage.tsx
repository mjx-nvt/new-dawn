import { Link, useNavigate } from "@tanstack/react-router";
import { useState, type MouseEvent, type ReactNode } from "react";
import { useToday, type Day } from "../../kernel/bridge/cockpit";
import { useCurrentWorkspace, useSetCurrentWorkspace } from "../../kernel/bridge/workspaces";
import StatusChip from "../meetings/StatusChip";
import { formatDay } from "../projects/format";
import { dueLabel, isPast } from "../tasks/due";
import styles from "./Today.module.css";

type Meeting = Day["meetings"][number];
/** A row and the workspace it belongs to. */
type In<T> = { day: Day; item: T };

/** Today (D-057): what needs you, across every open workspace — tasks due, projects whose
 *  milestone is near, proposals waiting for you, commitments to follow up and this week's
 *  meetings. It only reads; each row opens its page, in its workspace. */
export default function TodayPage() {
  const today = useToday();
  const days = today.data?.workspaces ?? [];
  const many = days.length > 1;
  const all = <T,>(pick: (d: Day) => T[]): In<T>[] => days.flatMap((day) => pick(day).map((item) => ({ day, item })));
  const tasks = all((d) => d.tasks);
  const projects = all((d) => d.projects);
  const commitments = all((d) => d.commitments);
  const meetings = all((d) => d.meetings).sort((a, b) => a.item.held_on.localeCompare(b.item.held_on));
  const waiting = days.filter((d) => d.waiting.updates + d.waiting.commitments + d.waiting.email_tasks > 0);
  const more = days.reduce((n, d) => n + d.open_commitments - d.commitments.length, 0);
  // A section's line when it has nothing: only once the page was read.
  const empty = (none: string) => (today.isSuccess ? none : today.isError ? "Couldn't be read" : "Loading…");
  const [via, switchError] = useVia();
  const unread = today.data?.unread ?? [];

  return (
    <div className="page">
      <p className="eyebrow">{longDay(today.data?.day)}</p>
      <h1 className="pageTitle">Today</h1>
      <p className={`fieldHint ${styles.lead}`}>
        What needs you{many ? ` across your ${days.length} workspaces` : ""}: what's due, what's near and what waits for your decision.
      </p>
      {(today.error ?? switchError) && (
        <p className="formError" role="alert">
          {(today.error ?? switchError)?.message}
        </p>
      )}
      {unread.length > 0 && (
        <p className="formError" role="alert">
          {unread.length === 1 ? `${unread[0]} couldn't be read` : `${unread.join(", ")} couldn't be read`} — the log in ~/Library/Logs says why.
        </p>
      )}

      <Section title="Due today" subtitle="Open tasks due today or earlier" empty={empty("Nothing due today")} count={tasks.length}>
        {tasks.map(({ day, item: t }) => (
          <Link key={t.id} to="/tasks" className={styles.row} onClick={via(day.slug)}>
            <Row day={day} many={many} title={t.owner ? `${t.owner}: ${t.title}` : t.title} meta={t.meeting ? t.meeting.title : t.reason}>
              <span className={t.due_on && isPast(t.due_on) ? `${styles.side} ${styles.late}` : styles.side}>{t.due_on ? dueLabel(t.due_on) : ""}</span>
            </Row>
          </Link>
        ))}
      </Section>

      <Section title="Waiting for you" subtitle="Proposals to apply or dismiss" empty={empty("Nothing waiting for you")} count={waiting.length}>
        {waiting.map((d) => (
          <div key={d.slug} className={styles.waiting}>
            {many && <span className="chip">{d.name}</span>}
            {d.waiting.updates > 0 && (
              <Link to="/projects" className={styles.link} aria-label={many ? `${plural(d.waiting.updates, "project update")} in ${d.name}` : undefined} onClick={via(d.slug)}>
                {plural(d.waiting.updates, "project update")}
              </Link>
            )}
            {d.waiting.commitments > 0 && (
              <Link to="/people" className={styles.link} aria-label={many ? `${plural(d.waiting.commitments, "commitment")} in ${d.name}` : undefined} onClick={via(d.slug)}>
                {plural(d.waiting.commitments, "commitment")}
              </Link>
            )}
            {d.waiting.email_tasks > 0 && (
              <Link to="/email" className={styles.link} aria-label={many ? `${plural(d.waiting.email_tasks, "task from an email", "tasks from emails")} in ${d.name}` : undefined} onClick={via(d.slug)}>
                {plural(d.waiting.email_tasks, "task from an email", "tasks from emails")}
              </Link>
            )}
          </div>
        ))}
      </Section>

      <Section title="Projects to watch" subtitle="Active projects whose next milestone is due within a week" empty={empty("No milestone due this week")} count={projects.length}>
        {projects.map(({ day, item: p }) => (
          <Link key={p.id} to="/projects/$projectId" params={{ projectId: p.id }} className={styles.row} onClick={via(day.slug)}>
            <Row day={day} many={many} title={p.name} meta={`${p.next_milestone || "Next milestone"}${p.waiting > 0 ? ` · ${plural(p.waiting, "update")} waiting` : ""}`}>
              <span className={p.milestone_due && isPast(p.milestone_due) ? `${styles.side} ${styles.late}` : styles.side}>{p.milestone_due ? milestoneWhen(p.milestone_due) : ""}</span>
            </Row>
          </Link>
        ))}
      </Section>

      <Section title="Commitments to follow up" subtitle="What others promised you, and you confirmed" empty={empty("No open commitments")} count={commitments.length}>
        {commitments.map(({ day, item: c }) => {
          const row = (
            <Row day={day} many={many} title={`${c.person_name} · ${c.what}`} meta={c.email_item_id ? `From the email “${c.email_subject || "No subject"}”` : c.meeting_title ? `From ${c.meeting_title}` : ""}>
              <span className={styles.side}>{c.due_text ? `By ${c.due_text}` : ""}</span>
            </Row>
          );
          return c.person_id ? (
            <Link key={c.id} to="/people/$personId" params={{ personId: c.person_id }} className={styles.row} onClick={via(day.slug)}>
              {row}
            </Link>
          ) : (
            <Link key={c.id} to="/people" className={styles.row} onClick={via(day.slug)}>
              {row}
            </Link>
          );
        })}
        {more > 0 && (
          <p className={`fieldHint ${styles.more}`}>
            {plural(more, "more open commitment")} in <Link to="/people">People</Link>
            {many ? " (each workspace's own)" : ""}
          </p>
        )}
      </Section>

      <Section title="This week's meetings" subtitle="Monday to Sunday" empty={empty("No meetings this week")} count={meetings.length}>
        {meetings.map(({ day, item: m }) => (
          <Link key={m.id} to="/meetings/$meetingId" params={{ meetingId: m.id }} search={{ tab: "summary" }} className={styles.row} onClick={via(day.slug)}>
            <Row day={day} many={many} title={m.title} meta={meetingWhen(m)}>
              <span className={styles.side}>{m.status !== "summarised" && <StatusChip status={m.status} />}</span>
            </Row>
          </Link>
        ))}
      </Section>
    </div>
  );
}

/** A section: its rows in one card, or a line and ghost rows when there are none. */
function Section({ title, subtitle, empty, count, children }: { title: string; subtitle: string; empty: string; count: number; children: ReactNode }) {
  return (
    <section className="section">
      <h2 className="sectionTitle">{title}</h2>
      <p className="sectionSubtitle">{subtitle}</p>
      {count > 0 ? (
        <div className={styles.list}>{children}</div>
      ) : (
        <div className={styles.empty}>
          <p className="ghostText">{empty}</p>
          {[0, 1].map((i) => (
            <div key={i} className={`${styles.slot} ghostCard`} />
          ))}
        </div>
      )}
    </section>
  );
}

/** A row's body: its title, a meta line (with the workspace when there are several) and, on the
 *  right, a date or a chip. The caller's <Link> opens its page. */
function Row({ day, many, title, meta, children }: { day: Day; many: boolean; title: string; meta: string; children: ReactNode }) {
  return (
    <>
      <span className={styles.main}>
        <span className={styles.title}>{title}</span>
        {(many || meta) && (
          <span className={styles.metaLine}>
            {many && <span className="chip">{day.name}</span>}
            {meta}
          </span>
        )}
      </span>
      {children}
    </>
  );
}

/** A link's click into another workspace's page: New Dawn switches to that workspace first,
 *  then opens the page. In the current workspace the link simply opens it. Also returns why a
 *  switch failed (an archived workspace, say). */
function useVia() {
  const { data: current } = useCurrentWorkspace();
  const switchTo = useSetCurrentWorkspace();
  const navigate = useNavigate();
  const [error, setError] = useState<Error | null>(null);
  const via = (slug: string) => (e: MouseEvent<HTMLAnchorElement>) => {
    if (slug === current?.slug) return;
    e.preventDefault();
    const href = e.currentTarget.getAttribute("href") ?? "/";
    setError(null);
    switchTo.mutate(slug, { onSuccess: () => void navigate({ href }), onError: setError });
  };
  return [via, error] as const;
}

function plural(n: number, one: string, many = `${one}s`): string {
  return `${n} ${n === 1 ? one : many}`;
}

/** "Saturday 31 October" for the Mac's day (YYYY-MM-DD). */
function longDay(day: string | undefined): string {
  if (!day) return "Today";
  const [y, m, d] = day.split("-").map(Number);
  return new Date(y, m - 1, d).toLocaleDateString("en-GB", { weekday: "long", day: "numeric", month: "long" });
}

/** A milestone's day: "Today", "Tomorrow", "Fri", "3 Nov" — or, past, "Overdue since 28 Oct". */
function milestoneWhen(day: string): string {
  return isPast(day) ? `Overdue since ${formatDay(day)}` : dueLabel(day);
}

/** "Thu 29 Oct · 09:38" in the Mac's time. */
function meetingWhen(m: Meeting): string {
  const at = new Date(m.held_on);
  return `${at.toLocaleDateString("en-GB", { weekday: "short", day: "numeric", month: "short" })} · ${at.toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" })}`;
}

