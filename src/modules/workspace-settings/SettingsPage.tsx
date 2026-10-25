import { getRouteApi, Link, useNavigate } from "@tanstack/react-router";
import { useBrandSync } from "../../kernel/bridge/brand";
import { useOpenSettingsTab, useWorkspacesSync } from "../../kernel/bridge/events";
import type { SettingsTab } from "../../kernel/bridge/settings";
import { useCurrentWorkspace } from "../../kernel/bridge/workspaces";
import { useBrandProps } from "../../kernel/theme/applyBrand";
import ModelsPanel from "../models/ModelsPanel";
import AboutTab from "./AboutTab";
import BackupPanel from "./BackupPanel";
import ConvertersPanel from "./ConvertersPanel";
import GeneralTab from "./GeneralTab";
import StoragePanel from "./StoragePanel";
import styles from "./SettingsPage.module.css";
import WorkspacesPanel from "./WorkspacesPanel";

const route = getRouteApi("/settings");

/** App-wide settings only; a workspace's own (brand, meeting types, people) are on the
 *  Workspace page in the main window. */
export const SETTINGS_TABS: { id: SettingsTab; label: string; group: string }[] = [
  { id: "general", label: "General", group: "App" },
  { id: "workspaces", label: "Workspaces", group: "App" },
  { id: "backup", label: "Backup", group: "App" },
  { id: "storage", label: "Storage", group: "App" },
  { id: "models", label: "Models", group: "AI on this Mac" },
  { id: "converters", label: "PDF converters", group: "AI on this Mac" },
  { id: "about", label: "About", group: "" },
];

function Pane({ tab }: { tab: SettingsTab }) {
  switch (tab) {
    case "general":
      return <GeneralTab />;
    case "workspaces":
      return <WorkspacesPanel />;
    case "backup":
      return <BackupPanel />;
    case "storage":
      return <StoragePanel />;
    case "models":
      return <ModelsPanel />;
    case "converters":
      return <ConvertersPanel />;
    case "about":
      return <AboutTab />;
  }
}

/** The Settings window: a glass sidebar of panes on the left, the chosen pane on the right. */
export default function SettingsPage() {
  const { tab } = route.useSearch();
  const navigate = useNavigate();
  const { data: workspace } = useCurrentWorkspace();
  const brand = useBrandProps(workspace?.slug);
  useWorkspacesSync();
  useBrandSync();
  useOpenSettingsTab((next) => void navigate({ to: "/settings", search: { tab: next }, replace: true }));
  const current = SETTINGS_TABS.find((t) => t.id === tab);
  const groups = [...new Set(SETTINGS_TABS.map((t) => t.group))];

  return (
    <div className={styles.window}>
      <div className={styles.strip} data-tauri-drag-region="deep" />
      <nav className={styles.sidebar} aria-label="Settings">
        {groups.map((group) => (
          <div key={group} className={styles.group}>
            {group && <p className={styles.groupLabel}>{group}</p>}
            {SETTINGS_TABS.filter((t) => t.group === group).map((item) => (
              <Link
                key={item.id}
                to="/settings"
                search={{ tab: item.id }}
                replace
                aria-current={item.id === tab ? "page" : undefined}
                className={item.id === tab ? `${styles.item} ${styles.active}` : styles.item}
              >
                {item.label}
              </Link>
            ))}
          </div>
        ))}
      </nav>
      <main className={`content-root ${styles.content}`} {...brand}>
        <div className={styles.pane}>
          <h1 className={styles.paneTitle}>{current?.label}</h1>
          <Pane tab={tab} />
        </div>
      </main>
    </div>
  );
}
