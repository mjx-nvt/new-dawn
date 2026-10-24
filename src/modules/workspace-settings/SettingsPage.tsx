import { getRouteApi, Link, useNavigate } from "@tanstack/react-router";
import { useOpenSettingsTab, useWorkspacesSync } from "../../kernel/bridge/events";
import type { SettingsTab } from "../../kernel/bridge/settings";
import AboutTab from "./AboutTab";
import ConvertersPanel from "./ConvertersPanel";
import GeneralTab from "./GeneralTab";
import ModelsPanel from "../models/ModelsPanel";
import styles from "./SettingsPage.module.css";
import WorkspacesPanel from "./WorkspacesPanel";

const route = getRouteApi("/settings");

export const SETTINGS_TABS: { id: SettingsTab; label: string }[] = [
  { id: "general", label: "General" },
  { id: "workspaces", label: "Workspaces" },
  { id: "models", label: "Models" },
  { id: "converters", label: "Converters" },
  { id: "about", label: "About" },
];

function TabContent({ tab }: { tab: SettingsTab }) {
  switch (tab) {
    case "general":
      return <GeneralTab />;
    case "workspaces":
      return <WorkspacesPanel />;
    case "models":
      return <ModelsPanel />;
    case "converters":
      return <ConvertersPanel />;
    case "about":
      return <AboutTab />;
  }
}

/** The Settings window: tabs in a glass title strip, content on the solid surface. */
export default function SettingsPage() {
  const { tab } = route.useSearch();
  const navigate = useNavigate();
  useWorkspacesSync();
  useOpenSettingsTab((next) => void navigate({ to: "/settings", search: { tab: next }, replace: true }));

  return (
    <div className={styles.window}>
      <header className={styles.strip} data-tauri-drag-region>
        <nav className={styles.tabs}>
          {SETTINGS_TABS.map((item) => (
            <Link
              key={item.id}
              to="/settings"
              search={{ tab: item.id }}
              replace
              className={item.id === tab ? `${styles.tab} ${styles.active}` : styles.tab}
            >
              {item.label}
            </Link>
          ))}
        </nav>
      </header>
      <main className={`content-root ${styles.content}`}>
        <TabContent tab={tab} />
      </main>
    </div>
  );
}
