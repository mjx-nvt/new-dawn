import { useQuery } from "@tanstack/react-query";
import { getIdentifier, getVersion } from "@tauri-apps/api/app";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

export function useAppInfo() {
  return useQuery({
    queryKey: ["app-info"],
    queryFn: async () => ({ version: await getVersion(), identifier: await getIdentifier() }),
  });
}

/** The main window — not Settings, not a work item's own window (D-074). */
export function isMainWindow(): boolean {
  return getCurrentWebviewWindow().label === "main";
}
