import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type StorageReport } from "../../bindings";
import { unwrap } from "./unwrap";

export type { StorageReport };

const key = ["storage-report"] as const;

export function useStorageReport() {
  return useQuery({ queryKey: key, queryFn: () => unwrap(commands.getStorageReport()) });
}

function useStorageMutation<V, R>(call: (vars: V) => Promise<R>) {
  const queryClient = useQueryClient();
  return useMutation({ mutationFn: call, onSettled: () => queryClient.invalidateQueries({ queryKey: key }) });
}

/** 0 = keep recordings forever. */
export function useSetRetentionDays() {
  return useStorageMutation((days: number) => unwrap(commands.setRetentionDays(days)));
}

export function useCompressRecordings() {
  return useStorageMutation((slug: string) => unwrap(commands.compressRecordings(slug)));
}

export function useTrashOldRecordings() {
  return useStorageMutation(({ slug, days }: { slug: string; days: number }) => unwrap(commands.trashOldRecordings(slug, days)));
}

export function formatBytes(bytes: number | null): string {
  bytes = bytes ?? 0;
  if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(2)} GB`;
  if (bytes >= 1e6) return `${(bytes / 1e6).toFixed(1)} MB`;
  return `${Math.max(1, Math.round(bytes / 1e3))} KB`;
}
