import { useEffect, useRef, useState } from "react";
import type { FieldType, RelationInputKind, SelectOption } from "../../bindings";
import { COLORS, CURRENCIES, FIELD_TYPES, RELATION_KINDS, typeLabel, useCreateField, useDeleteField, useSetFieldActive, useUpdateField } from "../../kernel/bridge/datamodel";
import { useObjects, type FieldMeta, type ObjectDef } from "../../kernel/bridge/records";
import DangerButton from "../../kernel/shell/DangerButton";
import styles from "./DataModel.module.css";
import OnOff from "./OnOff";

interface Draft {
  label: string;
  description: string;
  select: SelectOption[];
  defaultOption: string;
  decimals: string;
  currency: string;
  country: string;
  multiline: boolean;
  required: boolean;
  unique: boolean;
  target: string;
  kind: RelationInputKind;
  inverseLabel: string;
  junction: boolean;
}

function draftOf(field: FieldMeta | null): Draft {
  const o = field?.options;
  return {
    label: field?.label ?? "",
    description: field?.description ?? "",
    select: o?.select ?? [],
    defaultOption: typeof field?.default_value === "string" ? field.default_value : "",
    decimals: o?.decimals == null ? "" : String(o.decimals),
    currency: o?.currency ?? "",
    country: o?.country ?? "",
    multiline: o?.multiline ?? false,
    required: field ? !field.is_nullable : false,
    unique: field?.is_unique ?? false,
    target: "",
    kind: "MANY_TO_ONE",
    inverseLabel: "",
    junction: o?.relation?.junction ?? false,
  };
}

/** Twenty's field sheet (D-060, 0.10.5): the type picker for a new field, then the form —
 *  label, description, the type's options (select options with colours and a default, decimals,
 *  currency, country, multiline, the relation's target, kind and far name), required, unique;
 *  on an existing field also Deactivate / Activate and Delete. */
export default function DataModelField({ slug, object, field, onClose }: { slug: string; object: ObjectDef; field: FieldMeta | null; onClose: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const name = object.meta.name_singular;
  const create = useCreateField(slug, name);
  const update = useUpdateField(slug, name);
  const setActive = useSetFieldActive(slug, name);
  const remove = useDeleteField(slug, name);
  const objects = useObjects(slug);
  const [type, setType] = useState<FieldType | null>(field?.type ?? null);
  const [draft, setDraft] = useState<Draft>(() => draftOf(field));
  useEffect(() => {
    dialog.current?.showModal();
  }, []);
  const set = (patch: Partial<Draft>) => setDraft((d) => ({ ...d, ...patch }));
  const error = create.error ?? update.error ?? setActive.error ?? remove.error;
  const busy = create.isPending || update.isPending || setActive.isPending || remove.isPending;
  const isSelect = type === "SELECT" || type === "MULTI_SELECT";
  const targets = (objects.data ?? []).filter((o) => !o.is_system && o.is_active);
  const options = {
    select: draft.select,
    ...(type === "NUMBER" && draft.decimals !== "" ? { decimals: Number(draft.decimals) } : {}),
    ...(type === "CURRENCY" && draft.currency ? { currency: draft.currency } : {}),
    ...(type === "PHONES" && draft.country ? { country: draft.country.toUpperCase() } : {}),
    ...(type === "TEXT" ? { multiline: draft.multiline } : {}),
    ...(field?.options.relation ? { relation: { ...field.options.relation, junction: draft.junction } } : {}),
  };
  const submit = () => {
    if (!type || !draft.label.trim()) return;
    if (field) {
      update.mutate({ id: field.id, patch: { label: draft.label, description: draft.description, options, default_value: isSelect ? draft.defaultOption : "", required: draft.required, unique: draft.unique } }, { onSuccess: onClose });
    } else {
      create.mutate(
        {
          label: draft.label,
          type,
          description: draft.description,
          icon: "",
          options,
          default_value: isSelect && draft.defaultOption ? draft.defaultOption : "",
          required: draft.required,
          unique: draft.unique,
          ...(type === "RELATION" ? { relation: { target: draft.target, kind: draft.kind, inverse_label: draft.inverseLabel || null, junction: draft.junction } } : {}),
        },
        { onSuccess: onClose },
      );
    }
  };
  const optionAt = (i: number, patch: Partial<SelectOption>) => set({ select: draft.select.map((o, j) => (j === i ? { ...o, ...patch } : o)) });
  const moveOption = (i: number, d: -1 | 1) => {
    const j = i + d;
    if (j < 0 || j >= draft.select.length) return;
    const next = [...draft.select];
    [next[i], next[j]] = [next[j], next[i]];
    set({ select: next.map((o, k) => ({ ...o, position: k })) });
  };

  return (
    <dialog ref={dialog} className={`glass sheet ${styles.sheetWide}`} aria-label={field ? field.label : "New field"} onClose={onClose} onCancel={onClose}>
      <form
        className={`sheetBody ${styles.sheetScroll}`}
        onSubmit={(e) => {
          e.preventDefault();
          submit();
        }}
      >
        <h2 className="sheetHeading">{field ? field.label : type ? `New ${typeLabel(type).toLowerCase()} field` : "New field"}</h2>
        {!type ? (
          <div className={styles.typeGrid} role="group" aria-label="Field type">
            {FIELD_TYPES.map((t) => (
              <button key={t.type} type="button" className={styles.typeCard} onClick={() => setType(t.type)}>
                <span className={styles.typeName}>{t.label}</span>
                <span className={styles.typeHint}>{t.hint}</span>
              </button>
            ))}
          </div>
        ) : (
          <div className={styles.form}>
            {!field && (
              <p className="fieldHint">
                {typeLabel(type)} ·{" "}
                <button type="button" className="btn btnGhost btnCompact" onClick={() => setType(null)}>
                  Change type
                </button>
              </p>
            )}
            {field && <p className="fieldHint">{[typeLabel(field.type), field.is_custom ? "custom" : "standard", `column ${field.column || "—"}`].join(" · ")}</p>}
            <div className={styles.twoUp}>
              <label className="field">
                <span className="fieldLabel">Label</span>
                <input className="input" autoFocus value={draft.label} onChange={(e) => set({ label: e.target.value })} />
              </label>
              <label className="field">
                <span className="fieldLabel">Description</span>
                <input className="input" value={draft.description} onChange={(e) => set({ description: e.target.value })} />
              </label>
            </div>

            {isSelect && (
              <div className="field">
                <span className="fieldLabel">Options</span>
                <div className={styles.options}>
                  {draft.select.map((o, i) => (
                    <div key={i} className={styles.optionRow}>
                      <input className="input" aria-label={`Option ${i + 1}`} value={o.label} placeholder="Label" onChange={(e) => optionAt(i, { label: e.target.value })} />
                      <select className="input" aria-label="Colour" value={o.color} onChange={(e) => optionAt(i, { color: e.target.value })}>
                        {COLORS.map((c) => (
                          <option key={c} value={c}>
                            {c}
                          </option>
                        ))}
                      </select>
                      <button type="button" className="btn btnGhost btnCompact" aria-label="Move up" disabled={i === 0} onClick={() => moveOption(i, -1)}>
                        ▲
                      </button>
                      <button type="button" className="btn btnGhost btnCompact" aria-label="Move down" disabled={i === draft.select.length - 1} onClick={() => moveOption(i, 1)}>
                        ▼
                      </button>
                      <button type="button" className="btn btnDanger btnCompact" aria-label="Remove option" onClick={() => set({ select: draft.select.filter((_, j) => j !== i) })}>
                        ×
                      </button>
                    </div>
                  ))}
                  <div>
                    <button type="button" className="btn btnSecondary btnCompact" onClick={() => setDraft((d) => ({ ...d, select: [...d.select, { value: "", label: "", color: COLORS[d.select.length % COLORS.length], position: d.select.length }] }))}>
                      + Option
                    </button>
                  </div>
                </div>
                {type === "SELECT" && (
                  <label className="field">
                    <span className="fieldLabel">Default</span>
                    <select className="input" value={draft.defaultOption} onChange={(e) => set({ defaultOption: e.target.value })}>
                      <option value="">—</option>
                      {draft.select
                        .filter((o) => o.label.trim())
                        .map((o, i) => (
                          <option key={i} value={o.value || o.label.trim()}>
                            {o.label}
                          </option>
                        ))}
                    </select>
                  </label>
                )}
              </div>
            )}
            {type === "NUMBER" && (
              <label className="field">
                <span className="fieldLabel">Decimals</span>
                <input className="input" type="number" min={0} max={8} value={draft.decimals} placeholder="2" onChange={(e) => set({ decimals: e.target.value })} />
              </label>
            )}
            {type === "CURRENCY" && (
              <label className="field">
                <span className="fieldLabel">Currency</span>
                <select className="input" value={draft.currency} onChange={(e) => set({ currency: e.target.value })}>
                  <option value="">—</option>
                  {CURRENCIES.map((c) => (
                    <option key={c} value={c}>
                      {c}
                    </option>
                  ))}
                </select>
              </label>
            )}
            {type === "PHONES" && (
              <label className="field">
                <span className="fieldLabel">Default country</span>
                <input className="input" value={draft.country} placeholder="VN" maxLength={2} onChange={(e) => set({ country: e.target.value })} />
              </label>
            )}
            {type === "TEXT" && (
              <div className={styles.switchRow}>
                <span className="fieldValue">Several lines</span>
                <OnOff label="Several lines" value={draft.multiline} onChange={(v) => set({ multiline: v })} />
              </div>
            )}
            {type === "RELATION" && !field && (
              <>
                <div className={styles.twoUp}>
                  <label className="field">
                    <span className="fieldLabel">Object</span>
                    <select className="input" value={draft.target} onChange={(e) => set({ target: e.target.value })}>
                      <option value="">—</option>
                      {targets.map((o) => (
                        <option key={o.id} value={o.name_singular}>
                          {o.label_plural}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label className="field">
                    <span className="fieldLabel">Relation type</span>
                    <select className="input" value={draft.kind} onChange={(e) => set({ kind: RELATION_KINDS.find((k) => k.kind === e.target.value)?.kind ?? "MANY_TO_ONE" })}>
                      {RELATION_KINDS.map((k) => (
                        <option key={k.kind} value={k.kind}>
                          {k.label}
                        </option>
                      ))}
                    </select>
                  </label>
                </div>
                <p className="fieldHint">{RELATION_KINDS.find((k) => k.kind === draft.kind)?.hint}</p>
                <label className="field">
                  <span className="fieldLabel">Field name on the other object</span>
                  <input className="input" value={draft.inverseLabel} placeholder={draft.kind === "ONE_TO_MANY" ? object.meta.label_singular : object.meta.label_plural} onChange={(e) => set({ inverseLabel: e.target.value })} />
                </label>
              </>
            )}
            {type === "RELATION" && field?.options.relation && (
              <p className="fieldHint">
                {field.options.relation.kind === "MANY_TO_ONE" ? "Many to one" : "One to many"} → {targets.find((o) => o.name_singular === field.options.relation?.target)?.label_plural ?? field.options.relation.target}
                {field.options.relation.inverse ? ` · far field ${field.options.relation.inverse}` : ""}
              </p>
            )}
            {type === "RELATION" && (draft.kind === "ONE_TO_MANY" || field?.options.relation?.kind === "ONE_TO_MANY") && (
              <div className={styles.switchRow}>
                <span className={styles.switchText}>
                  <span className="fieldValue">This is a relation to a junction object</span>
                  <span className="fieldHint">Shows the far records instead of the junction's</span>
                </span>
                <OnOff label="Junction" value={draft.junction} onChange={(v) => set({ junction: v })} />
              </div>
            )}
            {type !== "RELATION" && (
              <div className={styles.twoUp}>
                <div className={styles.switchRow}>
                  <span className="fieldValue">Required</span>
                  <OnOff label="Required" value={draft.required} onChange={(v) => set({ required: v })} />
                </div>
                <div className={styles.switchRow}>
                  <span className="fieldValue">Unique</span>
                  <OnOff label="Unique" value={draft.unique} onChange={(v) => set({ unique: v })} />
                </div>
              </div>
            )}
          </div>
        )}
        {error && (
          <p className="formError" role="alert">
            {error.message}
          </p>
        )}
        <div className="sheetActions">
          {field && !field.is_system && (
            <span className={styles.danger}>
              <button type="button" className="btn btnSecondary" disabled={busy || (field.is_active && field.id === object.meta.label_identifier_field_id)} onClick={() => setActive.mutate({ id: field.id, active: !field.is_active }, { onSuccess: onClose })}>
                {field.is_active ? "Deactivate" : "Activate"}
              </button>
              {field.is_custom && !field.is_active && (
                <DangerButton compact={false} confirm="Delete this field?" disabled={busy} onConfirm={() => remove.mutate(field.id, { onSuccess: onClose })}>
                  Delete
                </DangerButton>
              )}
            </span>
          )}
          <button type="button" className="btn btnGhost" onClick={() => dialog.current?.close()}>
            Cancel
          </button>
          {type && (
            <button type="submit" className="btn btnPrimary" disabled={busy || !draft.label.trim() || (type === "RELATION" && !field && !draft.target)}>
              {busy ? "Saving…" : "Save"}
            </button>
          )}
        </div>
      </form>
    </dialog>
  );
}
