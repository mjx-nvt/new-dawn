import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type Attachment, type Layout, type LayoutInput, type Neighbours, type Tab, type TabInput, type TimelineEvent, type Widget, type WidgetInput } from "../../bindings";
import { asRecord, type RecordJson } from "./records";
import { unwrap } from "./unwrap";

export type { Attachment, Layout, LayoutInput, Neighbours, Tab, TabInput, TimelineEvent, Widget, WidgetInput };

/** An object's record page layout — tabs of widgets on a 12-column grid — Twenty's default
 *  until the layout editor saves one (D-062). */
export function useRecordLayout(slug: string | undefined, object: string | undefined) {
  return useQuery({ queryKey: ["layout", slug ?? "", object ?? ""], queryFn: () => unwrap(commands.getRecordLayout(slug ?? "", object ?? "")), enabled: !!slug && !!object });
}

export function useSaveRecordLayout(slug: string | undefined, object: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: LayoutInput) => unwrap(commands.saveRecordLayout(slug ?? "", object ?? "", input)),
    onSuccess: (layout) => queryClient.setQueryData(["layout", slug ?? "", object ?? ""], layout),
  });
}

export function useResetRecordLayout(slug: string | undefined, object: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => unwrap(commands.resetRecordLayout(slug ?? "", object ?? "")),
    onSuccess: (layout) => queryClient.setQueryData(["layout", slug ?? "", object ?? ""], layout),
  });
}

/** A record's timeline, newest first: made, each change with its diff, deleted, restored. */
export function useTimeline(slug: string | undefined, object: string | undefined, id: string | undefined) {
  return useQuery({
    queryKey: ["timeline", slug ?? "", object ?? "", id ?? ""],
    queryFn: () => unwrap(commands.listTimeline(slug ?? "", object ?? "", id ?? "", 100)),
    enabled: !!slug && !!object && !!id,
  });
}

/** The records before and after this one in the list's order, for ‹ › on the page. */
export function useNeighbours(slug: string | undefined, object: string | undefined, id: string | undefined) {
  return useQuery({
    queryKey: ["neighbours", slug ?? "", object ?? "", id ?? ""],
    queryFn: () => unwrap(commands.recordNeighbours(slug ?? "", object ?? "", id ?? "")),
    enabled: !!slug && !!object && !!id,
  });
}

export type LinkedKind = "task" | "note";

/** The tasks or notes that target a record, newest first. */
export function useLinked(slug: string | undefined, object: string | undefined, id: string | undefined, kind: LinkedKind) {
  return useQuery({
    queryKey: ["linked", slug ?? "", object ?? "", id ?? "", kind],
    queryFn: async (): Promise<RecordJson[]> => (await unwrap(commands.listLinked(slug ?? "", object ?? "", id ?? "", kind))).map(asRecord),
    enabled: !!slug && !!object && !!id,
  });
}

/** A new task or note with this title, targeting the record. */
export function useAddLinked(slug: string | undefined, object: string | undefined, id: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ kind, title }: { kind: LinkedKind; title: string }) => unwrap(commands.addLinked(slug ?? "", object ?? "", id ?? "", kind, title)),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["linked", slug ?? "", object ?? "", id ?? ""] });
      void queryClient.invalidateQueries({ queryKey: ["records", slug ?? ""] });
      for (const key of ["tasks", "open-task-count", "today"]) void queryClient.invalidateQueries({ queryKey: [key] });
    },
  });
}

/** A record's files, newest first (uploads come with the notes release). */
export function useAttachments(slug: string | undefined, object: string | undefined, id: string | undefined) {
  return useQuery({
    queryKey: ["attachments", slug ?? "", object ?? "", id ?? ""],
    queryFn: () => unwrap(commands.listAttachments(slug ?? "", object ?? "", id ?? "")),
    enabled: !!slug && !!object && !!id,
  });
}
