import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type Commitment, type CommitmentStatus, type MetIn, type PersonCard, type PersonDossier } from "../../bindings";
import { unwrap } from "./unwrap";

export type { Commitment, CommitmentStatus, MetIn, PersonCard, PersonDossier };

/** Everyone in the people memory, with last contact and commitments (D-055). */
export function usePeopleCards(slug: string | undefined) {
  return useQuery({ queryKey: ["people-cards", slug ?? ""], queryFn: () => unwrap(commands.listPeopleCards(slug ?? "")), enabled: !!slug });
}

/** Commitments from new summaries, waiting for you; they arrive after a summary, so it looks again
 *  every minute (the sidebar's count). */
export function useWaitingCommitments(slug: string | undefined) {
  return useQuery({
    queryKey: ["commitments-waiting", slug ?? ""],
    queryFn: () => unwrap(commands.listWaitingCommitments(slug ?? "")),
    enabled: !!slug,
    refetchInterval: 60_000,
  });
}

/** Confirmed commitments of someone the people memory doesn't know (an organisation, a name). */
export function useUnlinkedCommitments(slug: string | undefined) {
  return useQuery({ queryKey: ["commitments-unlinked", slug ?? ""], queryFn: () => unwrap(commands.listUnlinkedCommitments(slug ?? "")), enabled: !!slug });
}

/** The newest dismissed, for Undo. */
export function useDismissedCommitments(slug: string | undefined) {
  return useQuery({ queryKey: ["commitments-dismissed", slug ?? ""], queryFn: () => unwrap(commands.listDismissedCommitments(slug ?? "")), enabled: !!slug });
}

export function usePerson(slug: string | undefined, id: string) {
  return useQuery({ queryKey: ["person", slug ?? "", id], queryFn: () => unwrap(commands.getPerson(slug ?? "", id)), enabled: !!slug });
}

function useRefresh(slug: string | undefined) {
  const queryClient = useQueryClient();
  return () => {
    for (const key of ["people-cards", "commitments-waiting", "commitments-unlinked", "commitments-dismissed", "person"]) void queryClient.invalidateQueries({ queryKey: [key, slug ?? ""] });
  };
}

/** Confirm (open), kept, dropped or dismissed. */
export function useSetCommitmentStatus(slug: string | undefined) {
  const refresh = useRefresh(slug);
  return useMutation({ mutationFn: (v: { id: string; status: CommitmentStatus }) => unwrap(commands.setCommitmentStatus(slug ?? "", v.id, v.status)), onSettled: refresh });
}

/** "Look through earlier meetings": resolves with how many commitments it found. */
export function useCollectCommitments(slug: string | undefined) {
  const refresh = useRefresh(slug);
  return useMutation({ mutationFn: () => unwrap(commands.collectCommitments(slug ?? "")), onSuccess: refresh });
}

export function useSetPersonNotes(slug: string | undefined) {
  const refresh = useRefresh(slug);
  return useMutation({ mutationFn: (v: { id: string; notes: string }) => unwrap(commands.setPersonNotes(slug ?? "", v.id, v.notes)), onSuccess: refresh });
}
