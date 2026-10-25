import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type Profile, type ProfilePatch_Deserialize } from "../../bindings";
import { unwrap } from "./unwrap";

export type { Profile };
export type ProfilePatch = ProfilePatch_Deserialize;

const PROFILE_KEY = ["profile"] as const;

/** You: your name (app-wide; each workspace adds your title) and who attends by default. */
export function useProfile() {
  return useQuery({ queryKey: PROFILE_KEY, queryFn: () => unwrap(commands.getProfile()) });
}

export function useSetProfile() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (patch: ProfilePatch) => unwrap(commands.setProfile(patch)),
    onSuccess: (saved) => queryClient.setQueryData(PROFILE_KEY, saved),
  });
}
