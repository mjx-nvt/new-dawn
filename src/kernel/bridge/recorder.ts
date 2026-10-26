import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import {
  commands,
  events,
  type Language,
  type LiveDirection,
  type LiveStatus_Serialize,
  type PermissionState,
  type RecorderLevel,
  type RecorderStatus_Serialize,
  type SystemAudio,
} from "../../bindings";
import { unwrap } from "./unwrap";

export type RecorderStatus = RecorderStatus_Serialize;
export type LiveStatus = LiveStatus_Serialize;
export type { PermissionState, SystemAudio };

const STATUS_KEY = ["recorder-status"] as const;
const MIC_KEY = ["microphone-permission"] as const;
const LIVE_LINES_KEY = ["live-lines"] as const;
const LAST_QUICK_KEY = ["last-quick"] as const;

/** A quick translation's workspace and its last status. */
export interface QuickSession {
  slug: string;
  live: LiveStatus;
}

export function useRecorderStatus() {
  return useQuery({ queryKey: STATUS_KEY, queryFn: () => unwrap(commands.recorderStatus()) });
}

/** Keeps the recorder status query live from `RecorderStatusChanged`, and the last quick
 *  translation's status (`useLastQuick`) until a recording that saves starts. Mount once per window. */
export function useRecorderSync() {
  const queryClient = useQueryClient();
  useEffect(() => {
    const unlisten = events.recorderStatusChanged.listen(({ payload }) => {
      queryClient.setQueryData(STATUS_KEY, payload);
      if (payload.live?.quick && payload.workspace_slug) queryClient.setQueryData<QuickSession>(LAST_QUICK_KEY, { slug: payload.workspace_slug, live: payload.live });
      else if (payload.meeting_id) queryClient.setQueryData(LAST_QUICK_KEY, null);
    });
    return () => void unlisten.then((stop) => stop());
  }, [queryClient]);
}

/** The last quick translation (written by `useRecorderSync`): once it has ended, wherever it was
 *  stopped, the Live page still shows its lines (they stay in memory) to read and copy. */
export function useLastQuick(): QuickSession | null {
  return useQuery<QuickSession | null>({ queryKey: LAST_QUICK_KEY, queryFn: () => null, enabled: false, gcTime: Infinity }).data ?? null;
}

/** The 10 Hz meter (you / them / elapsed) while recording; null when idle. */
export function useRecorderLevel(active: boolean): RecorderLevel | null {
  const [level, setLevel] = useState<RecorderLevel | null>(null);
  useEffect(() => {
    if (!active) {
      setLevel(null);
      return;
    }
    const unlisten = events.recorderLevel.listen(({ payload }) => setLevel(payload));
    return () => void unlisten.then((stop) => stop());
  }, [active]);
  return level;
}

export interface StartRecordingInput {
  slug: string;
  title: string;
  language: Language;
  meetingTypeId: string | null;
  presentation: boolean;
  presenter: string | null;
  /** Transcribe and translate as people speak (D-041). */
  live: boolean;
  /** Live only: false = quick translation, nothing saved (default true). */
  save?: boolean;
  /** Live only: which way it translates (default Tiếng Việt → English). */
  direction?: LiveDirection;
}

export function useStartRecording() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: StartRecordingInput) =>
      unwrap(commands.startRecording(input.slug, input.title, input.language, input.meetingTypeId, input.presentation, input.presenter, input.live, input.save ?? true, input.direction ?? "vi_en")),
    // A live session clears the last quick translation's lines as it starts.
    onSuccess: (_, input) => {
      if (input.live) queryClient.setQueryData(LIVE_LINES_KEY, []);
    },
    onSettled: () =>
      Promise.all([
        queryClient.invalidateQueries({ queryKey: MIC_KEY }),
        queryClient.invalidateQueries({ queryKey: ["meetings"] }),
      ]),
  });
}

export function useSetPaused() {
  return useMutation({ mutationFn: (paused: boolean) => unwrap(commands.setRecordingPaused(paused)) });
}

export function useStopRecording() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => unwrap(commands.stopRecording()),
    // The meeting and its lines too: a live session's page, if open, gets no transcription event
    // until the recording is compressed (D-041).
    onSettled: () => Promise.all(["meetings", "meeting", "transcript"].map((key) => queryClient.invalidateQueries({ queryKey: [key] }))),
  });
}

/** A quick translation's lines (in memory only, never saved): every second while it runs, once
 *  more when it has ended (its last lines and translations land while it stops), then as they are. */
export function useLiveLines(running: boolean) {
  const queryClient = useQueryClient();
  useEffect(() => {
    if (!running) void queryClient.invalidateQueries({ queryKey: LIVE_LINES_KEY });
  }, [running, queryClient]);
  return useQuery({ queryKey: LIVE_LINES_KEY, queryFn: () => unwrap(commands.liveLines()), refetchInterval: running ? 1000 : false });
}

export function useMicrophonePermission() {
  return useQuery({ queryKey: MIC_KEY, queryFn: () => unwrap(commands.microphonePermission()) });
}

export function useRequestMicrophone() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => unwrap(commands.requestMicrophonePermission()),
    onSuccess: (state) => queryClient.setQueryData(MIC_KEY, state),
  });
}

/** System Settings → Privacy & Security → Microphone, or → Screen & System Audio Recording.
 *  Fire-and-forget from buttons: resolves false (and logs) instead of rejecting. */
export function openPrivacySettings(systemAudio: boolean): Promise<boolean> {
  return unwrap(commands.openPrivacySettings(systemAudio)).then(
    () => true,
    (err: unknown) => {
      console.warn("could not open System Settings", err);
      return false;
    },
  );
}
