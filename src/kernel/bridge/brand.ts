import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import { useEffect } from "react";
import { commands, events, type BrandPatch, type BrandView_Serialize } from "../../bindings";
import { unwrap } from "./unwrap";

export type BrandView = BrandView_Serialize;
export type { BrandPatch };

const key = (slug: string) => ["brand", slug] as const;

/** The workspace's resolved brand pack (the starter is copied in on first read). */
export function useBrand(slug: string | undefined) {
  return useQuery({
    queryKey: key(slug ?? ""),
    queryFn: () => unwrap(commands.getBrand(slug ?? "")),
    enabled: !!slug,
    staleTime: Infinity, // changes arrive as BrandChanged events
  });
}

/** Refreshes the cached brand pack (badge logo, Brand pane) when any window changes it. Mount once per window. */
export function useBrandSync() {
  const queryClient = useQueryClient();
  useEffect(() => {
    const stop = events.brandChanged.listen(({ payload }) => void queryClient.invalidateQueries({ queryKey: key(payload.slug) }));
    return () => void stop.then((s) => s());
  }, [queryClient]);
}

function useBrandMutation<V>(slug: string, call: (vars: V) => Promise<BrandView>) {
  const queryClient = useQueryClient();
  return useMutation({ mutationFn: call, onSuccess: (view) => queryClient.setQueryData(key(slug), view) });
}

export function useUpdateBrand(slug: string) {
  return useBrandMutation(slug, (patch: BrandPatch) => unwrap(commands.updateBrand(slug, patch)));
}

/** Picks an SVG/PNG (null path = clear). `onDark`: the variant for dark backgrounds. */
export function useSetBrandLogo(slug: string) {
  return useBrandMutation(slug, async ({ onDark, clear }: { onDark: boolean; clear?: boolean }) => {
    if (clear) return unwrap(commands.setBrandLogo(slug, null, onDark));
    const picked = await open({ multiple: false, directory: false, filters: [{ name: "Logo", extensions: ["svg", "png"] }] });
    if (typeof picked !== "string") return unwrap(commands.getBrand(slug)); // cancelled: unchanged
    return unwrap(commands.setBrandLogo(slug, picked, onDark));
  });
}

export function useResetBrand(slug: string) {
  return useBrandMutation(slug, () => unwrap(commands.resetBrand(slug)));
}
