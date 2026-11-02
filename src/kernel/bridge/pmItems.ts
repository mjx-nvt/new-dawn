import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { openPath, openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import {
  commands,
  type ItemInput_Deserialize,
  type ItemPatch_Deserialize,
  type PmActivity_Serialize,
  type PmAttachment,
  type PmBulkAction,
  type PmItemRef,
  type PmLink,
  type PmRelation,
  type PmRelationKind,
  type PmComment_Serialize,
  type PmDraft_Serialize,
  type PmItem_Serialize,
  type PmVersion_Serialize,
  type Priority,
  type Reaction,
} from "../../bindings";
import type { Attached } from "./files";
import { unwrap } from "./unwrap";

/** Plane's work items (D-074). */
export type PmItem = PmItem_Serialize;
export type ItemInput = ItemInput_Deserialize;
export type ItemPatch = ItemPatch_Deserialize;
export type PmComment = PmComment_Serialize;
export type PmActivity = PmActivity_Serialize;
export type PmVersion = PmVersion_Serialize;
export type PmDraft = PmDraft_Serialize;
export type { PmAttachment, PmBulkAction, PmItemRef, PmLink, PmRelation, PmRelationKind, Priority, Reaction };

/** After any change to items: the lists, the items, their comments and activity, the drafts. */
function useRefresh() {
  const queryClient = useQueryClient();
  return () => {
    for (const key of ["pm-items", "pm-item", "pm-sub-items", "pm-activity", "pm-versions", "pm-comments", "pm-reactions", "pm-drafts", "pm-relations", "pm-links", "pm-attachments", "notices", "notice-counts"]) void queryClient.invalidateQueries({ queryKey: [key] });
  };
}

export function usePmItems(slug: string | undefined, projectId: string, archived = false) {
  return useQuery({ queryKey: ["pm-items", slug ?? "", projectId, archived], queryFn: () => unwrap(commands.listPmItems(slug ?? "", projectId, archived)), enabled: !!slug && !!projectId });
}

export function usePmItem(slug: string | undefined, id: string | undefined) {
  return useQuery({ queryKey: ["pm-item", slug ?? "", id ?? ""], queryFn: () => unwrap(commands.getPmItem(slug ?? "", id ?? "")), enabled: !!slug && !!id });
}

export function usePmSubItems(slug: string | undefined, id: string) {
  return useQuery({ queryKey: ["pm-sub-items", slug ?? "", id], queryFn: () => unwrap(commands.listPmSubItems(slug ?? "", id)), enabled: !!slug && !!id });
}

export function useCreatePmItem(slug: string | undefined) {
  const refresh = useRefresh();
  return useMutation({ mutationFn: (input: ItemInput) => unwrap(commands.createPmItem(slug ?? "", input)), onSuccess: refresh });
}

export function useUpdatePmItem(slug: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, patch }: { id: string; patch: ItemPatch }) => unwrap(commands.updatePmItem(slug ?? "", id, patch)),
    // The item as answered; the lists and what hangs off items read again — other items too (a
    // parent's progress), not this one, just written. Autosaves of a description come here.
    onSuccess: (item) => {
      queryClient.setQueryData(["pm-item", slug ?? "", item.id], item);
      for (const key of ["pm-items", "pm-sub-items", "pm-activity", "pm-versions"]) void queryClient.invalidateQueries({ queryKey: [key] });
      void queryClient.invalidateQueries({ queryKey: ["pm-item"], predicate: (q) => q.queryKey[2] !== item.id });
    },
  });
}

export function useArchivePmItem(slug: string | undefined) {
  const refresh = useRefresh();
  return useMutation({ mutationFn: ({ id, archived }: { id: string; archived: boolean }) => unwrap(commands.archivePmItem(slug ?? "", id, archived)), onSuccess: refresh });
}

export function useDeletePmItem(slug: string | undefined) {
  const refresh = useRefresh();
  return useMutation({ mutationFn: (id: string) => unwrap(commands.deletePmItem(slug ?? "", id)), onSuccess: refresh });
}

export function useCopyPmItem(slug: string | undefined) {
  const refresh = useRefresh();
  return useMutation({ mutationFn: ({ id, projectId }: { id: string; projectId: string | null }) => unwrap(commands.copyPmItem(slug ?? "", id, projectId)), onSuccess: refresh });
}

export function useSubscribePmItem(slug: string | undefined) {
  const refresh = useRefresh();
  return useMutation({ mutationFn: ({ id, on }: { id: string; on: boolean }) => unwrap(commands.subscribePmItem(slug ?? "", id, on)), onSuccess: refresh });
}

export function usePmActivity(slug: string | undefined, id: string) {
  return useQuery({ queryKey: ["pm-activity", slug ?? "", id], queryFn: () => unwrap(commands.listPmActivity(slug ?? "", id)), enabled: !!slug && !!id });
}

export function usePmVersions(slug: string | undefined, id: string, enabled: boolean) {
  return useQuery({ queryKey: ["pm-versions", slug ?? "", id], queryFn: () => unwrap(commands.listPmVersions(slug ?? "", id)), enabled: !!slug && !!id && enabled });
}

export function useRestorePmVersion(slug: string | undefined) {
  const refresh = useRefresh();
  return useMutation({ mutationFn: (versionId: string) => unwrap(commands.restorePmVersion(slug ?? "", versionId)), onSuccess: refresh });
}

export function usePmComments(slug: string | undefined, itemId: string) {
  return useQuery({ queryKey: ["pm-comments", slug ?? "", itemId], queryFn: () => unwrap(commands.listPmComments(slug ?? "", itemId)), enabled: !!slug && !!itemId });
}

export function useCommentMutations(slug: string | undefined, itemId: string) {
  const queryClient = useQueryClient();
  const refresh = useRefresh();
  const done = (comments: PmComment[]) => {
    queryClient.setQueryData(["pm-comments", slug ?? "", itemId], comments);
    refresh();
  };
  return {
    add: useMutation({ mutationFn: ({ body, parentId }: { body: string; parentId: string | null }) => unwrap(commands.addPmComment(slug ?? "", itemId, body, parentId)), onSuccess: done }),
    edit: useMutation({ mutationFn: ({ id, body }: { id: string; body: string }) => unwrap(commands.editPmComment(slug ?? "", id, body)), onSuccess: done }),
    remove: useMutation({ mutationFn: (id: string) => unwrap(commands.deletePmComment(slug ?? "", id)), onSuccess: done }),
  };
}

export function usePmItemReactions(slug: string | undefined, itemId: string) {
  return useQuery({ queryKey: ["pm-reactions", slug ?? "", itemId], queryFn: () => unwrap(commands.listPmItemReactions(slug ?? "", itemId)), enabled: !!slug && !!itemId });
}

export function useToggleReaction(slug: string | undefined, itemId: string) {
  const refresh = useRefresh();
  return useMutation({ mutationFn: ({ commentId, emoji }: { commentId: string | null; emoji: string }) => unwrap(commands.togglePmReaction(slug ?? "", itemId, commentId, emoji)), onSuccess: refresh });
}

export function usePmDrafts(slug: string | undefined) {
  return useQuery({ queryKey: ["pm-drafts", slug ?? ""], queryFn: () => unwrap(commands.listPmDrafts(slug ?? "")), enabled: !!slug });
}

export function useDraftMutations(slug: string | undefined) {
  const refresh = useRefresh();
  return {
    save: useMutation({ mutationFn: ({ id, input }: { id: string | null; input: ItemInput }) => unwrap(commands.savePmDraft(slug ?? "", id, input)), onSuccess: refresh }),
    copy: useMutation({ mutationFn: (id: string) => unwrap(commands.copyPmDraft(slug ?? "", id)), onSuccess: refresh }),
    remove: useMutation({ mutationFn: (id: string) => unwrap(commands.deletePmDraft(slug ?? "", id)), onSuccess: refresh }),
    publish: useMutation({ mutationFn: ({ id, input }: { id: string; input: ItemInput }) => unwrap(commands.publishPmDraft(slug ?? "", id, input)), onSuccess: refresh }),
  };
}

/** Where an item's pictures go (the editor's files hook). */
export function itemFiles(slug: string, itemId: string) {
  return {
    add: async (path: string): Promise<Attached | null> => unwrap(commands.addPmItemFile(slug, itemId, path)),
    addBytes: async (name: string, bytes: Uint8Array): Promise<Attached | null> => unwrap(commands.addPmItemFileBytes(slug, itemId, name, Array.from(bytes))),
  };
}

export async function openItemWindow(projectId: string, itemId: string): Promise<void> {
  await unwrap(commands.openPmItemWindow(projectId, itemId));
}

// ── 0.11.2 (D-075): relations, links, attachments, votes, bulk changes ──

export function usePmRelations(slug: string | undefined, itemId: string) {
  return useQuery({ queryKey: ["pm-relations", slug ?? "", itemId], queryFn: () => unwrap(commands.listPmRelations(slug ?? "", itemId)), enabled: !!slug && !!itemId });
}

/** Work items to relate to, from every project (by key or title words). */
export function useSearchPmItems(slug: string | undefined, query: string, except: string, enabled: boolean) {
  return useQuery({ queryKey: ["pm-search", slug ?? "", query, except], queryFn: () => unwrap(commands.searchPmItems(slug ?? "", query, except)), enabled: !!slug && enabled, placeholderData: keepPreviousData });
}

export function useRelationMutations(slug: string | undefined, itemId: string) {
  const refresh = useRefresh();
  return {
    add: useMutation({ mutationFn: ({ kind, otherId }: { kind: PmRelationKind; otherId: string }) => unwrap(commands.addPmRelation(slug ?? "", itemId, kind, otherId)), onSuccess: refresh }),
    remove: useMutation({ mutationFn: (id: string) => unwrap(commands.removePmRelation(slug ?? "", id)), onSuccess: refresh }),
  };
}

export function usePmLinks(slug: string | undefined, itemId: string) {
  return useQuery({ queryKey: ["pm-links", slug ?? "", itemId], queryFn: () => unwrap(commands.listPmLinks(slug ?? "", itemId)), enabled: !!slug && !!itemId });
}

export function useLinkMutations(slug: string | undefined, itemId: string) {
  const refresh = useRefresh();
  return {
    add: useMutation({ mutationFn: ({ url, title }: { url: string; title: string }) => unwrap(commands.addPmLink(slug ?? "", itemId, url, title)), onSuccess: refresh }),
    edit: useMutation({ mutationFn: ({ id, url, title }: { id: string; url: string; title: string }) => unwrap(commands.editPmLink(slug ?? "", id, url, title)), onSuccess: refresh }),
    remove: useMutation({ mutationFn: (id: string) => unwrap(commands.removePmLink(slug ?? "", id)), onSuccess: refresh }),
  };
}

/** A link opens in the owner's browser or Mail — never inside New Dawn. */
export function openLink(url: string): Promise<void> {
  return openUrl(url);
}

export function usePmAttachments(slug: string | undefined, itemId: string) {
  return useQuery({ queryKey: ["pm-attachments", slug ?? "", itemId], queryFn: () => unwrap(commands.listPmAttachments(slug ?? "", itemId)), enabled: !!slug && !!itemId });
}

export function useAttachmentMutations(slug: string | undefined, itemId: string) {
  const refresh = useRefresh();
  return {
    add: useMutation({ mutationFn: (paths: string[]) => unwrap(commands.addPmAttachments(slug ?? "", itemId, paths)), onSuccess: refresh }),
    remove: useMutation({ mutationFn: (id: string) => unwrap(commands.removePmAttachment(slug ?? "", id)), onSuccess: refresh }),
  };
}

export function openAttachmentFile(a: PmAttachment, reveal = false): Promise<void> {
  return reveal ? revealItemInDir(a.path) : openPath(a.path);
}

export function useVotePmItem(slug: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, vote }: { id: string; vote: number }) => unwrap(commands.votePmItem(slug ?? "", id, vote)),
    onSuccess: (item) => queryClient.setQueryData(["pm-item", slug ?? "", item.id], item),
  });
}

/** Bulk changes (List): the same edits, an action, or copies — every item checked first. */
export function useBulkPmItems(slug: string | undefined) {
  const refresh = useRefresh();
  return {
    update: useMutation({ mutationFn: ({ ids, patch }: { ids: string[]; patch: ItemPatch }) => unwrap(commands.bulkUpdatePmItems(slug ?? "", ids, patch)), onSuccess: refresh }),
    act: useMutation({ mutationFn: ({ ids, action }: { ids: string[]; action: PmBulkAction }) => unwrap(commands.bulkPmItems(slug ?? "", ids, action)), onSuccess: refresh }),
    copy: useMutation({ mutationFn: ({ ids, projectId }: { ids: string[]; projectId: string }) => unwrap(commands.copyPmItems(slug ?? "", ids, projectId)), onSuccess: refresh }),
  };
}
