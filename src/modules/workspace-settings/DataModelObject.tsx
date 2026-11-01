import { useEffect, useState } from "react";
import { ICONS, typeLabel, useDeleteObject, useSetObjectActive, useUpdateObject } from "../../kernel/bridge/datamodel";
import { useRecordLayout } from "../../kernel/bridge/layouts";
import { useObject, useObjects, type FieldMeta, type ObjectDef } from "../../kernel/bridge/records";
import DangerButton from "../../kernel/shell/DangerButton";
import LayoutEditor from "../crm/record/LayoutEditor";
import styles from "./DataModel.module.css";
import DataModelField from "./DataModelField";
import OnOff from "./OnOff";

/** An object's page in the data model (D-060, 0.10.5): its labels and identifiers, its fields
 *  (each opens the field sheet; "+ Add field"), its record page layout, and whether it is on. */
export default function DataModelObject({ slug, objectName, onBack }: { slug: string; objectName: string; onBack: () => void }) {
  const object = useObject(slug, objectName);
  const def = object.data;
  const back = (
    <button type="button" className={`btn btnGhost btnCompact ${styles.back}`} onClick={onBack}>
      ‹ Data model
    </button>
  );
  if (!def) {
    return (
      <>
        {back}
        <section className="section">
          <h2 className="sectionTitle">{object.isError ? "Couldn't be read" : "…"}</h2>
          {object.error && (
            <p className="formError" role="alert">
              {object.error.message}
            </p>
          )}
          <div className={`${styles.ghostRow} ghostCard`} />
        </section>
      </>
    );
  }
  return (
    <>
      {back}
      <ObjectDetail key={def.meta.id} slug={slug} def={def} onBack={onBack} />
    </>
  );
}

function ObjectDetail({ slug, def, onBack }: { slug: string; def: ObjectDef; onBack: () => void }) {
  const name = def.meta.name_singular;
  const update = useUpdateObject(slug);
  const setActive = useSetObjectActive(slug);
  const remove = useDeleteObject(slug);
  const objects = useObjects(slug);
  const layout = useRecordLayout(slug, name);
  const [about, setAbout] = useState({ singular: def.meta.label_singular, plural: def.meta.label_plural, icon: def.meta.icon, description: def.meta.description });
  useEffect(() => setAbout({ singular: def.meta.label_singular, plural: def.meta.label_plural, icon: def.meta.icon, description: def.meta.description }), [def.meta.label_singular, def.meta.label_plural, def.meta.icon, def.meta.description]);
  const [field, setField] = useState<"new" | FieldMeta | null>(null);
  const [editingLayout, setEditingLayout] = useState(false);
  const dirty = about.singular !== def.meta.label_singular || about.plural !== def.meta.label_plural || about.icon !== def.meta.icon || about.description !== def.meta.description;
  const error = update.error ?? setActive.error ?? remove.error;
  const fields = def.fields.filter((f) => !f.is_system).sort((a, b) => Number(b.is_active) - Number(a.is_active) || (a.position ?? 0) - (b.position ?? 0));
  const textFields = def.fields.filter((f) => !f.is_system && (f.type === "TEXT" || f.type === "FULL_NAME"));
  const imageFields = def.fields.filter((f) => !f.is_system && (f.type === "TEXT" || f.type === "LINKS"));
  const targetLabel = (f: FieldMeta) => {
    const rel = f.options.relation;
    if (!rel) return "";
    const target = objects.data?.find((o) => o.name_singular === rel.target);
    const kind = rel.kind === "MANY_TO_ONE" ? "many to one" : rel.junction ? "many to many" : "one to many";
    return `→ ${target?.label_plural ?? rel.target} · ${kind}`;
  };

  return (
    <>
      <section className="section">
        <div className={styles.head}>
          <div>
            <h2 className={`sectionTitle ${styles.title}`}>
              {def.meta.label_plural}
              <span className="chip">{def.meta.is_custom ? "Custom" : "Standard"}</span>
              {!def.meta.is_active && <span className="chip">Deactivated</span>}
            </h2>
            <p className="sectionSubtitle">{def.meta.description || `The ${def.meta.label_singular.toLowerCase()} object: ${def.meta.table_name} in this workspace's database`}</p>
          </div>
        </div>
        {error && (
          <p className="formError" role="alert">
            {error.message}
          </p>
        )}
        <div className={styles.aboutGrid}>
          <label className="field">
            <span className="fieldLabel">Singular</span>
            <input className="input" value={about.singular} onChange={(e) => setAbout({ ...about, singular: e.target.value })} />
          </label>
          <label className="field">
            <span className="fieldLabel">Plural</span>
            <input className="input" value={about.plural} onChange={(e) => setAbout({ ...about, plural: e.target.value })} />
          </label>
          <label className="field">
            <span className="fieldLabel">Icon</span>
            <select className="input" value={ICONS.includes(about.icon as (typeof ICONS)[number]) ? about.icon : ""} onChange={(e) => setAbout({ ...about, icon: e.target.value })}>
              {!ICONS.includes(about.icon as (typeof ICONS)[number]) && <option value="">{about.icon.replace(/^Icon/, "") || "—"}</option>}
              {ICONS.map((i) => (
                <option key={i} value={i}>
                  {i.replace(/^Icon/, "")}
                </option>
              ))}
            </select>
          </label>
          <label className="field">
            <span className="fieldLabel">Description</span>
            <input className="input" value={about.description} onChange={(e) => setAbout({ ...about, description: e.target.value })} />
          </label>
        </div>
        <div className={styles.saveRow}>
          <button type="button" className="btn btnPrimary btnCompact" disabled={!dirty || update.isPending} onClick={() => update.mutate({ id: def.meta.id, patch: { label_singular: about.singular, label_plural: about.plural, icon: about.icon, description: about.description } })}>
            {update.isPending ? "Saving…" : "Save"}
          </button>
        </div>
      </section>

      <section className="section">
        <h2 className="sectionTitle">Identifiers</h2>
        <p className="sectionSubtitle">The field that names a record, and the one that pictures it</p>
        <div className="group">
          <div className="groupRow">
            <div className="rowText">
              <p className="rowLabel">Label identifier</p>
              <p className="rowHint">A text or full name field; the page title and the first column</p>
            </div>
            <div className="rowControl">
              <select className="input" aria-label="Label identifier" value={def.meta.label_identifier_field_id ?? ""} onChange={(e) => update.mutate({ id: def.meta.id, patch: { label_identifier_field_id: e.target.value } })}>
                <option value="">—</option>
                {textFields.map((f) => (
                  <option key={f.id} value={f.id}>
                    {f.label}
                  </option>
                ))}
              </select>
            </div>
          </div>
          <div className="groupRow">
            <div className="rowText">
              <p className="rowLabel">Image identifier</p>
              <p className="rowHint">A text or links field holding a picture's URL</p>
            </div>
            <div className="rowControl">
              <select className="input" aria-label="Image identifier" value={def.meta.image_identifier_field_id ?? ""} onChange={(e) => update.mutate({ id: def.meta.id, patch: { image_identifier_field_id: e.target.value } })}>
                <option value="">—</option>
                {imageFields.map((f) => (
                  <option key={f.id} value={f.id}>
                    {f.label}
                  </option>
                ))}
              </select>
            </div>
          </div>
        </div>
      </section>

      <section className="section">
        <div className={styles.head}>
          <div>
            <h2 className="sectionTitle">Fields</h2>
            <p className="sectionSubtitle">A field is a column of the table; a relation shows on both objects</p>
          </div>
          <button type="button" className="btn btnSecondary btnCompact" onClick={() => setField("new")}>
            + Add field
          </button>
        </div>
        <div className={styles.list} role="list">
          {fields.length
            ? fields.map((f) => (
                <button key={f.id} type="button" className={f.is_active ? styles.row : `${styles.row} ${styles.rowMuted}`} onClick={() => setField(f)}>
                  <span className={styles.mark} aria-hidden="true">
                    {f.label.slice(0, 1).toUpperCase()}
                  </span>
                  <span className={styles.text}>
                    <span className={styles.name}>{f.label}</span>
                    <span className={styles.meta}>{[typeLabel(f.type), targetLabel(f), f.description].filter(Boolean).join(" · ")}</span>
                  </span>
                  <span className={styles.chips}>
                    {!f.is_nullable && <span className="chip">Required</span>}
                    {f.is_unique && <span className="chip">Unique</span>}
                    {f.id === def.meta.label_identifier_field_id && <span className="chip chipPrimary">Label</span>}
                    {!f.is_active && <span className="chip">Deactivated</span>}
                  </span>
                </button>
              ))
            : [0, 1].map((i) => <div key={i} className={`${styles.ghostRow} ghostCard`} />)}
        </div>
      </section>

      <section className="section">
        <h2 className="sectionTitle">Layout</h2>
        <p className="sectionSubtitle">The record page's tabs and widgets (the same editor as ⋯ → Edit page layout on a record)</p>
        {editingLayout && layout.data ? (
          // A blank record marked deleted: every widget shows its structure and nothing edits.
          <LayoutEditor ctx={{ slug, object: def, record: { id: "", deletedAt: "layout" } }} layout={layout.data} onDone={() => setEditingLayout(false)} />
        ) : (
          <button type="button" className="btn btnSecondary btnCompact" disabled={!layout.data} onClick={() => setEditingLayout(true)}>
            Edit page layout
          </button>
        )}
      </section>

      <section className="section">
        <h2 className="sectionTitle">Status</h2>
        <div className="group">
          <div className="groupRow">
            <div className="rowText">
              <p className="rowLabel">Active</p>
              <p className="rowHint">Off hides the object everywhere and keeps its records</p>
            </div>
            <div className="rowControl">
              <OnOff label="Active" value={def.meta.is_active} disabled={setActive.isPending} onChange={(v) => setActive.mutate({ id: def.meta.id, active: v })} />
            </div>
          </div>
          <div className="groupRow">
            <div className="rowText">
              <p className="rowLabel">Delete object</p>
              <p className="rowHint">{def.meta.is_custom ? "A deactivated custom object and its records go for good" : "A standard object can be deactivated, not deleted"}</p>
            </div>
            <div className="rowControl">
              <DangerButton confirm={`Delete ${def.meta.label_plural}?`} disabled={!def.meta.is_custom || def.meta.is_active || remove.isPending} onConfirm={() => remove.mutate(def.meta.id, { onSuccess: onBack })}>
                Delete
              </DangerButton>
            </div>
          </div>
        </div>
      </section>

      {field && <DataModelField slug={slug} object={def} field={field === "new" ? null : field} onClose={() => setField(null)} />}
    </>
  );
}
