import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type InboxCounts, type InboxQuery, type InboxShow, type Notice_Serialize, type NoticeKind, type PmPrefs } from "../../bindings";
import { unwrap } from "./unwrap";

/** The Inbox (D-075): what agents and automations did on work items the owner follows, mentions,
 *  due-date reminders. */
export type Notice = Notice_Serialize;
export type { InboxCounts, InboxQuery, InboxShow, NoticeKind, PmPrefs };

export function useNotices(slug: string | undefined, query: InboxQuery) {
  return useQuery({ queryKey: ["notices", slug ?? "", query], queryFn: () => unwrap(commands.listNotices(slug ?? "", query)), enabled: !!slug, refetchInterval: 60_000 });
}

/** Unread in All and in Mentions — the sidebar's count; read again each minute (the automations
 *  run in the background). */
export function useNoticeCounts(slug: string | undefined) {
  return useQuery({ queryKey: ["notice-counts", slug ?? ""], queryFn: () => unwrap(commands.noticeCounts(slug ?? "")), enabled: !!slug, refetchInterval: 60_000 });
}

export function useNoticeMutations(slug: string | undefined) {
  const queryClient = useQueryClient();
  const refresh = () => {
    void queryClient.invalidateQueries({ queryKey: ["notices"] });
    void queryClient.invalidateQueries({ queryKey: ["notice-counts"] });
  };
  return {
    mark: useMutation({ mutationFn: ({ id, read }: { id: string; read: boolean }) => unwrap(commands.markNotice(slug ?? "", id, read)), onSuccess: refresh }),
    markAll: useMutation({ mutationFn: (mentions: boolean) => unwrap(commands.markAllNotices(slug ?? "", mentions)), onSuccess: refresh }),
    snooze: useMutation({ mutationFn: ({ id, until }: { id: string; until: string | null }) => unwrap(commands.snoozeNotice(slug ?? "", id, until)), onSuccess: refresh }),
    archive: useMutation({ mutationFn: ({ id, archived }: { id: string; archived: boolean }) => unwrap(commands.archiveNotice(slug ?? "", id, archived)), onSuccess: refresh }),
  };
}

/** The owner's choices for work items: the Inbox's kinds turned off, the attachment limit. */
export function usePmPrefs() {
  return useQuery({ queryKey: ["pm-prefs"], queryFn: () => unwrap(commands.getPmPrefs()) });
}

export function useSetPmPrefs() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (prefs: PmPrefs) => unwrap(commands.setPmPrefs(prefs)),
    onSuccess: (prefs) => {
      queryClient.setQueryData(["pm-prefs"], prefs);
      void queryClient.invalidateQueries({ queryKey: ["notices"] });
      void queryClient.invalidateQueries({ queryKey: ["notice-counts"] });
    },
  });
}
