import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type AssistantAgent, type AssistantAgentInput_Deserialize, type ChatMessage, type ChatThread } from "../../bindings";
import { unwrap } from "./unwrap";
import { refreshAfterRun } from "./workflows";

export type { AssistantAgent, ChatMessage, ChatThread };
export type AssistantAgentInput = AssistantAgentInput_Deserialize;

/** The CRM's assistant (D-070, 0.10.13): its agents, the chats, a question, the proposals. */

export const ASSISTANT_TOOLS: { name: string; label: string; writes: boolean }[] = [
  { name: "find_records", label: "Find records", writes: false },
  { name: "count_records", label: "Count records", writes: false },
  { name: "aggregate", label: "Sum, average, smallest, largest", writes: false },
  { name: "open_record", label: "Open a record", writes: false },
  { name: "propose_create", label: "Propose a new record", writes: true },
  { name: "propose_update", label: "Propose a change", writes: true },
  { name: "propose_note", label: "Propose a note", writes: true },
];

export function useAssistantAgents(slug: string | undefined) {
  return useQuery({ queryKey: ["assistant-agents", slug ?? ""], queryFn: () => unwrap(commands.listAssistantAgents(slug ?? "")), enabled: !!slug });
}

export function useSaveAssistantAgent(slug: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: AssistantAgentInput) => unwrap(commands.saveAssistantAgent(slug ?? "", input)),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["assistant-agents", slug ?? ""] }),
  });
}

export function useDeleteAssistantAgent(slug: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => unwrap(commands.deleteAssistantAgent(slug ?? "", id)),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["assistant-agents", slug ?? ""] });
      void queryClient.invalidateQueries({ queryKey: ["chat-threads", slug ?? ""] });
    },
  });
}

/** The chats, newest first: all, or a record's. */
export function useChatThreads(slug: string | undefined, on: { object: string; id: string } | null) {
  return useQuery({
    queryKey: ["chat-threads", slug ?? "", on?.object ?? "", on?.id ?? ""],
    queryFn: () => unwrap(commands.listChatThreads(slug ?? "", on?.object ?? null, on?.id ?? null)),
    enabled: !!slug,
  });
}

export function useNewChatThread(slug: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ agentId, on }: { agentId: string | null; on: { object: string; id: string } | null }) => unwrap(commands.newChatThread(slug ?? "", agentId, on?.object ?? "", on?.id ?? "")),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["chat-threads", slug ?? ""] }),
  });
}

export function useDeleteChatThread(slug: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => unwrap(commands.deleteChatThread(slug ?? "", id)),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["chat-threads", slug ?? ""] }),
  });
}

export function useChatMessages(slug: string | undefined, threadId: string | null) {
  return useQuery({ queryKey: ["chat-messages", slug ?? "", threadId ?? ""], queryFn: () => unwrap(commands.chatMessages(slug ?? "", threadId ?? "")), enabled: !!slug && !!threadId });
}

/** A question: the thread comes back with the agent's turn in it. */
export function useAskAssistant(slug: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ threadId, text }: { threadId: string; text: string }) => unwrap(commands.askAssistant(slug ?? "", threadId, text)),
    onSuccess: (messages, { threadId }) => {
      queryClient.setQueryData(["chat-messages", slug ?? "", threadId], messages);
      void queryClient.invalidateQueries({ queryKey: ["chat-threads", slug ?? ""] });
    },
    onError: (_e, { threadId }) => void queryClient.invalidateQueries({ queryKey: ["chat-messages", slug ?? "", threadId] }),
  });
}

/** Apply or dismiss a proposal; an applied one changed records, so they're read again. */
export function useProposalAction(slug: string | undefined, threadId: string | null) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ messageId, action }: { messageId: string; action: "apply" | "dismiss" }) => unwrap(action === "apply" ? commands.applyChatProposal(slug ?? "", messageId) : commands.dismissChatProposal(slug ?? "", messageId)),
    onSettled: () => {
      void queryClient.invalidateQueries({ queryKey: ["chat-messages", slug ?? "", threadId ?? ""] });
      refreshAfterRun(queryClient, slug);
    },
  });
}

/** Opens the assistant's panel — on a record, or for a chat of its own. */
export function askAssistant(on?: { object: string; id: string; label?: string; objectLabel?: string }) {
  window.dispatchEvent(new CustomEvent("newdawn:ask-ai", { detail: on ?? null }));
}
