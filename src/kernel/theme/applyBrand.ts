import { convertFileSrc } from "@tauri-apps/api/core";
import type { CSSProperties } from "react";
import type { BrandView } from "../bridge/brand";
import { useBrand } from "../bridge/brand";
import { useMediaQuery } from "../shell/useMediaQuery";

const STEPS = ["50", "100", "200", "300", "400", "500", "600", "700", "800", "900", "950"];
const SHAPE: Record<string, string> = { square: "6px", rounded: "var(--radius-control)", pill: "999px" };

/** The brand as CSS variables for one element (the content panel). In dark mode the pack's
 *  light-only values (canvas, text, status) stay with the system's dark tokens; the brand
 *  ramp is used inverted by the stylesheet. */
export function brandVars(brand: BrandView, dark: boolean): CSSProperties {
  const vars: Record<string, string> = {
    "--brand": brand.primary,
    "--secondary": brand.secondary ?? brand.ramp[7],
    "--accent": brand.accent ?? brand.primary,
    "--radius-card": `${brand.radius.card}px`,
    "--radius-chip": `${brand.radius.chip}px`,
    "--radius-control": `${brand.radius.control}px`,
    "--radius-modal": `${brand.radius.modal}px`,
    "--button-shape": SHAPE[brand.button.shape] ?? SHAPE.rounded,
    "--button-primary": brand.button.primary,
    "--button-secondary": brand.button.secondary,
    "--logo-url": brand.logo_primary ? `url("${logoUrl(brand.logo_primary, brand.logo_version)}")` : "none",
    "--logo-dark-url": brand.logo_on_dark ? `url("${logoUrl(brand.logo_on_dark, brand.logo_version)}")` : "none",
  };
  STEPS.forEach((step, i) => (vars[`--brand-${step}`] = brand.ramp[i]));
  if (!dark) {
    vars["--canvas"] = brand.canvas;
    vars["--status-positive"] = brand.status.positive;
    vars["--status-warning"] = brand.status.warning;
    vars["--status-negative"] = brand.status.negative;
  }
  return vars as CSSProperties;
}

/** What a `.content-root` gets: the brand as CSS variables, and the button styles as data
 *  attributes (`components.css` reads `[data-btn-primary]` / `[data-btn-secondary]`). */
export function brandProps(brand: BrandView, dark: boolean) {
  return {
    style: brandVars(brand, dark),
    "data-btn-primary": brand.button.primary,
    "data-btn-secondary": brand.button.secondary,
  };
}

export function useBrandProps(slug: string | undefined) {
  const { data } = useBrand(slug);
  const dark = useMediaQuery("(prefers-color-scheme: dark)");
  return data ? brandProps(data, dark) : {};
}

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
