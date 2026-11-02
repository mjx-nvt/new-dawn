import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { save } from "@tauri-apps/plugin-dialog";
import { commands, type DayCount, type ExportFormat, type ImportProblem, type PmExportReport, type PmImportPreview, type PmImportSummary, type PmOverview, type PmRule } from "../../bindings";
import { devPath } from "./transfer";
import { unwrap } from "./unwrap";

/** Plane's analytics, exports and CSV importer (D-083). */
export type { DayCount, ExportFormat, ImportProblem, PmExportReport, PmImportPreview, PmImportSummary, PmOverview };

export function usePmOverview(slug: string | undefined, projectId: string | null) {
  return useQuery({ queryKey: ["pm-items", slug ?? "", "overview", projectId ?? ""], queryFn: () => unwrap(commands.pmOverview(slug ?? "", projectId)), enabled: !!slug });
}

export function usePmCreatedVsResolved(slug: string | undefined, projectId: string | null, from: string, to: string) {
  return useQuery({ queryKey: ["pm-items", slug ?? "", "created-resolved", projectId ?? "", from, to], queryFn: () => unwrap(commands.pmCreatedVsResolved(slug ?? "", projectId, from, to)), enabled: !!slug });
}

export function usePmInsights(slug: string | undefined, projectId: string | null, axis: string) {
  return useQuery({ queryKey: ["pm-items", slug ?? "", "insights", projectId ?? "", axis], queryFn: () => unwrap(commands.pmInsights(slug ?? "", projectId, axis)), enabled: !!slug });
}

/** The save dialog, then the work items the rules keep as CSV, XLSX or JSON; null when
 *  cancelled. */
export async function exportPmItems(slug: string, projectId: string | null, rules: PmRule[], format: ExportFormat, label: string): Promise<PmExportReport | null> {
  const name = `${label.replace(/[/:]/g, "-") || "Work items"}.${format}`;
  const path = devPath("__newdawnDevSave") ?? (await save({ defaultPath: name, filters: [{ name: format.toUpperCase(), extensions: [format] }] }));
  if (!path) return null;
  return unwrap(commands.exportPmItems(slug, projectId, rules, format, path, label));
}

export function previewPmImport(slug: string, projectId: string, path: string): Promise<PmImportPreview> {
  return unwrap(commands.previewPmImport(slug, projectId, path));
}

export function useImportPmItems(slug: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ projectId, path }: { projectId: string; path: string }) => unwrap(commands.importPmItems(slug ?? "", projectId, path)),
    // What new work items touch (as a work item made), the states and labels made, the history.
    onSuccess: () =>
      Promise.all(
        ["pm-items", "pm-item", "pm-sub-items", "pm-activity", "pm-comments", "pm-relations", "pm-links", "notices", "notice-counts", "pm-cycles", "pm-modules", "pm-states", "pm-labels", "transfer-jobs"].map((key) =>
          queryClient.invalidateQueries({ queryKey: [key] }),
        ),
      ),
  });
}
