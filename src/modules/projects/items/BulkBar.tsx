import { useState } from "react";
import { usePmEstimates, usePmProjects, usePmStates, type PmProject, type StateGroup } from "../../../kernel/bridge/pm";
import { usePmCycles } from "../../../kernel/bridge/pmCycles";
import { useBulkPmItems, type ItemPatch } from "../../../kernel/bridge/pmItems";
import DangerButton from "../../../kernel/shell/DangerButton";
import { GROUP_LABEL, PRIORITIES } from "./items";
import { LabelPick, PeoplePick } from "./Props";
import styles from "./Items.module.css";

/** Plane's bulk operations (D-075) under the List: the properties to change — each left as it is
 *  until chosen — applied by Update; a copy into a project; Subscribe, Archive (completed or
 *  cancelled items only), Delete. Every item is checked first: a refusal names the item and the
 *  rule, and nothing is written. */
export default function BulkBar({ slug, project, ids, onClear }: { slug: string; project: PmProject; ids: string[]; onClear: () => void }) {
  const bulk = useBulkPmItems(slug);
  const states = (usePmStates(slug, project.id).data ?? []).filter((s) => s.group !== "triage");
  const groups = [...new Set(states.map((s) => s.group))] as StateGroup[];
  const estimate = (usePmEstimates(slug, project.id).data ?? []).find((e) => e.active);
  const cycles = (usePmCycles(slug, project.id).data ?? []).filter((c) => c.status !== "completed");
  const projects = (usePmProjects(slug).data ?? []).filter((p) => !p.archived_at);
  const [patch, setPatch] = useState<ItemPatch>({});
  const set = (p: ItemPatch) => setPatch({ ...patch, ...p });
  const busy = bulk.update.isPending || bulk.act.isPending || bulk.copy.isPending;
  const error = bulk.update.error ?? bulk.act.error ?? bulk.copy.error;
  const done = { onSuccess: onClear };
  const n = ids.length;
  return (
    <div className={`glass ${styles.bulk}`} role="toolbar" aria-label="Bulk changes">
      <div className={styles.inline}>
        <strong className={styles.bulkCount}>{n} selected</strong>
        <select className="input" aria-label="State for the selected" value={patch.state_id ?? ""} onChange={(e) => set({ state_id: e.target.value || undefined })}>
          <option value="">State</option>
          {groups.map((g) => (
            <optgroup key={g} label={GROUP_LABEL[g]}>
              {states
                .filter((s) => s.group === g)
                .map((s) => (
                  <option key={s.id} value={s.id}>
                    {s.name}
                  </option>
                ))}
            </optgroup>
          ))}
        </select>
        <select className="input" aria-label="Priority for the selected" value={patch.priority ?? ""} onChange={(e) => set({ priority: PRIORITIES.find((p) => p.key === e.target.value)?.key })}>
          <option value="">Priority</option>
          {PRIORITIES.map((p) => (
            <option key={p.key} value={p.key}>
              {p.label}
            </option>
          ))}
        </select>
        <PeoplePick slug={slug} label="Assignees" value={patch.assignees ?? []} onChange={(assignees) => set({ assignees: assignees.length ? assignees : undefined })} />
        <LabelPick slug={slug} projectId={project.id} value={patch.labels ?? []} onChange={(labels) => set({ labels: labels.length ? labels : undefined })} />
        <label className={styles.date}>
          <span className="fieldLabel">Start</span>
          <input className="input" type="date" value={patch.start_date ?? ""} onChange={(e) => set({ start_date: e.target.value || undefined })} />
        </label>
        <label className={styles.date}>
          <span className="fieldLabel">Due</span>
          <input className="input" type="date" value={patch.target_date ?? ""} onChange={(e) => set({ target_date: e.target.value || undefined })} />
        </label>
        {estimate && (
          <select className="input" aria-label="Estimate for the selected" value={patch.estimate_point_id == null ? "" : patch.estimate_point_id || "none"} onChange={(e) => set({ estimate_point_id: e.target.value === "none" ? "" : e.target.value || undefined })}>
            <option value="">Estimate</option>
            {estimate.points.map((p) => (
              <option key={p.id} value={p.id}>
                {p.value}
              </option>
            ))}
            <option value="none">No estimate</option>
          </select>
        )}
        {project.features.cycles && cycles.length > 0 && (
          <select className="input" aria-label="Cycle for the selected" value={patch.cycle_id == null ? "" : patch.cycle_id || "none"} onChange={(e) => set({ cycle_id: e.target.value === "none" ? "" : e.target.value || undefined })}>
            <option value="">Cycle</option>
            {cycles.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
            <option value="none">No cycle</option>
          </select>
        )}
        <button type="button" className="btn btnPrimary btnCompact" disabled={busy || !Object.values(patch).some((v) => v !== undefined)} onClick={() => bulk.update.mutate({ ids, patch }, { onSuccess: () => (setPatch({}), onClear()) })}>
          {bulk.update.isPending ? "Updating…" : "Update"}
        </button>
      </div>
      <div className={styles.inline}>
        <select className="input" aria-label="Make a copy of the selected in a project" value="" disabled={busy} onChange={(e) => e.target.value && bulk.copy.mutate({ ids, projectId: e.target.value }, done)}>
          <option value="">Make a copy in…</option>
          {projects.map((p) => (
            <option key={p.id} value={p.id}>
              {p.id === project.id ? `This project (${p.identifier})` : `${p.identifier} · ${p.name}`}
            </option>
          ))}
        </select>
        <button type="button" className="btn btnGhost btnCompact" disabled={busy} onClick={() => bulk.act.mutate({ ids, action: "subscribe" }, done)}>
          Subscribe
        </button>
        <button type="button" className="btn btnGhost btnCompact" disabled={busy} onClick={() => bulk.act.mutate({ ids, action: "archive" }, done)}>
          Archive
        </button>
        <DangerButton confirm={`Delete ${n} work item${n === 1 ? "" : "s"} with their sub-work items?`} disabled={busy} onConfirm={() => bulk.act.mutate({ ids, action: "delete" }, done)}>
          Delete
        </DangerButton>
        <span className={styles.spacer} />
        <button type="button" className="btn btnGhost btnCompact" onClick={onClear}>
          Clear selection
        </button>
      </div>
      {error && (
        <p className="formError" role="alert">
          {error.message}
        </p>
      )}
    </div>
  );
}
