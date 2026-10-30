import { useQuery } from "@tanstack/react-query";
import { commands, type Day_Serialize, type Today_Serialize } from "../../bindings";
import { unwrap } from "./unwrap";

export type Today = Today_Serialize;
export type Day = Day_Serialize;

/** What needs you today, across every open workspace (D-057). Read again each minute, so "today"
 *  turns over at midnight and what changed in another window shows. */
export function useToday() {
  return useQuery({ queryKey: ["today"], queryFn: () => unwrap(commands.cockpitToday()), refetchInterval: 60_000 });
}
