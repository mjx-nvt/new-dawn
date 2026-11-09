import { useEffect, useSyncExternalStore } from "react";

/** The toolbar's title when a page knows better than its route — a project's, a record's or a
 *  meeting's name instead of "Project", "Record", "Meeting" (D-090). One value, set by the page
 *  on screen and cleared when it leaves. */
let current: string | undefined;
const listeners = new Set<() => void>();

function set(title: string | undefined) {
  if (title === current) return;
  current = title;
  for (const fn of listeners) fn();
}

function subscribe(fn: () => void) {
  listeners.add(fn);
  return () => {
    listeners.delete(fn);
  };
}

/** Names the toolbar's title while the calling page is on screen (nothing while `title` is empty). */
export function usePageTitle(title: string | undefined) {
  useEffect(() => {
    const trimmed = title?.trim();
    if (!trimmed) return;
    set(trimmed);
    return () => set(undefined);
  }, [title]);
}

export function useCurrentPageTitle(): string | undefined {
  return useSyncExternalStore(subscribe, () => current, () => undefined);
}
