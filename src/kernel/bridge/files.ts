import { useMutation, useQueryClient } from "@tanstack/react-query";
import { open, save } from "@tauri-apps/plugin-dialog";
import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
import { commands } from "../../bindings";
import { unwrap } from "./unwrap";

/** A record's files (0.10.6): copied into the workspace's files folder, kept as attachment
 *  records, so every object has them. */
function useRefresh(slug: string | undefined, object: string | undefined, id: string | undefined) {
  const queryClient = useQueryClient();
  return () => {
    void queryClient.invalidateQueries({ queryKey: ["attachments", slug ?? "", object ?? "", id ?? ""] });
    void queryClient.invalidateQueries({ queryKey: ["timeline", slug ?? ""] });
  };
}

/** Adds files picked in the dialog or dropped on the widget. */
export function useAddAttachments(slug: string | undefined, object: string | undefined, id: string | undefined) {
  const refresh = useRefresh(slug, object, id);
  return useMutation({
    mutationFn: async (paths: string[]) => {
      for (const path of paths) await unwrap(commands.addAttachment(slug ?? "", object ?? "", id ?? "", path));
    },
    onSuccess: refresh,
  });
}

/** A picture that only exists in memory (pasted or dropped into the editor). */
export function useAddAttachmentBytes(slug: string | undefined, object: string | undefined, id: string | undefined) {
  const refresh = useRefresh(slug, object, id);
  return useMutation({
    mutationFn: ({ name, bytes }: { name: string; bytes: Uint8Array }) => unwrap(commands.addAttachmentBytes(slug ?? "", object ?? "", id ?? "", name, Array.from(bytes))),
    onSuccess: refresh,
  });
}

/** What the editor needs of a new attachment: its file (for `convertFileSrc`) and its name. */
export interface Attached {
  path: string;
  name: string;
}

function asAttached(value: unknown): Attached | null {
  if (!value || typeof value !== "object") return null;
  const r = value as { fullPath?: unknown; name?: unknown };
  return typeof r.fullPath === "string" && r.fullPath ? { path: r.fullPath, name: typeof r.name === "string" ? r.name : "" } : null;
}

/** One file, attached to the record (the editor's pictures). */
export async function addAttachment(slug: string, object: string, id: string, path: string): Promise<Attached | null> {
  return asAttached(await unwrap(commands.addAttachment(slug, object, id, path)));
}

export async function addAttachmentBytes(slug: string, object: string, id: string, name: string, bytes: Uint8Array): Promise<Attached | null> {
  return asAttached(await unwrap(commands.addAttachmentBytes(slug, object, id, name, Array.from(bytes))));
}

export function useDeleteAttachment(slug: string | undefined, object: string | undefined, id: string | undefined) {
  const refresh = useRefresh(slug, object, id);
  return useMutation({ mutationFn: (attachmentId: string) => unwrap(commands.deleteAttachment(slug ?? "", attachmentId)), onSuccess: refresh });
}

/** The file dialog, several files at once; null when cancelled. */
export async function pickFiles(images = false): Promise<string[]> {
  const picked = await open({ multiple: true, directory: false, ...(images ? { filters: [{ name: "Pictures", extensions: ["png", "jpg", "jpeg", "gif", "webp", "heic"] }] } : {}) });
  return Array.isArray(picked) ? picked : typeof picked === "string" ? [picked] : [];
}

/** Save as…: the save dialog, then a copy of the file. */
export async function saveAttachmentAs(slug: string, attachmentId: string, name: string): Promise<boolean> {
  const to = await save({ defaultPath: name });
  if (!to) return false;
  await unwrap(commands.saveAttachment(slug, attachmentId, to));
  return true;
}

export async function openAttachment(slug: string, attachmentId: string): Promise<void> {
  await openPath(await unwrap(commands.attachmentPath(slug, attachmentId)));
}

export async function revealAttachment(slug: string, attachmentId: string): Promise<void> {
  await revealItemInDir(await unwrap(commands.attachmentPath(slug, attachmentId)));
}

const IMAGE_TYPES = ["png", "jpg", "jpeg", "gif", "webp", "heic", "bmp", "svg"];

export function isImage(type: string): boolean {
  return IMAGE_TYPES.includes(type.toLowerCase());
}
