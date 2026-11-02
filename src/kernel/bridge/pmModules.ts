import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type ModuleDetails, type ModuleInput_Deserialize, type ModulePatch_Deserialize, type ModuleStatus, type PmModule_Serialize } from "../../bindings";
import { unwrap } from "./unwrap";

/** Plane's modules (D-079). */
export type PmModule = PmModule_Serialize;
export type ModuleInput = ModuleInput_Deserialize;
export type ModulePatch = ModulePatch_Deserialize;
export type { ModuleDetails, ModuleStatus };

export function usePmModules(slug: string | undefined, projectId: string) {
  return useQuery({ queryKey: ["pm-modules", slug ?? "", projectId], queryFn: () => unwrap(commands.listPmModules(slug ?? "", projectId)), enabled: !!slug && !!projectId });
}

/** The details panel. Its key starts like the items', so an item's change reads it again. */
export function usePmModuleDetails(slug: string | undefined, id: string | undefined) {
  return useQuery({ queryKey: ["pm-items", slug ?? "", "module-details", id ?? ""], queryFn: () => unwrap(commands.pmModuleDetails(slug ?? "", id ?? "")), enabled: !!slug && !!id });
}

export function useModuleMutations(slug: string | undefined) {
  const queryClient = useQueryClient();
  // The modules, the items (their modules, the lists filtered by them) and the sidebar's stars.
  const refresh = () => Promise.all(["pm-modules", "pm-items", "pm-item", "favorites"].map((key) => queryClient.invalidateQueries({ queryKey: [key] })));
  return {
    create: useMutation({ mutationFn: (input: ModuleInput) => unwrap(commands.createPmModule(slug ?? "", input)), onSuccess: refresh }),
    update: useMutation({ mutationFn: ({ id, patch }: { id: string; patch: ModulePatch }) => unwrap(commands.updatePmModule(slug ?? "", id, patch)), onSuccess: refresh }),
    favorite: useMutation({ mutationFn: ({ id, on }: { id: string; on: boolean }) => unwrap(commands.favoritePmModule(slug ?? "", id, on)), onSuccess: refresh }),
    addItems: useMutation({ mutationFn: ({ id, itemIds }: { id: string; itemIds: string[] }) => unwrap(commands.addPmModuleItems(slug ?? "", id, itemIds)), onSuccess: refresh }),
    archive: useMutation({ mutationFn: ({ id, archived }: { id: string; archived: boolean }) => unwrap(commands.archivePmModule(slug ?? "", id, archived)), onSuccess: refresh }),
    remove: useMutation({ mutationFn: (id: string) => unwrap(commands.deletePmModule(slug ?? "", id)), onSuccess: refresh }),
    addLink: useMutation({ mutationFn: ({ id, url, title }: { id: string; url: string; title: string }) => unwrap(commands.addPmModuleLink(slug ?? "", id, url, title)), onSuccess: refresh }),
    removeLink: useMutation({ mutationFn: (linkId: string) => unwrap(commands.removePmModuleLink(slug ?? "", linkId)), onSuccess: refresh }),
  };
}
