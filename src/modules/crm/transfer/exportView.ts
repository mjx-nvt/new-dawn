import type { ObjectDef } from "../../../kernel/bridge/records";
import { exportViewToFile, type ExportReport } from "../../../kernel/bridge/transfer";
import type { ViewDef } from "../../../kernel/bridge/views";
import { queryOf, type Working } from "../views/useViewState";

/** Export view (D-065): the save dialog, then the view's records — its visible fields in order,
 *  its filters, sorts and the search — as CSV; null when cancelled. */
export function exportCurrentView(slug: string, object: ObjectDef, current: ViewDef, working: Working, search: string): Promise<ExportReport | null> {
  const byId = new Map(object.fields.map((f) => [f.id, f.name]));
  const fields = working.fields.filter((f) => f.is_visible).flatMap((f) => byId.get(f.field_id) ?? []);
  const file = `${current.view.name === "" ? object.meta.label_plural : current.view.name}.csv`.replace(/[/\\:]/g, "-");
  return exportViewToFile(slug, object.meta.name_singular, current.view.name, queryOf(object, working, { search, deleted: false }), fields, file);
}
