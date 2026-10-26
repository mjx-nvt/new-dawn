import { useMutation, useQuery, useQueryClient, type QueryClient } from "@tanstack/react-query";
import { commands, type TaskPatch_Deserialize, type TaskView_Serialize } from "../../bindings";
import { unwrap } from "./unwrap";

export type TaskView = TaskView_Serialize;
export type TaskPatch = TaskPatch_Deserialize;

const keys = {
  tasks: (slug: string) => ["tasks", slug] as const,
  count: (slug: string) => ["task-count", slug] as const,
};

// "Today" turns over at midnight while the app stays open: the list and the badge follow within a minute.
const EVERY_MINUTE = 60_000;
/** How long a ticked task stays in place, struck through, before it moves to Completed. */
const SETTLE_MS = 1500;

/** The Tasks page and the sidebar badge, after anything that changes a workspace's tasks (a
 *  summary's tasks pass, a tick in a summary, a change here). */
export function invalidateTasks(queryClient: QueryClient, slug: string) {
  void queryClient.invalidateQueries({ queryKey: keys.tasks(slug) });
  void queryClient.invalidateQueries({ queryKey: keys.count(slug) });
}

export function useTasks(slug: string | undefined) {
  return useQuery({
    queryKey: keys.tasks(slug ?? ""),
    queryFn: () => unwrap(commands.listTasks(slug ?? "")),
    enabled: !!slug,
    refetchInterval: EVERY_MINUTE,
    // A new list each time, so the page redraws and "Today" / "Yesterday" follow the clock even
    // when no task changed section.
    structuralSharing: false,
  });
}

/** Open tasks due today or overdue: the sidebar badge. */
export function useOpenTaskCount(slug: string | undefined) {
  return useQuery({
    queryKey: keys.count(slug ?? ""),
    queryFn: () => unwrap(commands.openTaskCount(slug ?? "")),
    enabled: !!slug,
    refetchInterval: EVERY_MINUTE,
  });
}

/** A typed task (no date). */
export function useAddTask(slug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (title: string) => unwrap(commands.addTask(slug, title)),
    onSettled: () => invalidateTasks(queryClient, slug),
  });
}

/** Renames, dates (`due_on: null` removes the date) or ticks a task; a tick follows to its
 *  meeting's action item. A ticked task moves to Completed a moment later, so the row can show
 *  it done first (and be unticked in place). */
export function useUpdateTask(slug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, patch }: { id: string; patch: TaskPatch }) => unwrap(commands.updateTask(slug, id, patch)),
    onSettled: (_task, _error, { patch }) => {
      void queryClient.invalidateQueries({ queryKey: keys.count(slug) });
      const refresh = () => void queryClient.invalidateQueries({ queryKey: keys.tasks(slug) });
      if (patch.done) setTimeout(refresh, SETTLE_MS);
      else refresh();
    },
  });
}

/** Deletes a task; "Add to Tasks" on its action item brings it back. */
export function useDeleteTask(slug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => unwrap(commands.deleteTask(slug, id)),
    onSettled: () => invalidateTasks(queryClient, slug),
  });
}

/** "Add to Tasks" on action item `index` of a meeting's summary; the summary comes back linked. */
export function useAddActionItemToTasks(slug: string, meetingId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (index: number) => unwrap(commands.addActionItemToTasks(slug, meetingId, index)),
    onSuccess: (view) => queryClient.setQueryData(["summary", slug, meetingId], view),
    onSettled: () => invalidateTasks(queryClient, slug),
  });
}
