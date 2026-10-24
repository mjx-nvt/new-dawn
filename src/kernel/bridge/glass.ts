import { useQuery } from "@tanstack/react-query";
import { commands } from "../../bindings";
import { unwrap } from "./unwrap";

export function useGlassStatus() {
  return useQuery({ queryKey: ["glass-status"], queryFn: () => unwrap(commands.glassStatus()) });
}
