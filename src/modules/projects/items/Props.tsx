import { usePmEstimates, usePmLabels, usePmStates, type StateGroup } from "../../../kernel/bridge/pm";
import { usePmItems, type Priority } from "../../../kernel/bridge/pmItems";
import { usePeople } from "../../../kernel/bridge/people";
import { colorOf } from "../../../kernel/charts/Charts";
import { whoName } from "../pm";
import { GROUP_LABEL, PRIORITIES } from "./items";
import styles from "./Items.module.css";

/** The pickers a work item's properties use — in the create form and the item's sidebar
 *  (D-074). Native selects (the app's capsules); chips with ✕ for the lists. */

export function Dot({ color }: { color: string }) {
  return <span className={styles.dot} style={{ background: colorOf(0, color) }} aria-hidden />;
}

export function StatePick({ slug, projectId, value, disabled, onChange }: { slug: string; projectId: string; value: string | null; disabled?: boolean; onChange: (id: string) => void }) {
  const states = (usePmStates(slug, projectId).data ?? []).filter((s) => s.group !== "triage");
  const groups = [...new Set(states.map((s) => s.group))] as StateGroup[];
  const current = states.find((s) => s.id === value);
  return (
    <span className={styles.pick}>
      {current && <Dot color={current.color} />}
      <select className="input" aria-label="State" value={value ?? ""} disabled={disabled} onChange={(e) => e.target.value && onChange(e.target.value)}>
        {!value && <option value="">The default state</option>}
        {groups.map((g) => (
          <optgroup key={g} label={GROUP_LABEL[g]}>
            {states
              .filter((s) => s.group === g)
              .map((s) => (
                <option key={s.id} value={s.id}>
                  {s.name}
                </option>
              ))}
          </optgroup>
        ))}
      </select>
    </span>
  );
}

export function PriorityPick({ value, disabled, onChange }: { value: Priority; disabled?: boolean; onChange: (p: Priority) => void }) {
  return (
    <select className="input" aria-label="Priority" value={value} disabled={disabled} onChange={(e) => onChange(PRIORITIES.find((p) => p.key === e.target.value)?.key ?? "none")}>
      {PRIORITIES.map((p) => (
        <option key={p.key} value={p.key}>
          {p.label}
        </option>
      ))}
    </select>
  );
}

/** "Urgent", "High"… as a mark in a row (urgent in the negative status colour, none quiet). */
export function PriorityMark({ value }: { value: Priority }) {
  if (value === "none") return null;
  return <span className={`${styles.priority} ${styles[`p_${value}`]}`}>{PRIORITIES.find((p) => p.key === value)?.label}</span>;
}

/** You and the people of the CRM: chips, and an Add… list. */
export function PeoplePick({ slug, label, value, disabled, onChange }: { slug: string; label: string; value: string[]; disabled?: boolean; onChange: (who: string[]) => void }) {
  const people = usePeople(slug).data ?? [];
  return (
    <span className={styles.chips}>
      {value.map((w) => (
        <span key={w} className="chip">
          {whoName(w, people)}
          <button type="button" className={styles.chipX} aria-label={`Remove ${whoName(w, people)}`} disabled={disabled} onClick={() => onChange(value.filter((x) => x !== w))}>
            ✕
          </button>
        </span>
      ))}
      <select className={`input ${styles.add}`} aria-label={`Add ${label.toLowerCase()}`} value="" disabled={disabled} onChange={(e) => e.target.value && onChange([...value, e.target.value])}>
        <option value="">{value.length ? "Add…" : `No ${label.toLowerCase()} — add…`}</option>
        {!value.includes("me") && <option value="me">You</option>}
        {people
          .filter((p) => !value.includes(p.id))
          .map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
      </select>
    </span>
  );
}

export function LabelPick({ slug, projectId, value, disabled, onChange }: { slug: string; projectId: string; value: string[]; disabled?: boolean; onChange: (labels: string[]) => void }) {
  const labels = usePmLabels(slug, projectId).data ?? [];
  const groups = labels.filter((l) => !l.parent_id && labels.some((c) => c.parent_id === l.id));
  return (
    <span className={styles.chips}>
      {value.map((id) => {
        const l = labels.find((x) => x.id === id);
        return (
          <span key={id} className="chip">
            {l && <Dot color={l.color} />} {l?.name ?? "A label"}
            <button type="button" className={styles.chipX} aria-label={`Remove ${l?.name ?? "label"}`} disabled={disabled} onClick={() => onChange(value.filter((x) => x !== id))}>
              ✕
            </button>
          </span>
        );
      })}
      <select className={`input ${styles.add}`} aria-label="Add a label" value="" disabled={disabled || !labels.length} onChange={(e) => e.target.value && onChange([...value, e.target.value])}>
        <option value="">{labels.length ? (value.length ? "Add…" : "No labels — add…") : "No labels in this project"}</option>
        {labels
          .filter((l) => !value.includes(l.id) && !groups.some((g) => g.id === l.id))
          .map((l) => (
            <option key={l.id} value={l.id}>
              {l.parent_id ? `${labels.find((g) => g.id === l.parent_id)?.name ?? ""} › ` : ""}
              {l.name}
            </option>
          ))}
      </select>
    </span>
  );
}

export function EstimatePick({ slug, projectId, value, disabled, onChange }: { slug: string; projectId: string; value: string | null; disabled?: boolean; onChange: (id: string) => void }) {
  const active = (usePmEstimates(slug, projectId).data ?? []).find((e) => e.active);
  if (!active) return <span className="ghostText">No estimates in this project</span>;
  return (
    <select className="input" aria-label="Estimate" value={value ?? ""} disabled={disabled} onChange={(e) => onChange(e.target.value)}>
      <option value="">None</option>
      {active.points.map((p) => (
        <option key={p.id} value={p.id}>
          {p.value}
        </option>
      ))}
    </select>
  );
}

/** A parent among the project's items (not this one). */
export function ParentPick({ slug, projectId, value, self, disabled, onChange }: { slug: string; projectId: string; value: string | null; self?: string; disabled?: boolean; onChange: (id: string) => void }) {
  const all = usePmItems(slug, projectId).data ?? [];
  // Not the item, nor anything under it at any depth (a loop).
  const under = (i: { parent_id: string | null }): boolean => {
    for (let p = i.parent_id, n = 0; p && n < 50; p = all.find((x) => x.id === p)?.parent_id ?? null, n++) if (p === self) return true;
    return false;
  };
  const items = all.filter((i) => i.id !== self && !under(i));
  return (
    <select className="input" aria-label="Parent" value={value ?? ""} disabled={disabled} onChange={(e) => onChange(e.target.value)}>
      <option value="">No parent</option>
      {items.map((i) => (
        <option key={i.id} value={i.id}>
          {i.key} {i.name}
        </option>
      ))}
    </select>
  );
}
