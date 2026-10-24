import { useQuery } from "@tanstack/react-query";
import { getIdentifier, getVersion } from "@tauri-apps/api/app";

export function useAppInfo() {
  return useQuery({
    queryKey: ["app-info"],
    queryFn: async () => ({ version: await getVersion(), identifier: await getIdentifier() }),
  });
}
