import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  commands,
  type AggregateInput,
  type AggregateValue,
  type FiltersInput,
  type GroupCount,
  type GroupValueInput,
  type SaveInput,
  type SortInput,
  type ViewDef,
  type ViewFieldInput,
  type ViewPatch,
  type ViewRow,
} from "../../bindings";
import type { Query } from "./records";
import { unwrap } from "./unwrap";

export type { AggregateInput, AggregateValue, FiltersInput, GroupCount, GroupValueInput, SaveInput, SortInput, ViewDef, ViewFieldInput, ViewPatch, ViewRow };

/** An object's views, the default "All <X>" first (D-061). */
export function useViews(slug: string | undefined, object: string | undefined) {
  return useQuery({ queryKey: ["views", slug ?? "", object ?? ""], queryFn: () => unwrap(commands.listViews(slug ?? "", object ?? "")), enabled: !!slug && !!object });
}

export function useView(slug: string | undefined, id: string | undefined) {
  return useQuery({ queryKey: ["view", slug ?? "", id ?? ""], queryFn: () => unwrap(commands.getView(slug ?? "", id ?? "")), enabled: !!slug && !!id });
}

function useRefresh(slug: string | undefined, object: string | undefined) {
  const queryClient = useQueryClient();
  return (view?: ViewDef) => {
    if (view) queryClient.setQueryData(["view", slug ?? "", view.view.id], view);
    void queryClient.invalidateQueries({ queryKey: ["views", slug ?? "", object ?? ""] });
    void queryClient.invalidateQueries({ queryKey: ["view", slug ?? ""] });
  };
}

export function useCreateView(slug: string | undefined, object: string | undefined) {
  const refresh = useRefresh(slug, object);
  return useMutation({
    mutationFn: ({ name, icon = "", type = "TABLE" }: { name: string; icon?: string; type?: string }) => unwrap(commands.createView(slug ?? "", object ?? "", name, icon, type)),
    onSuccess: refresh,
  });
}

export function useUpdateView(slug: string | undefined, object: string | undefined) {
  const refresh = useRefresh(slug, object);
  return useMutation({ mutationFn: ({ id, patch }: { id: string; patch: ViewPatch }) => unwrap(commands.updateView(slug ?? "", id, patch)), onSuccess: refresh });
}

export function useDeleteView(slug: string | undefined, object: string | undefined) {
  const refresh = useRefresh(slug, object);
  return useMutation({ mutationFn: (id: string) => unwrap(commands.deleteView(slug ?? "", id)), onSuccess: () => refresh() });
}

export function useDuplicateView(slug: string | undefined, object: string | undefined) {
  const refresh = useRefresh(slug, object);
  return useMutation({ mutationFn: ({ id, name }: { id: string; name?: string }) => unwrap(commands.duplicateView(slug ?? "", id, name ?? null)), onSuccess: refresh });
}

export function useReorderViews(slug: string | undefined, object: string | undefined) {
  const refresh = useRefresh(slug, object);
  return useMutation({ mutationFn: (ids: string[]) => unwrap(commands.reorderViews(slug ?? "", object ?? "", ids)), onSuccess: () => refresh() });
}

/** Everything "Update view" writes, in one transaction (D-061). */
export function useSaveView(slug: string | undefined, object: string | undefined) {
  const refresh = useRefresh(slug, object);
  return useMutation({ mutationFn: ({ id, input }: { id: string; input: SaveInput }) => unwrap(commands.saveView(slug ?? "", id, input)), onSuccess: refresh });
}

export function useSetViewFields(slug: string | undefined, object: string | undefined) {
  const refresh = useRefresh(slug, object);
  return useMutation({ mutationFn: ({ id, fields }: { id: string; fields: ViewFieldInput[] }) => unwrap(commands.setViewFields(slug ?? "", id, fields)), onSuccess: refresh });
}

export function useSetViewFilters(slug: string | undefined, object: string | undefined) {
  const refresh = useRefresh(slug, object);
  return useMutation({ mutationFn: ({ id, input }: { id: string; input: FiltersInput }) => unwrap(commands.setViewFilters(slug ?? "", id, input)), onSuccess: refresh });
}

export function useSetViewSorts(slug: string | undefined, object: string | undefined) {
  const refresh = useRefresh(slug, object);
  return useMutation({ mutationFn: ({ id, sorts }: { id: string; sorts: SortInput[] }) => unwrap(commands.setViewSorts(slug ?? "", id, sorts)), onSuccess: refresh });
}

export function useSetViewGroups(slug: string | undefined, object: string | undefined) {
  const refresh = useRefresh(slug, object);
  return useMutation({ mutationFn: ({ id, groups }: { id: string; groups: GroupValueInput[] }) => unwrap(commands.setViewGroups(slug ?? "", id, groups)), onSuccess: refresh });
}

/** The footer's (or a group's) aggregates over what the query lists. */
export function useAggregates(slug: string | undefined, object: string | undefined, query: Query, ops: AggregateInput[]) {
  return useQuery({
    queryKey: ["aggregates", slug ?? "", object ?? "", query, ops],
    queryFn: () => unwrap(commands.aggregateRecords(slug ?? "", object ?? "", query, ops)),
    enabled: !!slug && !!object && ops.length > 0,
    placeholderData: keepPreviousData,
  });
}

/** How many records the query lists per value of a select field. */
export function useGroupCounts(slug: string | undefined, object: string | undefined, query: Query, field: string | undefined) {
  return useQuery({
    queryKey: ["group-counts", slug ?? "", object ?? "", query, field ?? ""],
    queryFn: () => unwrap(commands.groupCounts(slug ?? "", object ?? "", query, field ?? "")),
    enabled: !!slug && !!object && !!field,
    placeholderData: keepPreviousData,
  });
}
