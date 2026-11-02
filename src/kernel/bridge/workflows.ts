import { useMutation, useQueries, useQuery, useQueryClient, type QueryClient } from "@tanstack/react-query";
import { openUrl } from "@tauri-apps/plugin-opener";
import { commands, type ManualWorkflow, type PendingCode, type RunDetail, type Version } from "../../bindings";
import { unwrap } from "./unwrap";

export type { ManualWorkflow, PendingCode, RunDetail, Version };

/** Workflows (D-068, 0.10.11): the document a version holds — one trigger, the steps in order
 *  (an If/else's branches and Else, an Iterator's inner steps nested) — and the runs. */
export interface Trigger {
  /** RECORD · MANUAL · SCHEDULE · WEBHOOK */
  type: string;
  object?: string;
  /** CREATED · UPDATED · CREATED_OR_UPDATED · DELETED */
  event?: string;
  /** UPDATED: the fields watched (all when empty). */
  fields?: string[];
  /** MANUAL: GLOBAL · SINGLE · BULK */
  availability?: string;
  every?: number;
  /** MINUTES · HOURS · DAYS */
  unit?: string;
  cron?: string;
}

export interface Condition {
  left: string;
  /** is · isNot · contains · notContains · isEmpty · isNotEmpty · gt · gte · lt · lte · isBefore · isAfter */
  op: string;
  right: string;
}

export interface Branch {
  name: string;
  conditions: Condition[];
  /** AND · OR */
  logic: string;
  steps: Step[];
}

export interface Step {
  id: string;
  /** One of STEP_TYPES. */
  type: string;
  name: string;
  settings: Record<string, unknown>;
  branches?: Branch[];
  else_steps?: Step[];
  steps?: Step[];
}

export interface Definition {
  trigger: Trigger | null;
  steps: Step[];
}

export const STEP_TYPES: { type: string; label: string; hint: string }[] = [
  { type: "CREATE_RECORD", label: "Create record", hint: "A new record with these fields" },
  { type: "UPDATE_RECORD", label: "Update record", hint: "Fields of a record from the trigger or a step" },
  { type: "DELETE_RECORD", label: "Delete record", hint: "To Deleted records" },
  { type: "SEARCH_RECORDS", label: "Search records", hint: "A list for an iterator or a condition" },
  { type: "UPSERT_RECORD", label: "Upsert record", hint: "Update the match or create" },
  { type: "FILTER", label: "Filter", hint: "The run goes on only when it holds" },
  { type: "IF_ELSE", label: "If / else", hint: "The first route that holds" },
  { type: "ITERATOR", label: "Iterator", hint: "The inner steps once an item" },
  { type: "DELAY", label: "Delay", hint: "Wait for a while, or until a date" },
  { type: "FORM", label: "Form", hint: "Ask you, then go on with the answers" },
  { type: "AI_AGENT", label: "AI agent", hint: "A prompt on the local model" },
  { type: "SEND_EMAIL", label: "Send email", hint: "A Mail draft you send" },
  { type: "HTTP_REQUEST", label: "HTTP request", hint: "Call an address on this Mac" },
  { type: "CODE", label: "Code", hint: "JavaScript in New Dawn's window" },
];

export const CONDITION_OPS: [string, string][] = [
  ["is", "is"],
  ["isNot", "is not"],
  ["contains", "contains"],
  ["notContains", "doesn't contain"],
  ["isEmpty", "is empty"],
  ["isNotEmpty", "is not empty"],
  ["gt", ">"],
  ["gte", "≥"],
  ["lt", "<"],
  ["lte", "≤"],
  ["isBefore", "is before"],
  ["isAfter", "is after"],
];

export function stepLabel(type: string): string {
  return STEP_TYPES.find((s) => s.type === type)?.label ?? type;
}

export function definitionOf(json: unknown): Definition {
  const o = json && typeof json === "object" && !Array.isArray(json) ? (json as Partial<Definition>) : {};
  return { trigger: o.trigger && typeof o.trigger === "object" ? o.trigger : null, steps: Array.isArray(o.steps) ? o.steps : [] };
}

let counter = 0;
/** A step id: short, unique in the document, and readable in a template (`{{s3.id}}`). */
export function newStepId(def: Definition): string {
  const used = new Set<string>();
  const walk = (steps: Step[]) => {
    for (const s of steps) {
      used.add(s.id);
      for (const b of s.branches ?? []) walk(b.steps);
      walk(s.else_steps ?? []);
      walk(s.steps ?? []);
    }
  };
  walk(def.steps);
  let id = "";
  do id = `s${++counter}`;
  while (used.has(id));
  return id;
}

export function useWorkflowVersions(slug: string | undefined, workflowId: string | undefined) {
  return useQuery({ queryKey: ["workflow", slug ?? "", workflowId ?? "", "versions"], queryFn: () => unwrap(commands.listWorkflowVersions(slug ?? "", workflowId ?? "")), enabled: !!slug && !!workflowId });
}

/** The draft the builder edits — made from the active version the first time (then the
 *  versions list is read again, since it has one more). */
export function useWorkflowDraft(slug: string | undefined, workflowId: string | undefined) {
  const queryClient = useQueryClient();
  return useQuery({
    queryKey: ["workflow", slug ?? "", workflowId ?? "", "draft"],
    queryFn: async () => {
      const draft = await unwrap(commands.workflowDraft(slug ?? "", workflowId ?? ""));
      const known = queryClient.getQueryData<Version[]>(["workflow", slug ?? "", workflowId ?? "", "versions"]);
      if (known && !known.some((v) => v.id === draft.id)) void queryClient.invalidateQueries({ queryKey: ["workflow", slug ?? "", workflowId ?? "", "versions"] });
      return draft;
    },
    enabled: !!slug && !!workflowId,
  });
}

function useRefreshWorkflow(slug: string | undefined, workflowId: string | undefined) {
  const queryClient = useQueryClient();
  return () => {
    void queryClient.invalidateQueries({ queryKey: ["workflow", slug ?? "", workflowId ?? ""] });
    void queryClient.invalidateQueries({ queryKey: ["manual-workflows", slug ?? ""] });
    // The workflow's status column.
    void queryClient.invalidateQueries({ queryKey: ["record", slug ?? "", "workflow"] });
    void queryClient.invalidateQueries({ queryKey: ["records", slug ?? "", "workflow"] });
  };
}

/** The code steps waiting for this window to run them (D-069). */
export function pendingWorkflowCode(slug: string): Promise<PendingCode[]> {
  return unwrap(commands.pendingWorkflowCode(slug));
}

/** What the window's run of a code step gave: the run goes on with it, or fails with the error. */
export function answerWorkflowCode(slug: string, pending: PendingCode, outcome: { ok: true; output: unknown } | { ok: false; error: string }): Promise<RunDetail> {
  return unwrap(commands.answerWorkflowCode(slug, pending.run_id, pending.step_id, pending.attempt, outcome.ok ? (outcome.output ?? null) : null, outcome.ok ? null : outcome.error));
}

/** What a run may have changed — its page, records, tasks, Today — read again. */
export function refreshAfterRun(queryClient: QueryClient, slug: string | undefined) {
  for (const key of ["workflow-run", "records", "record", "linked", "timeline"]) void queryClient.invalidateQueries({ queryKey: [key, slug ?? ""] });
  for (const key of ["tasks", "open-task-count", "today"]) void queryClient.invalidateQueries({ queryKey: [key] });
}

/** A draft an email step made, opened in Mail — the owner presses Send there (D-069). */
export function openMailDraft(mailto: string): Promise<void> {
  if (!mailto.startsWith("mailto:")) return Promise.reject(new Error("Not a mail draft"));
  return openUrl(mailto);
}

/** A save that outlives the page (the builder saves what's left when it closes). */
export function saveWorkflowDraftNow(slug: string, workflowId: string, definition: Definition): Promise<Version> {
  return unwrap(commands.saveWorkflowDraft(slug, workflowId, definition));
}

export function useSaveWorkflowDraft(slug: string | undefined, workflowId: string | undefined) {
  const refresh = useRefreshWorkflow(slug, workflowId);
  return useMutation({ mutationFn: (definition: Definition) => unwrap(commands.saveWorkflowDraft(slug ?? "", workflowId ?? "", definition)), onSuccess: refresh });
}

export function useVersionAction(slug: string | undefined, workflowId: string | undefined) {
  const refresh = useRefreshWorkflow(slug, workflowId);
  return useMutation({
    mutationFn: async ({ action, versionId }: { action: "activate" | "deactivate" | "discard" | "use"; versionId: string }) => {
      if (action === "activate") return unwrap(commands.activateWorkflowVersion(slug ?? "", versionId));
      if (action === "deactivate") return unwrap(commands.deactivateWorkflowVersion(slug ?? "", versionId));
      if (action === "use") return unwrap(commands.useWorkflowVersionAsDraft(slug ?? "", versionId));
      await unwrap(commands.discardWorkflowDraft(slug ?? "", versionId));
      return null;
    },
    onSuccess: refresh,
  });
}

/** Runs a workflow by hand — once per record (Single), once with them all (Bulk), once with
 *  nothing (Global); the draft when `test`. A run writes records, so everything refreshes. */
export function useRunWorkflow(slug: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ workflowId, recordIds, test }: { workflowId: string; recordIds: string[]; test?: boolean }) => unwrap(commands.runWorkflow(slug ?? "", workflowId, recordIds, test ?? false)),
    onSuccess: () => refreshAfterRun(queryClient, slug),
  });
}

export function useWorkflowRun(slug: string | undefined, runId: string | undefined) {
  return useQuery({
    queryKey: ["workflow-run", slug ?? "", runId ?? ""],
    queryFn: () => unwrap(commands.workflowRunDetail(slug ?? "", runId ?? "")),
    enabled: !!slug && !!runId,
    // A running or waiting run is read again every few seconds.
    refetchInterval: (query) => (query.state.data && ["RUNNING", "WAITING", "NOT_STARTED"].includes(query.state.data.status) ? 5000 : false),
  });
}

/** Answer a form, retry a failed run, or cancel one. */
export function useRunAction(slug: string | undefined, runId: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ action, stepId, answers }: { action: "answer" | "retry" | "cancel"; stepId?: string; answers?: Record<string, unknown> }) => {
      if (action === "answer") return unwrap(commands.answerWorkflowForm(slug ?? "", runId ?? "", stepId ?? "", answers ?? {}));
      if (action === "retry") return unwrap(commands.retryWorkflowRun(slug ?? "", runId ?? ""));
      return unwrap(commands.cancelWorkflowRun(slug ?? "", runId ?? ""));
    },
    onSuccess: () => refreshAfterRun(queryClient, slug),
  });
}

/** The active manual workflows: the global ones (no object), or those that take a record of
 *  the object. */
export function useManualWorkflows(slug: string | undefined, object: string | null | undefined, enabled = true) {
  return useQuery({
    queryKey: ["manual-workflows", slug ?? "", object ?? ""],
    queryFn: () => unwrap(commands.listManualWorkflows(slug ?? "", object ?? null)),
    enabled: !!slug && enabled && object !== undefined,
  });
}

/** Several objects with their fields at once (the variables picker's record steps). */
export function useObjectDefs(slug: string | undefined, names: string[]) {
  return useQueries({
    queries: names.map((name) => ({ queryKey: ["object", slug ?? "", name], queryFn: () => unwrap(commands.getObject(slug ?? "", name)), enabled: !!slug && !!name })),
  });
}
