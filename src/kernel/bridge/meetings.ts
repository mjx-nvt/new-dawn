import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useEffect } from "react";
import {
  commands,
  events,
  type AppError,
  type Language,
  type MeetingHit_Serialize,
  type Meeting_Serialize,
  type MeetingStatus,
  type MeetingTypeSummary,
  type Stage,
  type TranscriptSegment,
} from "../../bindings";
import { unwrap } from "./unwrap";

export type { Language, MeetingStatus, MeetingTypeSummary, Stage, TranscriptSegment };
export type Meeting = Meeting_Serialize;
export type MeetingHit = MeetingHit_Serialize;

export interface TranscriptionProgress {
  stage: Stage;
  percent: number;
}

const AUDIO_EXTENSIONS = ["m4a", "mp4", "aac", "mp3", "wav", "aif", "aiff", "caf", "flac", "ogg"];

const keys = {
  meetings: (slug: string) => ["meetings", slug] as const,
  meeting: (slug: string, id: string) => ["meeting", slug, id] as const,
  transcript: (slug: string, id: string) => ["transcript", slug, id] as const,
  types: (slug: string) => ["meeting-types", slug] as const,
  progress: (id: string) => ["transcription-progress", id] as const,
  error: (id: string) => ["transcription-error", id] as const,
};

export function useMeetings(slug: string | undefined) {
  return useQuery({
    queryKey: keys.meetings(slug ?? ""),
    queryFn: () => unwrap(commands.listMeetings(slug ?? "")),
    enabled: !!slug,
  });
}

export function useMeeting(slug: string | undefined, id: string) {
  return useQuery({
    queryKey: keys.meeting(slug ?? "", id),
    queryFn: () => unwrap(commands.getMeeting(slug ?? "", id)),
    enabled: !!slug,
    retry: false,
  });
}

export function useTranscript(slug: string | undefined, id: string) {
  return useQuery({
    queryKey: keys.transcript(slug ?? "", id),
    queryFn: () => unwrap(commands.getTranscript(slug ?? "", id)),
    enabled: !!slug,
  });
}

/** Title + transcript + summary search (FTS5); disabled for an empty query. */
export function useSearchMeetings(slug: string | undefined, query: string) {
  const q = query.trim();
  return useQuery({
    queryKey: ["meeting-search", slug ?? "", q],
    queryFn: () => unwrap(commands.searchMeetings(slug ?? "", q)),
    enabled: !!slug && q.length > 0,
    placeholderData: (previous) => previous, // keep results on screen while typing
  });
}

export function useMeetingTypes(slug: string | undefined) {
  return useQuery({
    queryKey: keys.types(slug ?? ""),
    queryFn: () => unwrap(commands.listMeetingTypes(slug ?? "")),
    enabled: !!slug,
  });
}

/** Live progress of a running job (written by `useTranscriptionSync`), or undefined. */
export function useTranscriptionProgress(meetingId: string) {
  return useQuery<TranscriptionProgress | null>({ queryKey: keys.progress(meetingId), queryFn: () => null, enabled: false })
    .data;
}

/** The last job's error for a meeting (cleared when a new job starts). */
export function useTranscriptionError(meetingId: string) {
  return useQuery<AppError | null>({ queryKey: keys.error(meetingId), queryFn: () => null, enabled: false }).data;
}

/** Keeps meeting queries in step with transcription events. Mount once in the main window. */
export function useTranscriptionSync() {
  const queryClient = useQueryClient();
  useEffect(() => {
    const onProgress = events.transcriptionProgress.listen(({ payload }) => {
      const first = !queryClient.getQueryData(keys.progress(payload.meeting_id));
      queryClient.setQueryData(keys.progress(payload.meeting_id), { stage: payload.stage, percent: payload.percent });
      if (first) {
        queryClient.setQueryData(keys.error(payload.meeting_id), null);
        void queryClient.invalidateQueries({ queryKey: keys.meetings(payload.workspace_slug) });
        void queryClient.invalidateQueries({ queryKey: keys.meeting(payload.workspace_slug, payload.meeting_id) });
      }
    });
    const onFinished = events.transcriptionFinished.listen(({ payload }) => {
      queryClient.setQueryData(keys.progress(payload.meeting_id), null); // notifies mounted rows
      const error = payload.error?.code === "cancelled" ? null : payload.error;
      queryClient.setQueryData(keys.error(payload.meeting_id), error);
      void queryClient.invalidateQueries({ queryKey: keys.meetings(payload.workspace_slug) });
      void queryClient.invalidateQueries({ queryKey: keys.meeting(payload.workspace_slug, payload.meeting_id) });
      void queryClient.invalidateQueries({ queryKey: keys.transcript(payload.workspace_slug, payload.meeting_id) });
    });
    return () => {
      void onProgress.then((stop) => stop());
      void onFinished.then((stop) => stop());
    };
  }, [queryClient]);
}

export interface ImportAudioInput {
  slug: string;
  path: string;
  title: string;
  language: Language;
  meetingTypeId: string | null;
}

export function useImportAudio() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: ImportAudioInput) =>
      unwrap(commands.importAudio(input.slug, input.path, input.title, input.language, input.meetingTypeId)),
    onSuccess: (_, input) => queryClient.invalidateQueries({ queryKey: keys.meetings(input.slug) }),
  });
}

export function useRetranscribe() {
  return useMutation({
    mutationFn: ({ slug, meetingId }: { slug: string; meetingId: string }) =>
      unwrap(commands.transcribeMeeting(slug, meetingId)),
  });
}

export function cancelTranscription(meetingId: string): Promise<null> {
  return unwrap(commands.cancelTranscription(meetingId));
}

/** Native open dialog filtered to supported audio; null when the user cancels. */
export async function pickAudioFile(): Promise<string | null> {
  const picked = await open({ multiple: false, directory: false, filters: [{ name: "Audio", extensions: AUDIO_EXTENSIONS }] });
  return typeof picked === "string" ? picked : null;
}

/** A local file as a URL the webview can play (asset protocol, app data only). */
export function audioUrl(path: string): string {
  return convertFileSrc(path);
}
