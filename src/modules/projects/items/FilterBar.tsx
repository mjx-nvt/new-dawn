import { usePmLabels, usePmStates } from "../../../kernel/bridge/pm";
import { usePeople } from "../../../kernel/bridge/people";
import type { PmRule } from "../../../kernel/bridge/pmViews";
import { useState } from "react";
import { dayKeyIn, formatDay } from "../../../kernel/format/format";
import MenuButton from "../../../kernel/shell/MenuButton";
import { whoName } from "../pm";
import { GROUP_LABEL, PRIORITIES } from "./items";
import { PeoplePick } from "./Props";
import styles from "./Items.module.css";

type Kind = "text" | "list" | "people" | "date";

/** Plane's filter fields; state and labels are a project's (not offered across projects). */
const FIELDS: { field: string; label: string; kind: Kind; project?: boolean }[] = [
  { field: "title", label: "Title", kind: "text" },
  { field: "state", label: "State", kind: "list", project: true },
  { field: "state_group", label: "State group", kind: "list" },
  { field: "priority", label: "Priority", kind: "list" },
  { field: "assignees", label: "Assignees", kind: "people" },
  { field: "labels", label: "Labels", kind: "list", project: true },
  { field: "created_by", label: "Created by", kind: "people" },
  { field: "subscribers", label: "Subscribed by", kind: "people" },
  { field: "mentions", label: "Mentions", kind: "people" },
  { field: "start", label: "Start date", kind: "date" },
  { field: "due", label: "Due date", kind: "date" },
  { field: "created", label: "Created on", kind: "date" },
  { field: "updated", label: "Updated on", kind: "date" },
];

const DATE_OPS: { op: string; label: string }[] = [
  { op: "is", label: "is" },
  { op: "not", label: "is not" },
  { op: "before", label: "is before" },
  { op: "after", label: "is after" },
  { op: "on_or_before", label: "is on or before" },
  { op: "on_or_after", label: "is on or after" },
  { op: "between", label: "is between" },
  { op: "empty", label: "is empty" },
  { op: "not_empty", label: "is not empty" },
];

const DAY_WORDS: { value: string; label: string }[] = [
  { value: "today", label: "Today" },
  { value: "tomorrow", label: "Tomorrow" },
  { value: "yesterday", label: "Yesterday" },
  { value: "this_week", label: "This week" },
  { value: "next_week", label: "Next week" },
  { value: "last_week", label: "Last week" },
  { value: "this_month", label: "This month" },
  { value: "next_month", label: "Next month" },
  { value: "last_month", label: "Last month" },
];

const isDay = (v: string) => /^\d{4}-\d{2}-\d{2}$/.test(v);
const dayLabel = (v: string) => DAY_WORDS.find((w) => w.value === v)?.label.toLowerCase() ?? (isDay(v) ? formatDay(v, { short: true }) : v);

/** A day value: a word (today, this week…) or a day picked. */
function DayValue({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  return (
    <span className={styles.inline}>
      <select className="input" aria-label="Day" value={isDay(value) ? "day" : value} onChange={(e) => onChange(e.target.value === "day" ? dayKeyIn(new Date()) : e.target.value)}>
        {DAY_WORDS.map((w) => (
          <option key={w.value} value={w.value}>
            {w.label}
          </option>
        ))}
        <option value="day">A day…</option>
      </select>
      {isDay(value) && <input className="input" type="date" aria-label="The day" value={value} onChange={(e) => e.target.value && onChange(e.target.value)} />}
    </span>
  );
}

/** The title's words, kept here while typed (Vietnamese is composed key by key) and given to the
 *  filter on Enter or on leaving the field. */
function TitleWords({ label, value, onChange }: { label: string; value: string; onChange: (v: string) => void }) {
  const [draft, setDraft] = useState(value);
  return (
    <input
      className="input"
      aria-label={`${label} contains`}
      autoFocus
      value={draft}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={() => draft !== value && onChange(draft)}
      onKeyDown={(e) => e.key === "Enter" && !e.nativeEvent.isComposing && e.currentTarget.blur()}
    />
  );
}

/** Plane's filter bar (D-077): Filters adds a rule a field; each rule a chip saying what it keeps
 *  ("Due date is this week"), opened to change it, × to take it off; Clear all. The rules all
 *  hold; a rule's values, any one. */
export default function FilterBar({ slug, projectId, rules, onChange }: { slug: string; projectId: string | null; rules: PmRule[]; onChange: (rules: PmRule[]) => void }) {
  const states = (usePmStates(slug, projectId ?? "").data ?? []).filter((s) => s.group !== "triage");
  const labels = usePmLabels(slug, projectId ?? "").data ?? [];
  const people = usePeople(slug).data ?? [];
  const fields = FIELDS.filter((f) => projectId || !f.project);
  const set = (at: number, rule: PmRule) => onChange(rules.map((r, n) => (n === at ? rule : r)));

  const options = (field: string): { value: string; label: string }[] => {
    switch (field) {
      case "state":
        return states.map((s) => ({ value: s.id, label: s.name }));
      case "state_group":
        return (["backlog", "unstarted", "started", "completed", "cancelled"] as const).map((g) => ({ value: g, label: GROUP_LABEL[g] }));
      case "priority":
        return PRIORITIES.map((p) => ({ value: p.key, label: p.label }));
      case "labels":
        return [...labels.filter((l) => !labels.some((c) => c.parent_id === l.id)).map((l) => ({ value: l.id, label: l.name })), { value: "none", label: "No label" }];
      default:
        return [];
    }
  };
  const valueLabel = (field: string, v: string) => {
    const kind = FIELDS.find((f) => f.field === field)?.kind;
    if (kind === "people") return v === "none" ? "no one" : whoName(v, people) || v;
    if (kind === "date") return dayLabel(v);
    return options(field).find((o) => o.value === v)?.label ?? v;
  };
  const says = (r: PmRule) => {
    const label = FIELDS.find((f) => f.field === r.field)?.label ?? r.field;
    const kind = FIELDS.find((f) => f.field === r.field)?.kind;
    if (kind === "text") return `${label} contains “${r.values[0] ?? ""}”`;
    if (kind === "date") {
      const how = DATE_OPS.find((o) => o.op === r.op)?.label ?? r.op;
      if (r.op === "empty" || r.op === "not_empty") return `${label} ${how}`;
      return `${label} ${how} ${r.values.map((v) => valueLabel(r.field, v)).join(" and ")}`;
    }
    const values = r.values.length ? r.values.map((v) => valueLabel(r.field, v)).join(", ") : "…";
    return `${label} ${r.op === "not_in" ? "is none of" : "is any of"} ${values}`;
  };

  const add = (field: string) => {
    const kind = FIELDS.find((f) => f.field === field)?.kind;
    onChange([...rules, { field, op: kind === "text" ? "contains" : kind === "date" ? "is" : "in", values: kind === "date" ? ["this_week"] : kind === "people" ? ["me"] : [] }]);
  };

  return (
    <div className={styles.filterBar} role="group" aria-label="Filters">
      <MenuButton label="Add a filter" items={fields.filter((f) => !rules.some((r) => r.field === f.field)).map((f) => ({ label: f.label, onSelect: () => add(f.field) }))}>
        Filters
      </MenuButton>
      {rules.map((r, at) => {
        const field = FIELDS.find((f) => f.field === r.field);
        return (
          <details key={`${r.field}-${at}`} className={styles.rule}>
            <summary className="chip">{says(r)}</summary>
            <div className={`glass ${styles.rulePanel}`}>
              {field?.kind === "text" && <TitleWords label={field.label} value={r.values[0] ?? ""} onChange={(v) => set(at, { ...r, values: [v] })} />}
              {(field?.kind === "list" || field?.kind === "people") && (
                <select className="input" aria-label={`How ${field.label} filters`} value={r.op} onChange={(e) => set(at, { ...r, op: e.target.value })}>
                  <option value="in">is any of</option>
                  <option value="not_in">is none of</option>
                </select>
              )}
              {field?.kind === "list" &&
                options(r.field).map((o) => (
                  <label key={o.value} className={styles.check}>
                    <input type="checkbox" checked={r.values.includes(o.value)} onChange={(e) => set(at, { ...r, values: e.target.checked ? [...r.values, o.value] : r.values.filter((v) => v !== o.value) })} /> {o.label}
                  </label>
                ))}
              {field?.kind === "people" && (
                <>
                  <PeoplePick slug={slug} label={field.label} value={r.values.filter((v) => v !== "none")} onChange={(who) => set(at, { ...r, values: [...who, ...r.values.filter((v) => v === "none")] })} />
                  {r.field === "assignees" && (
                    <label className={styles.check}>
                      <input type="checkbox" checked={r.values.includes("none")} onChange={(e) => set(at, { ...r, values: e.target.checked ? [...r.values, "none"] : r.values.filter((v) => v !== "none") })} /> No assignee
                    </label>
                  )}
                </>
              )}
              {field?.kind === "date" && (
                <>
                  <select className="input" aria-label={`How ${field.label} filters`} value={r.op} onChange={(e) => set(at, { ...r, op: e.target.value, values: e.target.value === "between" ? [r.values[0] ?? "today", r.values[1] ?? "next_week"] : r.values.slice(0, 1).concat(r.values.length ? [] : ["this_week"]) })}>
                    {DATE_OPS.map((o) => (
                      <option key={o.op} value={o.op}>
                        {o.label}
                      </option>
                    ))}
                  </select>
                  {r.op !== "empty" && r.op !== "not_empty" && <DayValue value={r.values[0] ?? "today"} onChange={(v) => set(at, { ...r, values: [v, ...r.values.slice(1)] })} />}
                  {r.op === "between" && <DayValue value={r.values[1] ?? "next_week"} onChange={(v) => set(at, { ...r, values: [r.values[0] ?? "today", v] })} />}
                </>
              )}
              <button type="button" className="btn btnGhost btnCompact btnDanger" onClick={() => onChange(rules.filter((_, n) => n !== at))}>
                Remove this filter
              </button>
            </div>
          </details>
        );
      })}
      {rules.length > 0 && (
        <button type="button" className="btn btnGhost btnCompact" onClick={() => onChange([])}>
          Clear all
        </button>
      )}
      {!rules.length && <span className="fieldHint">Every work item — add a filter to narrow it</span>}
    </div>
  );
}
