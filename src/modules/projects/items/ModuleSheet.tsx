import { useEffect, useRef, useState } from "react";
import { useModuleMutations, type ModuleStatus, type PmModule } from "../../../kernel/bridge/pmModules";
import { PeoplePick, PersonPick } from "./Props";
import { MODULE_STATUS } from "./items";
import styles from "./Items.module.css";

/** A new module, or a module's edits (D-079): a name, a description, its status, both dates or
 *  either, a lead and members. */
export default function ModuleSheet({ slug, projectId, module: m, onClose }: { slug: string; projectId: string; module?: PmModule; onClose: () => void }) {
  const mutations = useModuleMutations(slug);
  const dialog = useRef<HTMLDialogElement>(null);
  const [name, setName] = useState(m?.name ?? "");
  const [description, setDescription] = useState(m?.description ?? "");
  const [status, setStatus] = useState<ModuleStatus>(m?.status ?? "backlog");
  const [start, setStart] = useState(m?.start_date ?? "");
  const [target, setTarget] = useState(m?.target_date ?? "");
  const [lead, setLead] = useState(m?.lead ?? "");
  const [members, setMembers] = useState<string[]>(m?.members ?? []);
  useEffect(() => {
    dialog.current?.showModal();
  }, []);
  const busy = mutations.create.isPending || mutations.update.isPending;
  const error = mutations.create.error ?? mutations.update.error;
  const save = () => {
    if (!name.trim()) return;
    const done = { onSuccess: onClose };
    if (!m) {
      mutations.create.mutate({ project_id: projectId, name, description, status, start_date: start || null, target_date: target || null, lead: lead || null, members }, done);
      return;
    }
    // The dates only when changed ("" clears one).
    const moved = start !== (m.start_date ?? "") || target !== (m.target_date ?? "");
    mutations.update.mutate({ id: m.id, patch: { name, description, status, lead, members, ...(moved ? { start_date: start, target_date: target } : {}) } }, done);
  };
  return (
    <dialog
      ref={dialog}
      className={`glass sheet ${styles.create}`}
      aria-labelledby="module-sheet-title"
      onCancel={(e) => {
        e.preventDefault();
        onClose();
      }}
      onPointerDown={(e) => e.target === e.currentTarget && onClose()}
    >
      <div className="sheetBody">
        <h2 id="module-sheet-title" className="sheetHeading">
          {m ? "Edit module" : "New module"}
        </h2>
        <label className="field">
          <span className="fieldLabel">Name</span>
          <input className="input" autoFocus maxLength={255} placeholder="Sandbox licence" value={name} onChange={(e) => setName(e.target.value)} onKeyDown={(e) => e.key === "Enter" && !e.nativeEvent.isComposing && save()} />
        </label>
        <label className="field">
          <span className="fieldLabel">Description</span>
          <input className="input" placeholder="What it delivers" value={description} onChange={(e) => setDescription(e.target.value)} />
        </label>
        <div className={styles.props}>
          <label className={styles.date}>
            <span className="fieldLabel">Status</span>
            <select className="input" value={status} onChange={(e) => setStatus(MODULE_STATUS.find((s) => s.key === e.target.value)?.key ?? status)}>
              {MODULE_STATUS.map((s) => (
                <option key={s.key} value={s.key}>
                  {s.label}
                </option>
              ))}
            </select>
          </label>
          <label className={styles.date}>
            <span className="fieldLabel">Start</span>
            <input className="input" type="date" value={start} onChange={(e) => setStart(e.target.value)} />
          </label>
          <label className={styles.date}>
            <span className="fieldLabel">Target</span>
            <input className="input" type="date" value={target} onChange={(e) => setTarget(e.target.value)} />
          </label>
          <label className={styles.date}>
            <span className="fieldLabel">Lead</span>
            <PersonPick slug={slug} label="Lead" value={lead || null} onChange={setLead} />
          </label>
        </div>
        <div className="field">
          <span className="fieldLabel">Members</span>
          <PeoplePick slug={slug} label="Members" value={members} onChange={setMembers} />
        </div>
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
            {m ? "Save" : "Create module"}
          </button>
        </div>
      </div>
    </dialog>
  );
}
