import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  commands,
  type Automations_Deserialize,
  type EstimateInput,
  type EstimateKind,
  type Features,
  type LabelInput_Deserialize,
  type Network,
  type NewPmProject_Deserialize,
  type PmEstimate_Serialize,
  type PmLabel_Serialize,
  type PmProject_Serialize,
  type PmProjectPatch_Deserialize,
  type PmState,
  type StateGroup,
  type StateInput,
  type StatePatch_Deserialize,
} from "../../bindings";
import { unwrap } from "./unwrap";

/** Plane's projects and their settings (D-073). */
export type PmProject = PmProject_Serialize;
export type PmProjectPatch = PmProjectPatch_Deserialize;
export type NewPmProject = NewPmProject_Deserialize;
export type PmLabel = PmLabel_Serialize;
export type PmEstimate = PmEstimate_Serialize;
export type Automations = Automations_Deserialize;
export type { EstimateInput, EstimateKind, Features, Network, PmState, StateGroup, StateInput };

const projectsKey = (slug: string) => ["pm-projects", slug] as const;

/** After a change to a project: both lists (Plane's and the dossier's), its page, Today. */
function useRefreshProjects() {
  const queryClient = useQueryClient();
  return () => {
    for (const key of ["pm-projects", "pm-project", "projects", "project", "today"]) void queryClient.invalidateQueries({ queryKey: [key] });
  };
}

export function usePmProjects(slug: string | undefined) {
  return useQuery({ queryKey: projectsKey(slug ?? ""), queryFn: () => unwrap(commands.listPmProjects(slug ?? "")), enabled: !!slug });
}

export function usePmProject(slug: string | undefined, id: string) {
  return useQuery({ queryKey: ["pm-project", slug ?? "", id], queryFn: () => unwrap(commands.getPmProject(slug ?? "", id)), enabled: !!slug && !!id });
}

export function useCreatePmProject(slug: string | undefined) {
  const refresh = useRefreshProjects();
  return useMutation({ mutationFn: (project: NewPmProject) => unwrap(commands.createPmProject(slug ?? "", project)), onSuccess: refresh });
}

export function useUpdatePmProject(slug: string | undefined) {
  const queryClient = useQueryClient();
  const refresh = useRefreshProjects();
  return useMutation({
    mutationFn: ({ id, patch }: { id: string; patch: PmProjectPatch }) => unwrap(commands.updatePmProject(slug ?? "", id, patch)),
    // The answer is the project now: the next edit builds on it, not on a stale copy.
    onSuccess: (p) => {
      queryClient.setQueryData(["pm-project", slug ?? "", p.id], p);
      refresh();
    },
  });
}

export function useArchivePmProject(slug: string | undefined) {
  const refresh = useRefreshProjects();
  return useMutation({ mutationFn: ({ id, archived }: { id: string; archived: boolean }) => unwrap(commands.archivePmProject(slug ?? "", id, archived)), onSuccess: refresh });
}

/** A project's states; every change answers with the whole list. */
export function usePmStates(slug: string | undefined, projectId: string) {
  return useQuery({ queryKey: ["pm-states", slug ?? "", projectId], queryFn: () => unwrap(commands.listPmStates(slug ?? "", projectId)), enabled: !!slug && !!projectId });
}

export function usePmStateMutation<V>(slug: string | undefined, projectId: string, call: (slug: string, vars: V) => Promise<PmState[]>) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (vars: V) => call(slug ?? "", vars),
    onSuccess: (states) => {
      queryClient.setQueryData(["pm-states", slug ?? "", projectId], states);
      void queryClient.invalidateQueries({ queryKey: ["pm-project", slug ?? "", projectId] });
    },
  });
}

export const stateCalls = {
  create: (slug: string, v: { projectId: string; input: StateInput }) => unwrap(commands.createPmState(slug, v.projectId, v.input)),
  update: (slug: string, v: { id: string; patch: StatePatch_Deserialize }) => unwrap(commands.updatePmState(slug, v.id, v.patch)),
  setDefault: (slug: string, id: string) => unwrap(commands.setDefaultPmState(slug, id)),
  remove: (slug: string, id: string) => unwrap(commands.deletePmState(slug, id)),
  move: (slug: string, v: { id: string; up: boolean }) => unwrap(commands.movePmState(slug, v.id, v.up)),
};

export function usePmLabels(slug: string | undefined, projectId: string) {
  return useQuery({ queryKey: ["pm-labels", slug ?? "", projectId], queryFn: () => unwrap(commands.listPmLabels(slug ?? "", projectId)), enabled: !!slug && !!projectId });
}

export function usePmLabelMutation<V>(slug: string | undefined, projectId: string, call: (slug: string, vars: V) => Promise<PmLabel[]>) {
  const queryClient = useQueryClient();
  return useMutation({ mutationFn: (vars: V) => call(slug ?? "", vars), onSuccess: (labels) => queryClient.setQueryData(["pm-labels", slug ?? "", projectId], labels) });
}

export const labelCalls = {
  create: (slug: string, v: { projectId: string; input: LabelInput_Deserialize }) => unwrap(commands.createPmLabel(slug, v.projectId, v.input)),
  update: (slug: string, v: { id: string; input: LabelInput_Deserialize }) => unwrap(commands.updatePmLabel(slug, v.id, v.input)),
  remove: (slug: string, id: string) => unwrap(commands.deletePmLabel(slug, id)),
};

export function usePmEstimates(slug: string | undefined, projectId: string) {
  return useQuery({ queryKey: ["pm-estimates", slug ?? "", projectId], queryFn: () => unwrap(commands.listPmEstimates(slug ?? "", projectId)), enabled: !!slug && !!projectId });
}

export function usePmEstimateMutation<V>(slug: string | undefined, projectId: string, call: (slug: string, vars: V) => Promise<PmEstimate[]>) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (vars: V) => call(slug ?? "", vars),
    onSuccess: (estimates) => {
      queryClient.setQueryData(["pm-estimates", slug ?? "", projectId], estimates);
      void queryClient.invalidateQueries({ queryKey: ["pm-project", slug ?? "", projectId] });
    },
  });
}

export const estimateCalls = {
  create: (slug: string, v: { projectId: string; input: EstimateInput }) => unwrap(commands.createPmEstimate(slug, v.projectId, v.input)),
  update: (slug: string, v: { id: string; input: EstimateInput }) => unwrap(commands.updatePmEstimate(slug, v.id, v.input)),
  disable: (slug: string, projectId: string) => unwrap(commands.disablePmEstimates(slug, projectId)),
};
