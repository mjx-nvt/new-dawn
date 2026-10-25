import { useRouter, type AnyRouter } from "@tanstack/react-router";
import { useSyncExternalStore } from "react";

/** Where we are in the window's in-app history, kept for the window's whole life (the toolbar
 *  can unmount, e.g. on the welcome screen). `__TSR_index` is the entry's position; a new
 *  navigation (PUSH) drops everything after it; redirects arrive as REPLACE. */
interface Position {
  index: number;
  max: number;
}

let position: Position | null = null;
const listeners = new Set<() => void>();

function track(router: AnyRouter): Position {
  if (position) return position;
  const read = () => router.history.location.state.__TSR_index ?? 0;
  position = { index: read(), max: read() };
  router.history.subscribe(({ action }: { action: { type: string } }) => {
    const index = read();
    const max = action.type === "PUSH" ? index : Math.max(position?.max ?? index, index);
    position = { index, max };
    listeners.forEach((notify) => notify());
  });
  return position;
}

export function useHistoryNav() {
  const router = useRouter();
  const { index, max } = useSyncExternalStore(
    (notify) => {
      listeners.add(notify);
      return () => listeners.delete(notify);
    },
    () => track(router),
  );
  const canBack = index > 0;
  const canForward = index < max;
  return {
    canBack,
    canForward,
    back: () => canBack && router.history.back(),
    forward: () => canForward && router.history.forward(),
  };
}
