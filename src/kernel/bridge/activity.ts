import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type Activity_Serialize, type Pace } from "../../bindings";
import { unwrap } from "./unwrap";

export type Activity = Activity_Serialize;
export type { Pace };

/** What the AI is doing and what waits for it, in the order it will run (D-048). Polled. */
export function useAiActivity() {
  return useQuery({ queryKey: ["ai-activity"], queryFn: () => unwrap(commands.aiActivity()), refetchInterval: 2000 });
}

const PACE_KEY = ["ai-pace"] as const;

export function useAiPace() {
  return useQuery({ queryKey: PACE_KEY, queryFn: () => unwrap(commands.getAiPace()) });
}

export function useSetAiPace() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (pace: Pace) => unwrap(commands.setAiPace(pace)),
    onSettled: () => queryClient.invalidateQueries({ queryKey: PACE_KEY }),
  });
}
