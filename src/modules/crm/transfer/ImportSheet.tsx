import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import type { ObjectDef } from "../../../kernel/bridge/records";
import { pickSpreadsheet, readImportFile, runImport, validateImport, type ColumnMap, type ImportFile, type ImportReport, type Preview } from "../../../kernel/bridge/transfer";
import styles from "./ImportSheet.module.css";

type Step = "file" | "map" | "check" | "done";
/** Rows drawn in the Validate data step (a file holds up to 10 000). */
const SHOWN = 500;
const STEPS: [Step, string][] = [
  ["file", "Select file"],
  ["map", "Match columns"],
  ["check", "Validate data"],
];

/** Twenty's spreadsheet import (D-065, 0.10.8): a CSV / XLSX / XLS file, its columns matched to
 *  the object's fields (a relation by id or by a field of the other object; an identifier that
 *  updates existing records), every row checked — cells edited and rows removed in place — then
 *  the import and its report. */
export default function ImportSheet({ slug, object, onClose }: { slug: string; object: ObjectDef; onClose: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const queryClient = useQueryClient();
  const name = object.meta.name_singular;
  const [step, setStep] = useState<Step>("file");
  const [file, setFile] = useState<ImportFile | null>(null);
  const [rows, setRows] = useState<string[][]>([]);
  const [mapping, setMapping] = useState<ColumnMap[]>([]);
  const [identifier, setIdentifier] = useState<string | null>(null);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [onlyErrors, setOnlyErrors] = useState(false);
  const [report, setReport] = useState<ImportReport | null>(null);
  const [busy, setBusy] = useState(false);
  const [trouble, setTrouble] = useState<string | null>(null);
  const recheck = useRef<number | null>(null);
  useEffect(() => {
    dialog.current?.showModal();
  }, []);
  const fail = (e: unknown) => setTrouble(e instanceof Error ? e.message : String(e));
  const headers = file?.table.headers ?? [];
  const fields = file?.fields ?? [];

  const pick = async () => {
    setTrouble(null);
    const path = await pickSpreadsheet();
    if (!path) return;
    setBusy(true);
    try {
      const opened = await readImportFile(slug, name, path);
      setFile(opened);
      setRows(opened.table.rows);
      setMapping(opened.mapping.map((m) => ({ header: m.header, field: m.field ?? null, sub: m.sub ?? null, match_by: m.match_by ?? null })));
      setIdentifier(opened.identifier ?? null);
      setStep("map");
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  };
  const check = async (current: string[][]) => {
    setBusy(true);
    setTrouble(null);
    try {
      setPreview(await validateImport(slug, name, { headers, rows: current }, mapping, identifier));
      setStep("check");
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  };
  /** A cell edited: the row is checked again once typing pauses. */
  const edit = (row: number, col: number, value: string) => {
    const next = rows.map((r, i) => (i === row ? r.map((c, j) => (j === col ? value : c)) : r));
    setRows(next);
    if (recheck.current) window.clearTimeout(recheck.current);
    recheck.current = window.setTimeout(() => void check(next), 600);
  };
  const removeRow = (row: number) => {
    const next = rows.filter((_, i) => i !== row);
    setRows(next);
    void check(next);
  };
  const confirm = async () => {
    if (!file) return;
    setBusy(true);
    setTrouble(null);
    try {
      setReport(await runImport(slug, name, file.file_name, { headers, rows }, mapping, identifier));
      setStep("done");
      for (const key of ["records", "record", "transfer-jobs"]) void queryClient.invalidateQueries({ queryKey: [key] });
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  };

  // The Match columns step: a select per column (the field, or a composite's part), a "match
  // by" for a relation, and the identifier that updates existing records.
  const mapped = mapping.map((m, col) => ({ m, col })).filter(({ m }) => m.field);
  const choiceOf = (fieldName: string | null | undefined) => fields.find((f) => f.field === fieldName);
  const identifierChoices = mapped.filter(({ m }) => m.field === "id" || choiceOf(m.field)?.unique || m.field === "emails" || m.field === "domainName");
  const setMap = (col: number, patch: Partial<ColumnMap>) => setMapping(mapping.map((m, i) => (i === col ? { ...m, ...patch } : m)));
  const sampleOf = (col: number) => rows.find((r) => r[col])?.[col] ?? "";

  // The Validate data step: the rows with their checks.
  const previewOf = (row: number) => preview?.rows[row];
  const shownRows = rows.map((r, i) => ({ r, i })).filter(({ i }) => !onlyErrors || previewOf(i)?.action === "error");
  const label = object.meta.label_plural;

  return (
    <dialog ref={dialog} className={`glass sheet ${styles.sheet}`} aria-label={`Import ${label.toLowerCase()}`} onClose={onClose} onCancel={(e) => (e.preventDefault(), onClose())}>
      <div className={styles.head}>
        <div>
          <p className="eyebrow">{file ? file.file_name : "Import"}</p>
          <h2 className="sheetHeading">{step === "done" ? "Import done" : `Import ${label.toLowerCase()}`}</h2>
        </div>
        {step !== "done" && (
          <ol className={styles.steps} aria-label="Steps">
            {STEPS.map(([s, text], i) => (
              <li key={s} className={s === step ? styles.stepOn : undefined} aria-current={s === step ? "step" : undefined}>
                {i + 1}. {text}
              </li>
            ))}
          </ol>
        )}
      </div>
      {trouble && (
        <p className="formError" role="alert">
          {trouble}
        </p>
      )}

      {step === "file" && (
        <div className={styles.body}>
          <p className={styles.intro}>
            A CSV, XLSX or XLS file with one row per {object.meta.label_singular.toLowerCase()} and a header row, up to 10 000 rows. Columns match the fields by their names — a file exported from New Dawn or Twenty maps by itself; a company is named by its domain, a person by their email, any record by its id. A row whose id or email is already here updates that record.
          </p>
        </div>
      )}

      {step === "map" && (
        <div className={styles.body}>
          <table className={styles.table}>
            <thead>
              <tr>
                <th>Column</th>
                <th>Field</th>
                <th>Match by</th>
              </tr>
            </thead>
            <tbody>
              {mapping.map((m, col) => {
                const choice = choiceOf(m.field);
                return (
                  <tr key={col}>
                    <td>
                      <span className={styles.header}>{m.header || `Column ${col + 1}`}</span>
                      <span className={styles.sample}>{sampleOf(col) || "—"}</span>
                    </td>
                    <td>
                      <select
                        className="input"
                        aria-label={`Field for ${m.header || `column ${col + 1}`}`}
                        value={m.field ? `${m.field}|${m.sub ?? ""}` : ""}
                        onChange={(e) => {
                          const [field, sub] = e.target.value.split("|");
                          const next = choiceOf(field);
                          setMap(col, { field: field || null, sub: sub || null, match_by: next?.match_options.length ? (next.match_options[1]?.value ?? "id") : null });
                        }}
                      >
                        <option value="">Do not map</option>
                        {fields.map((f) =>
                          f.subs.length ? (
                            <optgroup key={f.field} label={f.label}>
                              <option value={`${f.field}|`}>{f.label}</option>
                              {f.subs.map((s) => (
                                <option key={s.key} value={`${f.field}|${s.key}`}>
                                  {f.label} / {s.label}
                                </option>
                              ))}
                            </optgroup>
                          ) : (
                            <option key={f.field} value={`${f.field}|`}>
                              {f.label}
                              {f.required ? " *" : ""}
                            </option>
                          ),
                        )}
                      </select>
                    </td>
                    <td>
                      {choice && choice.match_options.length > 0 ? (
                        <select className="input" aria-label={`Match ${choice.label} by`} value={m.match_by ?? "id"} onChange={(e) => setMap(col, { match_by: e.target.value })}>
                          {choice.match_options.map((o) => (
                            <option key={o.value} value={o.value}>
                              {o.label}
                            </option>
                          ))}
                        </select>
                      ) : (
                        <span className="ghostText">—</span>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
          <p className={styles.identifier}>
            <label htmlFor="import-identifier">Update existing {label.toLowerCase()} by</label>
            <select id="import-identifier" className="input" value={identifier ?? ""} onChange={(e) => setIdentifier(e.target.value || null)}>
              <option value="">Nothing — every row is new</option>
              {identifierChoices.map(({ m }) => (
                <option key={m.field ?? ""} value={m.field ?? ""}>
                  {m.field === "id" ? "Id" : (choiceOf(m.field)?.label ?? m.field)}
                </option>
              ))}
            </select>
            <span className="fieldHint">{mapped.length} of {mapping.length} columns mapped · {rows.length} rows</span>
          </p>
        </div>
      )}

      {step === "check" && preview && (
        <div className={styles.body}>
          <p className={styles.summary}>
            <span>
              <strong>{preview.creates}</strong> to create · <strong>{preview.updates}</strong> to update · <strong>{preview.errors}</strong> with errors
              {rows.length > SHOWN ? ` · the first ${SHOWN} of ${rows.length} rows shown` : ""}
            </span>
            <label>
              <input type="checkbox" checked={onlyErrors} onChange={(e) => setOnlyErrors(e.target.checked)} />
              Show only rows with errors
            </label>
          </p>
          <table className={styles.table}>
            <thead>
              <tr>
                <th className={styles.number}>#</th>
                <th>Row</th>
                {mapped.map(({ m, col }) => (
                  <th key={col}>{m.header || `Column ${col + 1}`}</th>
                ))}
                <th />
              </tr>
            </thead>
            <tbody>
              {shownRows.slice(0, SHOWN).map(({ r, i }) => {
                const p = previewOf(i);
                const errors = p?.errors ?? [];
                const rowNotes = errors.filter((e) => e.column === null || e.column === undefined);
                return (
                  <tr key={i} className={p?.action === "error" ? styles.rowError : undefined}>
                    <td className={styles.number}>{i + 1}</td>
                    <td>
                      <span className="chip">{p?.action === "update" ? "Update" : p?.action === "error" ? "Error" : "New"}</span>
                      {rowNotes.map((e) => (
                        <p key={e.message} className={styles.note}>
                          {e.message}
                        </p>
                      ))}
                    </td>
                    {mapped.map(({ col }) => {
                      const problem = errors.find((e) => e.column === col);
                      return (
                        <td key={col}>
                          <input className={problem ? `${styles.cell} ${styles.cellError}` : styles.cell} value={r[col] ?? ""} title={problem?.message} aria-label={`Row ${i + 1}, ${headers[col] || `column ${col + 1}`}`} aria-invalid={!!problem} onChange={(e) => edit(i, col, e.target.value)} />
                          {problem && <p className={styles.note}>{problem.message}</p>}
                        </td>
                      );
                    })}
                    <td>
                      <button type="button" className="btn btnGhost btnCompact" aria-label={`Remove row ${i + 1}`} title="Remove this row" onClick={() => removeRow(i)}>
                        ×
                      </button>
                    </td>
                  </tr>
                );
              })}
              {shownRows.length === 0 && (
                <tr>
                  <td colSpan={mapped.length + 3}>
                    <span className="ghostText">{onlyErrors ? "No rows with errors" : "No rows"}</span>
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      )}

      {step === "done" && report && (
        <div className={styles.body}>
          <ul className={styles.report}>
            <li>
              <span className={styles.big}>{report.created}</span> created · <span className={styles.big}>{report.updated}</span> updated · <span className={styles.big}>{report.skipped}</span> skipped
            </li>
            {report.errors.slice(0, 50).map((e) => (
              <li key={`${e.row}-${e.message}`}>
                Row {e.row}: {e.message}
              </li>
            ))}
            {report.errors.length > 50 && <li className="ghostText">… and {report.errors.length - 50} more</li>}
          </ul>
        </div>
      )}

      <div className={styles.foot}>
        {step === "file" && (
          <>
            <button type="button" className={`btn btnGhost ${styles.spacer}`} onClick={onClose}>
              Cancel
            </button>
            <button type="button" className="btn btnPrimary" disabled={busy} aria-busy={busy} onClick={() => void pick()}>
              Select file…
            </button>
          </>
        )}
        {step === "map" && (
          <>
            <button type="button" className={`btn btnGhost ${styles.spacer}`} onClick={() => setStep("file")}>
              Back
            </button>
            <button type="button" className="btn btnPrimary" disabled={busy || mapped.length === 0} aria-busy={busy} onClick={() => void check(rows)}>
              Next step
            </button>
          </>
        )}
        {step === "check" && preview && (
          <>
            <button type="button" className={`btn btnGhost ${styles.spacer}`} onClick={() => setStep("map")}>
              Back
            </button>
            {preview.errors > 0 && <span className="fieldHint">Rows with errors are skipped — fix a cell or remove the row.</span>}
            <button type="button" className="btn btnPrimary" disabled={busy || preview.creates + preview.updates === 0} aria-busy={busy} onClick={() => void confirm()}>
              {preview.errors > 0 ? `Import ${preview.creates + preview.updates}, skip ${preview.errors}` : "Confirm"}
            </button>
          </>
        )}
        {step === "done" && (
          <button type="button" className="btn btnPrimary" onClick={onClose}>
            Done
          </button>
        )}
      </div>
    </dialog>
  );
}
