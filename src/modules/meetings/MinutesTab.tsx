import { useState } from "react";
import DangerButton from "../../kernel/shell/DangerButton";
import type { Meeting } from "../../kernel/bridge/meetings";
import { useMeetingTypeTemplates } from "../../kernel/bridge/templates";
import {
  openMinutesFile,
  pdfUrl,
  useConverters,
  useDeleteExport,
  useExportMinutes,
  useMakePdf,
  useMinutesExports,
  type MinutesExport,
  type MinutesFile,
} from "../../kernel/bridge/minutes";
import { useSummary } from "../../kernel/bridge/summary";
import { formatWhen } from "./format";
import styles from "./MinutesTab.module.css";

const CONVERTER_LABEL = { word: "Word", libreoffice: "LibreOffice", pages: "Pages" } as const;

/** "2026-10-24 Partner call", the default file name (local date, as the backend uses). */
function defaultName(meeting: Meeting): string {
  const d = new Date(meeting.held_on);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${meeting.title}`;
}

function baseName(path: string | null | undefined): string {
  return path?.split("/").pop() ?? "";
}

/** Minutes: the workspace's Word template filled from the summary; a PDF only when asked. */
export default function MinutesTab({ slug, meeting }: { slug: string; meeting: Meeting }) {
  const summary = useSummary(slug, meeting.id);
  const exports = useMinutesExports(slug, meeting.id);
  const converters = useConverters();
  const exportMinutes = useExportMinutes(slug, meeting.id);
  const makePdf = useMakePdf(slug, meeting.id);
  // The template export will use: the type's own file, else the default type's (as in Rust).
  const types = useMeetingTypeTemplates(slug);
  const ownType = types.data?.find((t) => t.id === meeting.meeting_type_id && t.template.path);
  const defaultType = types.data?.find((t) => t.is_default);
  const templateName = ownType?.name ?? (defaultType && `${defaultType.name} (default)`);
  const deleteExport = useDeleteExport(slug, meeting.id);
  const [withPdf, setWithPdf] = useState(false);
  const [fileName, setFileName] = useState("");
  const [openError, setOpenError] = useState<string | null>(null);

  const hasSummary = !!summary.data;
  const hasConverter = !!converters.data?.some((c) => c.path);
  const busy = exportMinutes.isPending || makePdf.isPending || deleteExport.isPending;
  const error = exportMinutes.error ?? makePdf.error ?? deleteExport.error ?? exports.error;
  const latestPdf = exports.data?.find((e) => e.pdf_path);

  const open = (exportId: string, file: MinutesFile, reveal: boolean) => {
    setOpenError(null);
    openMinutesFile(slug, exportId, file, reveal).catch((err: Error) => setOpenError(err.message));
  };

  return (
    <section className="section">
      <div className={`card ${styles.exportCard}`}>
        <div className={styles.exportHead}>
          <div>
            <p className={styles.cardTitle}>Meeting minutes</p>
            <p className="fieldHint">
              {templateName ? `Template: ${templateName}` : "The workspace's Word template"}, filled in from the summary. Change
              it with the meeting type (next to the date above) or in Settings → Meeting types.
            </p>
          </div>
          <div className="segmented" role="group" aria-label="Export as">
            <button type="button" aria-pressed={!withPdf} onClick={() => setWithPdf(false)}>
              Word
            </button>
            <button type="button" aria-pressed={withPdf} onClick={() => setWithPdf(true)}>
              Word + PDF
            </button>
          </div>
        </div>
        <div className={styles.exportRow}>
          <label className={`field ${styles.nameField}`}>
            <span className="fieldLabel">File name</span>
            <input className="input" value={fileName} placeholder={defaultName(meeting)} onChange={(e) => setFileName(e.target.value)} />
          </label>
          <button
            type="button"
            className="btn btnPrimary"
            disabled={!hasSummary || busy || (withPdf && !hasConverter)}
            onClick={() => exportMinutes.mutate({ fileName, pdf: withPdf })}
          >
            {exportMinutes.isPending ? (withPdf ? "Exporting… (the PDF can take a minute)" : "Exporting…") : "Export"}
          </button>
        </div>
        {!hasSummary && !summary.isPending && <p className="fieldHint">Write the summary first — the minutes are filled from it.</p>}
        {withPdf && converters.data && !hasConverter && (
          <p className="fieldHint">No PDF converter found. Install Microsoft Word, LibreOffice or Pages (Settings → Converters).</p>
        )}
        <p className="fieldHint">Filled from the summary, with your edits. The bracketed hints in the template (venue, chair, signatures) are for you to complete in Word.</p>
      </div>

      {(error ?? openError) && (
        <p className={styles.error} role="alert">
          {error?.message ?? openError}
        </p>
      )}

      <div className={styles.block}>
        <h2 className="sectionTitle">Exports</h2>
        <p className="sectionSubtitle">Newest first · saved in the workspace's exports folder</p>
        {exports.data?.length ? (
          exports.data.map((item) => (
            <ExportRow
              key={item.id}
              item={item}
              makingPdf={makePdf.isPending && makePdf.variables === item.id}
              disabled={busy || !hasConverter}
              deleting={busy}
              onMakePdf={() => makePdf.mutate(item.id)}
              onDelete={() => deleteExport.mutate(item.id)}
              onOpen={open}
            />
          ))
        ) : (
          <>
            <p className="ghostText">Not yet captured</p>
            <div className={`${styles.slot} ghostCard`} />
            <div className={`${styles.slot} ghostCard`} />
          </>
        )}
      </div>

      <div className={styles.block}>
        <h2 className="sectionTitle">PDF preview</h2>
        <p className="sectionSubtitle">{latestPdf ? baseName(latestPdf.pdf_path) : "The newest PDF export"}</p>
        {latestPdf?.pdf_path ? (
          <iframe className={styles.preview} title="PDF preview" src={pdfUrl(latestPdf.pdf_path, latestPdf.updated_at)} />
        ) : (
          <>
            <p className="ghostText">Not yet captured — make a PDF to preview it here</p>
            <div className={`${styles.previewSlot} ghostCard`} />
          </>
        )}
      </div>
    </section>
  );
}

function ExportRow({
  item,
  makingPdf,
  disabled,
  deleting,
  onMakePdf,
  onDelete,
  onOpen,
}: {
  item: MinutesExport;
  makingPdf: boolean;
  disabled: boolean;
  deleting: boolean;
  onMakePdf: () => void;
  onDelete: () => void;
  onOpen: (exportId: string, file: MinutesFile, reveal: boolean) => void;
}) {
  return (
    <div className={`card ${styles.row}`}>
      <div className={styles.rowText}>
        <p className={styles.rowName}>{baseName(item.docx_path)}</p>
        <p className={styles.rowMeta}>
          {formatWhen(item.created_at)}
          <span className="chip">DOCX</span>
          {item.pdf_path && <span className="chip chipBrand">PDF{item.converter && ` · ${CONVERTER_LABEL[item.converter]}`}</span>}
        </p>
      </div>
      <div className={styles.rowActions}>
        <button type="button" className="btn btnSecondary btnCompact" onClick={() => onOpen(item.id, "docx", false)}>
          Open
        </button>
        {item.pdf_path && (
          <button type="button" className="btn btnGhost btnCompact" onClick={() => onOpen(item.id, "pdf", false)}>
            Open PDF
          </button>
        )}
        <button type="button" className="btn btnGhost btnCompact" disabled={disabled} onClick={onMakePdf} title="Uses the DOCX as saved, including your edits in Word">
          {makingPdf ? "Making PDF…" : item.pdf_path ? "Update PDF" : "Make PDF"}
        </button>
        <button type="button" className="btn btnGhost btnCompact" onClick={() => onOpen(item.id, "docx", true)}>
          Reveal in Finder
        </button>
        <DangerButton confirm="Move to Trash?" onConfirm={onDelete} disabled={deleting} title="Moves the DOCX and PDF to the Trash">
          Delete
        </DangerButton>
      </div>
    </div>
  );
}
