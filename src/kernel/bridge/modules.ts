import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type Modules } from "../../bindings";
import { unwrap } from "./unwrap";

export type { Modules };

/** Until a workspace's choice is read, both modules show. */
export const ALL_MODULES: Modules = { projects: true, crm: true };

/** Which modules the workspace has on — Projects, CRM (D-059). */
export function useModules(slug: string | undefined) {
  return useQuery({ queryKey: ["modules", slug ?? ""], queryFn: () => unwrap(commands.getModules(slug ?? "")), enabled: !!slug });
}

export const setModules = (slug: string, modules: Modules) => unwrap(commands.setModules(slug, modules));

export function useSetModules(slug: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (modules: Modules) => setModules(slug ?? "", modules),
    onSuccess: (modules) => {
      queryClient.setQueryData(["modules", slug ?? ""], modules);
      void queryClient.invalidateQueries({ queryKey: ["today"] });
    },
  });
}
