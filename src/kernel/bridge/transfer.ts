import { useQuery } from "@tanstack/react-query";
import { open, save } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { commands, type ColumnMap, type ExportReport, type FieldChoice, type ImportFile, type ImportReport, type Preview, type Query, type Table, type TransferJob } from "../../bindings";
import { unwrap } from "./unwrap";

export type { ColumnMap, ExportReport, FieldChoice, ImportFile, ImportReport, Preview, Table, TransferJob };

/** Import and export (D-065, 0.10.8): Twenty's spreadsheet import and a view's CSV export. */

/** A dev check can't drive the native dialogs: in development, a path set on the window stands
 *  in for the file picked or the place to save. */
export function devPath(key: "__newdawnDevFile" | "__newdawnDevSave"): string | undefined {
  return import.meta.env.DEV ? (window as unknown as Record<string, string | undefined>)[key] : undefined;
}

/** The file dialog for a spreadsheet; null when cancelled. */
export async function pickSpreadsheet(): Promise<string | null> {
  const preset = devPath("__newdawnDevFile");
  if (preset) return preset;
  const picked = await open({ multiple: false, directory: false, filters: [{ name: "Spreadsheet", extensions: ["csv", "xlsx", "xls", "xlsm", "ods", "txt"] }] });
  return typeof picked === "string" ? picked : null;
}

export function readImportFile(slug: string, object: string, path: string): Promise<ImportFile> {
  return unwrap(commands.readImportFile(slug, object, path));
}

export function validateImport(slug: string, object: string, table: Table, mapping: ColumnMap[], identifier: string | null): Promise<Preview> {
  return unwrap(commands.validateImport(slug, object, table, mapping, identifier));
}

export function runImport(slug: string, object: string, fileName: string, table: Table, mapping: ColumnMap[], identifier: string | null): Promise<ImportReport> {
  return unwrap(commands.runImport(slug, object, fileName, table, mapping, identifier));
}

/** The save dialog, then the view's records as CSV; null when cancelled. */
export async function exportViewToFile(slug: string, object: string, viewName: string, query: Query, fields: string[], suggestedName: string): Promise<ExportReport | null> {
  const path = devPath("__newdawnDevSave") ?? (await save({ defaultPath: suggestedName, filters: [{ name: "CSV", extensions: ["csv"] }] }));
  if (!path) return null;
  return unwrap(commands.exportView(slug, object, viewName, query, fields, path));
}

export function revealFile(path: string): Promise<void> {
  return revealItemInDir(path);
}

export function useTransferJobs(slug: string | undefined) {
  return useQuery({ queryKey: ["transfer-jobs", slug ?? ""], queryFn: () => unwrap(commands.listTransferJobs(slug ?? "")), enabled: !!slug });
}
