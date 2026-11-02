import { getRouteApi, useNavigate } from "@tanstack/react-router";
import { usePmProjects } from "../../kernel/bridge/pm";
import { usePmCreatedVsResolved, usePmInsights, usePmOverview } from "../../kernel/bridge/pmAnalytics";
import { useActivePmCycles, usePmCycles } from "../../kernel/bridge/pmCycles";
import { usePmModules } from "../../kernel/bridge/pmModules";
import { useCurrentWorkspace } from "../../kernel/bridge/workspaces";
import { AggregateValue, BarChart, LineChart } from "../../kernel/charts/Charts";
import { dayString, formatDay } from "../../kernel/format/format";
import { Progress } from "./items/Cycles";
import { GROUP_LABEL, PRIORITIES } from "./items/items";
import styles from "./items/Items.module.css";

const route = getRouteApi("/shell/projects/analytics");
const TABS = [
  { key: "overview", label: "Overview" },
  { key: "items", label: "Work items" },
  { key: "cycles", label: "Cycles" },
  { key: "modules", label: "Modules" },
] as const;
const PERIODS = [
  { key: "7", label: "Last 7 days" },
  { key: "30", label: "Last 30 days" },
  { key: "90", label: "Last 90 days" },
] as const;
const AXES = [
  { key: "state", label: "State" },
  { key: "state_group", label: "State group" },
  { key: "priority", label: "Priority" },
  { key: "assignee", label: "Assignee" },
  { key: "label", label: "Label" },
  { key: "cycle", label: "Cycle" },
  { key: "module", label: "Module" },
] as const;

const daysAgo = (n: number) => {
  const d = new Date();
  return dayString(new Date(d.getFullYear(), d.getMonth(), d.getDate() - n));
};

/** A share's words (a state group's or a priority's key reads as its name). */
function wordFor(axis: string, key: string, label: string): string {
  if (axis === "state_group") return GROUP_LABEL[key as keyof typeof GROUP_LABEL] ?? label;
  if (axis === "priority") return PRIORITIES.find((p) => p.key === key)?.label ?? label;
  return label;
}

/** Plane's Analytics (D-083): the open projects or one — an overview, the work items (by state
 *  group, created against resolved day by day, counted by a property), and the cycles' and
 *  modules' progress. The choices are in the address. */
export default function AnalyticsPage() {
  const search = route.useSearch();
  const navigate = useNavigate();
  const { data: workspace } = useCurrentWorkspace();
  const slug = workspace?.slug;
  const projects = (usePmProjects(slug).data ?? []).filter((p) => !p.archived_at);
  // The address's project from the start (a deep link doesn't first read every project).
  const pid = search.aproj ?? null;
  const tab = TABS.find((t) => t.key === search.atab)?.key ?? "overview";
  const period = PERIODS.find((p) => p.key === search.aperiod)?.key ?? "7";
  const axis = AXES.find((a) => a.key === search.ax)?.key ?? "priority";
  const set = (patch: Record<string, string | undefined>) => void navigate({ to: "/projects/analytics", search: { ...search, ...patch }, replace: true });
  const overview = usePmOverview(slug, pid);
  const from = daysAgo(Number(period) - 1);
  const to = daysAgo(0);
  const flow = usePmCreatedVsResolved(slug, pid, from, to);
  const groups = usePmInsights(slug, pid, "state_group");
  const insight = usePmInsights(slug, pid, axis);
  const cycles = usePmCycles(slug, pid ?? "");
  const active = useActivePmCycles(slug);
  const modules = usePmModules(slug, pid ?? "");
  const o = overview.data;
  const days = flow.data ?? [];
  const lineData = {
    categories: days.map((d) => ({ key: d.day, label: formatDay(d.day, { short: true }) })),
    series: [
      { key: "created", label: "Created", values: days.map((d) => d.created) },
      { key: "resolved", label: "Resolved", values: days.map((d) => d.resolved) },
    ],
    total: null,
    truncated: false,
  };
  const bars = (shares: { key: string; label: string; total: number; done: number }[], by: string) => ({
    categories: shares.map((s) => ({ key: s.key, label: wordFor(by, s.key, s.label) })),
    series: [
      { key: "total", label: "Work items", values: shares.map((s) => s.total) },
      { key: "done", label: "Completed", values: shares.map((s) => s.done) },
    ],
    total: null,
    truncated: false,
  });
  const ghost = (text: string) =>
    [0, 1].map((n) => (
      <div key={n} className={`${styles.ghostRow} ghostCard`}>
        {n === 0 && <span className="ghostText">{text}</span>}
      </div>
    ));
  return (
    <div className="page">
      <p className="eyebrow">{workspace?.name ?? "No workspace"}</p>
      <h1 className="pageTitle">Analytics</h1>
      <p className="sectionSubtitle">How the work stands and moves — every open project, or one.</p>
      <div className={styles.toolbar}>
        <div className="segmented" role="tablist" aria-label="Analytics">
          {TABS.map((t) => (
            <button key={t.key} type="button" role="tab" aria-selected={tab === t.key} onClick={() => set({ atab: t.key === "overview" ? undefined : t.key })}>
              {t.label}
            </button>
          ))}
        </div>
        <select className="input" aria-label="Project" value={pid ?? ""} onChange={(e) => set({ aproj: e.target.value || undefined })}>
          <option value="">Every open project</option>
          {projects.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
      </div>
      {(overview.error ?? flow.error ?? groups.error ?? insight.error ?? cycles.error ?? active.error ?? modules.error) && <p className="formError">{(overview.error ?? flow.error ?? groups.error ?? insight.error ?? cycles.error ?? active.error ?? modules.error)?.message}</p>}
      {tab === "overview" && (
        <div className={styles.workCharts}>
          {o ? (
            <>
              <AggregateValue value={o.projects} hint="Projects" />
              <AggregateValue value={o.items} hint="Work items" />
              <AggregateValue value={o.open_items} hint="Open work items" />
              <AggregateValue value={o.active_cycles} hint="Active cycles" />
              <AggregateValue value={o.requests} hint="Requests in Intake" />
            </>
          ) : (
            ghost(overview.isError ? "Couldn't be read" : "Reading…")
          )}
        </div>
      )}
      {tab === "items" && (
        <>
          <div className={styles.subsHead}>
            <h2 className={styles.h3}>Created vs Resolved</h2>
            <span className={styles.spacer} />
            <select className="input" aria-label="Period" value={period} onChange={(e) => set({ aperiod: e.target.value === "7" ? undefined : e.target.value })}>
              {PERIODS.map((p) => (
                <option key={p.key} value={p.key}>
                  {p.label}
                </option>
              ))}
            </select>
          </div>
          <figure className={styles.analyticsChart} aria-label={`Work items created and resolved, ${from} to ${to}`}>
            {days.length ? <LineChart data={lineData} style={{ legend: true }} /> : ghost(flow.isError ? "Couldn't be read" : "Reading…")}
          </figure>
          <h2 className={styles.h3}>By state group</h2>
          <figure className={styles.analyticsChart} aria-label="Work items by state group">
            {groups.data?.length ? <BarChart data={bars(groups.data, "state_group")} style={{ legend: true, labels: true }} /> : ghost(groups.isError ? "Couldn't be read" : groups.data ? "No work items yet" : "Reading…")}
          </figure>
          <div className={styles.subsHead}>
            <h2 className={styles.h3}>Customized insights</h2>
            <span className={styles.spacer} />
            <select className="input" aria-label="Counted by" value={axis} onChange={(e) => set({ ax: e.target.value === "priority" ? undefined : e.target.value })}>
              {AXES.map((a) => (
                <option key={a.key} value={a.key}>
                  By {a.label.toLowerCase()}
                </option>
              ))}
            </select>
          </div>
          <figure className={styles.analyticsChart} aria-label={`Work items by ${axis}`}>
            {insight.data?.length ? <BarChart data={bars(insight.data, axis)} style={{ legend: true, labels: true }} /> : ghost(insight.isError ? "Couldn't be read" : insight.data ? "No work items yet" : "Reading…")}
          </figure>
        </>
      )}
      {tab === "cycles" && (
        <div className={styles.group}>
          {(pid ? (cycles.data ?? []).filter((c) => !c.archived_at) : active.data ?? []).map((c) => (
            <div key={c.id} className={styles.row}>
              <span className={styles.rowTitle}>{c.name}</span>
              <span className={styles.rowProps}>
                {!pid && <span className="fieldHint">{projects.find((p) => p.id === c.project_id)?.name ?? ""}</span>}
                <span className="chip">{c.status.charAt(0).toUpperCase() + c.status.slice(1)}</span>
                <Progress cycle={c} />
              </span>
            </div>
          ))}
          {!(pid ? cycles.data?.filter((c) => !c.archived_at).length : active.data?.length) &&
            ghost((pid ? cycles : active).isError ? "Couldn't be read" : !(pid ? cycles : active).data ? "Reading…" : pid ? "No cycles in this project" : "No cycle is running today — choose a project to see all of its cycles")}
        </div>
      )}
      {tab === "modules" && (
        <div className={styles.group}>
          {(modules.data ?? []).map((m) => (
            <div key={m.id} className={styles.row}>
              <span className={styles.rowTitle}>{m.name}</span>
              <span className={styles.rowProps}>
                <span className="fieldHint">
                  {m.done} of {m.items - m.cancelled} done
                </span>
                <span className={styles.progress}>
                  <span style={{ width: `${m.items - m.cancelled > 0 ? Math.round((m.done / (m.items - m.cancelled)) * 100) : 0}%` }} />
                </span>
              </span>
            </div>
          ))}
          {!modules.data?.length && ghost(!pid ? "Choose a project to see its modules" : modules.isError ? "Couldn't be read" : modules.data ? "No modules in this project" : "Reading…")}
        </div>
      )}
    </div>
  );
}
