import { useState } from "react";
import { selectOptions, useObject, useObjects, type FieldMeta, type ObjectDef } from "../../kernel/bridge/records";
import { useApiStatus } from "../../kernel/bridge/api";
import { useAssistantAgents } from "../../kernel/bridge/assistant";
import { useOllamaStatus } from "../../kernel/bridge/summary";
import { CONDITION_OPS, STEP_TYPES, type Branch, type Condition, type Step, type Trigger } from "../../kernel/bridge/workflows";
import { kindOf, needsValue, OPS, subsOf } from "../crm/views/useViewState";
import OnOff from "../workspace-settings/OnOff";
import { runCode, type CodeOutcome } from "./codeWorker";
import s from "./Workflows.module.css";

/** A value the owner can put in a setting: `{{path}}`, grouped for the picker. */
export interface Variable {
  group: string;
  path: string;
  label: string;
}

/** What a workflow never watches nor writes: workflows and their runs (the engine refuses them). */
const OWN_OBJECTS = ["workflow", "workflowRun"];

export const EVENTS: [string, string][] = [
  ["CREATED", "is created"],
  ["UPDATED", "is updated"],
  ["CREATED_OR_UPDATED", "is created or updated"],
  ["DELETED", "is deleted"],
];

const UNITS: [string, string][] = [
  ["MINUTES", "minutes"],
  ["HOURS", "hours"],
  ["DAYS", "days"],
];

const FORM_TYPES: [string, string][] = [
  ["text", "Text"],
  ["number", "Number"],
  ["date", "Date"],
  ["select", "Select"],
];

export interface FormField {
  name: string;
  label: string;
  type: string;
  options?: string;
}

export function formFields(step: Step | undefined): FormField[] {
  const raw = step?.settings.fields;
  return Array.isArray(raw) ? (raw as FormField[]) : [];
}

/** A composite's parts as a step writes them (a currency's code too); none for a plain field. */
export function partsOf(field: FieldMeta): { key: string; label: string }[] {
  if (field.type === "CURRENCY")
    return [
      { key: "amountMicros", label: "Amount in micros" },
      { key: "currencyCode", label: "Currency code" },
    ];
  return subsOf(field);
}

function str(v: unknown): string {
  return typeof v === "string" ? v : typeof v === "number" ? String(v) : "";
}

export function Row({ label, htmlFor, children }: { label: string; htmlFor?: string; children: React.ReactNode }) {
  return (
    <div className={s.row}>
      {htmlFor ? <label htmlFor={htmlFor}>{label}</label> : <span className={s.rowLabel}>{label}</span>}
      <div className={s.stack}>{children}</div>
    </div>
  );
}

/** A text setting with the variables picker beside it: picking one appends its `{{path}}`. */
export function TemplateInput({ id, value, onChange, variables, placeholder, rows }: { id?: string; value: string; onChange: (v: string) => void; variables: Variable[]; placeholder?: string; rows?: number }) {
  const groups = [...new Set(variables.map((v) => v.group))];
  return (
    <div className={s.template}>
      {rows ? (
        <textarea id={id} className="input" rows={rows} value={value} placeholder={placeholder} onChange={(e) => onChange(e.target.value)} />
      ) : (
        <input id={id} className="input" value={value} placeholder={placeholder} onChange={(e) => onChange(e.target.value)} />
      )}
      <select className="input" aria-label="Insert a variable" title="Insert a variable" value="" onChange={(e) => e.target.value && onChange(`${value}{{${e.target.value}}}`)}>
        <option value="">{"{x}"}</option>
        {groups.map((g) => (
          <optgroup key={g} label={g}>
            {variables
              .filter((v) => v.group === g)
              .map((v) => (
                <option key={v.path} value={v.path}>
                  {v.label}
                </option>
              ))}
          </optgroup>
        ))}
      </select>
    </div>
  );
}

function ObjectSelect({ slug, id, value, onChange, exclude = [] }: { slug: string; id?: string; value: string; onChange: (v: string) => void; exclude?: string[] }) {
  const objects = useObjects(slug);
  return (
    <select id={id} className="input" value={value} onChange={(e) => onChange(e.target.value)}>
      <option value="">Pick an object…</option>
      {(objects.data ?? [])
        .filter((o) => !o.is_system && o.is_active && !OWN_OBJECTS.includes(o.name_singular) && !exclude.includes(o.name_singular))
        .map((o) => (
          <option key={o.id} value={o.name_singular}>
            {o.label_plural}
          </option>
        ))}
    </select>
  );
}

/** The fields a step can write: no system fields, no positions, no one-to-many relations. */
export function writableFields(def: ObjectDef | undefined): FieldMeta[] {
  return (def?.fields ?? []).filter((f) => f.is_active && !f.is_system && f.type !== "POSITION" && f.type !== "ACTOR" && !(f.type === "RELATION" && f.options.relation?.kind !== "MANY_TO_ONE"));
}

/** The field values a Create / Update / Upsert step writes: one row a field, typed by the
 *  field — a select for a Select, On/Off for a Boolean, a part each for a composite, text with
 *  variables for the rest (a relation takes a record's id). */
function FieldsEditor({ slug, object, values, onChange, variables }: { slug: string; object: string; values: Record<string, unknown>; onChange: (v: Record<string, unknown>) => void; variables: Variable[] }) {
  const def = useObject(slug, object || undefined).data;
  const fields = writableFields(def);
  const set = (name: string, v: unknown) => onChange({ ...values, [name]: v });
  const remove = (name: string) => {
    const next = { ...values };
    delete next[name];
    onChange(next);
  };
  const unset = fields.filter((f) => !(f.name in values));
  if (!object) return <p className="fieldHint">Pick the object first.</p>;
  return (
    <div className={s.blocks}>
      {Object.keys(values).map((name) => {
        const field = fields.find((f) => f.name === name);
        const v = values[name];
        return (
          <div key={name} className={s.block}>
            <span className={s.fieldName} title={name}>
              {field?.label ?? name}
            </span>
            <div className={s.fieldRow}>
              {field?.type === "SELECT" ? (
                <select className="input" aria-label={field.label} value={str(v)} onChange={(e) => set(name, e.target.value)}>
                  <option value="">—</option>
                  {selectOptions(field).map((o) => (
                    <option key={o.value} value={o.value}>
                      {o.label}
                    </option>
                  ))}
                </select>
              ) : field?.type === "BOOLEAN" ? (
                <OnOff label={field.label} value={v === true} onChange={(b) => set(name, b)} />
              ) : field && partsOf(field).length > 0 ? (
                <div className={s.stack}>
                  {partsOf(field).map((sub) => {
                    const part = v && typeof v === "object" ? (v as Record<string, unknown>) : {};
                    return <TemplateInput key={sub.key} value={str(part[sub.key])} placeholder={sub.label} variables={variables} onChange={(t) => set(name, { ...part, [sub.key]: t })} />;
                  })}
                </div>
              ) : (
                <TemplateInput value={str(v)} variables={variables} placeholder={field?.type === "RELATION" ? "A record's id, as {{…}}" : field?.type === "DATE" ? "YYYY-MM-DD or {{today+3d}}" : undefined} onChange={(t) => set(name, t)} />
              )}
              <button type="button" className="btn btnGhost btnCompact" aria-label={`Remove ${field?.label ?? name}`} onClick={() => remove(name)}>
                ✕
              </button>
            </div>
          </div>
        );
      })}
      {unset.length > 0 && (
        <select className="input" aria-label="Add a field" value="" onChange={(e) => e.target.value && set(e.target.value, "")}>
          <option value="">Add a field…</option>
          {unset.map((f) => (
            <option key={f.id} value={f.name}>
              {f.label}
            </option>
          ))}
        </select>
      )}
    </div>
  );
}

interface Target {
  object: string;
  id: string;
}

/** The records a new task or note is attached to (Twenty's targets). */
function TargetsEditor({ slug, targets, onChange, variables }: { slug: string; targets: Target[]; onChange: (t: Target[]) => void; variables: Variable[] }) {
  const edit = (i: number, patch: Partial<Target>) => onChange(targets.map((t, j) => (j === i ? { ...t, ...patch } : t)));
  return (
    <div className={s.stack}>
      {targets.map((t, i) => (
        <div key={i} className={s.inline}>
          <ObjectSelect slug={slug} value={t.object} exclude={["task", "note", "workflow", "workflowRun", "dashboard"]} onChange={(object) => edit(i, { object })} />
          <TemplateInput value={t.id} placeholder="{{trigger.record.id}}" variables={variables} onChange={(id) => edit(i, { id })} />
          <button type="button" className="btn btnGhost btnCompact" aria-label="Remove target" onClick={() => onChange(targets.filter((_, j) => j !== i))}>
            ✕
          </button>
        </div>
      ))}
      <button type="button" className="btn btnGhost btnCompact" onClick={() => onChange([...targets, { object: "person", id: "" }])}>
        + Attach to a record
      </button>
    </div>
  );
}

/** Conditions of a Filter or an If/else route: left, operator, right — all with variables. */
function ConditionsEditor({ conditions, logic, onChange, variables }: { conditions: Condition[]; logic: string; onChange: (c: Condition[], logic: string) => void; variables: Variable[] }) {
  const edit = (i: number, patch: Partial<Condition>) =>
    onChange(
      conditions.map((c, j) => (j === i ? { ...c, ...patch } : c)),
      logic,
    );
  return (
    <div className={s.blocks}>
      {conditions.map((c, i) => (
        <div key={i} className={s.block}>
          <div className={s.inline}>
            <TemplateInput value={c.left} placeholder="{{trigger.record.stage}}" variables={variables} onChange={(left) => edit(i, { left })} />
            <button
              type="button"
              className="btn btnGhost btnCompact"
              aria-label="Remove condition"
              onClick={() =>
                onChange(
                  conditions.filter((_, j) => j !== i),
                  logic,
                )
              }
            >
              ✕
            </button>
          </div>
          <div className={s.inline}>
            <select className="input fit" aria-label="Operator" value={c.op} onChange={(e) => edit(i, { op: e.target.value })}>
              {CONDITION_OPS.map(([v, l]) => (
                <option key={v} value={v}>
                  {l}
                </option>
              ))}
            </select>
            {c.op === "isEmpty" || c.op === "isNotEmpty" ? null : <TemplateInput value={c.right} placeholder="CUSTOMER" variables={variables} onChange={(right) => edit(i, { right })} />}
          </div>
        </div>
      ))}
      <div className={s.inline}>
        <button type="button" className="btn btnGhost btnCompact" onClick={() => onChange([...conditions, { left: "", op: "is", right: "" }], logic)}>
          + Add a condition
        </button>
        {conditions.length > 1 && (
          <div className="segmented" role="group" aria-label="All or any">
            {(["AND", "OR"] as const).map((l) => (
              <button key={l} type="button" aria-pressed={logic === l} onClick={() => onChange(conditions, l)}>
                {l === "AND" ? "All" : "Any"}
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

interface SearchRule {
  field: string;
  op: string;
  value: unknown;
}

function searchRules(filter: unknown): SearchRule[] {
  const and = filter && typeof filter === "object" ? (filter as { and?: unknown }).and : undefined;
  return Array.isArray(and) ? (and as SearchRule[]) : [];
}

/** A Search step's filter: the view bar's rules on the object's fields, values with variables. */
function SearchFilterEditor({ slug, object, filter, onChange, variables }: { slug: string; object: string; filter: unknown; onChange: (filter: unknown) => void; variables: Variable[] }) {
  const def = useObject(slug, object || undefined).data;
  const fields = (def?.fields ?? []).filter((f) => f.is_active && f.type !== "POSITION" && f.type !== "ACTOR" && f.type !== "RICH_TEXT" && !(f.type === "RELATION" && f.options.relation?.kind !== "MANY_TO_ONE"));
  const rules = searchRules(filter);
  const emit = (next: SearchRule[]) =>
    onChange(
      next.length
        ? {
            and: next.map((r) => ({
              field: r.field,
              sub: null,
              op: r.op,
              value: r.value,
            })),
          }
        : null,
    );
  const edit = (i: number, patch: Partial<SearchRule>) => emit(rules.map((r, j) => (j === i ? { ...r, ...patch } : r)));
  if (!object) return <p className="fieldHint">Pick the object first.</p>;
  return (
    <div className={s.blocks}>
      {rules.map((r, i) => {
        const field = fields.find((f) => f.name === r.field);
        const ops = field ? OPS[kindOf(field)] : [];
        return (
          <div key={i} className={s.block}>
            <div className={s.inline}>
              <select className="input" aria-label="Field" value={r.field} onChange={(e) => edit(i, { field: e.target.value, op: "is", value: "" })}>
                <option value="">Field…</option>
                {fields.map((f) => (
                  <option key={f.id} value={f.name}>
                    {f.label}
                  </option>
                ))}
              </select>
              <button type="button" className="btn btnGhost btnCompact" aria-label="Remove rule" onClick={() => emit(rules.filter((_, j) => j !== i))}>
                ✕
              </button>
            </div>
            <div className={s.inline}>
              <select className="input fit" aria-label="Operator" value={r.op} onChange={(e) => edit(i, { op: e.target.value })}>
                {ops.map((o) => (
                  <option key={o.op} value={o.op}>
                    {o.label}
                  </option>
                ))}
              </select>
              {needsValue(r.op) ? (
                field?.type === "SELECT" ? (
                  <select className="input" aria-label="Value" value={str(r.value)} onChange={(e) => edit(i, { value: e.target.value })}>
                    <option value="">—</option>
                    {selectOptions(field).map((o) => (
                      <option key={o.value} value={o.value}>
                        {o.label}
                      </option>
                    ))}
                  </select>
                ) : (
                  <TemplateInput value={str(r.value)} variables={variables} onChange={(value) => edit(i, { value })} />
                )
              ) : null}
            </div>
          </div>
        );
      })}
      <button type="button" className="btn btnGhost btnCompact" onClick={() => emit([...rules, { field: fields[0]?.name ?? "", op: "is", value: "" }])}>
        + Add a rule
      </button>
    </div>
  );
}

/** The trigger's settings: what starts a run. */
export function TriggerSettings({ slug, workflowId, trigger, onChange }: { slug: string; workflowId: string; trigger: Trigger | null; onChange: (t: Trigger | null) => void }) {
  const api = useApiStatus().data;
  const t = trigger;
  const object = useObject(slug, t?.object || undefined).data;
  const set = (patch: Partial<Trigger>) => onChange({ ...(t ?? { type: "" }), ...patch });
  const watched = t?.fields ?? [];
  return (
    <div className={s.settings}>
      <Row label="Trigger" htmlFor="wf-trigger">
        <select
          id="wf-trigger"
          className="input"
          value={t?.type ?? ""}
          onChange={(e) =>
            onChange(
              e.target.value
                ? {
                    type: e.target.value,
                    ...(e.target.value === "MANUAL" ? { availability: "GLOBAL" } : e.target.value === "SCHEDULE" ? { every: 1, unit: "DAYS" } : e.target.value === "RECORD" ? { event: "CREATED" } : {}),
                  }
                : null,
            )
          }
        >
          <option value="">Choose…</option>
          <option value="RECORD">A record changes</option>
          <option value="MANUAL">By hand</option>
          <option value="SCHEDULE">On a schedule</option>
          <option value="WEBHOOK">A webhook</option>
        </select>
      </Row>
      {t?.type === "RECORD" && (
        <>
          <Row label="Object" htmlFor="wf-object">
            <ObjectSelect slug={slug} id="wf-object" value={t.object ?? ""} onChange={(object) => set({ object, fields: [] })} />
          </Row>
          <Row label="When it" htmlFor="wf-event">
            <select id="wf-event" className="input" value={t.event ?? "CREATED"} onChange={(e) => set({ event: e.target.value })}>
              {EVENTS.map(([v, l]) => (
                <option key={v} value={v}>
                  {l}
                </option>
              ))}
            </select>
          </Row>
          {(t.event === "UPDATED" || t.event === "CREATED_OR_UPDATED") && (
            <Row label="Fields watched">
              <div className={s.chips}>
                {writableFields(object).map((f) => (
                  <button
                    key={f.id}
                    type="button"
                    className={s.chipPick}
                    aria-pressed={watched.includes(f.name)}
                    onClick={() =>
                      set({
                        fields: watched.includes(f.name) ? watched.filter((n) => n !== f.name) : [...watched, f.name],
                      })
                    }
                  >
                    {f.label}
                  </button>
                ))}
              </div>
              <p className="fieldHint">{watched.length ? "A run starts when one of these changes." : "None picked: any change starts a run."}</p>
            </Row>
          )}
        </>
      )}
      {t?.type === "MANUAL" && (
        <>
          <Row label="Available" htmlFor="wf-availability">
            <select
              id="wf-availability"
              className="input"
              value={t.availability ?? "GLOBAL"}
              onChange={(e) =>
                set({
                  availability: e.target.value,
                  ...(e.target.value === "GLOBAL" ? { object: "" } : {}),
                })
              }
            >
              <option value="GLOBAL">Everywhere (⌘K)</option>
              <option value="SINGLE">On a record — once a record</option>
              <option value="BULK">On records — once with them all</option>
            </select>
          </Row>
          {t.availability && t.availability !== "GLOBAL" && (
            <Row label="Object" htmlFor="wf-manual-object">
              <ObjectSelect slug={slug} id="wf-manual-object" value={t.object ?? ""} onChange={(object) => set({ object })} />
            </Row>
          )}
        </>
      )}
      {t?.type === "SCHEDULE" && (
        <>
          <Row label="Every" htmlFor="wf-every">
            <div className={s.inline}>
              <input id="wf-every" className="input narrow" type="number" min={1} value={t.every ?? 1} onChange={(e) => set({ every: Math.max(1, Number(e.target.value) || 1) })} />
              <select className="input" aria-label="Unit" value={t.unit ?? "DAYS"} onChange={(e) => set({ unit: e.target.value })}>
                {UNITS.map(([v, l]) => (
                  <option key={v} value={v}>
                    {l}
                  </option>
                ))}
              </select>
            </div>
          </Row>
          <Row label="Or a cron" htmlFor="wf-cron">
            <input id="wf-cron" className="input" value={t.cron ?? ""} placeholder="0 9 * * 1" onChange={(e) => set({ cron: e.target.value })} />
            <p className="fieldHint">minute hour day month weekday, in this Mac's time; a cron replaces the interval.</p>
          </Row>
        </>
      )}
      {t?.type === "WEBHOOK" && (
        <Row label="Address">
          {api?.running ? (
            <>
              <code className={s.address}>{`${api.address}/webhooks/workflows/${slug}/${workflowId}`}</code>
              <p className="fieldHint">POST a JSON body there from this Mac once the workflow is active: the body is the run's trigger data ({"{{trigger.field}}"}).</p>
            </>
          ) : (
            <p className="fieldHint">Turn the local API on (Workspace → CRM → Developers) and the address shows here.</p>
          )}
        </Row>
      )}
    </div>
  );
}

/** A step's settings, by its type. */
export function StepSettings({ slug, step, variables, onChange }: { slug: string; step: Step; variables: Variable[]; onChange: (s: Step) => void }) {
  const set = (patch: Record<string, unknown>) => onChange({ ...step, settings: { ...step.settings, ...patch } });
  const st = step.settings;
  const object = str(st.object);
  const def = useObject(slug, object || undefined).data;
  const fields = st.fields && typeof st.fields === "object" && !Array.isArray(st.fields) ? (st.fields as Record<string, unknown>) : {};
  const targets = Array.isArray(st.targets) ? (st.targets as Target[]) : [];
  const conditions = Array.isArray(st.conditions) ? (st.conditions as Condition[]) : [];
  const branches = step.branches ?? [];
  const kind = step.type;
  const recordStep = ["CREATE_RECORD", "UPDATE_RECORD", "DELETE_RECORD", "SEARCH_RECORDS", "UPSERT_RECORD"].includes(kind);
  const identifiers = (def?.fields ?? []).filter(
    (f) => f.is_active && (f.is_unique || f.id === def?.meta.label_identifier_field_id || ["emails", "domainName"].includes(f.name)) && ["TEXT", "FULL_NAME", "EMAILS", "DOMAIN", "LINKS", "UUID"].includes(f.type),
  );
  const editBranch = (i: number, patch: Partial<Branch>) =>
    onChange({
      ...step,
      branches: branches.map((b, j) => (j === i ? { ...b, ...patch } : b)),
    });
  return (
    <div className={s.settings}>
      <Row label="Name" htmlFor="wf-step-name">
        <input id="wf-step-name" className="input" value={step.name} placeholder={STEP_TYPES.find((t) => t.type === kind)?.label} onChange={(e) => onChange({ ...step, name: e.target.value })} />
      </Row>
      {recordStep && (
        <Row label="Object" htmlFor="wf-step-object">
          <ObjectSelect
            slug={slug}
            id="wf-step-object"
            value={object}
            onChange={(o) =>
              set({
                object: o,
                fields: {},
                targets: [],
                filter: null,
                identifier: "id",
              })
            }
          />
        </Row>
      )}
      {(kind === "UPDATE_RECORD" || kind === "DELETE_RECORD") && (
        <Row label="Record" htmlFor="wf-step-record">
          <TemplateInput id="wf-step-record" value={str(st.record_id)} placeholder="{{trigger.record.id}}" variables={variables} onChange={(record_id) => set({ record_id })} />
        </Row>
      )}
      {kind === "UPSERT_RECORD" && (
        <Row label="Match by" htmlFor="wf-step-identifier">
          <select id="wf-step-identifier" className="input" value={str(st.identifier) || "id"} onChange={(e) => set({ identifier: e.target.value })}>
            <option value="id">Id</option>
            {identifiers.map((f) => (
              <option key={f.id} value={f.name}>
                {f.label}
              </option>
            ))}
          </select>
        </Row>
      )}
      {(kind === "CREATE_RECORD" || kind === "UPDATE_RECORD" || kind === "UPSERT_RECORD") && (
        <Row label="Fields">
          <FieldsEditor slug={slug} object={object} values={fields} variables={variables} onChange={(v) => set({ fields: v })} />
        </Row>
      )}
      {(kind === "CREATE_RECORD" || kind === "UPSERT_RECORD") && (object === "task" || object === "note") && (
        <Row label="Attached to">
          <TargetsEditor slug={slug} targets={targets} variables={variables} onChange={(t) => set({ targets: t })} />
        </Row>
      )}
      {kind === "SEARCH_RECORDS" && (
        <>
          <Row label="Where">
            <SearchFilterEditor slug={slug} object={object} filter={st.filter} variables={variables} onChange={(filter) => set({ filter })} />
          </Row>
          <Row label="At most" htmlFor="wf-step-limit">
            <input
              id="wf-step-limit"
              className="input narrow"
              type="number"
              min={1}
              max={200}
              value={typeof st.limit === "number" ? st.limit : 200}
              onChange={(e) =>
                set({
                  limit: Math.min(200, Math.max(1, Number(e.target.value) || 1)),
                })
              }
            />
          </Row>
        </>
      )}
      {kind === "FILTER" && (
        <Row label="Go on when">
          <ConditionsEditor conditions={conditions} logic={str(st.logic) || "AND"} variables={variables} onChange={(c, logic) => set({ conditions: c, logic })} />
        </Row>
      )}
      {kind === "IF_ELSE" && (
        <>
          {branches.map((b, i) => (
            <Row key={i} label={i === 0 ? "If" : "Else if"}>
              <div className={s.inline}>
                <input className="input" aria-label="Route name" value={b.name} placeholder={`Route ${i + 1}`} onChange={(e) => editBranch(i, { name: e.target.value })} />
                {branches.length > 1 && (
                  <button
                    type="button"
                    className="btn btnGhost btnCompact"
                    aria-label="Remove route"
                    onClick={() =>
                      onChange({
                        ...step,
                        branches: branches.filter((_, j) => j !== i),
                      })
                    }
                  >
                    ✕
                  </button>
                )}
              </div>
              <ConditionsEditor conditions={b.conditions} logic={b.logic || "AND"} variables={variables} onChange={(c, logic) => editBranch(i, { conditions: c, logic })} />
            </Row>
          ))}
          <Row label="">
            <button
              type="button"
              className="btn btnGhost btnCompact"
              onClick={() =>
                onChange({
                  ...step,
                  branches: [
                    ...branches,
                    {
                      name: `Route ${branches.length + 1}`,
                      conditions: [],
                      logic: "AND",
                      steps: [],
                    },
                  ],
                })
              }
            >
              + Add a route
            </button>
            <p className="fieldHint">Else takes the runs no route holds for.</p>
          </Row>
        </>
      )}
      {kind === "ITERATOR" && (
        <Row label="Items" htmlFor="wf-step-items">
          <TemplateInput id="wf-step-items" value={str(st.items)} placeholder="{{s1}} — a search's records" variables={variables} onChange={(items) => set({ items })} />
          <p className="fieldHint">
            The inner steps run once an item: {"{{iterator.currentItem}}"}, {"{{iterator.index}}"}.
          </p>
        </Row>
      )}
      {kind === "DELAY" && (
        <>
          <Row label="Wait" htmlFor="wf-step-amount">
            <div className={s.inline}>
              <input id="wf-step-amount" className="input narrow" type="number" min={0} value={typeof st.amount === "number" ? st.amount : 0} onChange={(e) => set({ amount: Math.max(0, Number(e.target.value) || 0) })} />
              <select className="input" aria-label="Unit" value={str(st.unit) || "MINUTES"} onChange={(e) => set({ unit: e.target.value })}>
                {UNITS.map(([v, l]) => (
                  <option key={v} value={v}>
                    {l}
                  </option>
                ))}
              </select>
            </div>
          </Row>
          <Row label="Or until" htmlFor="wf-step-until">
            <TemplateInput id="wf-step-until" value={str(st.until)} placeholder="{{trigger.record.closeDate}}" variables={variables} onChange={(until) => set({ until })} />
            <p className="fieldHint">A date (YYYY-MM-DD) or a date and time; it replaces the wait.</p>
          </Row>
        </>
      )}
      {kind === "FORM" && (
        <>
          <Row label="Title" htmlFor="wf-step-title">
            <input id="wf-step-title" className="input" value={str(st.title)} onChange={(e) => set({ title: e.target.value })} />
          </Row>
          <Row label="Instructions" htmlFor="wf-step-instructions">
            <input id="wf-step-instructions" className="input" value={str(st.instructions)} onChange={(e) => set({ instructions: e.target.value })} />
          </Row>
          <Row label="Fields">
            <div className={s.stack}>
              {formFields(step).map((f, i) => {
                const edit = (patch: Partial<FormField>) =>
                  set({
                    fields: formFields(step).map((x, j) => (j === i ? { ...x, ...patch } : x)),
                  });
                return (
                  <div key={i} className={s.stack}>
                    <div className={s.inline}>
                      <input className="input" aria-label="Field name" value={f.name} placeholder="name" onChange={(e) => edit({ name: e.target.value.replace(/[^\w]/g, "") })} />
                      <input className="input" aria-label="Field label" value={f.label} placeholder="Label" onChange={(e) => edit({ label: e.target.value })} />
                      <select className="input" aria-label="Field type" value={f.type} onChange={(e) => edit({ type: e.target.value })}>
                        {FORM_TYPES.map(([v, l]) => (
                          <option key={v} value={v}>
                            {l}
                          </option>
                        ))}
                      </select>
                      <button
                        type="button"
                        className="btn btnGhost btnCompact"
                        aria-label="Remove field"
                        onClick={() =>
                          set({
                            fields: formFields(step).filter((_, j) => j !== i),
                          })
                        }
                      >
                        ✕
                      </button>
                    </div>
                    {f.type === "select" && <input className="input" aria-label="Options" value={f.options ?? ""} placeholder="Options, comma-separated" onChange={(e) => edit({ options: e.target.value })} />}
                  </div>
                );
              })}
              <button
                type="button"
                className="btn btnGhost btnCompact"
                onClick={() =>
                  set({
                    fields: [
                      ...formFields(step),
                      {
                        name: `field${formFields(step).length + 1}`,
                        label: "",
                        type: "text",
                      },
                    ],
                  })
                }
              >
                + Add a field
              </button>
              <p className="fieldHint">The answers: {"{{" + step.id + ".answers.<name>}}"}.</p>
            </div>
          </Row>
        </>
      )}
      {kind === "AI_AGENT" && <AiSettings slug={slug} step={step} set={set} variables={variables} />}
      {kind === "SEND_EMAIL" && (
        <>
          <Row label="To" htmlFor="wf-mail-to">
            <TemplateInput id="wf-mail-to" value={str(st.to)} placeholder="{{trigger.record.emails.primaryEmail}}" variables={variables} onChange={(to) => set({ to })} />
          </Row>
          <Row label="Cc" htmlFor="wf-mail-cc">
            <TemplateInput id="wf-mail-cc" value={str(st.cc)} variables={variables} onChange={(cc) => set({ cc })} />
          </Row>
          <Row label="Subject" htmlFor="wf-mail-subject">
            <TemplateInput id="wf-mail-subject" value={str(st.subject)} variables={variables} onChange={(subject) => set({ subject })} />
          </Row>
          <Row label="Body" htmlFor="wf-mail-body">
            <TemplateInput id="wf-mail-body" rows={6} value={str(st.body)} variables={variables} onChange={(body) => set({ body })} />
            <p className="fieldHint">The run makes the draft; its page opens it in Mail, where you press Send.</p>
          </Row>
        </>
      )}
      {kind === "HTTP_REQUEST" && <HttpSettings step={step} set={set} variables={variables} />}
      {kind === "CODE" && <CodeSettings step={step} set={set} variables={variables} />}
    </div>
  );
}

type Patch = (patch: Record<string, unknown>) => void;

/** The AI step: a prompt with variables on the local model; outputs make it answer in JSON. */
function AiSettings({ slug, step, set, variables }: { slug: string; step: Step; set: Patch; variables: Variable[] }) {
  const ollama = useOllamaStatus();
  const agents = useAssistantAgents(slug);
  const st = step.settings;
  const outputs = Array.isArray(st.outputs) ? (st.outputs as { name: string; type: string }[]) : [];
  const edit = (i: number, patch: Partial<{ name: string; type: string }>) => set({ outputs: outputs.map((o, j) => (j === i ? { ...o, ...patch } : o)) });
  return (
    <>
      <Row label="Prompt" htmlFor="wf-ai-prompt">
        <TemplateInput id="wf-ai-prompt" rows={6} value={str(st.prompt)} placeholder="Two lines on {{trigger.record.name}} for the weekly review" variables={variables} onChange={(prompt) => set({ prompt })} />
      </Row>
      <Row label="Agent" htmlFor="wf-ai-agent">
        <select id="wf-ai-agent" className="input" value={str(st.agent)} onChange={(e) => set({ agent: e.target.value })}>
          <option value="">None — the model alone</option>
          {(agents.data ?? []).map((a) => (
            <option key={a.id} value={a.id}>
              {a.name}
            </option>
          ))}
        </select>
        <p className="fieldHint">An agent brings its instructions, its model and its lookups (find, count, open — never a change).</p>
      </Row>
      {!str(st.agent) && (
        <Row label="Model" htmlFor="wf-ai-model">
          <select id="wf-ai-model" className="input" value={str(st.model)} onChange={(e) => set({ model: e.target.value })}>
            <option value="">App default{ollama.data?.model ? ` (${ollama.data.model})` : ""}</option>
            {(ollama.data?.installed ?? []).map((m) => (
              <option key={m.name} value={m.name}>
                {m.name}
              </option>
            ))}
          </select>
          {ollama.data && !ollama.data.running && <p className="fieldHint">Ollama isn't running: a run stops at this step until it is, then Retry.</p>}
        </Row>
      )}
      <Row label="Outputs">
        <div className={s.blocks}>
          {outputs.map((o, i) => (
            <div key={i} className={s.inline}>
              <input className="input" aria-label="Output name" value={o.name} placeholder="name" onChange={(e) => edit(i, { name: e.target.value.replace(/[^\w]/g, "") })} />
              <select className="input fit" aria-label="Output type" value={o.type} onChange={(e) => edit(i, { type: e.target.value })}>
                <option value="text">Text</option>
                <option value="number">Number</option>
                <option value="boolean">True / false</option>
              </select>
              <button type="button" className="btn btnGhost btnCompact" aria-label="Remove output" onClick={() => set({ outputs: outputs.filter((_, j) => j !== i) })}>
                ✕
              </button>
            </div>
          ))}
          <button type="button" className="btn btnGhost btnCompact" onClick={() => set({ outputs: [...outputs, { name: `value${outputs.length + 1}`, type: "text" }] })}>
            + Add an output
          </button>
          <p className="fieldHint">
            The answer is {"{{" + step.id + ".text}}"}; with outputs the model answers in JSON and each is {"{{" + step.id + ".<name>}}"} (an output can't be called text or model).
          </p>
        </div>
      </Row>
    </>
  );
}

const METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE"];

/** The HTTP step: an address on this Mac, a method, headers and a body with variables. */
function HttpSettings({ step, set, variables }: { step: Step; set: Patch; variables: Variable[] }) {
  const st = step.settings;
  const method = str(st.method) || "GET";
  const headers = Array.isArray(st.headers) ? (st.headers as { key: string; value: string }[]) : [];
  const edit = (i: number, patch: Partial<{ key: string; value: string }>) => set({ headers: headers.map((h, j) => (j === i ? { ...h, ...patch } : h)) });
  return (
    <>
      <Row label="Method" htmlFor="wf-http-method">
        <select id="wf-http-method" className="input" value={method} onChange={(e) => set({ method: e.target.value })}>
          {METHODS.map((m) => (
            <option key={m} value={m}>
              {m}
            </option>
          ))}
        </select>
      </Row>
      <Row label="URL" htmlFor="wf-http-url">
        <TemplateInput id="wf-http-url" value={str(st.url)} placeholder="http://localhost:3000/hook" variables={variables} onChange={(url) => set({ url })} />
        <p className="fieldHint">This Mac only (localhost) — any other address is refused. 30 s, 10 MB.</p>
      </Row>
      <Row label="Headers">
        <div className={s.blocks}>
          {headers.map((h, i) => (
            <div key={i} className={s.inline}>
              <input className="input" aria-label="Header name" value={h.key} placeholder="Authorization" onChange={(e) => edit(i, { key: e.target.value })} />
              <TemplateInput value={h.value} variables={variables} onChange={(value) => edit(i, { value })} />
              <button type="button" className="btn btnGhost btnCompact" aria-label="Remove header" onClick={() => set({ headers: headers.filter((_, j) => j !== i) })}>
                ✕
              </button>
            </div>
          ))}
          <button type="button" className="btn btnGhost btnCompact" onClick={() => set({ headers: [...headers, { key: "", value: "" }] })}>
            + Add a header
          </button>
        </div>
      </Row>
      {method !== "GET" && (
        <Row label="Body" htmlFor="wf-http-body">
          <TemplateInput id="wf-http-body" rows={5} value={str(st.body)} placeholder='{"name": "{{trigger.record.name}}"}' variables={variables} onChange={(body) => set({ body })} />
        </Row>
      )}
      <p className="fieldHint">
        The answer: {"{{" + step.id + ".status}}"} and {"{{" + step.id + ".body}}"} (its JSON, or its text). An answer that isn't 2xx stops the run.
      </p>
    </>
  );
}

export const DEFAULT_CODE = `export const main = async (params) => {
  const { name } = params;
  return { greeting: \`Hello, \${name}\` };
};
`;

/** The code step: JavaScript run in the window (a Web Worker), its inputs with variables; Test
 *  runs it with the inputs as typed. */
function CodeSettings({ step, set, variables }: { step: Step; set: Patch; variables: Variable[] }) {
  const st = step.settings;
  const inputs = st.inputs && typeof st.inputs === "object" && !Array.isArray(st.inputs) ? (st.inputs as Record<string, unknown>) : {};
  const entries = Object.entries(inputs);
  const [tried, setTried] = useState<CodeOutcome | null>(null);
  const [trying, setTrying] = useState(false);
  const rename = (i: number, name: string) => set({ inputs: Object.fromEntries(entries.map(([k, v], j) => (j === i ? [name, v] : [k, v]))) });
  const test = async () => {
    setTrying(true);
    setTried(await runCode(str(st.code), inputs));
    setTrying(false);
  };
  return (
    <>
      <Row label="Inputs">
        <div className={s.blocks}>
          {entries.map(([name, value], i) => (
            <div key={i} className={s.inline}>
              <input className="input narrow" aria-label="Input name" value={name} onChange={(e) => rename(i, e.target.value.replace(/[^\w]/g, ""))} />
              <TemplateInput value={str(value)} variables={variables} onChange={(v) => set({ inputs: { ...inputs, [name]: v } })} />
              <button type="button" className="btn btnGhost btnCompact" aria-label={`Remove ${name}`} onClick={() => set({ inputs: Object.fromEntries(entries.filter((_, j) => j !== i)) })}>
                ✕
              </button>
            </div>
          ))}
          <button type="button" className="btn btnGhost btnCompact" onClick={() => set({ inputs: { ...inputs, [`input${entries.length + 1}`]: "" } })}>
            + Add an input
          </button>
        </div>
      </Row>
      <Row label="Code" htmlFor="wf-code">
        <textarea id="wf-code" className={`input ${s.code}`} rows={12} spellCheck={false} value={str(st.code)} onChange={(e) => set({ code: e.target.value })} />
        <p className="fieldHint">JavaScript: main gets the inputs and returns an object — {"{{" + step.id + ".<key>}}"} after. It runs in New Dawn's window, without the network, 10 s at most.</p>
      </Row>
      <div className={s.inline}>
        <button type="button" className="btn btnSecondary btnCompact" disabled={trying} onClick={() => void test()}>
          Test
        </button>
        <span className="fieldHint">With the inputs as typed</span>
      </div>
      {tried && (
        <pre className={s.json} role="status">
          {tried.ok ? JSON.stringify(tried.output, null, 2) : tried.error}
        </pre>
      )}
    </>
  );
}

/** One line under a card's title. */
export function summarize(step: Step): string {
  const st = step.settings;
  const object = str(st.object);
  switch (step.type) {
    case "CREATE_RECORD":
      return object ? `A new ${object} with ${Object.keys(st.fields && typeof st.fields === "object" ? (st.fields as object) : {}).length} field(s)` : "Pick an object";
    case "UPDATE_RECORD":
      return object ? `${object} ${str(st.record_id) || "(which?)"}` : "Pick an object";
    case "DELETE_RECORD":
      return object ? `${object} ${str(st.record_id) || "(which?)"}` : "Pick an object";
    case "SEARCH_RECORDS":
      return object ? `${object}, ${searchRules(st.filter).length} rule(s)` : "Pick an object";
    case "UPSERT_RECORD":
      return object ? `${object} by ${str(st.identifier) || "id"}` : "Pick an object";
    case "FILTER":
      return `${Array.isArray(st.conditions) ? st.conditions.length : 0} condition(s)`;
    case "IF_ELSE":
      return `${(step.branches ?? []).length} route(s) and Else`;
    case "ITERATOR":
      return str(st.items) || "Pick the items";
    case "DELAY":
      return str(st.until) ? `Until ${str(st.until)}` : `${typeof st.amount === "number" ? st.amount : 0} ${(str(st.unit) || "MINUTES").toLowerCase()}`;
    case "FORM":
      return str(st.title) || `${formFields(step).length} field(s)`;
    case "AI_AGENT":
      return str(st.prompt).slice(0, 80) || "Write the prompt";
    case "SEND_EMAIL":
      return str(st.to) ? `To ${str(st.to)}` : "Pick the address";
    case "HTTP_REQUEST":
      return str(st.url) ? `${str(st.method) || "GET"} ${str(st.url)}` : "Pick the address";
    case "CODE":
      return `${Object.keys(st.inputs && typeof st.inputs === "object" ? (st.inputs as object) : {}).length} input(s)`;
    default:
      return "";
  }
}

export function triggerSummary(t: Trigger | null): string {
  if (!t) return "Record, by hand, a schedule or a webhook";
  switch (t.type) {
    case "RECORD":
      return `${t.object || "An object"} ${EVENTS.find(([v]) => v === t.event)?.[1] ?? "changes"}${t.fields?.length ? ` · ${t.fields.join(", ")}` : ""}`;
    case "MANUAL":
      return t.availability === "SINGLE" ? `On a ${t.object || "record"}` : t.availability === "BULK" ? `On ${t.object || "record"}s` : "Everywhere, from ⌘K";
    case "SCHEDULE":
      return t.cron ? `Cron ${t.cron}` : `Every ${t.every ?? 1} ${(t.unit ?? "DAYS").toLowerCase()}`;
    case "WEBHOOK":
      return "A webhook — with the local API";
    default:
      return "";
  }
}

export function triggerTitle(t: Trigger | null): string {
  if (!t) return "Choose a trigger";
  return (
    {
      RECORD: "A record changes",
      MANUAL: "By hand",
      SCHEDULE: "On a schedule",
      WEBHOOK: "A webhook",
    }[t.type] ?? t.type
  );
}
