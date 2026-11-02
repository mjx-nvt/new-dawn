import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type PmRule, type PmView_Serialize, type PmViewInput_Deserialize, type PmViewPatch_Deserialize } from "../../bindings";
import { unwrap } from "./unwrap";

/** Plane's filters and views (D-077). */
export type PmView = PmView_Serialize;
export type PmViewInput = PmViewInput_Deserialize;
export type PmViewPatch = PmViewPatch_Deserialize;
export type { PmRule };

/** Work items as the filters say — a project's, or the workspace's open projects' (null). Its key
 *  starts like the lists', so every item change reads it again. */
export function usePmQuery(slug: string | undefined, projectId: string | null, rules: PmRule[], enabled = true) {
  return useQuery({
    queryKey: ["pm-items", slug ?? "", "query", projectId ?? "", rules],
    queryFn: () => unwrap(commands.queryPmItems(slug ?? "", projectId, rules)),
    enabled: !!slug && enabled,
    placeholderData: keepPreviousData,
  });
}

/** A project's views, or the workspace's (null). */
export function usePmViews(slug: string | undefined, projectId: string | null) {
  return useQuery({ queryKey: ["pm-views", slug ?? "", projectId ?? ""], queryFn: () => unwrap(commands.listPmViews(slug ?? "", projectId)), enabled: !!slug });
}

export function useViewMutations(slug: string | undefined) {
  const queryClient = useQueryClient();
  // Awaited: a mutation's own callbacks (open the saved view, clear the changes) run on the fresh
  // list.
  const refresh = () => Promise.all([queryClient.invalidateQueries({ queryKey: ["pm-views", slug ?? ""] }), queryClient.invalidateQueries({ queryKey: ["favorites", slug ?? ""] })]);
  return {
    create: useMutation({ mutationFn: (input: PmViewInput) => unwrap(commands.createPmView(slug ?? "", input)), onSuccess: refresh }),
    update: useMutation({ mutationFn: ({ id, patch }: { id: string; patch: PmViewPatch }) => unwrap(commands.updatePmView(slug ?? "", id, patch)), onSuccess: refresh }),
    duplicate: useMutation({ mutationFn: (id: string) => unwrap(commands.duplicatePmView(slug ?? "", id)), onSuccess: refresh }),
    remove: useMutation({ mutationFn: (id: string) => unwrap(commands.deletePmView(slug ?? "", id)), onSuccess: refresh }),
    favorite: useMutation({ mutationFn: ({ id, on }: { id: string; on: boolean }) => unwrap(commands.favoritePmView(slug ?? "", id, on)), onSuccess: refresh }),
  };
}

/** The filters as the address keeps them (`f`): JSON, read leniently — what doesn't read is none. */
export function readRules(text: string | undefined): PmRule[] | undefined {
  if (!text) return undefined;
  try {
    const parsed: unknown = JSON.parse(text);
    return Array.isArray(parsed)
      ? parsed.filter((r): r is PmRule => !!r && typeof r === "object" && typeof (r as PmRule).field === "string" && typeof (r as PmRule).op === "string" && Array.isArray((r as PmRule).values) && (r as PmRule).values.every((v) => typeof v === "string"))
      : undefined;
  } catch {
    return undefined;
  }
}
