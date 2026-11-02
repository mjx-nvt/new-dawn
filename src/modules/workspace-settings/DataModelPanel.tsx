import { useEffect, useRef, useState } from "react";
import { ICONS, useCreateObject } from "../../kernel/bridge/datamodel";
import { useObjects, type ObjectMeta } from "../../kernel/bridge/records";
import { revealFile, useTransferJobs } from "../../kernel/bridge/transfer";
import { formatDateTime } from "../../kernel/format/format";
import styles from "./DataModel.module.css";
import DataModelObject from "./DataModelObject";
import OnOff from "./OnOff";

/** Workspace → CRM → Data model (D-060, 0.10.5): Twenty's objects list — standard and custom,
 *  active and inactive — and "+ New object"; an object opens its page. */
export default function DataModelPanel({ slug, objectName, onOpen, onBack }: { slug: string; objectName: string | undefined; onOpen: (name: string) => void; onBack: () => void }) {
  const objects = useObjects(slug);
  const [filter, setFilter] = useState<"active" | "inactive" | "all">("active");
  const [adding, setAdding] = useState(false);
  const jobs = useTransferJobs(slug);
  const [trouble, setTrouble] = useState<string | null>(null);
  if (objectName) return <DataModelObject slug={slug} objectName={objectName} onBack={onBack} />;
  const shown = (objects.data ?? []).filter((o) => !o.is_system && (filter === "all" || (filter === "active") === o.is_active));
  // Projects' work items (D-083) sit in the history beside the CRM's objects.
  const labelOf = (singular: string) => (singular === "pm_item" ? "Work items" : (objects.data ?? []).find((o) => o.name_singular === singular)?.label_plural ?? singular);
  const when = (iso: string) => formatDateTime(iso);
  return (
    <section className="section">
      <div className={styles.head}>
        <div>
          <h2 className="sectionTitle">Data model</h2>
          <p className="sectionSubtitle">Twenty's objects and fields — the standard ones and yours. A new object gets its table, its views and its record page at once.</p>
        </div>
        <button type="button" className="btn btnPrimary btnCompact" onClick={() => setAdding(true)}>
          + New object
        </button>
      </div>
      <div className={`segmented ${styles.filters}`} role="group" aria-label="Show">
        {(["active", "inactive", "all"] as const).map((f) => (
          <button key={f} type="button" aria-pressed={filter === f} onClick={() => setFilter(f)}>
            {f === "active" ? "Active" : f === "inactive" ? "Deactivated" : "All"}
          </button>
        ))}
      </div>
      {objects.error && (
        <p className="formError" role="alert">
          {objects.error.message}
        </p>
      )}
      <div className={styles.list} role="list">
        {shown.length
          ? shown.map((o) => <ObjectRow key={o.id} object={o} onOpen={() => onOpen(o.name_plural)} />)
          : [0, 1].map((i) => (
              <div key={i} className={`${styles.ghostRow} ghostCard`}>
                {i === 0 && <span className="ghostText">{filter === "inactive" ? "No deactivated objects" : "Not yet captured"}</span>}
              </div>
            ))}
      </div>
      <h3 className="groupTitle">Imports and exports</h3>
      <p className="sectionSubtitle">The last fifty runs. Import records and Export view live in a view's ⋯ menu and in ⌘K (D-065).</p>
      <div className="group" role="list">
        {(jobs.data ?? []).length ? (
          (jobs.data ?? []).map((j) => (
            <div key={j.id} className="groupRow" role="listitem">
              <div className="rowText">
                <div className="rowLabel">
                  {j.kind === "import" ? "Import" : "Export"} · {j.label || "—"}
                </div>
                <div className="rowHint">
                  {labelOf(j.object)} · {j.detail} · {when(j.created_at)}
                </div>
              </div>
              {j.file_path && (
                <div className="rowControl">
                  <button type="button" className="btn btnGhost btnCompact" onClick={() => revealFile(j.file_path).catch((e: Error) => setTrouble(e.message))}>
                    Reveal
                  </button>
                </div>
              )}
            </div>
          ))
        ) : (
          [0, 1].map((i) => (
            <div key={i} className={`${styles.ghostRow} ghostCard`}>
              {i === 0 && <span className="ghostText">Not yet captured</span>}
            </div>
          ))
        )}
      </div>
      {trouble && (
        <p className="formError" role="alert">
          {trouble}
        </p>
      )}
      {adding && (
        <NewObjectSheet
          slug={slug}
          onClose={() => setAdding(false)}
          onMade={(name) => {
            setAdding(false);
            onOpen(name);
          }}
        />
      )}
    </section>
  );
}

function ObjectRow({ object, onOpen }: { object: ObjectMeta; onOpen: () => void }) {
  return (
    <button type="button" className={object.is_active ? styles.row : `${styles.row} ${styles.rowMuted}`} onClick={onOpen}>
      <span className={styles.mark} aria-hidden="true">
        {(object.label_plural || "?").slice(0, 1).toUpperCase()}
      </span>
      <span className={styles.text}>
        <span className={styles.name}>{object.label_plural}</span>
        <span className={styles.meta}>{[object.label_singular, object.description].filter(Boolean).join(" · ")}</span>
      </span>
      <span className={styles.chips}>
        <span className="chip">{object.is_custom ? "Custom" : "Standard"}</span>
        {!object.is_active && <span className="chip">Deactivated</span>}
      </span>
    </button>
  );
}

/** Twenty's "+ New object": singular and plural labels, an icon, a description, and "Skip
 *  creating a Name field" for a junction object. */
function NewObjectSheet({ slug, onClose, onMade }: { slug: string; onClose: () => void; onMade: (name: string) => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const create = useCreateObject(slug);
  const [singular, setSingular] = useState("");
  const [plural, setPlural] = useState("");
  const [pluralTouched, setPluralTouched] = useState(false);
  const [icon, setIcon] = useState<string>(ICONS[8]);
  const [description, setDescription] = useState("");
  const [skipName, setSkipName] = useState(false);
  useEffect(() => {
    dialog.current?.showModal();
  }, []);
  const pluralShown = pluralTouched ? plural : singular ? `${singular}s` : "";
  const submit = () =>
    create.mutate(
      { label_singular: singular, label_plural: pluralShown, icon, description, skip_name: skipName },
      { onSuccess: (made) => onMade(made.meta.name_plural) },
    );
  return (
    <dialog ref={dialog} className="glass sheet" aria-label="New object" onClose={onClose} onCancel={onClose}>
      <form
        className="sheetBody"
        onSubmit={(e) => {
          e.preventDefault();
          submit();
        }}
      >
        <h2 className="sheetHeading">New object</h2>
        <div className={styles.twoUp}>
          <label className="field">
            <span className="fieldLabel">Singular</span>
            <input className="input" autoFocus value={singular} placeholder="Partnership" onChange={(e) => setSingular(e.target.value)} />
          </label>
          <label className="field">
            <span className="fieldLabel">Plural</span>
            <input
              className="input"
              value={pluralShown}
              placeholder="Partnerships"
              onChange={(e) => {
                setPluralTouched(true);
                setPlural(e.target.value);
              }}
            />
          </label>
        </div>
        <label className="field">
          <span className="fieldLabel">Icon</span>
          <select className="input" value={icon} onChange={(e) => setIcon(e.target.value)}>
            {ICONS.map((i) => (
              <option key={i} value={i}>
                {i.replace(/^Icon/, "")}
              </option>
            ))}
          </select>
        </label>
        <label className="field">
          <span className="fieldLabel">Description</span>
          <input className="input" value={description} placeholder="What these records are" onChange={(e) => setDescription(e.target.value)} />
        </label>
        <div className={styles.switchRow}>
          <span className={styles.switchText}>
            <span className="fieldValue">Skip creating a Name field</span>
            <span className="fieldHint">For a junction object: its records are the links, not things with names</span>
          </span>
          <OnOff label="Skip creating a Name field" value={skipName} onChange={setSkipName} />
        </div>
        {create.error && (
          <p className="formError" role="alert">
            {create.error.message}
          </p>
        )}
        <div className="sheetActions">
          <button type="button" className="btn btnGhost" onClick={() => dialog.current?.close()}>
            Cancel
          </button>
          <button type="submit" className="btn btnPrimary" disabled={!singular.trim() || !pluralShown.trim() || create.isPending}>
            {create.isPending ? "Making…" : "Save"}
          </button>
        </div>
      </form>
    </dialog>
  );
}
