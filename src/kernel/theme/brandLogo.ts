import { convertFileSrc } from "@tauri-apps/api/core";
import { useBrand } from "../bridge/brand";
import { useMediaQuery } from "../shell/useMediaQuery";

/** The one piece of a workspace's brand the UI shows: its logo, as the workspace badge. The
 *  pack's colours and fonts are for exported documents; the app keeps its own palette (D-033). */

/** Cache-busting version for the logos: changes whenever a logo file is replaced. */
export function logoUrl(path: string, version: string): string {
  return `${convertFileSrc(path)}?v=${encodeURIComponent(version)}`;
}

/** The logo to show on the current background, as a URL the webview can load. */
export function useBrandLogo(slug: string | undefined): string | null {
  const { data } = useBrand(slug);
  const dark = useMediaQuery("(prefers-color-scheme: dark)");
  const path = (dark && data?.logo_on_dark) || data?.logo_primary;
  return path && data ? logoUrl(path, data.logo_version) : null;
}
