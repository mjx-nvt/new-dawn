import { getRouteApi, useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { useNoticeCounts, useNoticeMutations, useNotices, type InboxShow, type Notice } from "../../kernel/bridge/notices";
import { useCurrentWorkspace } from "../../kernel/bridge/workspaces";
import { dayString, formatDateTime, formatDay } from "../../kernel/format/format";
import MenuButton from "../../kernel/shell/MenuButton";
import { sentence, useWho } from "./items/items";
import ItemView from "./items/ItemView";
import styles from "./Inbox.module.css";
import PageHeader from "../../kernel/shell/PageHeader";

const route = getRouteApi("/shell/projects/inbox");

const SHOWS: { key: InboxShow; label: string }[] = [
  { key: "inbox", label: "All notices" },
  { key: "unread", label: "Unread" },
  { key: "snoozed", label: "Snoozed" },
  { key: "archived", label: "Archived" },
];
const TYPES = [
  { key: "assigned", label: "Assigned to me" },
  { key: "created", label: "Created by me" },
  { key: "subscribed", label: "Subscribed by me" },
] as const;
/** Plane's snooze times, in days. */
const SNOOZES = [
  { days: 1, label: "1 day" },
  { days: 3, label: "3 days" },
  { days: 5, label: "5 days" },
  { days: 7, label: "1 week" },
  { days: 14, label: "2 weeks" },
];

const later = (days: number) => new Date(Date.now() + days * 86_400_000).toISOString();

/** "Due tomorrow", "Due today", "Due 12 Nov". */
function due(on: string | null): string {
  if (!on) return "Due soon";
  const today = new Date();
  const tomorrow = new Date(today.getFullYear(), today.getMonth(), today.getDate() + 1);
  return on === dayString(today) ? "Due today" : on === dayString(tomorrow) ? "Due tomorrow" : `Due ${formatDay(on, { short: true })}`;
}

function Row({ slug, notice: n, open, m, onOpen }: { slug: string; notice: Notice; open: boolean; m: ReturnType<typeof useNoticeMutations>; onOpen: () => void }) {
  const who = useWho(slug);
  const [custom, setCustom] = useState(false);
  const reminder = n.kind === "reminder";
  const line = reminder ? due(n.due_on) : sentence({ kind: n.activity_kind, field: n.field, old_value: n.old_value, new_value: n.new_value, actor: n.actor }, who);
  return (
    <li className={`${styles.row} ${open ? styles.open : ""} ${n.read_at ? "" : styles.unread}`}>
      <button type="button" className={styles.main} onClick={onOpen} aria-current={open}>
        <span className={styles.dot} aria-hidden />
        {!n.read_at && <span className="srOnly">Unread: </span>}
        <span className={styles.text}>
          <span className={styles.line}>
            {reminder && <span className="chip">Reminder</span>} {n.kind === "mention" ? `${who(n.actor)} mentioned you` : line}
          </span>
          <span className={styles.item}>
            <span className={styles.key}>{n.item_key}</span> {n.item_name}
          </span>
          <span className="fieldHint">{n.snoozed_until && !n.archived_at ? `Till ${formatDateTime(n.snoozed_until)}` : formatDateTime(n.created_at)}</span>
        </span>
      </button>
      <span className={styles.actions}>
        <button type="button" className="btn btnGhost btnCompact" disabled={m.mark.isPending} onClick={() => m.mark.mutate({ id: n.id, read: !n.read_at })}>
          {n.read_at ? "Mark unread" : "Mark read"}
        </button>
        {!n.archived_at && (
          <MenuButton
            label={`Snooze ${n.item_key}`}
            items={[
              ...SNOOZES.map((s) => ({ label: s.label, onSelect: () => m.snooze.mutate({ id: n.id, until: later(s.days) }) })),
              { label: "Pick a date and time…", onSelect: () => setCustom(true) },
              ...(n.snoozed_until ? [{ label: "Unsnooze", separator: true, onSelect: () => m.snooze.mutate({ id: n.id, until: null }) }] : []),
            ]}
          >
            Snooze
          </MenuButton>
        )}
        <button type="button" className="btn btnGhost btnCompact" disabled={m.archive.isPending} onClick={() => m.archive.mutate({ id: n.id, archived: !n.archived_at })}>
          {n.archived_at ? "Unarchive" : "Archive"}
        </button>
      </span>
      {custom && (
        <span className={styles.custom}>
          <input
            className="input"
            type="datetime-local"
            aria-label="Snooze until"
            autoFocus
            onBlur={(e) => {
              setCustom(false);
              if (e.target.value) m.snooze.mutate({ id: n.id, until: new Date(e.target.value).toISOString() });
            }}
          />
        </span>
      )}
      {(m.snooze.error ?? m.archive.error ?? m.mark.error) && <span className="formError">{(m.snooze.error ?? m.archive.error ?? m.mark.error)?.message}</span>}
    </li>
  );
}

/** Plane's Inbox (D-075): what agents and automations did on work items you follow, your
 *  mentions and due-date reminders — All and Mentions, each with its unread count; shown by
 *  unread, snoozed or archived; filtered by assigned, created or subscribed; read, snoozed,
 *  archived; a notice opens its work item beside the list. Everything is URL state. */
export default function InboxPage() {
  const search = route.useSearch();
  const navigate = useNavigate();
  const { data: workspace } = useCurrentWorkspace();
  const slug = workspace?.slug;
  const mentions = search.tab === "mentions";
  const show = SHOWS.find((s) => s.key === search.show)?.key ?? "inbox";
  const types = (search.filter ?? "").split(",").filter((t) => TYPES.some((x) => x.key === t));
  const notices = useNotices(slug, { mentions, show, assigned: types.includes("assigned"), created: types.includes("created"), subscribed: types.includes("subscribed") });
  const counts = useNoticeCounts(slug).data;
  const m = useNoticeMutations(slug);
  const set = (patch: Record<string, string | undefined>) => void navigate({ to: "/projects/inbox", search: { ...search, ...patch }, replace: true });
  const list = notices.data ?? [];
  const toggleType = (t: string) => {
    const next = types.includes(t) ? types.filter((x) => x !== t) : [...types, t];
    set({ filter: next.length ? next.join(",") : undefined });
  };
  return (
    <div className="page">
      <PageHeader eyebrow="Projects" title="Inbox" meta="What agents and automations do on work items you follow, your mentions and due-date reminders — here only, never mailed." />
      <div className={styles.bar}>
        <div className="segmented" role="tablist" aria-label="Inbox">
          <button type="button" role="tab" aria-selected={!mentions} onClick={() => set({ tab: undefined, item: undefined, notice: undefined })}>
            All{counts?.all ? ` · ${counts.all}` : ""}
          </button>
          <button type="button" role="tab" aria-selected={mentions} onClick={() => set({ tab: "mentions", item: undefined, notice: undefined })}>
            Mentions{counts?.mentions ? ` · ${counts.mentions}` : ""}
          </button>
        </div>
        <select className="input" aria-label="Show" value={show} onChange={(e) => set({ show: e.target.value === "inbox" ? undefined : e.target.value })}>
          {SHOWS.map((s) => (
            <option key={s.key} value={s.key}>
              {s.label}
            </option>
          ))}
        </select>
        <span className={styles.types} role="group" aria-label="Filter">
          {TYPES.map((t) => (
            <button key={t.key} type="button" className={`chip ${types.includes(t.key) ? "chipPrimary" : ""}`} aria-pressed={types.includes(t.key)} onClick={() => toggleType(t.key)}>
              {t.label}
            </button>
          ))}
        </span>
        <span className={styles.spacer} />
        <button type="button" className="btn btnGhost btnCompact" disabled={m.markAll.isPending || !(mentions ? counts?.mentions : counts?.all)} onClick={() => m.markAll.mutate(mentions)}>
          Mark all as read
        </button>
      </div>
      {(notices.error ?? m.markAll.error) && <p className="formError">{(notices.error ?? m.markAll.error)?.message}</p>}
      <div className={styles.split}>
        <ul className={styles.list} aria-label="Notices">
          {slug &&
            list.map((n) => (
              <Row
                key={n.id}
                slug={slug}
                notice={n}
                open={n.id === search.notice}
                m={m}
                onOpen={() => {
                  if (!n.read_at) m.mark.mutate({ id: n.id, read: true });
                  set({ notice: n.id, item: n.item_id, activity: undefined });
                }}
              />
            ))}
          {!list.length &&
            [0, 1, 2].map((k) => (
              <li key={k} className={`${styles.ghost} ghostCard`}>
                {k === 0 && (
                  <span className="ghostText">
                    {notices.isError
                      ? "Couldn't be read"
                      : !notices.data
                        ? "Reading…"
                        : mentions
                          ? "No mentions — when an agent's comment names you, it lands here"
                          : show === "inbox"
                            ? "Nothing new — what agents and automations do on work items you follow lands here, with due-date reminders"
                            : `Nothing ${SHOWS.find((s) => s.key === show)?.label.toLowerCase()}`}
                  </span>
                )}
              </li>
            ))}
        </ul>
        <section className={styles.panel} aria-label="Work item">
          {slug && search.item ? (
            <ItemView key={search.item} slug={slug} itemId={search.item} tab={search.activity} onTab={(activity) => set({ activity })} onOpen={(id) => set({ item: id, notice: undefined, activity: undefined })} onGone={() => set({ item: undefined, notice: undefined })} />
          ) : (
            [0, 1].map((k) => (
              <div key={k} className={`${styles.ghost} ghostCard`}>
                {k === 0 && <span className="ghostText">Pick a notice to read its work item here</span>}
              </div>
            ))
          )}
        </section>
      </div>
    </div>
  );
}
