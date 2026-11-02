import { useEffect, useRef, useState } from "react";
import { usePmProjects } from "../../../kernel/bridge/pm";
import { useCreatePmItem, useDraftMutations, type ItemInput, type PmDraft, type PmItem } from "../../../kernel/bridge/pmItems";
import { richPlain } from "../../../kernel/editor/plain";
import Editor from "../../../kernel/editor/Editor";
import { CyclePick, EstimatePick, LabelPick, ModulePick, ParentPick, PeoplePick, PriorityPick, StatePick } from "./Props";
import styles from "./Items.module.css";

const blank = (projectId: string, preset?: Partial<ItemInput>): ItemInput => ({
  project_id: projectId,
  name: "",
  description: null,
  state_id: null,
  priority: "none",
  assignees: [],
  labels: [],
  start_date: null,
  target_date: null,
  estimate_point_id: null,
  parent_id: null,
  modules: [],
  ...preset,
});

/** A picture needs the item's own folder: it can go in once the item is made. */
const NO_FILES = {
  add: async () => {
    throw new Error("Pictures go in once the work item is made");
  },
  addBytes: async () => {
    throw new Error("Pictures go in once the work item is made");
  },
};

/** Plane's create modal (D-074): the project, the title, the description, every property;
 *  Create more keeps it open for the next; closing it with something written asks to keep it as
 *  a draft. With a draft, it edits the draft — Save draft, or Create. */
export default function CreateItem({ slug, projectId, preset, draft, onClose, onCreated }: { slug: string; projectId: string; preset?: Partial<ItemInput>; draft?: PmDraft; onClose: () => void; onCreated?: (item: PmItem) => void }) {
  const projects = (usePmProjects(slug).data ?? []).filter((p) => !p.archived_at);
  const create = useCreatePmItem(slug);
  const drafts = useDraftMutations(slug);
  const dialog = useRef<HTMLDialogElement>(null);
  const [input, setInput] = useState<ItemInput>(draft ? { ...blank(projectId), ...draft.input, project_id: draft.input.project_id || projectId } : blank(projectId, preset));
  // A draft is asked about only when it was changed here.
  const opened = useRef(JSON.stringify(input));
  const [more, setMore] = useState(false);
  const [round, setRound] = useState(0);
  const [asking, setAsking] = useState(false);
  const busy = create.isPending || drafts.save.isPending || drafts.publish.isPending;
  const error = create.error ?? drafts.save.error ?? drafts.publish.error;
  const set = (patch: Partial<ItemInput>) => setInput({ ...input, ...patch });
  const cyclesOn = !!projects.find((p) => p.id === input.project_id)?.features.cycles;
  const modulesOn = !!projects.find((p) => p.id === input.project_id)?.features.modules;
  const written = draft ? JSON.stringify(input) !== opened.current : !!input.name.trim() || !!richPlain(input.description ?? "").trim();
  useEffect(() => {
    dialog.current?.showModal();
  }, []);

  /** Escape, the backdrop, Cancel: with something written, asked first; while asked, back to it. */
  const close = () => {
    if (busy) return;
    if (asking) {
      setAsking(false);
      return;
    }
    if (written) {
      setAsking(true);
      return;
    }
    onClose();
  };
  const made = (item: PmItem) => {
    onCreated?.(item);
    if (more) {
      // The next one: the same project and properties, a new title and description.
      setInput({ ...input, name: "", description: null });
      setRound(round + 1);
    } else {
      onClose();
    }
  };
  const submit = () => (draft ? drafts.publish.mutate({ id: draft.id, input }, { onSuccess: made }) : create.mutate(input, { onSuccess: made }));
  const keep = () => drafts.save.mutate({ id: draft?.id ?? null, input }, { onSuccess: onClose });

  return (
    <dialog
      ref={dialog}
      className={`glass sheet ${styles.create}`}
      aria-labelledby="create-item-title"
      onCancel={(e) => {
        e.preventDefault();
        close();
      }}
      onPointerDown={(e) => e.target === e.currentTarget && close()}
    >
      <div className="sheetBody">
        <h2 id="create-item-title" className="sheetHeading">
          {draft ? "Draft" : input.parent_id ? "New sub-work item" : "New work item"}
        </h2>
        {asking ? (
          <div className={styles.ask} role="alertdialog" aria-labelledby="keep-draft">
            <p id="keep-draft" className={styles.askTitle}>
              {draft ? "Save the changes to this draft?" : "Keep it as a draft?"}
            </p>
            <p className="fieldHint">{draft ? "Discard leaves the draft as it was." : "What you wrote waits under Drafts, to finish later."}</p>
            <div className="sheetActions">
              <button type="button" className="btn btnGhost" onClick={onClose}>
                Discard
              </button>
              <button type="button" className="btn btnSecondary" onClick={() => setAsking(false)}>
                Keep editing
              </button>
              <button type="button" className="btn btnPrimary" disabled={busy} onClick={keep}>
                Save draft
              </button>
            </div>
          </div>
        ) : (
          <>
            <label className="field">
              <span className="fieldLabel">Project</span>
              <select className="input" value={input.project_id} onChange={(e) => set({ project_id: e.target.value, state_id: null, labels: [], estimate_point_id: null, parent_id: null, cycle_id: null, modules: [] })}>
                {projects.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.identifier} · {p.name}
                  </option>
                ))}
              </select>
            </label>
            <label className="field">
              <span className="fieldLabel">Title</span>
              <input key={round} className="input" autoFocus value={input.name} maxLength={255} placeholder="What needs doing" onChange={(e) => set({ name: e.target.value })} />
            </label>
            <div className="field">
              <span className="fieldLabel">Description</span>
              <div className={styles.editorBox}>
                <Editor key={`${round}-${input.project_id}`} value={input.description ?? null} placeholder="Add a description…" home={{ slug, object: "pm-items", recordId: "", files: NO_FILES }} onChange={(json) => set({ description: json })} />
              </div>
            </div>
            <div className={styles.props}>
              <StatePick slug={slug} projectId={input.project_id} value={input.state_id ?? null} onChange={(state_id) => set({ state_id })} />
              <PriorityPick value={input.priority ?? "none"} onChange={(priority) => set({ priority })} />
              <PeoplePick slug={slug} label="Assignees" value={input.assignees} onChange={(assignees) => set({ assignees })} />
              <LabelPick slug={slug} projectId={input.project_id} value={input.labels} onChange={(labels) => set({ labels })} />
              <label className={styles.date}>
                <span className="fieldLabel">Start</span>
                <input className="input" type="date" value={input.start_date ?? ""} onChange={(e) => set({ start_date: e.target.value || null })} />
              </label>
              <label className={styles.date}>
                <span className="fieldLabel">Due</span>
                <input className="input" type="date" value={input.target_date ?? ""} onChange={(e) => set({ target_date: e.target.value || null })} />
              </label>
              <EstimatePick slug={slug} projectId={input.project_id} value={input.estimate_point_id ?? null} onChange={(id) => set({ estimate_point_id: id || null })} />
              <ParentPick slug={slug} projectId={input.project_id} value={input.parent_id ?? null} onChange={(id) => set({ parent_id: id || null })} />
              {cyclesOn && <CyclePick slug={slug} projectId={input.project_id} value={input.cycle_id ?? null} onChange={(id) => set({ cycle_id: id || null })} />}
              {modulesOn && <ModulePick slug={slug} projectId={input.project_id} value={input.modules ?? []} onChange={(modules) => set({ modules })} />}
            </div>
            {error && (
              <p className="formError" role="alert">
                {error.message}
              </p>
            )}
            <div className="sheetActions">
              {!draft && (
                <label className={styles.more}>
                  <input type="checkbox" checked={more} onChange={(e) => setMore(e.target.checked)} /> Create more
                </label>
              )}
              <button type="button" className="btn btnGhost" disabled={busy} onClick={close}>
                {draft ? "Close" : "Cancel"}
              </button>
              {draft && (
                <button type="button" className="btn btnSecondary" disabled={busy} onClick={keep}>
                  Save draft
                </button>
              )}
              <button type="button" className="btn btnPrimary" disabled={busy || !input.name.trim() || !input.project_id} onClick={submit}>
                {create.isPending || drafts.publish.isPending ? "Creating…" : "Create"}
              </button>
            </div>
          </>
        )}
      </div>
    </dialog>
  );
}
