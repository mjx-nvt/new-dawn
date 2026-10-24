import { useEffect } from "react";
import { useSetCurrentWorkspace, useWorkspaces } from "../bridge/workspaces";

/** ⌘1…⌘9 switch to the nth active workspace (same order as the switcher list). */
export function useWorkspaceShortcuts() {
  const { data: workspaces } = useWorkspaces();
  const { mutate: choose } = useSetCurrentWorkspace();

  useEffect(() => {
    const active = (workspaces ?? []).filter((w) => !w.archived);
    function onKeyDown(event: KeyboardEvent) {
      if (!event.metaKey || event.shiftKey || event.altKey || event.ctrlKey) return;
      const target = active[Number(event.key) - 1];
      if (!target) return;
      event.preventDefault();
      choose(target.slug);
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [workspaces, choose]);
}
