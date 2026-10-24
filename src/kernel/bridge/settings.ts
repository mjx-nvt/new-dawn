import { commands, type SettingsTab } from "../../bindings";
import { unwrap } from "./unwrap";

export type { SettingsTab };

/** Opens the Settings window (or brings it forward) on `tab`. */
export function openSettings(tab: SettingsTab): Promise<null> {
  return unwrap(commands.openSettings(tab));
}
