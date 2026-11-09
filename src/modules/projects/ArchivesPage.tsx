import { useQueryClient } from "@tanstack/react-query";
import { getRouteApi, Link, useNavigate } from "@tanstack/react-router";
import { useArchivePmProject } from "../../kernel/bridge/pm";
import { useCycleMutations } from "../../kernel/bridge/pmCycles";
import { usePmArchives, type PmArchived as Archived } from "../../kernel/bridge/pmHome";
import { useArchivePmItem } from "../../kernel/bridge/pmItems";
import { useModuleMutations } from "../../kernel/bridge/pmModules";
import { usePageMutations } from "../../kernel/bridge/pmPages";
import { useCurrentWorkspace } from "../../kernel/bridge/workspaces";
import { formatDateTime } from "../../kernel/format/format";
import styles from "./items/Items.module.css";
import PageHeader from "../../kernel/shell/PageHeader";

const route = getRouteApi("/shell/projects/archives");
const TABS = [
  { key: "projects", label: "Projects" },
  { key: "items", label: "Work items" },
  { key: "cycles", label: "Cycles" },
  { key: "modules", label: "Modules" },
  { key: "pages", label: "Pages" },
] as const;
type Kind = (typeof TABS)[number]["key"];

/** Plane's Archives (D-082, `g r`): what's archived across the projects — projects, and the open
 *  projects' work items, cycles, modules and pages — each restored from here or opened where it
 *  lives. */
export default function ArchivesPage() {
  const search = route.useSearch();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { data: workspace } = useCurrentWorkspace();
  const slug = workspace?.slug;
  const archives = usePmArchives(slug);
  const project = useArchivePmProject(slug);
  const item = useArchivePmItem(slug);
  const cycle = useCycleMutations(slug).archive;
  const module = useModuleMutations(slug).archive;
  const page = usePageMutations(slug).archive;
  const tab: Kind = TABS.find((t) => t.key === search.atab)?.key ?? "projects";
  const list: Archived[] = archives.data?.[tab] ?? [];
  const busy = project.isPending || item.isPending || cycle.isPending || module.isPending || page.isPending;
  const error = archives.error ?? project.error ?? item.error ?? cycle.error ?? module.error ?? page.error;
  const done = { onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["pm-archives"] }) };
  const restore = (a: Archived) => {
    const v = { id: a.id, archived: false };
    if (tab === "projects") project.mutate(v, done);
    else if (tab === "items") item.mutate(v, done);
    else if (tab === "cycles") cycle.mutate(v, done);
    else if (tab === "modules") module.mutate(v, done);
    else page.mutate(v, done);
  };
  // Where it lives: the project, or its tab opened on it.
  const where = (a: Archived) =>
    tab === "projects" ? {} : tab === "items" ? { tab: "archives" } : tab === "cycles" ? { tab: "cycles", ctab: "completed", carch: "1", cycle: a.id } : tab === "modules" ? { tab: "modules", march: "1", module: a.id } : { tab: "pages", ptab: "archived", page: a.id };
  return (
    <div className="page">
      <PageHeader eyebrow="Projects" title="Archives" meta="What's archived across your projects, to restore or look up." />
      <div className={styles.toolbar}>
        <div className="segmented" role="tablist" aria-label="Archives">
          {TABS.map((t) => (
            <button key={t.key} type="button" role="tab" aria-selected={tab === t.key} onClick={() => void navigate({ to: "/projects/archives", search: t.key === "projects" ? {} : { atab: t.key }, replace: true })}>
              {t.label} · {archives.data ? `${archives.data[t.key].length}${t.key === "items" && archives.data.items.length >= 500 ? "+" : ""}` : "…"}
            </button>
          ))}
        </div>
      </div>
      {error && <p className="formError">{error.message}</p>}
      <div className={styles.group}>
        {list.map((a) => (
          <div key={a.id} className={styles.row}>
            <Link to="/projects/$projectId" params={{ projectId: a.project_id }} search={where(a)} className={styles.rowTitle}>
              {a.label}
            </Link>
            <span className={styles.rowProps}>
              {tab !== "projects" && <span className="fieldHint">{a.project_name}</span>}
              <span className="fieldHint">Archived {formatDateTime(a.archived_at)}</span>
              <button type="button" className="btn btnSecondary btnCompact" disabled={busy} onClick={() => restore(a)}>
                Restore
              </button>
            </span>
          </div>
        ))}
        {!list.length &&
          [0, 1].map((n) => (
            <div key={n} className={`${styles.row} ghostCard`}>
              {n === 0 && <span className="ghostText">{archives.isError ? "Couldn't be read" : archives.data ? `No archived ${TABS.find((t) => t.key === tab)?.label.toLowerCase()}` : "Reading…"}</span>}
            </div>
          ))}
      </div>
    </div>
  );
}
