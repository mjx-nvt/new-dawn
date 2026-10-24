import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type WorkspacePatch_Deserialize, type Workspace_Serialize } from "../../bindings";
import { unwrap } from "./unwrap";

export type Workspace = Workspace_Serialize;
export type WorkspacePatch = WorkspacePatch_Deserialize;

const WORKSPACES_KEY = ["workspaces"] as const;
const CURRENT_WORKSPACE_KEY = ["current-workspace"] as const;

export function useWorkspaces() {
  return useQuery({ queryKey: WORKSPACES_KEY, queryFn: () => unwrap(commands.listWorkspaces()) });
}

/** The workspace named by `setting.current_workspace`, else the first active one, else null. */
export function useCurrentWorkspace() {
  return useQuery({ queryKey: CURRENT_WORKSPACE_KEY, queryFn: () => unwrap(commands.getCurrentWorkspace()) });
}

/** Every workspace mutation refreshes the list and the current workspace. */
function useWorkspaceMutation<Args>(run: (args: Args) => Promise<unknown>) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: run,
    onSettled: () =>
      Promise.all([
        queryClient.invalidateQueries({ queryKey: WORKSPACES_KEY }),
        queryClient.invalidateQueries({ queryKey: CURRENT_WORKSPACE_KEY }),
      ]),
  });
}

export function useCreateWorkspace() {
  return useWorkspaceMutation(({ name, slug }: { name: string; slug: string }) =>
    unwrap(commands.createWorkspace(name, slug)),
  );
}

export function useUpdateWorkspace() {
  return useWorkspaceMutation(({ slug, patch }: { slug: string; patch: WorkspacePatch }) =>
    unwrap(commands.updateWorkspace(slug, patch)),
  );
}

export function useDeleteWorkspace() {
  return useWorkspaceMutation((slug: string) => unwrap(commands.deleteWorkspace(slug)));
}

export function useSetCurrentWorkspace() {
  return useWorkspaceMutation((slug: string) => unwrap(commands.setCurrentWorkspace(slug)));
}
