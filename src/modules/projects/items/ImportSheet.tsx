import { useEffect, useRef, useState } from "react";
import { previewPmImport, useImportPmItems, type PmImportPreview, type PmImportSummary } from "../../../kernel/bridge/pmAnalytics";
import { pickSpreadsheet } from "../../../kernel/bridge/transfer";
import styles from "./Items.module.css";

/** Plane's CSV importer (D-083): a file (Plane's export, CSV or a workbook), what it holds —
 *  Plane's columns found, the rows, what won't come in as it says — then Import, and its summary. */
export default function ImportSheet({ slug, projectId, projectName, onClose }: { slug: string; projectId: string; projectName: string; onClose: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const run = useImportPmItems(slug);
  const [path, setPath] = useState<string | null>(null);
  const [preview, setPreview] = useState<PmImportPreview | null>(null);
  const [summary, setSummary] = useState<PmImportSummary | null>(null);
  const [trouble, setTrouble] = useState<string | null>(null);
  // The file picked last: a slow preview of an earlier one doesn't land.
  const latest = useRef<string | null>(null);
  useEffect(() => {
    dialog.current?.showModal();
  }, []);
  // Not closed mid-import (its summary would be lost).
  const close = () => !run.isPending && onClose();
  const pick = () =>
    void pickSpreadsheet().then(
      (p) => {
        if (!p) return;
        setPath(p);
        setPreview(null);
        setSummary(null);
        setTrouble(null);
        latest.current = p;
        previewPmImport(slug, projectId, p).then(
          (v) => latest.current === p && setPreview(v),
          (e: Error) => latest.current === p && setTrouble(e.message),
        );
      },
      (e: Error) => setTrouble(e.message),
    );
  const problems = summary?.problems ?? preview?.problems ?? [];
  return (
    <dialog
      ref={dialog}
      className={`glass sheet ${styles.create}`}
      aria-labelledby="import-sheet-title"
      onCancel={(e) => {
        e.preventDefault();
        close();
      }}
      onPointerDown={(e) => e.target === e.currentTarget && close()}
    >
      <div className="sheetBody">
        <h2 id="import-sheet-title" className="sheetHeading">
          Import work items into {projectName}
        </h2>
        <p className="fieldHint">A CSV as Plane exports it (or a workbook): Name, and any of Description, State Name, Priority, Assignees, Labels, dates, Estimate, Cycle Name, Modules, Parent, Links and Comments.</p>
        <div className={styles.inline}>
          {/* One primary at a time: the file until it's read, then Import. */}
          <button type="button" className={preview ? "btn btnSecondary btnCompact" : "btn btnPrimary"} disabled={run.isPending} onClick={pick}>
            {path ? "Choose another file…" : "Choose a file…"}
          </button>
          {path && <span className="fieldHint">{path.split("/").pop()}</span>}
        </div>
        {preview && !summary && (
          <p className="fieldHint">
            {preview.rows} row{preview.rows === 1 ? "" : "s"} · {preview.columns.length} of Plane's columns ({preview.columns.join(", ")})
            {preview.ignored.length ? ` · left aside: ${preview.ignored.join(", ")}` : ""}
          </p>
        )}
        {summary && (
          <p className="fieldHint" role="status">
            {summary.created} work item{summary.created === 1 ? "" : "s"} made{summary.skipped ? `, ${summary.skipped} skipped` : ""}.
          </p>
        )}
        {problems.length > 0 && (
          <ul className={styles.importProblems} aria-label="What won't come in as it says">
            {problems.slice(0, 50).map((p, n) => (
              <li key={n}>
                {p.row ? `Row ${p.row}` : "The file"}: {p.message}
              </li>
            ))}
            {problems.length > 50 && <li>…and {problems.length - 50} more</li>}
          </ul>
        )}
        {(trouble ?? run.error?.message) && (
          <p className="formError" role="alert">
            {trouble ?? run.error?.message}
          </p>
        )}
        <div className="sheetActions">
          <button type="button" className="btn btnGhost" disabled={run.isPending} onClick={close}>
            {summary ? "Close" : "Cancel"}
          </button>
          {!summary && (
            <button type="button" className="btn btnPrimary" disabled={!path || !preview || run.isPending} onClick={() => path && run.mutate({ projectId, path }, { onSuccess: setSummary })}>
              {run.isPending ? "Importing…" : preview ? `Import ${preview.rows} work item${preview.rows === 1 ? "" : "s"}` : "Import"}
            </button>
          )}
        </div>
      </div>
    </dialog>
  );
}
