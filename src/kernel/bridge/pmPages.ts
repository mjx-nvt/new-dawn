import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { save } from "@tauri-apps/plugin-dialog";
import { commands, type Hit, type PageInput_Deserialize, type PagePatch_Deserialize, type PmPageDoc_Serialize, type PmPageVersion_Serialize, type PmPage_Serialize } from "../../bindings";
import type { Attached } from "./files";
import { devPath } from "./transfer";
import { unwrap } from "./unwrap";

/** Plane's pages (D-080). */
export type PmPage = PmPage_Serialize;
export type PmPageDoc = PmPageDoc_Serialize;
export type PmPageVersion = PmPageVersion_Serialize;
export type PageInput = PageInput_Deserialize;
export type PagePatch = PagePatch_Deserialize;

export function usePmPages(slug: string | undefined, projectId: string) {
  return useQuery({ queryKey: ["pm-pages", slug ?? "", projectId], queryFn: () => unwrap(commands.listPmPages(slug ?? "", projectId)), enabled: !!slug && !!projectId });
}

/** A page with its document. */
export function usePmPage(slug: string | undefined, id: string | undefined) {
  return useQuery({ queryKey: ["pm-page", slug ?? "", id ?? ""], queryFn: () => unwrap(commands.getPmPage(slug ?? "", id ?? "")), enabled: !!slug && !!id });
}

export function usePmPageVersions(slug: string | undefined, id: string, enabled: boolean) {
  return useQuery({ queryKey: ["pm-page-versions", slug ?? "", id], queryFn: () => unwrap(commands.listPmPageVersions(slug ?? "", id)), enabled: !!slug && !!id && enabled });
}

export function usePageMutations(slug: string | undefined) {
  const queryClient = useQueryClient();
  // The pages, the page open, its versions and the sidebar's stars.
  const refresh = () => Promise.all(["pm-pages", "pm-page", "pm-page-versions", "favorites"].map((key) => queryClient.invalidateQueries({ queryKey: [key] })));
  return {
    create: useMutation({ mutationFn: (input: PageInput) => unwrap(commands.createPmPage(slug ?? "", input)), onSuccess: refresh }),
    update: useMutation({ mutationFn: ({ id, patch }: { id: string; patch: PagePatch }) => unwrap(commands.updatePmPage(slug ?? "", id, patch)), onSuccess: refresh }),
    archive: useMutation({ mutationFn: ({ id, archived }: { id: string; archived: boolean }) => unwrap(commands.archivePmPage(slug ?? "", id, archived)), onSuccess: refresh }),
    remove: useMutation({ mutationFn: (id: string) => unwrap(commands.deletePmPage(slug ?? "", id)), onSuccess: refresh }),
    favorite: useMutation({ mutationFn: ({ id, on }: { id: string; on: boolean }) => unwrap(commands.favoritePmPage(slug ?? "", id, on)), onSuccess: refresh }),
    duplicate: useMutation({ mutationFn: (id: string) => unwrap(commands.duplicatePmPage(slug ?? "", id)), onSuccess: refresh }),
    restore: useMutation({ mutationFn: (versionId: string) => unwrap(commands.restorePmPageVersion(slug ?? "", versionId)), onSuccess: refresh }),
  };
}

/** Where a page's pictures go: its own folder (as a work item's, D-074). */
export function pageFiles(slug: string, pageId: string) {
  return {
    add: async (path: string): Promise<Attached | null> => unwrap(commands.addPmPageFile(slug, pageId, path)),
    addBytes: async (name: string, bytes: Uint8Array): Promise<Attached | null> => unwrap(commands.addPmPageFileBytes(slug, pageId, name, Array.from(bytes))),
  };
}

/** Work items as `@` mentions in a page: the key and title. */
export async function searchItemMentions(slug: string, query: string): Promise<Hit[]> {
  const refs = await unwrap(commands.searchPmItems(slug, query, null, null));
  return refs.slice(0, 8).map((r) => ({ object: "pm-item", object_plural: "pm-items", record_id: r.id, label: `${r.key} ${r.name}` }));
}

/** The save dialog, then the page as Markdown; null when cancelled. In the dev app a preset path
 *  (`__newdawnDevSave`) stands in for the dialog. */
export async function exportPageMarkdown(slug: string, id: string, name: string): Promise<string | null> {
  const file = (name || "Untitled").replace(/[/:]/g, "-");
  const path = devPath("__newdawnDevSave") ?? (await save({ defaultPath: `${file}.md`, filters: [{ name: "Markdown", extensions: ["md"] }] }));
  if (!path) return null;
  await unwrap(commands.exportPmPageMarkdown(slug, id, path));
  return path;
}

/** The Mac's print sheet (Save as PDF is in it); the page prints alone. */
export async function printWindow(): Promise<void> {
  await unwrap(commands.printWindow());
}
