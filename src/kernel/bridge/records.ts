import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type FieldMeta, type FieldType, type Hit, type Json, type ObjectDef, type ObjectMeta, type Page, type Query_Deserialize, type SelectOption } from "../../bindings";
import { unwrap } from "./unwrap";

export type { FieldMeta, FieldType, Hit, Json, ObjectDef, ObjectMeta, Page, SelectOption };
export type Query = Query_Deserialize;

/** A record as the engine returns it: its fields by name, composites as objects, a many-to-one
 *  relation as `<name>Id` and `<name>: { id, label }`. Its shape comes from the metadata, so the
 *  values are `unknown` and the cells narrow them by field type. */
export type RecordJson = Record<string, unknown>;

export function asRecord(value: unknown): RecordJson {
  return value && typeof value === "object" && !Array.isArray(value) ? (value as RecordJson) : {};
}

export function recordId(record: RecordJson): string {
  return typeof record.id === "string" ? record.id : "";
}

/** Every object of the workspace's CRM, system ones included (D-060). */
export function useObjects(slug: string | undefined) {
  return useQuery({ queryKey: ["objects", slug ?? ""], queryFn: () => unwrap(commands.listObjects(slug ?? "")), enabled: !!slug });
}

/** An object with its fields, by singular name, plural name or id. */
export function useObject(slug: string | undefined, name: string | undefined) {
  return useQuery({ queryKey: ["object", slug ?? "", name ?? ""], queryFn: () => unwrap(commands.getObject(slug ?? "", name ?? "")), enabled: !!slug && !!name });
}

/** A page of an object's records; the previous page stays while the next loads. */
export function useRecords(slug: string | undefined, object: string | undefined, query: Query, enabled = true) {
  return useQuery({
    queryKey: ["records", slug ?? "", object ?? "", query],
    queryFn: () => unwrap(commands.listRecords(slug ?? "", object ?? "", query)),
    enabled: !!slug && !!object && enabled,
    placeholderData: keepPreviousData,
  });
}

export function useRecord(slug: string | undefined, object: string | undefined, id: string | undefined) {
  return useQuery({
    queryKey: ["record", slug ?? "", object ?? "", id ?? ""],
    queryFn: () => unwrap(commands.getRecord(slug ?? "", object ?? "", id ?? "")),
    enabled: !!slug && !!object && !!id,
  });
}

function useRefresh(slug: string | undefined, object: string | undefined) {
  const queryClient = useQueryClient();
  return () => {
    void queryClient.invalidateQueries({ queryKey: ["records", slug ?? "", object ?? ""] });
    void queryClient.invalidateQueries({ queryKey: ["record", slug ?? "", object ?? ""] });
    void queryClient.invalidateQueries({ queryKey: ["search", slug ?? ""] });
    // What the engine keeps beside its own columns: the people memory, Tasks, Today.
    for (const key of ["people", "people-cards", "tasks", "open-task-count", "today"]) void queryClient.invalidateQueries({ queryKey: [key] });
  };
}

export function useCreateRecord(slug: string | undefined, object: string | undefined) {
  const refresh = useRefresh(slug, object);
  return useMutation({ mutationFn: (data: RecordJson) => unwrap(commands.createRecord(slug ?? "", object ?? "", data)), onSuccess: refresh });
}

export function useUpdateRecord(slug: string | undefined, object: string | undefined) {
  const refresh = useRefresh(slug, object);
  return useMutation({
    mutationFn: ({ id, data }: { id: string; data: RecordJson }) => unwrap(commands.updateRecord(slug ?? "", object ?? "", id, data)),
    onSuccess: refresh,
  });
}

export function useDeleteRecord(slug: string | undefined, object: string | undefined) {
  const refresh = useRefresh(slug, object);
  return useMutation({ mutationFn: (id: string) => unwrap(commands.deleteRecord(slug ?? "", object ?? "", id)), onSuccess: refresh });
}

export function useRestoreRecord(slug: string | undefined, object: string | undefined) {
  const refresh = useRefresh(slug, object);
  return useMutation({ mutationFn: (id: string) => unwrap(commands.restoreRecord(slug ?? "", object ?? "", id)), onSuccess: refresh });
}

export function useDestroyRecord(slug: string | undefined, object: string | undefined) {
  const refresh = useRefresh(slug, object);
  return useMutation({ mutationFn: (id: string) => unwrap(commands.destroyRecord(slug ?? "", object ?? "", id)), onSuccess: refresh });
}

/** Records of every object whose words match (`/`). */
export function useSearchRecords(slug: string | undefined, text: string) {
  return useQuery({
    queryKey: ["search", slug ?? "", text],
    queryFn: () => unwrap(commands.searchRecords(slug ?? "", text, 40)),
    enabled: !!slug && text.trim().length > 0,
    placeholderData: keepPreviousData,
  });
}

/** A field that occupies columns (not a one-to-many relation). */
export function hasColumns(field: FieldMeta): boolean {
  return field.type !== "RELATION" || field.options.relation?.kind === "MANY_TO_ONE";
}

/** The fields a table shows, in order: the label identifier first, then by position; never the
 *  system ones, the position, or a one-to-many relation. */
export function tableFields(object: ObjectDef): FieldMeta[] {
  const fields = object.fields.filter((f) => f.is_active && !f.is_system && f.type !== "POSITION" && hasColumns(f));
  const label = fields.find((f) => f.id === object.meta.label_identifier_field_id);
  const rest = fields.filter((f) => f !== label);
  // Who made a record reads last.
  const actor = rest.filter((f) => f.type === "ACTOR");
  return [...(label ? [label] : []), ...rest.filter((f) => f.type !== "ACTOR"), ...actor];
}

export function selectOptions(field: FieldMeta): SelectOption[] {
  return field.options.select ?? [];
}

/** The text that names a record: its label identifier, rendered. */
export function recordLabel(object: ObjectDef, record: RecordJson): string {
  const field = object.fields.find((f) => f.id === object.meta.label_identifier_field_id);
  if (!field) return "";
  const value = record[field.name];
  if (field.type === "FULL_NAME") {
    const v = asRecord(value);
    return `${String(v.firstName ?? "")} ${String(v.lastName ?? "")}`.trim();
  }
  return typeof value === "string" ? value : "";
}

/** A new record from what the owner typed in the Add row: the label identifier, split for a
 *  full name at its last space (the engine's rule). */
export function labelInput(object: ObjectDef, text: string): RecordJson {
  const field = object.fields.find((f) => f.id === object.meta.label_identifier_field_id);
  if (!field) return {};
  const trimmed = text.trim();
  if (field.type === "FULL_NAME") {
    const at = trimmed.lastIndexOf(" ");
    return at > 0 ? { [field.name]: { firstName: trimmed.slice(0, at).trim(), lastName: trimmed.slice(at + 1).trim() } } : { [field.name]: { firstName: trimmed, lastName: "" } };
  }
  return { [field.name]: trimmed };
}
