import { Link } from "@tanstack/react-router";
import { useState } from "react";
import { usePmProjects } from "../../../kernel/bridge/pm";
import { useDraftMutations, usePmDrafts, type PmDraft } from "../../../kernel/bridge/pmItems";
import { useCurrentWorkspace } from "../../../kernel/bridge/workspaces";
import { formatDateTime } from "../../../kernel/format/format";
import DangerButton from "../../../kernel/shell/DangerButton";
import CreateItem from "./CreateItem";
import styles from "./Items.module.css";

/** Drafts (D-074, Plane's): work items not made yet, across the workspace's projects — edited
 *  (then saved or made), copied, moved to another project, deleted. */
export default function Drafts() {
  const { data: workspace } = useCurrentWorkspace();
  const slug = workspace?.slug;
  const drafts = usePmDrafts(slug);
  const projects = (usePmProjects(slug).data ?? []).filter((p) => !p.archived_at);
  const m = useDraftMutations(slug);
  const [editing, setEditing] = useState<PmDraft | null>(null);
  const list = drafts.data ?? [];
  const error = drafts.error ?? m.copy.error ?? m.remove.error ?? m.save.error;
  return (
    <div className="page">
      <p className="eyebrow">{workspace?.name ?? "No workspace"}</p>
      <h1 className="pageTitle">Drafts</h1>
      <p className={`fieldHint ${styles.lead}`}>
        Work items you started and kept for later. <Link to="/projects">Projects</Link>
      </p>
      {error && <p className="formError">{error.message}</p>}
      <div className={styles.group}>
        {list.map((d) => (
          <div key={d.id} className={styles.row}>
            <button type="button" className={styles.rowTitle} onClick={() => setEditing(d)}>
              {d.input.name || <span className="ghostText">Untitled</span>}
            </button>
            <span className={styles.rowProps}>
              <select className="input" aria-label={`Move “${d.input.name}” to a project`} value={d.project_id ?? ""} disabled={m.save.isPending} onChange={(e) => e.target.value && m.save.mutate({ id: d.id, input: { ...d.input, project_id: e.target.value, state_id: null, labels: [], estimate_point_id: null, parent_id: null } })}>
                {!d.project_id && <option value="">No project</option>}
                {projects.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.identifier} · {p.name}
                  </option>
                ))}
              </select>
              <span className="fieldHint">{formatDateTime(d.updated_at)}</span>
              <button type="button" className="btn btnGhost btnCompact" disabled={m.copy.isPending} onClick={() => m.copy.mutate(d.id)}>
                Copy
              </button>
              <DangerButton confirm="Delete this draft?" disabled={m.remove.isPending} onConfirm={() => m.remove.mutate(d.id)}>
                Delete
              </DangerButton>
            </span>
          </div>
        ))}
        {!list.length &&
          [0, 1].map((n) => (
            <div key={n} className={`${styles.row} ghostCard`}>
              {n === 0 && <span className="ghostText">{drafts.isError ? "Couldn't be read" : drafts.data ? "No drafts — closing a new work item with something written keeps it here" : "Reading…"}</span>}
            </div>
          ))}
      </div>
      {editing && slug && <CreateItem slug={slug} projectId={editing.project_id ?? projects[0]?.id ?? ""} draft={editing} onClose={() => setEditing(null)} />}
    </div>
  );
}
