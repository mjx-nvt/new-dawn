import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import {
  commands,
  events,
  type Language,
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

export function useRecorderStatus() {
  return useQuery({ queryKey: STATUS_KEY, queryFn: () => unwrap(commands.recorderStatus()) });
}

/** Keeps the recorder status query live from `RecorderStatusChanged`. Mount once per window. */
export function useRecorderSync() {
  const queryClient = useQueryClient();
  useEffect(() => {
    const unlisten = events.recorderStatusChanged.listen(({ payload }) => queryClient.setQueryData(STATUS_KEY, payload));
    return () => void unlisten.then((stop) => stop());
  }, [queryClient]);
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
  /** Transcribe (Vietnamese) and translate (English) as people speak (D-041). */
  live: boolean;
}

export function useStartRecording() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: StartRecordingInput) =>
      unwrap(commands.startRecording(input.slug, input.title, input.language, input.meetingTypeId, input.presentation, input.presenter, input.live)),
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
