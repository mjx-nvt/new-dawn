import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { openUrl } from "@tauri-apps/plugin-opener";
import { commands, type Blocked, type CalendarEvent, type EmailMessage, type EmailThread, type EmailThreads, type MailSettings, type Participant } from "../../bindings";
import { unwrap } from "./unwrap";

export type { Blocked, CalendarEvent, EmailMessage, EmailThread, EmailThreads, MailSettings, Participant };

/** Apple Mail as the connected account (D-066, 0.10.9): a record's threads and calendar, the
 *  mail settings and the blocklist. Mail answers on this Mac; nothing is fetched or sent. */

const threadsKey = (slug: string | undefined, object: string, id: string) => ["email-threads", slug ?? "", object, id];

/** The record's threads from the cache (Mail is asked again when the last read is old). */
export function useEmailThreads(slug: string | undefined, object: string, id: string) {
  return useQuery({ queryKey: threadsKey(slug, object, id), queryFn: () => unwrap(commands.listEmailThreads(slug ?? "", object, id, false)), enabled: !!slug && !!object && !!id, staleTime: 60_000 });
}

/** Refresh: Mail is asked now. */
export function useRefreshEmailThreads(slug: string | undefined, object: string, id: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => unwrap(commands.listEmailThreads(slug ?? "", object, id, true)),
    onSuccess: (data) => queryClient.setQueryData(threadsKey(slug, object, id), data),
  });
}

export function useEmailMessages(slug: string | undefined, threadId: string | null) {
  return useQuery({ queryKey: ["email-messages", slug ?? "", threadId ?? ""], queryFn: () => unwrap(commands.listEmailMessages(slug ?? "", threadId ?? "")), enabled: !!slug && !!threadId });
}

export function useMailSettings(slug: string | undefined) {
  return useQuery({ queryKey: ["mail-settings", slug ?? ""], queryFn: () => unwrap(commands.mailSettings(slug ?? "")), enabled: !!slug });
}

export function useSetMailSettings(slug: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (settings: MailSettings) => unwrap(commands.setMailSettings(slug ?? "", settings)),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["mail-settings", slug ?? ""] });
      void queryClient.invalidateQueries({ queryKey: ["email-threads"] });
    },
  });
}

export function useBlocklist(slug: string | undefined) {
  return useQuery({ queryKey: ["blocklist", slug ?? ""], queryFn: () => unwrap(commands.listBlocklist(slug ?? "")), enabled: !!slug });
}

function useRefreshBlocklist(slug: string | undefined) {
  const queryClient = useQueryClient();
  return () => {
    void queryClient.invalidateQueries({ queryKey: ["blocklist", slug ?? ""] });
    void queryClient.invalidateQueries({ queryKey: ["email-threads"] });
  };
}

export function useBlockHandle(slug: string | undefined) {
  const refresh = useRefreshBlocklist(slug);
  return useMutation({ mutationFn: (handle: string) => unwrap(commands.blockHandle(slug ?? "", handle)), onSuccess: refresh });
}

export function useUnblockHandle(slug: string | undefined) {
  const refresh = useRefreshBlocklist(slug);
  return useMutation({ mutationFn: (id: string) => unwrap(commands.unblockHandle(slug ?? "", id)), onSuccess: refresh });
}

/** A person the CRM didn't know, from a thread's participant. */
export function useAddPersonFromEmail(slug: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ handle, name }: { handle: string; name: string }) => unwrap(commands.addPersonFromEmail(slug ?? "", handle, name)),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["email-threads"] });
      void queryClient.invalidateQueries({ queryKey: ["records", slug ?? ""] });
    },
  });
}

export function useCalendarEvents(slug: string | undefined, object: string, id: string) {
  return useQuery({ queryKey: ["calendar-events", slug ?? "", object, id], queryFn: () => unwrap(commands.calendarEvents(slug ?? "", object, id)), enabled: !!slug && !!object && !!id });
}

/** Whether Mail is running on this Mac, looked at now and then (the Accounts pane). */
export function useMailRunning() {
  return useQuery({ queryKey: ["mail-running"], queryFn: () => unwrap(commands.mailRunning()), refetchInterval: 15_000 });
}

/** Mail shows the message (its `message:` URL names the Message-ID). */
export function openInMail(messageId: string): Promise<void> {
  return openUrl(`message://${encodeURIComponent(`<${messageId}>`)}`);
}

/** A pre-filled draft in Mail; sending stays yours. */
export function draftEmail(address: string): Promise<void> {
  return openUrl(`mailto:${address}`);
}
