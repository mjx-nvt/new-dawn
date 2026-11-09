import { useState } from "react";
import { asRecord, recordLabel, selectOptions, useRecords, type FieldMeta, type Json, type ObjectDef } from "../../../kernel/bridge/records";
import { useObject } from "../../../kernel/bridge/records";
import { chipClass } from "../cells";
import crm from "../Crm.module.css";
import styles from "./Views.module.css";
import { chosenOf, kindOf, needsValue, newRule, nextPosition, OPS, opLabel, subsOf, tempId, usableFields, type Chosen, type Rule, type RuleGroup, type Working } from "./useViewState";

/** Twenty's filter rules (D-061): a field, an operator for its type, a value; and the advanced
 *  filter — rule groups joined by And, Or or Not, nested. */

const str = (v: unknown): string => (typeof v === "string" ? v : typeof v === "number" ? String(v) : "");

/** One rule's editor: field · part · operator · value. */
export function RuleEditor({ slug, object, rule, onChange, onRemove }: { slug: string; object: ObjectDef; rule: Rule; onChange: (next: Rule) => void; onRemove: () => void }) {
  const fields = usableFields(object);
  const field = fields.find((f) => f.id === rule.field_id) ?? fields[0];
  if (!field) return null;
  const kind = kindOf(field);
  const subs = subsOf(field);
  const ops = OPS[kind];
  const pickField = (id: string) => {
    const next = fields.find((f) => f.id === id);
    if (next) onChange(newRule(next, rule.group_id, rule.position));
  };
  return (
    <div className={styles.rule}>
      <div className={styles.ruleLine}>
        <select className="input" aria-label="Field" value={field.id} onChange={(e) => pickField(e.target.value)}>
          {fields.map((f) => (
            <option key={f.id} value={f.id}>
              {f.label}
            </option>
          ))}
        </select>
        {subs.length > 1 && (
          <select className="input" aria-label="Part" value={rule.sub ?? ""} onChange={(e) => onChange({ ...rule, sub: e.target.value })}>
            {subs.map((s) => (
              <option key={s.key} value={s.key}>
                {s.label}
              </option>
            ))}
          </select>
        )}
        <select className="input" aria-label="Operator" value={rule.operand} onChange={(e) => onChange({ ...rule, operand: e.target.value, value: needsValue(e.target.value) ? rule.value : null })}>
          {ops.map((o) => (
            <option key={o.op} value={o.op}>
              {o.label}
            </option>
          ))}
        </select>
        <button type="button" className={styles.iconButton} aria-label="Remove rule" onClick={onRemove}>
          ✕
        </button>
      </div>
      {needsValue(rule.operand) && (
        <div className={styles.ruleLine}>
          <ValueEditor slug={slug} field={field} kind={kind} op={rule.operand} value={rule.value} onChange={(value) => onChange({ ...rule, value })} />
        </div>
      )}
    </div>
  );
}

function ValueEditor({ slug, field, kind, op, value, onChange }: { slug: string; field: FieldMeta; kind: ReturnType<typeof kindOf>; op: string; value: Json; onChange: (v: Json) => void }) {
  switch (kind) {
    case "number":
      return <input className={`input ${styles.grow}`} type="number" step="any" aria-label="Value" value={str(value)} onChange={(e) => onChange(e.target.value === "" ? null : Number(e.target.value))} />;
    case "date":
      if (op === "isRelative") {
        const v = asRecord(value);
        const set = (patch: Record<string, unknown>) => onChange({ direction: str(v.direction) || "PAST", amount: typeof v.amount === "number" ? v.amount : 1, unit: str(v.unit) || "DAY", ...patch });
        return (
          <>
            <select className="input" aria-label="Direction" value={str(v.direction) || "PAST"} onChange={(e) => set({ direction: e.target.value })}>
              <option value="PAST">Past</option>
              <option value="NEXT">Next</option>
              <option value="THIS">This</option>
            </select>
            {str(v.direction) !== "THIS" && <input className={`input ${styles.narrow}`} type="number" min={1} aria-label="Amount" value={typeof v.amount === "number" ? v.amount : 1} onChange={(e) => set({ amount: Math.max(1, Number(e.target.value) || 1) })} />}
            <select className="input" aria-label="Unit" value={str(v.unit) || "DAY"} onChange={(e) => set({ unit: e.target.value })}>
              <option value="DAY">Day{str(v.direction) !== "THIS" ? "s" : ""}</option>
              <option value="WEEK">Week{str(v.direction) !== "THIS" ? "s" : ""}</option>
              <option value="MONTH">Month{str(v.direction) !== "THIS" ? "s" : ""}</option>
              <option value="YEAR">Year{str(v.direction) !== "THIS" ? "s" : ""}</option>
            </select>
          </>
        );
      }
      return <input className={`input ${styles.grow}`} type="date" aria-label="Date" value={str(value)} onChange={(e) => onChange(e.target.value || null)} />;
    case "select":
    case "multi": {
      const chosen = Array.isArray(value) ? value.map((v) => str(v)) : [];
      return (
        <div className={styles.options}>
          {selectOptions(field).map((o) => (
            <label key={o.value} className={styles.option}>
              <input type="checkbox" checked={chosen.includes(o.value)} onChange={(e) => onChange(e.target.checked ? [...chosen, o.value] : chosen.filter((v) => v !== o.value))} />
              <span className={`${crm.chip} ${chipClass(o.color)}`}>{o.label}</span>
            </label>
          ))}
        </div>
      );
    }
    case "boolean":
      return (
        <select className="input" aria-label="Value" value={value === false ? "false" : "true"} onChange={(e) => onChange(e.target.value === "true")}>
          <option value="true">True</option>
          <option value="false">False</option>
        </select>
      );
    case "relation":
      return <RelationPicker slug={slug} field={field} chosen={chosenOf(value)} onChange={(c) => onChange(c)} />;
    default:
      return <input className={`input ${styles.grow}`} aria-label="Value" value={str(value)} onChange={(e) => onChange(e.target.value)} />;
  }
}

/** Records of the other object, chosen by typing; the chosen ones as chips. */
function RelationPicker({ slug, field, chosen, onChange }: { slug: string; field: FieldMeta; chosen: Chosen[]; onChange: (c: Chosen[]) => void }) {
  const [text, setText] = useState("");
  const target = field.options.relation?.target;
  const targetDef = useObject(slug, target);
  const matches = useRecords(slug, target, { search: text.trim() || null, limit: 8 }, !!target);
  const rows = (matches.data?.records ?? []).map(asRecord).filter((r) => !chosen.some((c) => c.id === r.id));
  return (
    <div className={styles.grow}>
      <div className={styles.picked}>
        {chosen.map((c) => (
          <span key={c.id} className={`${styles.chip} chipNeutral`}>
            {c.label}
            <button type="button" className={styles.chipX} aria-label={`Remove ${c.label}`} onClick={() => onChange(chosen.filter((x) => x.id !== c.id))}>
              ✕
            </button>
          </span>
        ))}
      </div>
      <input className="input" placeholder={`Search ${field.label.toLowerCase()}…`} aria-label={field.label} value={text} onChange={(e) => setText(e.target.value)} />
      <div className={styles.list}>
        {rows.map((r) => {
          const label = targetDef.data ? recordLabel(targetDef.data, r) : str(r.name) || str(r.title);
          return (
            <button key={str(r.id)} type="button" className={styles.listButton} onClick={() => onChange([...chosen, { id: str(r.id), label }])}>
              {label || "Unnamed"}
            </button>
          );
        })}
      </div>
    </div>
  );
}

/** The advanced filter: the top-level rules, then the groups — each And / Or / Not with its own
 *  rules and nested groups. */
export function AdvancedFilter({ slug, object, working, onChange }: { slug: string; object: ObjectDef; working: Working; onChange: (next: Working) => void }) {
  const { filters } = working;
  const set = (next: Working["filters"]) => onChange({ ...working, filters: next });
  const addRule = (groupId: string | null) => {
    const field = usableFields(object)[0];
    if (!field) return;
    set({ ...filters, filters: [...filters.filters, newRule(field, groupId, nextPosition(filters.filters))] });
  };
  const addGroup = (parentId: string | null) => set({ ...filters, groups: [...filters.groups, { id: tempId(), parent_id: parentId, logical: "OR", position: nextPosition(filters.groups) }] });
  const removeGroup = (id: string) => {
    const gone = new Set<string>([id]);
    let more = true;
    while (more) {
      more = false;
      for (const g of filters.groups) if (g.parent_id && gone.has(g.parent_id) && !gone.has(g.id)) (gone.add(g.id), (more = true));
    }
    set({ groups: filters.groups.filter((g) => !gone.has(g.id)), filters: filters.filters.filter((f) => !f.group_id || !gone.has(f.group_id)) });
  };
  const render = (groupId: string | null, depth: number) => {
    const rules = filters.filters.map((r, i) => ({ r, i })).filter(({ r }) => (r.group_id ?? null) === groupId);
    const groups = filters.groups.filter((g) => (g.parent_id ?? null) === groupId);
    return (
      <div className={groupId ? styles.group : undefined}>
        {rules.map(({ r, i }) => (
          <RuleEditor key={i} slug={slug} object={object} rule={r} onChange={(next) => set({ ...filters, filters: filters.filters.map((x, j) => (j === i ? next : x)) })} onRemove={() => set({ ...filters, filters: filters.filters.filter((_, j) => j !== i) })} />
        ))}
        {groups.map((g) => (
          <div key={g.id} className={styles.group}>
            <div className={styles.row}>
              <div className="segmented" role="group" aria-label="Rule group">
                {(["AND", "OR", "NOT"] as const).map((l) => (
                  <button key={l} type="button" aria-pressed={g.logical === l} onClick={() => set({ ...filters, groups: filters.groups.map((x) => (x.id === g.id ? { ...x, logical: l } : x)) })}>
                    {l === "AND" ? "And" : l === "OR" ? "Or" : "Not"}
                  </button>
                ))}
              </div>
              <span className={styles.grow} />
              <button type="button" className={styles.iconButton} aria-label="Remove group" onClick={() => removeGroup(g.id)}>
                ✕
              </button>
            </div>
            {render(g.id, depth + 1)}
          </div>
        ))}
        <div className={styles.actions}>
          <button type="button" className="btn btnGhost btnCompact" onClick={() => addRule(groupId)}>
            + Add rule
          </button>
          {depth < 3 && (
            <button type="button" className="btn btnGhost btnCompact" onClick={() => addGroup(groupId)}>
              + Add group
            </button>
          )}
        </div>
      </div>
    );
  };
  return render(null, 0);
}

/** A rule as a chip: "City · contains · Hanoi". */
export function ruleText(object: ObjectDef, rule: Rule): string {
  const field = object.fields.find((f) => f.id === rule.field_id);
  if (!field) return "?";
  const kind = kindOf(field);
  const part = subsOf(field).find((s) => s.key === rule.sub);
  const name = part && subsOf(field).length > 1 ? `${field.label} ${part.label.toLowerCase()}` : field.label;
  const value = (): string => {
    if (!needsValue(rule.operand)) return "";
    const v = rule.value;
    switch (kind) {
      case "select":
      case "multi":
        return (Array.isArray(v) ? v : []).map((x) => selectOptions(field).find((o) => o.value === str(x))?.label ?? str(x)).join(", ");
      case "relation":
        return chosenOf(v)
          .map((c) => c.label)
          .join(", ");
      case "boolean":
        return v === false ? "false" : "true";
      case "date":
        if (rule.operand === "isRelative") {
          const r = asRecord(v);
          return `${str(r.direction).toLowerCase()} ${str(r.direction) === "THIS" ? "" : str(r.amount)} ${str(r.unit).toLowerCase()}${str(r.direction) !== "THIS" && r.amount !== 1 ? "s" : ""}`.replace(/\s+/g, " ").trim();
        }
        return str(v);
      default:
        return str(v);
    }
  };
  return [name, opLabel(kind, rule.operand).toLowerCase(), value()].filter(Boolean).join(" · ");
}

export type { Rule, RuleGroup };
