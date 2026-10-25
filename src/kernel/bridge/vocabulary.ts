import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type Term, type TermInput } from "../../bindings";
import { unwrap } from "./unwrap";

export type { Term, TermInput };

const key = (slug: string) => ["vocabulary", slug] as const;
const promptKey = (slug: string) => ["whisper-prompt", slug] as const;

export function useVocabulary(slug: string) {
  return useQuery({ queryKey: key(slug), queryFn: () => unwrap(commands.listVocabulary(slug)), enabled: !!slug });
}

/** What Whisper is told before the next transcription. */
export function useWhisperPrompt(slug: string) {
  return useQuery({ queryKey: promptKey(slug), queryFn: () => unwrap(commands.whisperPromptPreview(slug)), enabled: !!slug });
}

function useRefresh(slug: string) {
  const queryClient = useQueryClient();
  return () => {
    void queryClient.invalidateQueries({ queryKey: key(slug) });
    void queryClient.invalidateQueries({ queryKey: promptKey(slug) });
  };
}

export function useSaveTerm(slug: string) {
  const refresh = useRefresh(slug);
  return useMutation({ mutationFn: (term: TermInput) => unwrap(commands.saveVocabularyTerm(slug, term)), onSettled: refresh });
}

export function useDeleteTerm(slug: string) {
  const refresh = useRefresh(slug);
  return useMutation({ mutationFn: (id: string) => unwrap(commands.deleteVocabularyTerm(slug, id)), onSettled: refresh });
}
