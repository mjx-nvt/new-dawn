import { Link, useNavigate } from "@tanstack/react-router";
import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { asRecord, recordId, useRecords, useSearchRecords, type FieldMeta } from "../../kernel/bridge/records";
import {
  definitionOf,
  newStepId,
  saveWorkflowDraftNow,
  STEP_TYPES,
  stepLabel,
  useObjectDefs,
  useRunWorkflow,
  useSaveWorkflowDraft,
  useVersionAction,
  useWorkflowDraft,
  useWorkflowVersions,
  type Definition,
  type Step,
} from "../../kernel/bridge/workflows";
import MenuButton from "../../kernel/shell/MenuButton";
import type { WidgetContext } from "../crm/record/widgets";
import { formatWhen } from "../meetings/format";
import { DEFAULT_CODE, formFields, partsOf, StepSettings, summarize, TriggerSettings, triggerSummary, triggerTitle, type Variable } from "./StepSettings";
import s from "./Workflows.module.css";

type Slot = "root" | "branch" | "else" | "steps";
/** A list of steps in the document: the root, a route's, the Else, an iterator's. */
interface Loc {
  owner: string | null;
  slot: Slot;
  index: number;
}

function editList(steps: Step[], loc: Loc, fn: (list: Step[]) => Step[]): Step[] {
  if (loc.owner === null) return fn(steps);
  return steps.map((st) => {
    if (st.id === loc.owner) {
      if (loc.slot === "branch") return { ...st, branches: (st.branches ?? []).map((b, i) => (i === loc.index ? { ...b, steps: fn(b.steps) } : b)) };
      if (loc.slot === "else") return { ...st, else_steps: fn(st.else_steps ?? []) };
      return { ...st, steps: fn(st.steps ?? []) };
    }
    return {
      ...st,
      ...(st.branches ? { branches: st.branches.map((b) => ({ ...b, steps: editList(b.steps, loc, fn) })) } : {}),
      ...(st.else_steps ? { else_steps: editList(st.else_steps, loc, fn) } : {}),
      ...(st.steps ? { steps: editList(st.steps, loc, fn) } : {}),
    };
  });
}

function mapStep(steps: Step[], id: string, fn: (st: Step) => Step): Step[] {
  return steps.map((st) => {
    if (st.id === id) return fn(st);
    return {
      ...st,
      ...(st.branches ? { branches: st.branches.map((b) => ({ ...b, steps: mapStep(b.steps, id, fn) })) } : {}),
      ...(st.else_steps ? { else_steps: mapStep(st.else_steps, id, fn) } : {}),
      ...(st.steps ? { steps: mapStep(st.steps, id, fn) } : {}),
    };
  });
}

function removeStep(steps: Step[], id: string): Step[] {
  return steps
    .filter((st) => st.id !== id)
    .map((st) => ({
      ...st,
      ...(st.branches ? { branches: st.branches.map((b) => ({ ...b, steps: removeStep(b.steps, id) })) } : {}),
      ...(st.else_steps ? { else_steps: removeStep(st.else_steps, id) } : {}),
      ...(st.steps ? { steps: removeStep(st.steps, id) } : {}),
    }));
}

function innerSteps(st: Step): Step[] {
  return [...(st.branches ?? []).flatMap((b) => b.steps), ...(st.else_steps ?? []), ...(st.steps ?? [])];
}

function allInner(st: Step): Step[] {
  return innerSteps(st).flatMap((x) => [x, ...allInner(x)]);
}

function findStep(steps: Step[], id: string): Step | undefined {
  for (const st of steps) {
    if (st.id === id) return st;
    const hit = findStep(innerSteps(st), id);
    if (hit) return hit;
  }
  return undefined;
}

/** The steps that have run by the time `id` runs — those before it in its list and in the lists
 *  around it (the container's own output included), never a sibling route's — and the iterators
 *  it sits in, outermost first. */
function pathBefore(steps: Step[], id: string, iterators: Step[]): { before: Step[]; iterators: Step[] } | null {
  const acc: Step[] = [];
  for (const st of steps) {
    if (st.id === id) return { before: acc, iterators };
    const lists: [Step[], Step[]][] = [...(st.branches ?? []).map((b): [Step[], Step[]] => [b.steps, iterators]), [st.else_steps ?? [], iterators], [st.steps ?? [], st.type === "ITERATOR" ? [...iterators, st] : iterators]];
    for (const [list, its] of lists) {
      const hit = pathBefore(list, id, its);
      if (hit) return { before: [...acc, st, ...hit.before], iterators: hit.iterators };
    }
    acc.push(st, ...allInner(st));
  }
  return null;
}

function newStep(def: Definition, type: string): Step {
  const base: Step = { id: newStepId(def), type, name: "", settings: {} };
  if (type === "IF_ELSE") return { ...base, branches: [{ name: "Route 1", conditions: [], logic: "AND", steps: [] }], else_steps: [] };
  if (type === "ITERATOR") return { ...base, steps: [] };
  if (type === "DELAY") return { ...base, settings: { amount: 1, unit: "HOURS" } };
  if (type === "SEARCH_RECORDS") return { ...base, settings: { limit: 200 } };
  if (type === "FORM") return { ...base, settings: { title: "", fields: [] } };
  if (type === "AI_AGENT") return { ...base, settings: { prompt: "", model: "", outputs: [] } };
  if (type === "SEND_EMAIL") return { ...base, settings: { to: "", cc: "", subject: "", body: "" } };
  if (type === "HTTP_REQUEST") return { ...base, settings: { method: "GET", url: "", headers: [], body: "" } };
  if (type === "CODE") return { ...base, settings: { code: DEFAULT_CODE, inputs: { name: "" } } };
  return base;
}

function fieldPaths(prefix: string, fields: FieldMeta[], group: string): Variable[] {
  const out: Variable[] = [{ group, path: `${prefix}.id`, label: "Id" }];
  for (const f of fields.filter((f) => f.is_active && !f.is_system && f.type !== "POSITION" && !(f.type === "RELATION" && f.options.relation?.kind !== "MANY_TO_ONE"))) {
    if (f.type === "RELATION") {
      out.push({ group, path: `${prefix}.${f.name}.id`, label: `${f.label} · id` }, { group, path: `${prefix}.${f.name}.label`, label: `${f.label} · name` });
    } else if (partsOf(f).length) {
      for (const sub of partsOf(f)) out.push({ group, path: `${prefix}.${f.name}.${sub.key}`, label: `${f.label} · ${sub.label}` });
    } else out.push({ group, path: `${prefix}.${f.name}`, label: f.label });
  }
  return out;
}

/** JSON with every object's keys in order: the engine hands documents back with sorted keys, so
 *  "unsaved" compares this, never the text as typed. */
function canon(value: unknown): string {
  return JSON.stringify(value, (_key, v: unknown) => (v && typeof v === "object" && !Array.isArray(v) ? Object.fromEntries(Object.entries(v).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))) : v));
}

const RECORD_STEPS = ["CREATE_RECORD", "UPDATE_RECORD", "UPSERT_RECORD", "DELETE_RECORD"];
const DATES: Variable[] = [
  { group: "Dates", path: "today", label: "Today" },
  { group: "Dates", path: "now", label: "Now" },
  { group: "Dates", path: "today+1d", label: "Tomorrow" },
  { group: "Dates", path: "today+3d", label: "In 3 days" },
  { group: "Dates", path: "today+1w", label: "In a week" },
  { group: "Dates", path: "today-1w", label: "A week ago" },
];
const AUTOSAVE_MS = 800;

/** A workflow's page (D-068): the flow — trigger first, then the steps, routes and inner steps
 *  indented — on the left, the chosen card's settings on the right, with the versions and the
 *  runs under them. The builder edits the draft and saves it as you go (and when you leave);
 *  Activate publishes it. Keyed on the workflow by its page. */
export default function WorkflowBuilder({ ctx }: { ctx: WidgetContext }) {
  const { slug, record } = ctx;
  const id = recordId(record);
  const navigate = useNavigate();
  const draft = useWorkflowDraft(slug, id);
  const versions = useWorkflowVersions(slug, id);
  const save = useSaveWorkflowDraft(slug, id);
  const act = useVersionAction(slug, id);
  const run = useRunWorkflow(slug);
  const [def, setDef] = useState<Definition | null>(null);
  const [selected, setSelected] = useState("trigger");
  const [adding, setAdding] = useState<Loc | null>(null);
  const [testing, setTesting] = useState(false);
  const [testQuery, setTestQuery] = useState("");
  const [trouble, setTrouble] = useState<string | null>(null);
  // The draft our own save just made: its arrival doesn't replace what was typed meanwhile.
  const ownSave = useRef<string | null>(null);
  const draftId = draft.data?.id;
  useEffect(() => {
    if (draft.data && draft.data.id !== ownSave.current) setDef(definitionOf(draft.data.definition));
  }, [draftId]); // eslint-disable-line react-hooks/exhaustive-deps -- another version on screen replaces the document; refetches of the same one keep the edits
  const saved = draft.data ? canon(definitionOf(draft.data.definition)) : "";
  const dirty = !!def && canon(def) !== saved;
  const active = versions.data?.find((v) => v.status === "ACTIVE");
  const status = typeof record.status === "string" ? record.status : "DRAFT";
  const shownStatus = draft.data?.status;

  // Saved as you go, a moment after the last change, one save at a time.
  const saveMutate = save.mutate;
  const savePending = save.isPending;
  useEffect(() => {
    if (!dirty || !def || savePending) return;
    const timer = window.setTimeout(
      () =>
        saveMutate(def, {
          onSuccess: (v) => {
            ownSave.current = v.id;
            setTrouble(null);
          },
          onError: (e: Error) => setTrouble(e.message),
        }),
      AUTOSAVE_MS,
    );
    return () => window.clearTimeout(timer);
  }, [def, dirty, savePending, saveMutate]);
  // Leaving with a change not saved yet (another page, another workflow): saved now.
  const latest = useRef<{ def: Definition | null; dirty: boolean }>({ def: null, dirty: false });
  useEffect(() => {
    latest.current = { def, dirty };
  });
  useEffect(
    () => () => {
      const { def: unsavedDef, dirty: unsaved } = latest.current;
      if (unsaved && unsavedDef && slug) void saveWorkflowDraftNow(slug, id, unsavedDef).catch(() => undefined);
    },
    [slug, id],
  );

  // The variables a step can use: the trigger's record (and, for an update, the record before
  // it), the steps that have run by then, the iterators it sits in, dates.
  const step = def && selected !== "trigger" ? findStep(def.steps, selected) : undefined;
  const path = def && step ? (pathBefore(def.steps, step.id, []) ?? { before: [], iterators: [] }) : { before: [], iterators: [] };
  const trigger = def?.trigger ?? null;
  const triggerObject = trigger && (trigger.type === "RECORD" || (trigger.type === "MANUAL" && trigger.availability !== "GLOBAL")) ? (trigger.object ?? "") : "";
  const recordSteps = path.before.filter((st) => RECORD_STEPS.includes(st.type) && typeof st.settings.object === "string" && st.settings.object);
  const objectNames = [...new Set([triggerObject, ...recordSteps.map((st) => st.settings.object as string)].filter(Boolean))];
  const defs = useObjectDefs(slug, objectNames);
  const defOf = (name: string) => defs[objectNames.indexOf(name)]?.data;
  const variables: Variable[] = [];
  if (triggerObject) {
    const fields = defOf(triggerObject)?.fields ?? [];
    if (trigger?.availability === "BULK") variables.push({ group: "Trigger", path: "trigger.records", label: "The records" });
    else variables.push(...fieldPaths("trigger.record", fields, "Trigger"));
    if (trigger?.type === "RECORD" && (trigger.event === "UPDATED" || trigger.event === "CREATED_OR_UPDATED")) variables.push(...fieldPaths("trigger.before", fields, "Before the change"));
  }
  if (trigger?.type === "SCHEDULE") variables.push({ group: "Trigger", path: "trigger.scheduled_at", label: "Scheduled at" });
  for (const st of path.before) {
    const group = `${st.name || stepLabel(st.type)} (${st.id})`;
    if (RECORD_STEPS.includes(st.type)) variables.push(...fieldPaths(st.id, defOf(st.settings.object as string)?.fields ?? [], group));
    else if (st.type === "SEARCH_RECORDS") variables.push({ group, path: st.id, label: "The records" }, { group, path: `${st.id}.0.id`, label: "First record · id" });
    else if (st.type === "FORM") for (const f of formFields(st)) variables.push({ group, path: `${st.id}.answers.${f.name}`, label: f.label || f.name });
    else if (st.type === "IF_ELSE") variables.push({ group, path: `${st.id}.branch`, label: "Route taken" });
    else if (st.type === "AI_AGENT") {
      variables.push({ group, path: `${st.id}.text`, label: "The answer" });
      for (const o of Array.isArray(st.settings.outputs) ? (st.settings.outputs as { name: string }[]) : []) if (o.name) variables.push({ group, path: `${st.id}.${o.name}`, label: o.name });
    } else if (st.type === "SEND_EMAIL") variables.push({ group, path: `${st.id}.to`, label: "To" }, { group, path: `${st.id}.subject`, label: "Subject" }, { group, path: `${st.id}.mailto`, label: "The draft's link" });
    else if (st.type === "HTTP_REQUEST") variables.push({ group, path: `${st.id}.status`, label: "Status" }, { group, path: `${st.id}.body`, label: "The answer" });
    else if (st.type === "CODE") variables.push({ group, path: st.id, label: "The result" });
    else if (st.type === "ITERATOR" && path.iterators.includes(st)) variables.push({ group, path: `${st.id}.currentItem`, label: "Its current item" }, { group, path: `${st.id}.index`, label: "Its index" });
  }
  if (path.iterators.length)
    variables.push({ group: "Iterator", path: "iterator.currentItem", label: "Current item" }, { group: "Iterator", path: "iterator.currentItem.id", label: "Current item · id" }, { group: "Iterator", path: "iterator.index", label: "Index" });
  variables.push(...DATES);

  // The test's record, when the trigger wants one.
  const needsRecord = !!triggerObject;
  const hits = useSearchRecords(slug, testing && needsRecord ? testQuery : "");
  const runs = useRecords(slug, "workflowRun", { filter: { and: [{ field: "workflow", sub: null, op: "is", value: id }] }, sort: [{ field: "startedAt", sub: null, desc: true }], limit: 12, offset: null, search: null, deleted: false });

  const goRun = (ids: string[]) => {
    if (ids[0]) void navigate({ to: "/crm/$object/$recordId", params: { object: "workflowRuns", recordId: ids[0] }, search: {} });
  };
  const saveFirst = async () => {
    if (!def) return null;
    if (!dirty) return draft.data ?? null;
    const v = await save.mutateAsync(def);
    ownSave.current = v.id;
    return v;
  };
  const activate = async () => {
    setTrouble(null);
    try {
      const v = await saveFirst();
      if (v) await act.mutateAsync({ action: "activate", versionId: v.id });
    } catch (e) {
      setTrouble((e as Error).message);
    }
  };
  const test = async (recordIds: string[]) => {
    setTrouble(null);
    try {
      await saveFirst();
      const ids = await run.mutateAsync({ workflowId: id, recordIds, test: true });
      setTesting(false);
      goRun(ids);
    } catch (e) {
      setTrouble((e as Error).message);
    }
  };
  const useAsDraft = (versionId: string) =>
    act.mutate(
      { action: "use", versionId },
      {
        onSuccess: (next) => {
          if (!next) return;
          ownSave.current = next.id;
          setDef(definitionOf(next.definition));
        },
        onError: (e: Error) => setTrouble(e.message),
      },
    );
  const select = (next: string) => {
    setSelected(next);
    setAdding(null);
  };
  const add = (loc: Loc, type: string) => {
    if (!def) return;
    const st = newStep(def, type);
    setDef({ ...def, steps: editList(def.steps, loc, (list) => [...list, st]) });
    setAdding(null);
    setSelected(st.id);
  };
  const remove = (stepId: string) => {
    if (!def) return;
    setDef({ ...def, steps: removeStep(def.steps, stepId) });
    if (selected === stepId) setSelected("trigger");
  };
  const move = (loc: Loc, i: number, dir: -1 | 1) => {
    if (!def) return;
    setDef({
      ...def,
      steps: editList(def.steps, loc, (list) => {
        const j = i + dir;
        if (j < 0 || j >= list.length) return list;
        const next = [...list];
        [next[i], next[j]] = [next[j] as Step, next[i] as Step];
        return next;
      }),
    });
  };
  const onCardKey = (e: KeyboardEvent, what: string) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      select(what);
    }
  };

  // Rendered by calling, not as components: a card keeps its focus and state across edits.
  const renderAdd = (loc: Loc) => {
    const open = adding && adding.owner === loc.owner && adding.slot === loc.slot && adding.index === loc.index;
    if (!open)
      return (
        <div>
          <button type="button" className="btn btnGhost btnCompact" onClick={() => setAdding(loc)}>
            + Add a step
          </button>
        </div>
      );
    return (
      <div className={s.picker} role="group" aria-label="Step types">
        {STEP_TYPES.map((t) => (
          <button key={t.type} type="button" className={s.pickerItem} onClick={() => add(loc, t.type)}>
            <span>{t.label}</span>
            <small>{t.hint}</small>
          </button>
        ))}
        <button type="button" className={s.pickerItem} onClick={() => setAdding(null)}>
          <span>Cancel</span>
        </button>
      </div>
    );
  };
  const renderSteps = (list: Step[], loc: Loc) =>
    list.map((st, i) => (
      <div key={st.id} className={s.flow}>
        <div role="button" tabIndex={0} className={selected === st.id ? `${s.card} ${s.cardOn}` : s.card} aria-pressed={selected === st.id} onClick={() => select(st.id)} onKeyDown={(e) => onCardKey(e, st.id)}>
          <div className={s.cardText}>
            <span className={s.cardKind}>{stepLabel(st.type)}</span>
            <span className={s.cardTitle}>{st.name || stepLabel(st.type)}</span>
            <span className={s.cardSummary}>{summarize(st)}</span>
          </div>
          <div className={s.cardTools}>
            <button type="button" className="btn btnGhost btnCompact" aria-label="Move up" disabled={i === 0} onClick={(e) => (e.stopPropagation(), move(loc, i, -1))}>
              ▲
            </button>
            <button type="button" className="btn btnGhost btnCompact" aria-label="Move down" disabled={i === list.length - 1} onClick={(e) => (e.stopPropagation(), move(loc, i, 1))}>
              ▼
            </button>
            <button type="button" className="btn btnGhost btnCompact" aria-label={`Remove ${st.name || stepLabel(st.type)}`} onClick={(e) => (e.stopPropagation(), remove(st.id))}>
              ✕
            </button>
          </div>
        </div>
        {st.type === "IF_ELSE" && (
          <div className={s.nested}>
            {(st.branches ?? []).map((b, bi) => (
              <div key={bi} className={s.flow}>
                <p className={s.routeLabel}>
                  {bi === 0 ? "If" : "Else if"} · {b.name || `Route ${bi + 1}`}
                </p>
                {renderSteps(b.steps, { owner: st.id, slot: "branch", index: bi })}
                {renderAdd({ owner: st.id, slot: "branch", index: bi })}
              </div>
            ))}
            <p className={s.routeLabel}>Else</p>
            {renderSteps(st.else_steps ?? [], { owner: st.id, slot: "else", index: 0 })}
            {renderAdd({ owner: st.id, slot: "else", index: 0 })}
          </div>
        )}
        {st.type === "ITERATOR" && (
          <div className={s.nested}>
            <p className={s.routeLabel}>Once an item</p>
            {renderSteps(st.steps ?? [], { owner: st.id, slot: "steps", index: 0 })}
            {renderAdd({ owner: st.id, slot: "steps", index: 0 })}
          </div>
        )}
      </div>
    ));

  if (!def) {
    return (
      <div className={s.flow} style={{ marginTop: "var(--space-5)" }}>
        <div className={`${s.ghostStep} ghostCard`}>{draft.error ? <span className="ghostText">Couldn't be read — {draft.error.message}</span> : <span className="ghostText">Reading…</span>}</div>
      </div>
    );
  }
  const statusClass = status === "ACTIVE" ? `chip ${s.statusOk}` : status === "DEACTIVATED" ? `chip ${s.statusWarn}` : "chip";
  const others = (versions.data ?? []).some((v) => v.id !== draft.data?.id);
  return (
    <>
      <div className={s.bar}>
        <span className={statusClass}>{status === "ACTIVE" ? "Active" : status === "DEACTIVATED" ? "Deactivated" : "Draft"}</span>
        {shownStatus === "DRAFT" && active && draft.data && (
          <span className="fieldHint">
            Editing {draft.data.name}; {active.name} is live.
          </span>
        )}
        <span className="fieldHint" aria-live="polite">
          {dirty || save.isPending ? "Saving…" : ""}
        </span>
        <span className={s.spacer} />
        <button type="button" className="btn btnSecondary btnCompact" disabled={!def.trigger || run.isPending || save.isPending} onClick={() => (needsRecord ? setTesting((t) => !t) : void test([]))}>
          Test
        </button>
        <button type="button" className="btn btnPrimary btnCompact" disabled={!def.trigger || act.isPending || save.isPending || (!dirty && shownStatus === "ACTIVE")} onClick={() => void activate()}>
          Activate
        </button>
        <MenuButton
          label="More"
          items={[
            { label: "Deactivate", disabled: !active || act.isPending, onSelect: () => active && act.mutate({ action: "deactivate", versionId: active.id }) },
            {
              label: "Discard draft",
              danger: true,
              separator: true,
              // Never the only version: that would empty a workflow that was never live.
              disabled: shownStatus !== "DRAFT" || !others || act.isPending,
              onSelect: () => draft.data && act.mutate({ action: "discard", versionId: draft.data.id }),
            },
          ]}
        />
      </div>
      {trouble && (
        <p className="formError" role="alert">
          {trouble}
        </p>
      )}
      {testing && needsRecord && (
        <div className={s.test} role="group" aria-label="Test with a record">
          <label className="fieldLabel" htmlFor="wf-test-search">
            Test with a {triggerObject}
          </label>
          <input id="wf-test-search" className="input" autoFocus placeholder={`Search ${triggerObject}s…`} value={testQuery} onChange={(e) => setTestQuery(e.target.value)} />
          <div className={s.hits}>
            {(hits.data ?? [])
              .filter((h) => h.object === triggerObject)
              .map((h) => (
                <button key={h.record_id} type="button" className={s.hit} onClick={() => void test([h.record_id])}>
                  <span>{h.label || "Unnamed"}</span>
                  <small>Run the draft on it</small>
                </button>
              ))}
            {testQuery && !hits.data?.some((h) => h.object === triggerObject) && <span className="ghostText">{hits.isFetching ? "Searching…" : "Nothing matches"}</span>}
          </div>
        </div>
      )}
      <div className={s.builder}>
        <div className={s.flow}>
          <div role="button" tabIndex={0} className={selected === "trigger" ? `${s.card} ${s.cardOn}` : s.card} aria-pressed={selected === "trigger"} onClick={() => select("trigger")} onKeyDown={(e) => onCardKey(e, "trigger")}>
            <div className={s.cardText}>
              <span className={s.cardKind}>Trigger</span>
              <span className={s.cardTitle}>{triggerTitle(def.trigger)}</span>
              <span className={s.cardSummary}>{triggerSummary(def.trigger)}</span>
            </div>
          </div>
          {renderSteps(def.steps, { owner: null, slot: "root", index: 0 })}
          {!def.steps.length && !adding && <div className={`${s.ghostStep} ghostCard`}>No steps yet — the run would do nothing.</div>}
          {renderAdd({ owner: null, slot: "root", index: 0 })}
        </div>
        <aside className={s.side}>
          <div className="card">
            <p className="eyebrow">{selected === "trigger" ? "Trigger" : step ? stepLabel(step.type) : "Step"}</p>
            {selected === "trigger" ? (
              <TriggerSettings slug={slug} trigger={def.trigger} onChange={(t) => setDef({ ...def, trigger: t })} />
            ) : step ? (
              <StepSettings slug={slug} step={step} variables={variables} onChange={(next) => setDef({ ...def, steps: mapStep(def.steps, next.id, () => next) })} />
            ) : (
              <p className="ghostText">Pick a card.</p>
            )}
          </div>
          <section>
            <h2 className="sectionTitle">Versions</h2>
            <div className={s.list}>
              {(versions.data ?? []).map((v) => (
                <div key={v.id} className={s.item}>
                  <div className={s.itemText}>
                    <span>{v.name}</span>
                    <span className={s.itemMeta}>{formatWhen(v.updated_at)}</span>
                  </div>
                  <span className={v.status === "ACTIVE" ? `chip ${s.statusOk}` : v.status === "DRAFT" ? "chip chipPrimary" : "chip"}>
                    {v.status === "ACTIVE" ? "Active" : v.status === "DRAFT" ? "Draft" : v.status === "ARCHIVED" ? "Archived" : "Deactivated"}
                  </span>
                  {v.status !== "DRAFT" && (
                    <button type="button" className="btn btnGhost btnCompact" disabled={act.isPending || save.isPending} onClick={() => useAsDraft(v.id)}>
                      Use as draft
                    </button>
                  )}
                </div>
              ))}
              {!versions.data?.length && (
                <div className={`${s.item} ghostCard`}>
                  <span className="ghostText">No version yet</span>
                </div>
              )}
            </div>
          </section>
          <section>
            <h2 className="sectionTitle">Runs</h2>
            <div className={s.list}>
              {(runs.data?.records ?? []).map((raw) => {
                const r = asRecord(raw);
                const rid = recordId(r);
                const st = typeof r.status === "string" ? r.status : "";
                return (
                  <div key={rid} className={s.item}>
                    <div className={s.itemText}>
                      <Link to="/crm/$object/$recordId" params={{ object: "workflowRuns", recordId: rid }} search={{}} className={s.itemLink}>
                        {typeof r.name === "string" ? r.name : "Run"}
                      </Link>
                      <span className={s.itemMeta}>{typeof r.startedAt === "string" ? formatWhen(r.startedAt) : typeof r.createdAt === "string" ? formatWhen(r.createdAt) : ""}</span>
                    </div>
                    <span className={st === "COMPLETED" ? `chip ${s.statusOk}` : st === "FAILED" ? `chip ${s.statusBad}` : st === "WAITING" || st === "RUNNING" ? `chip ${s.statusWarn}` : "chip"}>{st.toLowerCase().replace("_", " ")}</span>
                  </div>
                );
              })}
              {!runs.data?.records.length && (
                <div className={`${s.item} ghostCard`}>
                  <span className="ghostText">{runs.isError ? "Couldn't be read" : "No run yet — Test runs the draft"}</span>
                </div>
              )}
            </div>
          </section>
        </aside>
      </div>
    </>
  );
}
