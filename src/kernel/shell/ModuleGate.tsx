import { Navigate } from "@tanstack/react-router";
import type { ReactNode } from "react";
import { useModules, type Modules } from "../bridge/modules";
import { useCurrentWorkspace } from "../bridge/workspaces";

/** A module's page: Meetings instead when the workspace has that module off (D-059). */
export default function ModuleGate({ module, children }: { module: keyof Modules; children: ReactNode }) {
  const { data: workspace } = useCurrentWorkspace();
  const modules = useModules(workspace?.slug);
  if (workspace && modules.isPending) return null; // one read on an open pool: no flash of a hidden page
  if (modules.data && !modules.data[module]) return <Navigate to="/meetings" search={{}} replace />;
  return <>{children}</>;
}
