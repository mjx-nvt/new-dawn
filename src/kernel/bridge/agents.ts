import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type Agent, type AgentGuide, type Judged, type Lesson } from "../../bindings";
import { unwrap } from "./unwrap";

export type { Agent, AgentGuide, Judged, Lesson };

const key = (slug: string) => ["agents", slug] as const;

/** The agents that talk to the model, each with the guide in force and its last run. */
export function useAgents(slug: string) {
  return useQuery({ queryKey: key(slug), queryFn: () => unwrap(commands.listAgents(slug)), enabled: !!slug });
}

/** Saves the workspace's own guide; empty text restores the bundled default. */
export function useSetAgentGuide(slug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (v: { agent: Agent; text: string }) => unwrap(commands.setAgentGuide(slug, v.agent, v.text)),
    onSuccess: (guides) => queryClient.setQueryData(key(slug), guides),
  });
}

/** Chooses the model an agent runs on (null = the app's selected model). */
export function useSetAgentModel(slug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (v: { agent: Agent; model: string | null }) => unwrap(commands.setAgentModel(slug, v.agent, v.model)),
    onSuccess: (guides) => queryClient.setQueryData(key(slug), guides),
  });
}

export function useResetAgentGuide(slug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (agent: Agent) => unwrap(commands.resetAgentGuide(slug, agent)),
    onSuccess: (guides) => queryClient.setQueryData(key(slug), guides),
  });
}

/** "Check now": reads your newest decisions about the agent's work without waiting for a free Mac
 *  (D-052); resolves with how many lines it proposes. */
export function useLearnNow(slug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (agent: Agent) => unwrap(commands.learnNow(slug, agent)),
    onSettled: () => queryClient.invalidateQueries({ queryKey: key(slug) }),
  });
}

/** Approve adds the proposed line to the agent's guide; Dismiss sets it aside for good. */
export function useJudgeLesson(slug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (v: { id: string; approve: boolean }) => unwrap(v.approve ? commands.approveLesson(slug, v.id) : commands.dismissLesson(slug, v.id)),
    onSuccess: (guides) => queryClient.setQueryData(key(slug), guides),
  });
}

/** Stops using a line approved from "What I learned"; it isn't proposed again. */
export function useForgetLearned(slug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (v: { agent: Agent; line: string }) => unwrap(commands.forgetLearned(slug, v.agent, v.line)),
    onSuccess: (guides) => queryClient.setQueryData(key(slug), guides),
  });
}
