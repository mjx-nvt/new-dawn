import { Link } from "@tanstack/react-router";
import { usePmProjects } from "../../kernel/bridge/pm";
import { useActivePmCycles } from "../../kernel/bridge/pmCycles";
import { useCurrentWorkspace } from "../../kernel/bridge/workspaces";
import { formatDay } from "../../kernel/format/format";
import { Progress } from "./items/Cycles";
import styles from "./items/Items.module.css";
import PageHeader from "../../kernel/shell/PageHeader";

/** Plane's Active cycles (D-078): every open project's cycle running today — its project, its
 *  days left, its progress; each opens in its project's Cycles tab. */
export default function ActiveCycles() {
  const { data: workspace } = useCurrentWorkspace();
  const slug = workspace?.slug;
  const cycles = useActivePmCycles(slug);
  const projects = usePmProjects(slug).data ?? [];
  const list = cycles.data ?? [];
  const left = (end: string | null) => {
    if (!end) return "";
    const days = Math.round((new Date(`${end}T00:00`).getTime() - new Date(new Date().toDateString()).getTime()) / 86_400_000);
    return days <= 0 ? "Ends today" : `${days} day${days === 1 ? "" : "s"} left`;
  };
  return (
    <div className="page">
      <PageHeader eyebrow="Projects" title="Active cycles" meta="The cycle each project is running today." />
      {cycles.error && <p className="formError">{cycles.error.message}</p>}
      <div className={styles.group}>
        {list.map((c) => {
          const p = projects.find((x) => x.id === c.project_id);
          return (
            <div key={c.id} className={styles.row}>
              <span className={styles.key}>{p?.identifier ?? ""}</span>
              <Link to="/projects/$projectId" params={{ projectId: c.project_id }} search={{ tab: "cycles", cycle: c.id }} className={styles.rowTitle}>
                {c.name}
              </Link>
              <span className={styles.rowProps}>
                <span className="fieldHint">
                  {p?.name ?? ""} · {c.start_date ? formatDay(c.start_date, { short: true }) : ""} – {c.end_date ? formatDay(c.end_date, { short: true }) : ""} · {left(c.end_date)}
                </span>
                <Progress cycle={c} />
              </span>
            </div>
          );
        })}
        {!list.length &&
          [0, 1].map((n) => (
            <div key={n} className={`${styles.row} ghostCard`}>
              {n === 0 && <span className="ghostText">{cycles.isError ? "Couldn't be read" : cycles.data ? "No cycle is running today" : "Reading…"}</span>}
            </div>
          ))}
      </div>
    </div>
  );
}
