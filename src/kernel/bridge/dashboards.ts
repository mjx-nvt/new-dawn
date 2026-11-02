import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type Axis, type ChartConfig, type ChartData, type LayoutInput, type Measure } from "../../bindings";
import { unwrap } from "./unwrap";

export type { Axis, ChartConfig, ChartData, Measure };

/** Dashboards (D-067, 0.10.10): a dashboard's own layout, its copy, and the chart query. */

export function useDashboardLayout(slug: string | undefined, id: string | undefined) {
  return useQuery({ queryKey: ["dashboard-layout", slug ?? "", id ?? ""], queryFn: () => unwrap(commands.dashboardLayout(slug ?? "", id ?? "")), enabled: !!slug && !!id });
}

export function useSaveDashboardLayout(slug: string | undefined, id: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: LayoutInput) => unwrap(commands.saveDashboardLayout(slug ?? "", id ?? "", input)),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["dashboard-layout", slug ?? "", id ?? ""] }),
  });
}

export function useDuplicateDashboard(slug: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => unwrap(commands.duplicateDashboard(slug ?? "", id)),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["records", slug ?? "", "dashboard"] }),
  });
}

/** A chart's data. The key starts like the records' queries, so every change the UI already
 *  announces for the object refreshes the chart too. */
export function useChartData(slug: string | undefined, config: ChartConfig | null) {
  return useQuery({
    queryKey: ["records", slug ?? "", config?.source ?? "", "chart", config],
    queryFn: () => unwrap(commands.chartData(slug ?? "", config as ChartConfig)),
    enabled: !!slug && !!config,
    placeholderData: (previous) => previous,
  });
}
