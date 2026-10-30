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
  /** When this job first reported (ms since epoch), for the time-left estimate. */
  startedAt: number;
}

const AUDIO_EXTENSIONS = ["m4a", "mp4", "aac", "mp3", "wav", "aif", "aiff", "caf", "flac", "ogg"];

const keys = {
  meetings: (slug: string) => ["meetings", slug] as const,
  meeting: (slug: string, id: string) => ["meeting", slug, id] as const,
  transcript: (slug: string, id: string) => ["transcript", slug, id] as const,
  waveform: (slug: string, id: string) => ["waveform", slug, id] as const,
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

/** `live`: a live session is writing it (D-041), so new lines arrive by polling every second. */
export function useTranscript(slug: string | undefined, id: string, live = false) {
  return useQuery({
    queryKey: keys.transcript(slug ?? "", id),
    queryFn: () => unwrap(commands.getTranscript(slug ?? "", id)),
    enabled: !!slug,
    refetchInterval: live ? 1000 : false,
  });
}

/** The recording's peaks for the player; the first call decodes the file (seconds), then it's cached. */
export function useWaveform(slug: string | undefined, id: string, enabled: boolean) {
  return useQuery({
    queryKey: keys.waveform(slug ?? "", id),
    queryFn: () => unwrap(commands.getWaveform(slug ?? "", id)),
    enabled: !!slug && enabled,
    staleTime: Infinity,
    retry: false,
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

/** Search by meaning (D-053): meetings close to what the words mean, whatever words they used,
 *  with how many meetings are indexed so far. Needs the embedding model; off below three characters. */
export function useSearchByMeaning(slug: string | undefined, query: string, enabled: boolean) {
  const q = query.trim();
  return useQuery({
    queryKey: ["meeting-meaning", slug ?? "", q],
    queryFn: () => unwrap(commands.searchByMeaning(slug ?? "", q)),
    enabled: enabled && !!slug && q.length >= 3,
    placeholderData: (previous) => previous,
    staleTime: 60_000,
    retry: false,
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
      const prior = queryClient.getQueryData<TranscriptionProgress | null>(keys.progress(payload.meeting_id));
      const first = !prior;
      queryClient.setQueryData(keys.progress(payload.meeting_id), { stage: payload.stage, percent: payload.percent, startedAt: prior && prior.stage !== "queued" ? prior.startedAt : Date.now() });
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
  /** One main presenter (else a discussion), and their name if the audio may not say it. */
  presentation: boolean;
  presenter: string | null;
}

export function useImportAudio() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: ImportAudioInput) =>
      unwrap(commands.importAudio(input.slug, input.path, input.title, input.language, input.meetingTypeId, input.presentation, input.presenter)),
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

/** Corrects one transcript line; the summary's "transcript changed" flag and search follow. */
export function useUpdateSegment(slug: string, meetingId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ seq, text }: { seq: number; text: string }) => unwrap(commands.updateTranscriptSegment(slug, meetingId, seq, text)),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: keys.transcript(slug, meetingId) });
      void queryClient.invalidateQueries({ queryKey: ["summary", slug, meetingId] });
      void queryClient.invalidateQueries({ queryKey: ["meeting-search"] });
      void queryClient.invalidateQueries({ queryKey: ["vocabulary", slug] }); // a correction may have taught it
      void queryClient.invalidateQueries({ queryKey: ["whisper-prompt", slug] });
    },
  });
}

/** Undoes the vocabulary's correction of one line (back to what Whisper wrote). */
export function useRestoreSegment(slug: string, meetingId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (seq: number) => unwrap(commands.restoreTranscriptSegment(slug, meetingId, seq)),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: keys.transcript(slug, meetingId) });
      void queryClient.invalidateQueries({ queryKey: ["summary", slug, meetingId] });
      void queryClient.invalidateQueries({ queryKey: ["meeting-search"] });
    },
  });
}

/** Discussion, or a presentation by one presenter (D-038); the next transcript and summary use it. */
export function useSetMeetingFormat(slug: string, meetingId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (v: { presentation: boolean; presenter: string | null }) =>
      unwrap(commands.setMeetingFormat(slug, meetingId, v.presentation ? "presentation" : "discussion", v.presenter)),
    onSuccess: (meeting) => {
      queryClient.setQueryData(keys.meeting(slug, meetingId), meeting);
      void queryClient.invalidateQueries({ queryKey: keys.meetings(slug) });
    },
  });
}

export function useSaveNotes(slug: string, meetingId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (notes: string) => unwrap(commands.setMeetingNotes(slug, meetingId, notes)),
    onSuccess: (meeting) => queryClient.setQueryData(keys.meeting(slug, meetingId), meeting),
  });
}

/** New title; the summary's email subject follows. */
export function useRenameMeeting(slug: string, meetingId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (title: string) => unwrap(commands.renameMeeting(slug, meetingId, title)),
    onSuccess: (meeting) => {
      queryClient.setQueryData(keys.meeting(slug, meetingId), meeting);
      void queryClient.invalidateQueries({ queryKey: keys.meetings(slug) });
      void queryClient.invalidateQueries({ queryKey: ["summary", slug, meetingId] });
      void queryClient.invalidateQueries({ queryKey: ["meeting-search"] });
    },
  });
}

/** Recording to the Trash; transcript, summary and export rows deleted (exported files stay). */
export function useDeleteMeeting(slug: string, meetingId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => unwrap(commands.deleteMeeting(slug, meetingId)),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: keys.meetings(slug) });
      void queryClient.invalidateQueries({ queryKey: ["meeting-search"] });
    },
  });
}

/** Changes (or clears) a meeting's type; the next minutes export uses its template. */
export function useSetMeetingType(slug: string, meetingId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (typeId: string | null) => unwrap(commands.setMeetingType(slug, meetingId, typeId)),
    onSuccess: (meeting) => {
      queryClient.setQueryData(keys.meeting(slug, meetingId), meeting);
      void queryClient.invalidateQueries({ queryKey: keys.meetings(slug) });
    },
  });
}
