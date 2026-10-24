import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import {
  commands,
  events,
  type AppError,
  type SummaryProgress as SummaryProgressEvent,
  type SummaryStage,
  type SummaryView_Serialize,
} from "../../bindings";
import { unwrap } from "./unwrap";

export type SummaryView = SummaryView_Serialize;
export type { SummaryStage };

export type SummaryProgress = Omit<SummaryProgressEvent, "meeting_id" | "workspace_slug">;

const keys = {
  summary: (slug: string, id: string) => ["summary", slug, id] as const,
  progress: (id: string) => ["summary-progress", id] as const,
  error: (id: string) => ["summary-error", id] as const,
  ollama: ["ollama-status"] as const,
};

export function useSummary(slug: string | undefined, meetingId: string) {
  return useQuery({
    queryKey: keys.summary(slug ?? "", meetingId),
    queryFn: () => unwrap(commands.getSummary(slug ?? "", meetingId)),
    enabled: !!slug,
  });
}

/** Live progress of a running summary (written by `useSummarySync`), or undefined. */
export function useSummaryProgress(meetingId: string) {
  return useQuery<SummaryProgress | null>({ queryKey: keys.progress(meetingId), queryFn: () => null, enabled: false }).data;
}

/** The last summary job's error for a meeting (cleared when a new job starts). */
export function useSummaryError(meetingId: string) {
  return useQuery<AppError | null>({ queryKey: keys.error(meetingId), queryFn: () => null, enabled: false }).data;
}

/** Keeps summary queries in step with summary events. Mount once in the main window. */
export function useSummarySync() {
  const queryClient = useQueryClient();
  useEffect(() => {
    const onProgress = events.summaryProgress.listen(({ payload }) => {
      const first = !queryClient.getQueryData(keys.progress(payload.meeting_id));
      queryClient.setQueryData(keys.progress(payload.meeting_id), {
        stage: payload.stage,
        part: payload.part,
        parts: payload.parts,
        tokens: payload.tokens,
      });
      if (first) {
        queryClient.setQueryData(keys.error(payload.meeting_id), null);
        void queryClient.invalidateQueries({ queryKey: ["meeting", payload.workspace_slug, payload.meeting_id] });
        void queryClient.invalidateQueries({ queryKey: ["meetings", payload.workspace_slug] });
      }
    });
    const onFinished = events.summaryFinished.listen(({ payload }) => {
      queryClient.setQueryData(keys.progress(payload.meeting_id), null); // notifies mounted cards
      queryClient.setQueryData(keys.error(payload.meeting_id), payload.error?.code === "cancelled" ? null : payload.error);
      void queryClient.invalidateQueries({ queryKey: keys.summary(payload.workspace_slug, payload.meeting_id) });
      void queryClient.invalidateQueries({ queryKey: ["meeting", payload.workspace_slug, payload.meeting_id] });
      void queryClient.invalidateQueries({ queryKey: ["meetings", payload.workspace_slug] });
      void queryClient.invalidateQueries({ queryKey: ["meeting-search"] });
    });
    return () => {
      void onProgress.then((stop) => stop());
      void onFinished.then((stop) => stop());
    };
  }, [queryClient]);
}

export function useSummarise() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ slug, meetingId, deep, keepEdits }: { slug: string; meetingId: string; deep: boolean; keepEdits: boolean }) =>
      unwrap(commands.summariseMeeting(slug, meetingId, deep, keepEdits)),
    onSuccess: (_, { meetingId }) => queryClient.setQueryData(keys.error(meetingId), null),
  });
}

export function cancelSummary(meetingId: string): Promise<null> {
  return unwrap(commands.cancelSummary(meetingId));
}

export function useUpdateSection(slug: string, meetingId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ index, markdown }: { index: number; markdown: string }) =>
      unwrap(commands.updateSummarySection(slug, meetingId, index, markdown)),
    onSuccess: (view) => queryClient.setQueryData(keys.summary(slug, meetingId), view),
  });
}

export function useCopyEmail() {
  return useMutation({
    mutationFn: ({ slug, meetingId }: { slug: string; meetingId: string }) => unwrap(commands.copySummaryEmail(slug, meetingId)),
  });
}

export function useOpenInMail() {
  return useMutation({
    mutationFn: ({ slug, meetingId }: { slug: string; meetingId: string }) => unwrap(commands.openSummaryInMail(slug, meetingId)),
  });
}

/** Is Ollama running with the model? Rechecked every 10 s while it isn't. */
export function useOllamaStatus() {
  return useQuery({
    queryKey: keys.ollama,
    queryFn: () => unwrap(commands.ollamaStatus()),
    refetchInterval: (query) => (query.state.data?.running && query.state.data.model_installed ? false : 10_000),
  });
}

export function openOllamaApp(): Promise<boolean> {
  return unwrap(commands.openOllamaApp()).then(
    () => true,
    (err: unknown) => {
      console.warn("could not open Ollama", err);
      return false;
    },
  );
}
