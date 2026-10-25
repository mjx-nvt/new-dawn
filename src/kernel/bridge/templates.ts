import { useMutation, useQuery, useQueryClient, type QueryClient } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import {
  commands,
  type Language,
  type MeetingTypeView,
  type TemplateCheck,
  type TemplateField,
  type TemplateInfo,
  type TemplatePreview,
} from "../../bindings";
import { unwrap } from "./unwrap";

export type { MeetingTypeView, TemplateCheck, TemplateField, TemplateInfo, TemplatePreview };

const keys = {
  types: (slug: string) => ["meeting-type-templates", slug] as const,
  fields: ["minutes-template-fields"] as const,
  preview: (slug: string) => ["template-preview", slug] as const,
};

/** The workspace's meeting types with their templates (a first one is made if there are none). */
export function useMeetingTypeTemplates(slug: string) {
  return useQuery({ queryKey: keys.types(slug), queryFn: () => unwrap(commands.listMeetingTypeTemplates(slug)), enabled: !!slug });
}

export function useTemplateFields() {
  return useQuery({ queryKey: keys.fields, queryFn: () => unwrap(commands.minutesTemplateFields()), staleTime: Infinity });
}

/** The default type's template filled with a made-up meeting, as a picture (Quick Look takes a
 *  second or two; a failure isn't retried). */
export function useTemplatePreview(slug: string, enabled: boolean) {
  return useQuery({
    queryKey: keys.preview(slug),
    queryFn: () => unwrap(commands.previewMinutesTemplate(slug)),
    enabled: !!slug && enabled,
    retry: false,
  });
}

/** Draws the preview again (its header shows the slogans, which fill in at export). */
export function useRedrawTemplatePreview(slug: string) {
  const queryClient = useQueryClient();
  return () => void queryClient.invalidateQueries({ queryKey: keys.preview(slug) });
}

/** Native picker for a .docx; null when cancelled. */
export async function pickTemplateFile(): Promise<string | null> {
  const picked = await open({ multiple: false, directory: false, filters: [{ name: "Word document", extensions: ["docx"] }] });
  return typeof picked === "string" ? picked : null;
}

export function checkTemplate(path: string): Promise<TemplateCheck> {
  return unwrap(commands.checkMinutesTemplate(path));
}

/** Every type mutation returns the fresh list; pickers elsewhere (["meeting-types"]) and the
 *  preview (the default type's template may have changed) refresh too. */
function useTypesMutation<V>(slug: string, call: (vars: V) => Promise<MeetingTypeView[]>) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: call,
    onSuccess: (list) => store(queryClient, slug, list),
  });
}

function store(queryClient: QueryClient, slug: string, list: MeetingTypeView[]) {
  queryClient.setQueryData(keys.types(slug), list);
  void queryClient.invalidateQueries({ queryKey: ["meeting-types", slug] });
  void queryClient.invalidateQueries({ queryKey: keys.preview(slug) });
}

export function useCreateMeetingType(slug: string) {
  return useTypesMutation(slug, (v: { name: string; language: Language; template: string | null }) =>
    unwrap(commands.createMeetingType(slug, v.name, v.language, v.template)),
  );
}

export function useUpdateMeetingType(slug: string) {
  return useTypesMutation(slug, (v: { id: string; name: string; language: Language }) =>
    unwrap(commands.updateMeetingType(slug, v.id, v.name, v.language)),
  );
}

export function useSetDefaultMeetingType(slug: string) {
  return useTypesMutation(slug, (id: string) => unwrap(commands.setDefaultMeetingType(slug, id)));
}

export function useDeleteMeetingType(slug: string) {
  return useTypesMutation(slug, (id: string) => unwrap(commands.deleteMeetingType(slug, id)));
}

export function useReplaceTypeTemplate(slug: string) {
  return useTypesMutation(slug, (v: { id: string; path: string }) => unwrap(commands.replaceTypeTemplate(slug, v.id, v.path)));
}

export function useResetTypeTemplate(slug: string) {
  return useTypesMutation(slug, (id: string) => unwrap(commands.resetTypeTemplate(slug, id)));
}

/** Makes the minutes template from the brand again (the default type's; the old one goes to the Trash). */
export function useGenerateTemplate(slug: string) {
  return useTypesMutation(slug, () => unwrap(commands.generateMinutesTemplate(slug)));
}

/** Opens the type's template in Word (or Finder); refreshes the list afterwards. */
export function useOpenTypeTemplate(slug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (v: { id: string; reveal: boolean }) => unwrap(commands.openTypeTemplate(slug, v.id, v.reveal)),
    onSettled: () => queryClient.invalidateQueries({ queryKey: keys.types(slug) }),
  });
}
