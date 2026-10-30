import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  commands,
  type Activity_Serialize,
  type AiPolicy,
  type CheckKind,
  type ModelCheck_Serialize,
  type Pick,
  type Reference,
} from "../../bindings";
import { unwrap } from "./unwrap";

export type Activity = Activity_Serialize;
export type ModelCheck = ModelCheck_Serialize;
export type { AiPolicy, CheckKind, Pick, Reference };

const ACTIVITY_KEY = ["ai-activity"] as const;

/** What the AI is doing and what waits for it, in the order it will run (D-048). Polled. */
export function useAiActivity() {
  return useQuery({ queryKey: ACTIVITY_KEY, queryFn: () => unwrap(commands.aiActivity()), refetchInterval: 2000 });
}

/** Run now for a catch-up job the Mac's state holds back (D-050). */
export function runAiJobNow(id: string): Promise<null> {
  return unwrap(commands.runAiJobNow(id));
}

const POLICY_KEY = ["ai-policy"] as const;

/** How AI work shares the Mac: the pace and whether catch-up work may run while you work. */
export function useAiPolicy() {
  return useQuery({ queryKey: POLICY_KEY, queryFn: () => unwrap(commands.getAiPolicy()) });
}

export function useSetAiPolicy() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (policy: AiPolicy) => unwrap(commands.setAiPolicy(policy)),
    onSettled: () => queryClient.invalidateQueries({ queryKey: POLICY_KEY }),
  });
}

const checksKey = (slug: string | undefined) => ["model-checks", slug] as const;
const referencesKey = (slug: string | undefined) => ["references", slug] as const;

/** The workspace's model checks, newest first; polled while one runs (D-050). */
export function useModelChecks(slug: string | undefined) {
  return useQuery({
    queryKey: checksKey(slug),
    queryFn: () => unwrap(commands.listModelChecks(slug ?? "")),
    enabled: !!slug,
    refetchInterval: (query) =>
      query.state.data?.some((c) => c.status === "waiting" || c.status === "running") ? 3000 : false,
  });
}

export function useStartModelCheck(slug: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (v: { kind: CheckKind; candidate: string }) => unwrap(commands.startModelCheck(slug ?? "", v.kind, v.candidate)),
    onSettled: () => queryClient.invalidateQueries({ queryKey: checksKey(slug) }),
  });
}

export function cancelModelCheck(slug: string, id: string): Promise<null> {
  return unwrap(commands.cancelModelCheck(slug, id));
}

export function usePickModelCheck(slug: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (v: { id: string; pairId: string; pick: Pick }) => unwrap(commands.pickModelCheck(slug ?? "", v.id, v.pairId, v.pick)),
    onSuccess: (check) =>
      queryClient.setQueryData<ModelCheck[]>(checksKey(slug), (all) => all?.map((c) => (c.id === check.id ? check : c))),
  });
}

/** The meetings marked as references (their transcript and summary are the gold). */
export function useReferences(slug: string | undefined) {
  return useQuery({ queryKey: referencesKey(slug), queryFn: () => unwrap(commands.listReferences(slug ?? "")), enabled: !!slug });
}

export function useSetMeetingReference(slug: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (v: { meetingId: string; on: boolean }) => unwrap(commands.setMeetingReference(slug ?? "", v.meetingId, v.on)),
    onSettled: () => {
      void queryClient.invalidateQueries({ queryKey: referencesKey(slug) });
      void queryClient.invalidateQueries({ queryKey: ["meetings", slug] });
      void queryClient.invalidateQueries({ queryKey: ["meeting", slug] });
    },
  });
}
