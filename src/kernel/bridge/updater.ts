import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { useCallback, useEffect, useRef, useState } from "react";

export interface UpdateState {
  phase: "idle" | "checking" | "none" | "available" | "downloading" | "ready" | "error";
  version?: string;
  notes?: string;
  date?: string;
  /** Download progress, 0–100 (when the size is known). */
  percent?: number;
  error?: string;
}

const AUTO_KEY = "newdawn.updates.auto";

/** Daily check at launch, on by default; a per-Mac convenience in localStorage (guarded). */
export function autoCheckEnabled(): boolean {
  try {
    return localStorage.getItem(AUTO_KEY) !== "off";
  } catch {
    return true;
  }
}

export function setAutoCheck(on: boolean) {
  try {
    localStorage.setItem(AUTO_KEY, on ? "on" : "off");
  } catch {
    // not remembered
  }
}

/** Checks the releases repo, downloads on request, restarts when ready. */
export function useUpdater() {
  const [state, setState] = useState<UpdateState>({ phase: "idle" });
  const found = useRef<Update | null>(null);

  const checkNow = useCallback(async () => {
    setState({ phase: "checking" });
    try {
      const update = await check({ timeout: 15_000 });
      found.current = update;
      if (!update) return setState({ phase: "none" });
      setState({ phase: "available", version: update.version, notes: update.body ?? undefined, date: update.date ?? undefined });
    } catch (err) {
      console.warn("update check failed", err);
      setState({ phase: "error", error: "Couldn't reach the releases page. Try again later." });
    }
  }, []);

  const install = useCallback(async () => {
    const update = found.current;
    if (!update) return;
    setState((s) => ({ ...s, phase: "downloading", percent: 0 }));
    try {
      let total = 0;
      let got = 0;
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") total = event.data.contentLength ?? 0;
        else if (event.event === "Progress") {
          got += event.data.chunkLength;
          if (total > 0) setState((s) => ({ ...s, percent: Math.min(100, Math.round((got / total) * 100)) }));
        } else if (event.event === "Finished") setState((s) => ({ ...s, phase: "ready", percent: 100 }));
      });
      setState((s) => ({ ...s, phase: "ready", percent: 100 }));
      await relaunch();
    } catch (err) {
      setState({ phase: "error", error: err instanceof Error ? err.message : String(err) });
    }
  }, []);

  return { ...state, checkNow, install };
}

/** Mount once in the main window: a check shortly after launch and every 24 h after that
 *  (the app lives in the tray for weeks), when the daily check is on. */
export function useUpdateCheckAtLaunch(onAvailable: (version: string) => void) {
  const running = useRef(false);
  useEffect(() => {
    if (!autoCheckEnabled() || import.meta.env.DEV) return;
    const run = () => {
      if (running.current || !autoCheckEnabled()) return;
      running.current = true;
      check({ timeout: 15_000 })
        .then((update) => update && onAvailable(update.version))
        .catch((err: unknown) => console.warn("update check failed", err))
        .finally(() => (running.current = false));
    };
    const first = setTimeout(run, 15_000);
    const daily = setInterval(run, 86_400_000);
    return () => {
      clearTimeout(first);
      clearInterval(daily);
    };
  }, [onAvailable]);
}
