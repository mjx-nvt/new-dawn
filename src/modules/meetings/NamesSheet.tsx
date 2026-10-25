import { useEffect, useRef, useState } from "react";
import type { NameDecision, NameSuspect } from "../../bindings";
import { useConfirmNames } from "../../kernel/bridge/summary";
import { SparkleIcon } from "../../kernel/shell/icons";
import styles from "./NamesSheet.module.css";

interface Row extends NameDecision {
  reason: string;
  context: string;
  suggestion: string;
  /** confirm = use `name`; keep = leave the spelling as heard. */
  action: "confirm" | "keep";
}

const toRow = (s: NameSuspect): Row => ({
  heard_as: s.heard_as,
  name: s.suggestion,
  title: s.title,
  organisation: s.organisation,
  remember: true,
  reason: s.reason,
  context: s.context,
  suggestion: s.suggestion,
  action: "confirm",
});

/** The naming agent's doubts, one row each: heard as → use this name (editable), why, where it
 *  was said; Confirm or Keep. Applying renames across the summary, email and minutes and
 *  teaches the people memory — no AI run. */
export default function NamesSheet({ slug, meetingId, suspects, onClose }: { slug: string; meetingId: string; suspects: NameSuspect[]; onClose: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const confirm = useConfirmNames(slug, meetingId);
  const [rows, setRows] = useState<Row[]>(() => suspects.map(toRow));
  const set = (i: number, patch: Partial<Row>) => setRows((rs) => rs.map((r, j) => (j === i ? { ...r, ...patch } : r)));
  const close = () => !confirm.isPending && dialog.current?.close();

  useEffect(() => {
    dialog.current?.showModal();
  }, []);

  return (
    <dialog
      ref={dialog}
      className={`glass sheet ${styles.sheet}`}
      aria-labelledby="names-title"
      onClose={onClose}
      onCancel={(e) => confirm.isPending && e.preventDefault()}
      onPointerDown={(e) => e.target === e.currentTarget && close()}
    >
      <div className="sheetBody">
        <div>
          <h2 id="names-title" className={`sheetHeading aiMark ${styles.title}`}>
            <SparkleIcon />
            Names to confirm
          </h2>
          <p className="fieldHint">The naming agent wasn't sure about these. Confirmed names are fixed across the summary, the email and the minutes.</p>
        </div>

        <div className={styles.list}>
          {rows.map((r, i) => (
            <div key={r.heard_as} className={styles.row}>
              <div className={styles.heard}>
                <span className="fieldLabel">Heard as</span>
                <span className={styles.heardName}>{r.heard_as}</span>
                <span className="fieldHint">{r.reason}</span>
                {r.context && <span className={`fieldHint ${styles.context}`}>“{r.context}”</span>}
              </div>
              <div className={styles.fields}>
                <input className="input" aria-label="Name to use" placeholder="Name" value={r.name} disabled={r.action === "keep"} onChange={(e) => set(i, { name: e.target.value })} />
                <input className="input" aria-label="Job title" placeholder="Job title" value={r.title} onChange={(e) => set(i, { title: e.target.value })} />
                <input className="input" aria-label="Organisation" placeholder="Organisation" value={r.organisation} onChange={(e) => set(i, { organisation: e.target.value })} />
              </div>
              <div className="segmented" role="group" aria-label={`What to do with ${r.heard_as}`}>
                <button type="button" aria-pressed={r.action === "confirm"} onClick={() => set(i, { action: "confirm", name: r.name === r.heard_as ? r.suggestion : r.name })}>
                  Use this
                </button>
                <button type="button" aria-pressed={r.action === "keep"} onClick={() => set(i, { action: "keep", name: r.heard_as })}>
                  Keep as heard
                </button>
              </div>
            </div>
          ))}
        </div>

        {confirm.error && (
          <p className="formError" role="alert">
            {confirm.error.message}
          </p>
        )}
        <div className={styles.footer}>
          <label className={styles.remember}>
            <input type="checkbox" checked={rows.every((r) => r.remember)} onChange={(e) => setRows(rows.map((r) => ({ ...r, remember: e.target.checked })))} />
            Remember these people
          </label>
          <span className={styles.grow} />
          <button type="button" className="btn btnGhost" onClick={close}>
            Later
          </button>
          <button
            type="button"
            className="btn btnPrimary"
            disabled={confirm.isPending || rows.some((r) => r.action === "confirm" && !r.name.trim())}
            onClick={() =>
              confirm.mutate(
                rows.map((r) => ({
                  heard_as: r.heard_as,
                  name: r.action === "keep" ? r.heard_as : r.name,
                  title: r.title,
                  organisation: r.organisation,
                  remember: r.remember,
                })),
                { onSuccess: () => dialog.current?.close() },
              )
            }
          >
            {confirm.isPending ? "Applying…" : "Apply"}
          </button>
        </div>
      </div>
    </dialog>
  );
}
