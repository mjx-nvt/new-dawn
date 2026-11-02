import { getRouteApi, useNavigate } from "@tanstack/react-router";
import { usePmMyActivity } from "../../kernel/bridge/pmHome";
import type { PmItem } from "../../kernel/bridge/pmItems";
import { usePmQuery, type PmRule } from "../../kernel/bridge/pmViews";
import { useCurrentWorkspace } from "../../kernel/bridge/workspaces";
import { PieChart } from "../../kernel/charts/Charts";
import { formatDateTime } from "../../kernel/format/format";
import ItemPeek, { type PeekMode } from "./items/ItemPeek";
import { Dot, PriorityMark } from "./items/Props";
import { PRIORITIES, sentence, useWho } from "./items/items";
import styles from "./items/Items.module.css";

const route = getRouteApi("/shell/projects/your-work");
/** The owner in a field. */
const mine = (field: string): PmRule => ({ field, op: "in", values: ["me"] });
const TABS: { key: "assigned" | "created" | "subscribed"; label: string; rule: PmRule }[] = [
  { key: "assigned", label: "Assigned", rule: mine("assignees") },
  { key: "created", label: "Created", rule: mine("created_by") },
  { key: "subscribed", label: "Subscribed", rule: mine("subscribers") },
];
/** Plane's workload groups. */
const GROUPS = [
  { key: "backlog", label: "Backlog", color: "gray" },
  { key: "unstarted", label: "Not started", color: "blue" },
  { key: "started", label: "Working on", color: "orange" },
  { key: "completed", label: "Completed", color: "green" },
  { key: "cancelled", label: "Cancelled", color: "red" },
] as const;

function Bars({ title, rows }: { title: string; rows: { key: string; label: string; n: number }[] }) {
  const max = Math.max(1, ...rows.map((r) => r.n));
  return (
    <div className={styles.breakdown}>
      <h3 className={styles.h3}>{title}</h3>
      {rows.map((r) => (
        <div key={r.key} className={styles.shareRow}>
          <span>{r.label}</span>
          <span className={styles.progress}>
            <span style={{ width: `${Math.round((r.n / max) * 100)}%` }} />
          </span>
          <span className="fieldHint">{r.n}</span>
        </div>
      ))}
    </div>
  );
}

/** Plane's Your work (D-082): what you're assigned, made and follow across the open projects —
 *  how many of each, the chosen tab's work by state group and by priority with its pie, your
 *  recent activity, and the items themselves (each opens in the peek). */
export default function YourWorkPage() {
  const search = route.useSearch();
  const navigate = useNavigate();
  const { data: workspace } = useCurrentWorkspace();
  const slug = workspace?.slug;
  const who = useWho(slug ?? "");
  const tab = TABS.find((t) => t.key === search.ytab) ?? TABS[0];
  const lists = { assigned: usePmQuery(slug, null, [TABS[0].rule]), created: usePmQuery(slug, null, [TABS[1].rule]), subscribed: usePmQuery(slug, null, [TABS[2].rule]) };
  const items: PmItem[] = lists[tab.key].data ?? [];
  const activityQuery = usePmMyActivity(slug);
  const activity = activityQuery.data ?? [];
  const set = (patch: Record<string, string | undefined>) => void navigate({ to: "/projects/your-work", search: { ...search, ...patch }, replace: true });
  const byGroup = GROUPS.map((g) => ({ key: g.key, label: g.label, n: items.filter((i) => i.state_group === g.key).length }));
  const pie = { categories: GROUPS.map((g) => ({ key: g.key, label: g.label, color: g.color })), series: [{ key: "items", label: "Work items", values: byGroup.map((g) => g.n) }], total: items.length, truncated: false };
  return (
    <div className="page">
      <p className="eyebrow">{workspace?.name ?? "No workspace"}</p>
      <h1 className="pageTitle">Your work</h1>
      <p className="sectionSubtitle">What you're assigned, made and follow, across the open projects.</p>
      <div className={styles.toolbar}>
        <div className="segmented" role="tablist" aria-label="Your work">
          {TABS.map((t) => (
            <button key={t.key} type="button" role="tab" aria-selected={tab.key === t.key} onClick={() => set({ ytab: t.key === "assigned" ? undefined : t.key })}>
              {t.label} · {lists[t.key].data?.length ?? "…"}
            </button>
          ))}
        </div>
      </div>
      <div className={styles.cycleBody}>
        <div className={styles.viewMain}>
          <div className={styles.workCharts}>
            <Bars title="By state group" rows={byGroup} />
            <Bars title="By priority" rows={PRIORITIES.map((p) => ({ key: p.key, label: p.label, n: items.filter((i) => i.priority === p.key).length }))} />
            <figure className={styles.burndown} aria-label="Work items by state group">
              {items.length ? <PieChart data={pie} style={{ legend: true, showCenter: true }} /> : <p className="ghostText">No work items here yet</p>}
            </figure>
          </div>
          <h2 className={styles.h3}>{tab.label} · {items.length}</h2>
          {items.map((i) => (
            <button key={i.id} type="button" className={styles.subRow} onClick={() => set({ peek: i.id })}>
              <span className={styles.key}>{i.key}</span> {i.name}
              <span className={styles.prop}>
                <Dot color={i.state_color} /> {i.state_name}
              </span>
              <PriorityMark value={i.priority} />
            </button>
          ))}
          {!items.length &&
            [0, 1].map((n) => (
              <div key={n} className={`${styles.ghostRow} ghostCard`}>
                {n === 0 && <span className="ghostText">{lists[tab.key].isError ? "Couldn't be read" : lists[tab.key].data ? "Nothing here yet" : "Reading…"}</span>}
              </div>
            ))}
        </div>
        <aside className={styles.side} aria-label="Your recent activity">
          <h3 className={styles.h3}>Recent activity</h3>
          {activity.map((a) => (
            <button key={a.id} type="button" className={styles.activityRow} onClick={() => set({ peek: a.item_id })}>
              <span>
                <span className={styles.key}>{a.key}</span> {sentence(a, who)}
              </span>
              <span className="fieldHint">{formatDateTime(a.created_at)}</span>
            </button>
          ))}
          {!activity.length &&
            [0, 1].map((n) => (
              <div key={n} className={`${styles.ghostRow} ghostCard`}>
                {n === 0 && <span className="ghostText">{activityQuery.isError ? "Couldn't be read" : activityQuery.data ? "What you do on work items shows here" : "Reading…"}</span>}
              </div>
            ))}
        </aside>
      </div>
      {slug && search.peek && (
        <ItemPeek
          slug={slug}
          itemId={search.peek}
          mode={(["side", "modal", "full"] as PeekMode[]).find((x) => x === search.mode) ?? "side"}
          tab={search.activity}
          onTab={(activity) => set({ activity })}
          onMode={(x) => set({ mode: x === "side" ? undefined : x })}
          onOpen={(id) => set({ peek: id, activity: undefined })}
          onClose={() => set({ peek: undefined, mode: undefined, activity: undefined })}
        />
      )}
    </div>
  );
}
