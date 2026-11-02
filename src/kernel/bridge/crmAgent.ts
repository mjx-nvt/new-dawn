import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type CrmProposal_Serialize, type CrmProposalKind } from "../../bindings";
import { unwrap } from "./unwrap";

export type CrmProposal = CrmProposal_Serialize;
export type { CrmProposalKind };

/** The CRM agent's proposals waiting (D-072): all, or those about one record. They arrive in the
 *  background after a summary or an email, so the list looks again every minute. */
export function useCrmProposals(slug: string | undefined, about?: string | null) {
  return useQuery({
    queryKey: ["crm-proposals", slug ?? "", about ?? ""],
    queryFn: () => unwrap(commands.listCrmProposals(slug ?? "", about ?? null)),
    enabled: !!slug,
    refetchInterval: 60_000,
  });
}

/** After a decision: the proposals, the CRM's lists and records, Tasks and Today. */
function useRefresh(slug: string | undefined) {
  const queryClient = useQueryClient();
  return () => {
    for (const key of ["crm-proposals", "records", "record", "search", "timeline", "linked"]) void queryClient.invalidateQueries({ queryKey: [key, slug ?? ""] });
    for (const key of ["tasks", "open-task-count", "today"]) void queryClient.invalidateQueries({ queryKey: [key] });
  };
}

export function useApplyCrmProposal(slug: string | undefined) {
  const refresh = useRefresh(slug);
  return useMutation({ mutationFn: (id: string) => unwrap(commands.applyCrmProposal(slug ?? "", id)), onSettled: refresh });
}

export function useDismissCrmProposal(slug: string | undefined) {
  const refresh = useRefresh(slug);
  return useMutation({ mutationFn: (id: string) => unwrap(commands.dismissCrmProposal(slug ?? "", id)), onSettled: refresh });
}

/** A meeting's ⋯ → Find CRM proposals: reads its summary now; how many it proposed. */
export function useFindCrmProposals(slug: string | undefined) {
  const refresh = useRefresh(slug);
  return useMutation({ mutationFn: (meetingId: string) => unwrap(commands.findCrmProposals(slug ?? "", meetingId)), onSuccess: refresh });
}
