import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { commands, events, type AppError, type ModelInfo, type SpeechJob } from "../../bindings";
import { unwrap } from "./unwrap";

export type { ModelInfo, SpeechJob };

const MODELS_KEY = ["models"] as const;

export interface DownloadProgress {
  downloaded: number;
  total: number;
  bytesPerSec: number;
}

export function useModels() {
  return useQuery({ queryKey: MODELS_KEY, queryFn: () => unwrap(commands.listModels()) });
}

/** Live progress per model id, plus the last error per model; refreshes the list when a download ends. */
export function useModelDownloads() {
  const queryClient = useQueryClient();
  const [progress, setProgress] = useState<Record<string, DownloadProgress>>({});
  const [errors, setErrors] = useState<Record<string, AppError | null>>({});

  useEffect(() => {
    const onProgress = events.modelDownloadProgress.listen(({ payload }) =>
      setProgress((p) => ({
        ...p,
        [payload.id]: { downloaded: payload.downloaded, total: payload.total, bytesPerSec: payload.bytes_per_sec },
      })),
    );
    const onFinished = events.modelDownloadFinished.listen(({ payload }) => {
      setProgress(({ [payload.id]: _, ...rest }) => rest);
      // A user cancel is not a failure; the row simply shows "Paused".
      const error = payload.error?.code === "cancelled" ? null : payload.error;
      setErrors((e) => ({ ...e, [payload.id]: error }));
      void queryClient.invalidateQueries({ queryKey: MODELS_KEY });
      void queryClient.invalidateQueries({ queryKey: SPEECH_CHOICES_KEY });
    });
    return () => {
      void onProgress.then((stop) => stop());
      void onFinished.then((stop) => stop());
    };
  }, [queryClient]);

  return { progress, errors };
}

function useModelMutation(run: (id: string) => Promise<unknown>) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: run,
    onSettled: () => {
      void queryClient.invalidateQueries({ queryKey: MODELS_KEY });
      void queryClient.invalidateQueries({ queryKey: SPEECH_CHOICES_KEY });
    },
  });
}

export const useStartModelDownload = () => useModelMutation((id) => unwrap(commands.startModelDownload(id)));
export const useCancelModelDownload = () => useModelMutation((id) => unwrap(commands.cancelModelDownload(id)));
export const useDeleteModel = () => useModelMutation((id) => unwrap(commands.deleteModel(id)));

export function revealModelsFolder(): Promise<null> {
  return unwrap(commands.revealModelsFolder());
}

export function useWhisperIdleUnload() {
  return useQuery({ queryKey: ["whisper-idle-unload"], queryFn: () => unwrap(commands.getWhisperIdleUnload()) });
}

export function useSetWhisperIdleUnload() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (secs: number) => unwrap(commands.setWhisperIdleUnload(secs)),
    onSettled: () => queryClient.invalidateQueries({ queryKey: ["whisper-idle-unload"] }),
  });
}

const SPEECH_CHOICES_KEY = ["speech-choices"] as const;

/** Each speech job's Whisper model: the owner's choice (null = automatic) and what runs now. */
export function useSpeechChoices() {
  return useQuery({ queryKey: SPEECH_CHOICES_KEY, queryFn: () => unwrap(commands.getSpeechChoices()) });
}

export function useSetSpeechModel() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (v: { job: SpeechJob; model: string | null }) => unwrap(commands.setSpeechModel(v.job, v.model)),
    onSettled: () => queryClient.invalidateQueries({ queryKey: SPEECH_CHOICES_KEY }),
  });
}
