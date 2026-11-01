import { Link } from "@tanstack/react-router";
import { useEffect } from "react";
import { useRecordLayout } from "../../../kernel/bridge/layouts";
import { asRecord, recordLabel, useRecord, type ObjectDef } from "../../../kernel/bridge/records";
import styles from "./Record.module.css";
import { configOf, fieldsOf, FieldsWidget } from "./widgets";

/** Twenty's side panel (D-062): a record opened from a list without leaving it — its label, the
 *  Overview tab's fields (edited in place), Open to the page, × or Esc to close. */
export default function RecordSidePanel({ slug, object, id, onClose }: { slug: string; object: ObjectDef; id: string; onClose: () => void }) {
  const name = object.meta.name_singular;
  const record = useRecord(slug, name, id);
  const layout = useRecordLayout(slug, name);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // Escape in a field's editor cancels that edit, not the panel.
      const inField = e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || e.target instanceof HTMLSelectElement;
      if (e.key === "Escape" && !inField) onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);
  const data = record.data ? asRecord(record.data) : null;
  const overview = layout.data?.tabs.find((t) => t.tab.is_default) ?? layout.data?.tabs[0];
  const fieldsWidget = overview?.widgets.find((w) => w.type === "FIELDS");
  const fields = fieldsOf(object, fieldsWidget ? configOf(fieldsWidget) : {});
  const label = data ? recordLabel(object, data) : "";
  return (
    <aside className={`glass ${styles.panel}`} role="dialog" aria-label={label || object.meta.label_singular}>
      <div className={styles.panelHead}>
        <div>
          <p className="eyebrow">{object.meta.label_singular}</p>
          <h2 className={styles.panelTitle}>{data ? label || <span className="ghostText">Unnamed</span> : "…"}</h2>
        </div>
        <div className={styles.panelActions}>
          <Link to="/crm/$object/$recordId" params={{ object: object.meta.name_plural, recordId: id }} className="btn btnSecondary btnCompact">
            Open
          </Link>
          <button type="button" className="btn btnGhost btnCompact" aria-label="Close" onClick={onClose}>
            ×
          </button>
        </div>
      </div>
      <div className={styles.panelBody}>
        {data ? (
          <FieldsWidget ctx={{ slug, object, record: data }} fields={fields} />
        ) : record.isError ? (
          <p className="formError" role="alert">
            {record.error.message}
          </p>
        ) : (
          <div className={`${styles.ghostRow} ghostCard`} />
        )}
      </div>
    </aside>
  );
}
