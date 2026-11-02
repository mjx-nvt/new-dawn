import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { commands, type HomeWidget, type MyActivity, type PmArchived, type PmArchives, type PmHit, type PmQuickLink, type PmRecent, type PmSticky_Serialize, type RecentKind } from "../../bindings";
import { unwrap } from "./unwrap";

/** Plane's Home, Your work and navigation (D-082). */
export type PmSticky = PmSticky_Serialize;
export type { HomeWidget, MyActivity, PmArchived, PmArchives, PmHit, PmQuickLink, PmRecent, RecentKind };

export function usePmStickies(slug: string | undefined) {
  return useQuery({ queryKey: ["pm-stickies", slug ?? ""], queryFn: () => unwrap(commands.listPmStickies(slug ?? "")), enabled: !!slug });
}

export function useStickyMutations(slug: string | undefined) {
  const queryClient = useQueryClient();
  const refresh = () => queryClient.invalidateQueries({ queryKey: ["pm-stickies", slug ?? ""] });
  return {
    add: useMutation({ mutationFn: (color: string) => unwrap(commands.addPmSticky(slug ?? "", color)), onSuccess: refresh }),
    update: useMutation({ mutationFn: ({ id, description, color }: { id: string; description?: string; color?: string }) => unwrap(commands.updatePmSticky(slug ?? "", id, description ?? null, color ?? null)), onSuccess: refresh }),
    remove: useMutation({ mutationFn: (id: string) => unwrap(commands.deletePmSticky(slug ?? "", id)), onSuccess: refresh }),
  };
}

export function usePmQuickLinks(slug: string | undefined) {
  return useQuery({ queryKey: ["pm-quick-links", slug ?? ""], queryFn: () => unwrap(commands.listPmQuickLinks(slug ?? "")), enabled: !!slug });
}

export function useQuickLinkMutations(slug: string | undefined) {
  const queryClient = useQueryClient();
  const refresh = () => queryClient.invalidateQueries({ queryKey: ["pm-quick-links", slug ?? ""] });
  return {
    add: useMutation({ mutationFn: ({ title, url }: { title: string; url: string }) => unwrap(commands.addPmQuickLink(slug ?? "", title, url)), onSuccess: refresh }),
    remove: useMutation({ mutationFn: (id: string) => unwrap(commands.deletePmQuickLink(slug ?? "", id)), onSuccess: refresh }),
  };
}

export function usePmHomeWidgets(slug: string | undefined) {
  return useQuery({ queryKey: ["pm-home-widgets", slug ?? ""], queryFn: () => unwrap(commands.pmHomeWidgets(slug ?? "")), enabled: !!slug });
}

export function useSetPmHomeWidgets(slug: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({ mutationFn: (widgets: HomeWidget[]) => unwrap(commands.setPmHomeWidgets(slug ?? "", widgets)), onSuccess: (w) => queryClient.setQueryData(["pm-home-widgets", slug ?? ""], w) });
}

/** The Projects section's items hidden (Customize navigation). */
export function usePmHiddenNavigation(slug: string | undefined) {
  return useQuery({ queryKey: ["pm-navigation", slug ?? ""], queryFn: () => unwrap(commands.pmHiddenNavigation(slug ?? "")), enabled: !!slug });
}

export function useSetPmNavigation(slug: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({ mutationFn: ({ key, hidden }: { key: string; hidden: boolean }) => unwrap(commands.setPmNavigation(slug ?? "", key, hidden)), onSuccess: (h) => queryClient.setQueryData(["pm-navigation", slug ?? ""], h) });
}

export function usePmRecents(slug: string | undefined, kind: RecentKind | null) {
  return useQuery({ queryKey: ["pm-recents", slug ?? "", kind ?? "all"], queryFn: () => unwrap(commands.listPmRecents(slug ?? "", kind)), enabled: !!slug });
}

/** Opened now (each time it is): first in the Home's Recents. A failure costs nothing but the
 *  entry. */
export function useVisitPm(slug: string | undefined, kind: RecentKind, id: string | undefined) {
  const queryClient = useQueryClient();
  useEffect(() => {
    if (!slug || !id) return;
    void unwrap(commands.visitPm(slug, kind, id)).then(
      () => queryClient.invalidateQueries({ queryKey: ["pm-recents", slug] }),
      () => undefined,
    );
  }, [slug, kind, id, queryClient]);
}

export function usePmArchives(slug: string | undefined) {
  return useQuery({ queryKey: ["pm-archives", slug ?? ""], queryFn: () => unwrap(commands.pmArchives(slug ?? "")), enabled: !!slug });
}

export function usePmMyActivity(slug: string | undefined) {
  return useQuery({ queryKey: ["pm-items", slug ?? "", "my-activity"], queryFn: () => unwrap(commands.pmMyActivity(slug ?? "")), enabled: !!slug });
}

/** ⌘K: projects, cycles, modules and pages by name. */
export function useSearchPm(slug: string | undefined, query: string) {
  return useQuery({ queryKey: ["pm-search-all", slug ?? "", query], queryFn: () => unwrap(commands.searchPm(slug ?? "", query)), enabled: !!slug && query.trim().length > 1 });
}
