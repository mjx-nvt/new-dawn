import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { commands, events, type AppError, type BackupStatus } from "../../bindings";
import { unwrap } from "./unwrap";

export type { BackupStatus };

const keys = {
  backup: ["backup-status"] as const,
  ollama: ["ollama-status"] as const,
};

export function getOnboardingDone(): Promise<boolean> {
  return unwrap(commands.getOnboardingDone());
}

export function useFinishOnboarding() {
  return useMutation({ mutationFn: (done: boolean) => unwrap(commands.setOnboardingDone(done)) });
}

/** Shows a sample notification (the first one makes macOS ask for permission). */
export function testNotification(): Promise<null> {
  return unwrap(commands.testNotification());
}

export interface PullState {
  /** The model being (or last) downloaded. */
  model: string;
  running: boolean;
  status: string;
  completedMb: number;
  totalMb: number;
  error: AppError | null;
  done: boolean;
}

/** Downloads Qwen through Ollama, with live progress from the pull events. */
export function useOllamaPull() {
  const queryClient = useQueryClient();
  const [state, setState] = useState<PullState>({ model: "", running: false, status: "", completedMb: 0, totalMb: 0, error: null, done: false });

  useEffect(() => {
    const onProgress = events.ollamaPullProgress.listen(({ payload }) =>
      setState((s) => ({
        ...s,
        model: payload.model,
        running: true,
        status: payload.status,
        completedMb: payload.completed_mb,
        totalMb: payload.total_mb,
      })),
    );
    const onFinished = events.ollamaPullFinished.listen(({ payload }) => {
      setState((s) => ({ ...s, model: payload.model, running: false, error: payload.error, done: !payload.error }));
      void queryClient.invalidateQueries({ queryKey: keys.ollama });
    });
    return () => {
      void onProgress.then((stop) => stop());
      void onFinished.then((stop) => stop());
    };
  }, [queryClient]);

  /** Downloads `model`, or the selected text model when omitted. */
  const start = (model?: string) => {
    setState({ model: model ?? "", running: true, status: "starting", completedMb: 0, totalMb: 0, error: null, done: false });
    unwrap(commands.pullOllamaModel(model ?? null)).catch((error: AppError) => setState((s) => ({ ...s, running: false, error })));
  };
  return { ...state, start };
}

export function useBackupStatus() {
  return useQuery({ queryKey: keys.backup, queryFn: () => unwrap(commands.getBackupStatus()) });
}

/** Opens the folder picker; `null` (cancel) leaves the setting alone. */
export function useChooseBackupFolder() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (off: boolean) => {
      if (off) return unwrap(commands.setBackupFolder(null));
      const picked = await open({ directory: true, multiple: false, title: "Choose a backup folder" });
      if (typeof picked !== "string") return null;
      return unwrap(commands.setBackupFolder(picked));
    },
    onSuccess: (status) => status && queryClient.setQueryData(keys.backup, status),
  });
}

export function useBackupNow() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => unwrap(commands.backupNow()),
    onSettled: () => queryClient.invalidateQueries({ queryKey: keys.backup }),
  });
}
