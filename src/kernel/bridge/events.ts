import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef } from "react";
import { events, type MenuAction, type SettingsTab } from "../../bindings";

/** Refreshes workspace data (and your profile, edited in Settings) when any window changes it —
 *  with the previews that show your name or the workspace's (every workspace's).
 *  Mount once per window. */
export function useWorkspacesSync() {
  const queryClient = useQueryClient();
  useEffect(() => {
    const unlisten = events.workspacesChanged.listen(() => {
      void queryClient.invalidateQueries({ queryKey: ["workspaces"] });
      void queryClient.invalidateQueries({ queryKey: ["current-workspace"] });
      void queryClient.invalidateQueries({ queryKey: ["profile"] });
      void queryClient.invalidateQueries({ queryKey: ["template-preview"] });
      void queryClient.invalidateQueries({ queryKey: ["whisper-prompt"] });
    });
    return () => void unlisten.then((stop) => stop());
  }, [queryClient]);
}

/** Native menu items the UI has to act on. */
export function useMenuAction(handler: (action: MenuAction) => void) {
  const latest = useRef(handler);
  latest.current = handler;
  useEffect(() => {
    const unlisten = events.menuAction.listen((event) => latest.current(event.payload));
    return () => void unlisten.then((stop) => stop());
  }, []);
}

/** The Settings window being asked to show a tab while it is already open. */
export function useOpenSettingsTab(handler: (tab: SettingsTab) => void) {
  const latest = useRef(handler);
  latest.current = handler;
  useEffect(() => {
    const unlisten = events.openSettingsTab.listen((event) => latest.current(event.payload.tab));
    return () => void unlisten.then((stop) => stop());
  }, []);
}
