import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type Favorite, type FavoriteFolder, type FavoriteTarget, type Favorites, type NavItem, type NavItemInput, type Recent, type ViewEntry } from "../../bindings";
import { unwrap } from "./unwrap";

export type { Favorite, FavoriteFolder, FavoriteTarget, Favorites, NavItem, NavItemInput, Recent, ViewEntry };

/** The sidebar's CRM section and ⌘K (D-064, 0.10.7): favorites in folders, the Workspace
 *  section's objects, every view, the records opened lately. */
export function useFavorites(slug: string | undefined) {
  return useQuery({ queryKey: ["favorites", slug ?? ""], queryFn: () => unwrap(commands.listFavorites(slug ?? "")), enabled: !!slug });
}

function useRefreshFavorites(slug: string | undefined) {
  const queryClient = useQueryClient();
  return () => {
    void queryClient.invalidateQueries({ queryKey: ["favorites", slug ?? ""] });
    // A Projects view's, cycle's or module's ★ reads the favorites too (D-077, D-079).
    for (const key of ["pm-views", "pm-cycles", "pm-modules"]) void queryClient.invalidateQueries({ queryKey: [key, slug ?? ""] });
  };
}

export function useAddFavorite(slug: string | undefined) {
  const refresh = useRefreshFavorites(slug);
  return useMutation({ mutationFn: (target: FavoriteTarget) => unwrap(commands.addFavorite(slug ?? "", target)), onSuccess: refresh });
}

export function useRemoveFavorite(slug: string | undefined) {
  const refresh = useRefreshFavorites(slug);
  return useMutation({ mutationFn: (id: string) => unwrap(commands.removeFavorite(slug ?? "", id)), onSuccess: refresh });
}

export function useMoveFavorite(slug: string | undefined) {
  const refresh = useRefreshFavorites(slug);
  return useMutation({
    mutationFn: ({ id, folderId, position }: { id: string; folderId: string | null; position: number }) => unwrap(commands.moveFavorite(slug ?? "", id, folderId, position)),
    onSuccess: refresh,
  });
}

export function useCreateFavoriteFolder(slug: string | undefined) {
  const refresh = useRefreshFavorites(slug);
  return useMutation({ mutationFn: (name: string) => unwrap(commands.createFavoriteFolder(slug ?? "", name)), onSuccess: refresh });
}

export function useRenameFavoriteFolder(slug: string | undefined) {
  const refresh = useRefreshFavorites(slug);
  return useMutation({ mutationFn: ({ id, name }: { id: string; name: string }) => unwrap(commands.renameFavoriteFolder(slug ?? "", id, name)), onSuccess: refresh });
}

export function useDeleteFavoriteFolder(slug: string | undefined) {
  const refresh = useRefreshFavorites(slug);
  return useMutation({ mutationFn: (id: string) => unwrap(commands.deleteFavoriteFolder(slug ?? "", id)), onSuccess: refresh });
}

/** The Workspace section: every record object, hidden ones included (the sidebar skips them). */
export function useNavigation(slug: string | undefined) {
  return useQuery({ queryKey: ["navigation", slug ?? ""], queryFn: () => unwrap(commands.listNavigation(slug ?? "")), enabled: !!slug });
}

export function useSetNavigation(slug: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (items: NavItemInput[]) => unwrap(commands.setNavigation(slug ?? "", items)),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["navigation", slug ?? ""] }),
  });
}

/** Every view of every record object, for ⌘K's "go to". */
export function useAllViews(slug: string | undefined, enabled = true) {
  return useQuery({ queryKey: ["all-views", slug ?? ""], queryFn: () => unwrap(commands.listAllViews(slug ?? "")), enabled: !!slug && enabled, placeholderData: keepPreviousData });
}

export function useRecents(slug: string | undefined, enabled = true) {
  return useQuery({ queryKey: ["recents", slug ?? ""], queryFn: () => unwrap(commands.listRecents(slug ?? "", 8)), enabled: !!slug && enabled });
}

/** A record page was opened. */
export async function noteVisit(slug: string, object: string, id: string): Promise<void> {
  await unwrap(commands.noteVisit(slug, object, id));
}
