import { useEffect, useRef, useState } from "react";
import type { PmDisplay } from "../../../kernel/bridge/pmItems";
import { useViewMutations, type PmRule, type PmView } from "../../../kernel/bridge/pmViews";
import styles from "./Items.module.css";

/** Saving as a view (D-077): a name, a description and Private — with the filters, the layout and
 *  the display as they stand. A project's view, or the workspace's (no project). */
export default function ViewSheet({ slug, projectId, rules, display, onClose, onSaved }: { slug: string; projectId: string | null; rules: PmRule[]; display: PmDisplay; onClose: () => void; onSaved: (view: PmView) => void }) {
  const m = useViewMutations(slug);
  const dialog = useRef<HTMLDialogElement>(null);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [isPrivate, setPrivate] = useState(false);
  useEffect(() => {
    dialog.current?.showModal();
  }, []);
  const save = () => name.trim() && m.create.mutate({ project_id: projectId, name, description, rules, display, is_private: isPrivate }, { onSuccess: onSaved });
  return (
    <dialog
      ref={dialog}
      className={`glass sheet ${styles.create}`}
      aria-labelledby="view-sheet-title"
      onCancel={(e) => {
        e.preventDefault();
        onClose();
      }}
      onPointerDown={(e) => e.target === e.currentTarget && onClose()}
    >
      <div className="sheetBody">
        <h2 id="view-sheet-title" className="sheetHeading">
          New view
        </h2>
        <p className="fieldHint">
          {rules.length ? `${rules.length} filter${rules.length === 1 ? "" : "s"}` : "No filters"} · the {display.layout} layout as it is now{projectId ? "" : " · across the workspace's projects"}.
        </p>
        <label className="field">
          <span className="fieldLabel">Name</span>
          <input className="input" autoFocus maxLength={255} placeholder="Mine this week" value={name} onChange={(e) => setName(e.target.value)} onKeyDown={(e) => e.key === "Enter" && !e.nativeEvent.isComposing && save()} />
        </label>
        <label className="field">
          <span className="fieldLabel">Description</span>
          <input className="input" placeholder="What it shows" value={description} onChange={(e) => setDescription(e.target.value)} />
        </label>
        <label className={styles.check}>
          <input type="checkbox" checked={isPrivate} onChange={(e) => setPrivate(e.target.checked)} /> Private — a label for you; nothing here is shared
        </label>
        {m.create.error && (
          <p className="formError" role="alert">
            {m.create.error.message}
          </p>
        )}
        <div className="sheetActions">
          <button type="button" className="btn btnGhost" onClick={onClose}>
            Cancel
          </button>
          <button type="button" className="btn btnPrimary" disabled={!name.trim() || m.create.isPending} onClick={save}>
            Save view
          </button>
        </div>
      </div>
    </dialog>
  );
}
