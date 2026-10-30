import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import { commands, type EmailItem_Serialize, type EmailTask_Serialize } from "../../bindings";
import { invalidateTasks } from "./tasks";
import { unwrap } from "./unwrap";

export type EmailItem = EmailItem_Serialize;
export type EmailTask = EmailTask_Serialize;

/** The emails read, newest first, with what came of each (D-056). */
export function useEmails(slug: string | undefined) {
  return useQuery({ queryKey: ["emails", slug ?? ""], queryFn: () => unwrap(commands.listEmails(slug ?? "")), enabled: !!slug });
}

/** After a reading: the lists its proposals landed in. Resolves once the emails list is fresh, so
 *  a button stays busy until its row shows the change. */
function useRefresh(slug: string | undefined) {
  const queryClient = useQueryClient();
  return () => {
    for (const key of ["projects", "project", "commitments-waiting", "commitments-unlinked", "commitments-dismissed", "people-cards", "person"]) void queryClient.invalidateQueries({ queryKey: [key, slug ?? ""] });
    invalidateTasks(queryClient, slug ?? "");
    return queryClient.invalidateQueries({ queryKey: ["emails", slug ?? ""] });
  };
}

/** Reads the email selected in Apple Mail. */
export function useReadMail(slug: string | undefined) {
  const refresh = useRefresh(slug);
  return useMutation({ mutationFn: () => unwrap(commands.readMailSelection(slug ?? "")), onSettled: refresh });
}

/** Picks an .eml file and reads it; resolves with null when the picker is cancelled. */
export function useReadEmailFile(slug: string | undefined) {
  const refresh = useRefresh(slug);
  return useMutation({
    mutationFn: async () => {
      const picked = await open({ multiple: false, directory: false, filters: [{ name: "Email", extensions: ["eml"] }] });
      return typeof picked === "string" ? unwrap(commands.readEmailFile(slug ?? "", picked)) : null;
    },
    onSettled: refresh,
  });
}

/** Forget: the email's text and its proposals still waiting go. */
export function useForgetEmail(slug: string | undefined) {
  const refresh = useRefresh(slug);
  return useMutation({ mutationFn: (id: string) => unwrap(commands.forgetEmail(slug ?? "", id)), onSettled: refresh });
}

/** Add to Tasks: an email's proposed task becomes yours. */
export function useAddEmailTask(slug: string | undefined) {
  const refresh = useRefresh(slug);
  return useMutation({ mutationFn: (id: string) => unwrap(commands.addEmailTask(slug ?? "", id)), onSettled: refresh });
}

/** Dismiss an email's proposed task. */
export function useDismissEmailTask(slug: string | undefined) {
  const refresh = useRefresh(slug);
  return useMutation({ mutationFn: (id: string) => unwrap(commands.dismissEmailTask(slug ?? "", id)), onSettled: refresh });
}
