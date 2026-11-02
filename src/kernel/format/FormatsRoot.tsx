import type { ReactNode } from "react";
import { useFormats } from "../bridge/formats";
import { DEFAULT_FORMATS, setFormats } from "./format";

/** Reads the owner's formats before the window draws and keeps the formatters on them
 *  (Settings → General → Formats, D-072). The main window's page draws again when they change
 *  (AppLayout keys its outlet); the shell, its dialogs and its once-a-window hooks stay. */
export default function FormatsRoot({ children }: { children: ReactNode }) {
  const formats = useFormats();
  if (formats.isPending) return null;
  setFormats(formats.data ?? DEFAULT_FORMATS);
  return children;
}
