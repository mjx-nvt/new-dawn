import { useEffect, useRef, useState } from "react";
import { useCycleMutations, type PmCycle } from "../../../kernel/bridge/pmCycles";
import styles from "./Items.module.css";

/** A new cycle, or a cycle's edits (D-078): a name, a description, both dates or none (a draft). */
export default function CycleSheet({ slug, projectId, cycle, onClose, onSaved }: { slug: string; projectId: string; cycle?: PmCycle; onClose: () => void; onSaved?: (cycle: PmCycle) => void }) {
  const m = useCycleMutations(slug);
  const dialog = useRef<HTMLDialogElement>(null);
  const [name, setName] = useState(cycle?.name ?? "");
  const [description, setDescription] = useState(cycle?.description ?? "");
  const [start, setStart] = useState(cycle?.start_date ?? "");
  const [end, setEnd] = useState(cycle?.end_date ?? "");
  useEffect(() => {
    dialog.current?.showModal();
  }, []);
  const busy = m.create.isPending || m.update.isPending;
  const error = m.create.error ?? m.update.error;
  const done = (c: PmCycle) => {
    onSaved?.(c);
    onClose();
  };
  const save = () => {
    if (!name.trim()) return;
    // The dates only when changed (a completed cycle keeps them, and is still renamed).
    const moved = start !== (cycle?.start_date ?? "") || end !== (cycle?.end_date ?? "");
    if (cycle) m.update.mutate({ id: cycle.id, patch: { name, description, ...(moved ? { start_date: start, end_date: end } : {}) } }, { onSuccess: done });
    else m.create.mutate({ project_id: projectId, name, description, start_date: start || null, end_date: end || null }, { onSuccess: done });
  };
  return (
    <dialog
      ref={dialog}
      className={`glass sheet ${styles.create}`}
      aria-labelledby="cycle-sheet-title"
      onCancel={(e) => {
        e.preventDefault();
        onClose();
      }}
      onPointerDown={(e) => e.target === e.currentTarget && onClose()}
    >
      <div className="sheetBody">
        <h2 id="cycle-sheet-title" className="sheetHeading">
          {cycle ? "Edit cycle" : "New cycle"}
        </h2>
        <label className="field">
          <span className="fieldLabel">Name</span>
          <input className="input" autoFocus maxLength={255} placeholder="Sprint 12" value={name} onChange={(e) => setName(e.target.value)} onKeyDown={(e) => e.key === "Enter" && !e.nativeEvent.isComposing && save()} />
        </label>
        <label className="field">
          <span className="fieldLabel">Description</span>
          <input className="input" placeholder="What it's for" value={description} onChange={(e) => setDescription(e.target.value)} />
        </label>
        <div className={styles.props}>
          <label className={styles.date}>
            <span className="fieldLabel">Start</span>
            <input className="input" type="date" value={start} disabled={cycle?.status === "completed"} onChange={(e) => setStart(e.target.value)} />
          </label>
          <label className={styles.date}>
            <span className="fieldLabel">End</span>
            <input className="input" type="date" value={end} disabled={cycle?.status === "completed"} onChange={(e) => setEnd(e.target.value)} />
          </label>
        </div>
        <p className="fieldHint">Both dates, or none for a draft. A project's cycles never overlap.</p>
        {error && (
          <p className="formError" role="alert">
            {error.message}
          </p>
        )}
        <div className="sheetActions">
          <button type="button" className="btn btnGhost" onClick={onClose}>
            Cancel
          </button>
          <button type="button" className="btn btnPrimary" disabled={busy || !name.trim()} onClick={save}>
            {cycle ? "Save" : "Create cycle"}
          </button>
        </div>
      </div>
    </dialog>
  );
}
