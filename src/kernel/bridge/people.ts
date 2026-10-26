import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type AttendeeEdit, type Person, type PersonInput } from "../../bindings";
import { invalidateTasks } from "./tasks";
import { unwrap } from "./unwrap";

export type { AttendeeEdit, Person, PersonInput };

const key = (slug: string) => ["people", slug] as const;

export function usePeople(slug: string) {
  return useQuery({ queryKey: key(slug), queryFn: () => unwrap(commands.listPeople(slug)), enabled: !!slug });
}

export function useSavePerson(slug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (person: PersonInput) => unwrap(commands.savePerson(slug, person)),
    onSettled: () => queryClient.invalidateQueries({ queryKey: key(slug) }),
  });
}

export function useDeletePerson(slug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => unwrap(commands.deletePerson(slug, id)),
    onSettled: () => queryClient.invalidateQueries({ queryKey: key(slug) }),
  });
}

/** `other` becomes another spelling of `keep`. */
export function useMergePeople(slug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (v: { keep: string; other: string }) => unwrap(commands.mergePeople(slug, v.keep, v.other)),
    onSettled: () => queryClient.invalidateQueries({ queryKey: key(slug) }),
  });
}

/** Saves a meeting's corrected attendees; `remember` teaches the people memory. */
export function useUpdateAttendees(slug: string, meetingId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (v: { attendees: AttendeeEdit[]; remember: boolean }) =>
      unwrap(commands.updateAttendees(slug, meetingId, v.attendees, v.remember)),
    onSuccess: (view) => {
      queryClient.setQueryData(["summary", slug, meetingId], view);
      void queryClient.invalidateQueries({ queryKey: key(slug) });
      invalidateTasks(queryClient, slug); // a rename reaches the meeting's tasks
    },
  });
}
