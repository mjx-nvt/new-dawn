import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type IntakeInput_Deserialize, type IntakeStatus, type PmIntakeItem_Serialize } from "../../bindings";
import { unwrap } from "./unwrap";

/** Plane's intake (D-081). */
export type PmIntakeItem = PmIntakeItem_Serialize;
export type IntakeInput = IntakeInput_Deserialize;
export type { IntakeStatus };

export function usePmIntake(slug: string | undefined, projectId: string) {
  return useQuery({ queryKey: ["pm-intake", slug ?? "", projectId], queryFn: () => unwrap(commands.listPmIntake(slug ?? "", projectId)), enabled: !!slug && !!projectId });
}

/** How many requests wait today (the tab's count). */
export function usePmIntakeCount(slug: string | undefined, projectId: string, enabled: boolean) {
  return useQuery({ queryKey: ["pm-intake", slug ?? "", projectId, "count"], queryFn: () => unwrap(commands.pmIntakeCount(slug ?? "", projectId)), enabled: !!slug && !!projectId && enabled });
}

export function useIntakeMutations(slug: string | undefined) {
  const queryClient = useQueryClient();
  // The requests, and the work items (an accepted one joins them) with their activity.
  const refresh = () => Promise.all(["pm-intake", "pm-items", "pm-item", "pm-activity", "pm-relations"].map((key) => queryClient.invalidateQueries({ queryKey: [key] })));
  return {
    create: useMutation({ mutationFn: (input: IntakeInput) => unwrap(commands.createPmIntake(slug ?? "", input)), onSuccess: refresh }),
    accept: useMutation({ mutationFn: ({ id, stateId }: { id: string; stateId: string | null }) => unwrap(commands.acceptPmIntake(slug ?? "", id, stateId)), onSuccess: refresh }),
    decline: useMutation({ mutationFn: (id: string) => unwrap(commands.declinePmIntake(slug ?? "", id)), onSuccess: refresh }),
    snooze: useMutation({ mutationFn: ({ id, until }: { id: string; until: string }) => unwrap(commands.snoozePmIntake(slug ?? "", id, until)), onSuccess: refresh }),
    duplicate: useMutation({ mutationFn: ({ id, of }: { id: string; of: string }) => unwrap(commands.duplicatePmIntake(slug ?? "", id, of)), onSuccess: refresh }),
    remove: useMutation({ mutationFn: (id: string) => unwrap(commands.deletePmIntake(slug ?? "", id)), onSuccess: refresh }),
  };
}
