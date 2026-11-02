import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type CycleDetails_Serialize, type CycleInput_Deserialize, type CyclePatch_Deserialize, type CycleStatus, type PmCycle_Serialize } from "../../bindings";
import { unwrap } from "./unwrap";

/** Plane's cycles (D-078). */
export type PmCycle = PmCycle_Serialize;
export type CycleInput = CycleInput_Deserialize;
export type CyclePatch = CyclePatch_Deserialize;
export type CycleDetails = CycleDetails_Serialize;
export type { CycleStatus };

export function usePmCycles(slug: string | undefined, projectId: string) {
  return useQuery({ queryKey: ["pm-cycles", slug ?? "", projectId], queryFn: () => unwrap(commands.listPmCycles(slug ?? "", projectId)), enabled: !!slug && !!projectId });
}

/** Every open project's active cycles. */
export function useActivePmCycles(slug: string | undefined) {
  return useQuery({ queryKey: ["pm-cycles", slug ?? "", "active"], queryFn: () => unwrap(commands.listActivePmCycles(slug ?? "")), enabled: !!slug });
}

/** The details panel: burn-down and breakdowns. Its key starts like the items', so an item's
 *  change (Done, say) reads it again. */
export function usePmCycleDetails(slug: string | undefined, id: string | undefined) {
  return useQuery({ queryKey: ["pm-items", slug ?? "", "cycle-details", id ?? ""], queryFn: () => unwrap(commands.pmCycleDetails(slug ?? "", id ?? "")), enabled: !!slug && !!id });
}

export function useCycleMutations(slug: string | undefined) {
  const queryClient = useQueryClient();
  // The cycles, the items (their cycle, the lists filtered by it) and the sidebar's stars (D-079).
  const refresh = () => Promise.all([queryClient.invalidateQueries({ queryKey: ["pm-cycles", slug ?? ""] }), ...["pm-items", "pm-item", "favorites"].map((key) => queryClient.invalidateQueries({ queryKey: [key] }))]);
  return {
    create: useMutation({ mutationFn: (input: CycleInput) => unwrap(commands.createPmCycle(slug ?? "", input)), onSuccess: refresh }),
    update: useMutation({ mutationFn: ({ id, patch }: { id: string; patch: CyclePatch }) => unwrap(commands.updatePmCycle(slug ?? "", id, patch)), onSuccess: refresh }),
    start: useMutation({ mutationFn: (id: string) => unwrap(commands.startPmCycle(slug ?? "", id)), onSuccess: refresh }),
    complete: useMutation({ mutationFn: (id: string) => unwrap(commands.completePmCycle(slug ?? "", id)), onSuccess: refresh }),
    addItems: useMutation({ mutationFn: ({ id, itemIds }: { id: string; itemIds: string[] }) => unwrap(commands.addPmCycleItems(slug ?? "", id, itemIds)), onSuccess: refresh }),
    transfer: useMutation({ mutationFn: ({ id, to }: { id: string; to: string }) => unwrap(commands.transferPmCycle(slug ?? "", id, to)), onSuccess: refresh }),
    archive: useMutation({ mutationFn: ({ id, archived }: { id: string; archived: boolean }) => unwrap(commands.archivePmCycle(slug ?? "", id, archived)), onSuccess: refresh }),
    remove: useMutation({ mutationFn: (id: string) => unwrap(commands.deletePmCycle(slug ?? "", id)), onSuccess: refresh }),
  };
}
