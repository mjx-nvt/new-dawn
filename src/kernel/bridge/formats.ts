import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { commands, events, type Formats_Serialize as Formats } from "../../bindings";
import { unwrap } from "./unwrap";

const KEY = ["formats"] as const;

/** The owner's formats (D-072). Changes from any window arrive as `FormatsChanged`. */
export function useFormats() {
  const queryClient = useQueryClient();
  useEffect(() => {
    const stop = events.formatsChanged.listen(({ payload }) => queryClient.setQueryData(KEY, payload.formats));
    return () => void stop.then((s) => s());
  }, [queryClient]);
  // No retries: a window waits for its formats before it draws.
  return useQuery({ queryKey: KEY, queryFn: () => unwrap(commands.getFormats()), staleTime: Infinity, retry: false });
}

export function useSetFormats() {
  const queryClient = useQueryClient();
  return useMutation({ mutationFn: (formats: Formats) => unwrap(commands.setFormats(formats)), onSuccess: (saved) => queryClient.setQueryData(KEY, saved) });
}
