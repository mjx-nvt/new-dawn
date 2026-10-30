import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type NewProject, type Project, type ProjectDossier, type ProjectPatch, type ProjectStatus, type ProjectUpdate, type UpdateField } from "../../bindings";
import { unwrap } from "./unwrap";

export type { NewProject, Project, ProjectDossier, ProjectPatch, ProjectStatus, ProjectUpdate, UpdateField };

const listKey = (slug: string) => ["projects", slug] as const;
const projectKey = (slug: string, id: string) => ["project", slug, id] as const;

/** The workspace's projects (D-054): active first, then on hold, then done. */
export function useProjects(slug: string | undefined) {
  // Proposals arrive in the background after a summary: look again every minute.
  return useQuery({ queryKey: listKey(slug ?? ""), queryFn: () => unwrap(commands.listProjects(slug ?? "")), enabled: !!slug, refetchInterval: 60_000 });
}

/** A project's page: the project, the updates waiting for you, its history and meetings. */
export function useProject(slug: string | undefined, id: string) {
  return useQuery({ queryKey: projectKey(slug ?? "", id), queryFn: () => unwrap(commands.getProject(slug ?? "", id)), enabled: !!slug });
}

function useRefresh(slug: string | undefined) {
  const queryClient = useQueryClient();
  return () => {
    void queryClient.invalidateQueries({ queryKey: ["projects", slug ?? ""] });
    void queryClient.invalidateQueries({ queryKey: ["project", slug ?? ""] });
  };
}

export function useCreateProject(slug: string | undefined) {
  const refresh = useRefresh(slug);
  return useMutation({ mutationFn: (project: NewProject) => unwrap(commands.createProject(slug ?? "", project)), onSuccess: refresh });
}

export function useUpdateProject(slug: string | undefined) {
  const refresh = useRefresh(slug);
  return useMutation({ mutationFn: (v: { id: string; patch: ProjectPatch }) => unwrap(commands.updateProject(slug ?? "", v.id, v.patch)), onSuccess: refresh });
}

export function useDeleteProject(slug: string | undefined) {
  const refresh = useRefresh(slug);
  return useMutation({ mutationFn: (id: string) => unwrap(commands.deleteProject(slug ?? "", id)), onSuccess: refresh });
}

/** Apply puts the proposed change into the project; Dismiss sets it aside. */
export function useDecideUpdate(slug: string | undefined) {
  const refresh = useRefresh(slug);
  return useMutation({
    mutationFn: (v: { id: string; apply: boolean }) => unwrap(v.apply ? commands.applyProjectUpdate(slug ?? "", v.id) : commands.dismissProjectUpdate(slug ?? "", v.id)),
    onSettled: refresh,
  });
}

/** "Find project updates" for a meeting, now; resolves with how many it proposed. */
export function useFindProjectUpdates(slug: string | undefined) {
  const refresh = useRefresh(slug);
  return useMutation({ mutationFn: (meetingId: string) => unwrap(commands.findProjectUpdates(slug ?? "", meetingId)), onSuccess: refresh });
}
