import { convertFileSrc } from "@tauri-apps/api/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type ConverterInfo, type MinutesExport, type MinutesFile } from "../../bindings";
import { unwrap } from "./unwrap";

export type { ConverterInfo, MinutesExport, MinutesFile };

const keys = {
  exports: (slug: string, id: string) => ["minutes-exports", slug, id] as const,
  converters: ["converters"] as const,
};

export function useMinutesExports(slug: string, meetingId: string) {
  return useQuery({
    queryKey: keys.exports(slug, meetingId),
    queryFn: () => unwrap(commands.listMinutesExports(slug, meetingId)),
  });
}

/** Fills the template (DOCX); makes the PDF too only when asked. */
export function useExportMinutes(slug: string, meetingId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ fileName, pdf }: { fileName: string; pdf: boolean }) => {
      const options = { include_transcript: false, file_name: fileName.trim() || null };
      const exported = await unwrap(commands.exportMinutesDocx(slug, meetingId, options));
      void queryClient.invalidateQueries({ queryKey: keys.exports(slug, meetingId) }); // the DOCX shows even if the PDF fails
      return pdf ? unwrap(commands.exportMinutesPdf(slug, exported.id)) : exported;
    },
    onSettled: () => queryClient.invalidateQueries({ queryKey: keys.exports(slug, meetingId) }),
  });
}

/** PDF of an existing export's DOCX (includes edits made in Word since). */
export function useMakePdf(slug: string, meetingId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (exportId: string) => unwrap(commands.exportMinutesPdf(slug, exportId)),
    onSettled: () => queryClient.invalidateQueries({ queryKey: keys.exports(slug, meetingId) }),
  });
}

/** Moves the export's files to the Trash and drops it from the list. */
export function useDeleteExport(slug: string, meetingId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (exportId: string) => unwrap(commands.deleteMinutesExport(slug, exportId)),
    onSettled: () => queryClient.invalidateQueries({ queryKey: keys.exports(slug, meetingId) }),
  });
}

export function openMinutesFile(slug: string, exportId: string, file: MinutesFile, reveal: boolean): Promise<null> {
  return unwrap(commands.openMinutesFile(slug, exportId, file, reveal));
}

export function useConverters() {
  return useQuery({ queryKey: keys.converters, queryFn: () => unwrap(commands.listConverters()) });
}

/** A PDF in the workspace's exports folder as a URL the webview can show; `version` busts the cache. */
export function pdfUrl(path: string, version: string): string {
  return `${convertFileSrc(path)}?v=${encodeURIComponent(version)}`;
}
