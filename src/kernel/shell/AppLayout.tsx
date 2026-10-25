import { Outlet, useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { useBrandSync } from "../bridge/brand";
import { openSettings } from "../bridge/settings";
import { useUpdateCheckAtLaunch } from "../bridge/updater";
import { useMenuAction, useWorkspacesSync } from "../bridge/events";
import { useTranscriptionSync } from "../bridge/meetings";
import { useRecorderStatus, useRecorderSync } from "../bridge/recorder";
import { useSummarySync } from "../bridge/summary";
import RecorderBanner from "../../modules/recorder/RecorderBanner";
import StartRecordingDialog from "../../modules/recorder/StartRecordingDialog";
import { useStopAndOpen } from "../../modules/recorder/useStopAndOpen";
import styles from "./AppLayout.module.css";
import ContentRoot from "./ContentRoot";
import Sidebar from "./Sidebar";
import Toolbar from "./Toolbar";
import { useHistoryNav } from "./useHistoryNav";
import { useMediaQuery } from "./useMediaQuery";
import { useWorkspaceShortcuts } from "./useWorkspaceShortcuts";

/** A text field has focus (menu shortcuts that leave the page wait). */
function typing(): boolean {
  const el = document.activeElement;
  return el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement || el instanceof HTMLSelectElement || (el instanceof HTMLElement && el.isContentEditable);
}

/** "auto": labels when the window is wide, icons when it's narrow; a hand-picked mode wins. */
type SidebarPref = "auto" | "expanded" | "icons";
const PREF_KEY = "newdawn.sidebar";

// Per-Mac convenience only: storage may be unavailable, so every access is guarded.
function readPref(): SidebarPref {
  try {
    const value = localStorage.getItem(PREF_KEY);
    return value === "expanded" || value === "icons" ? value : "auto";
  } catch {
    return "auto";
  }
}

function savePref(pref: SidebarPref) {
  try {
    localStorage.setItem(PREF_KEY, pref);
  } catch {
    // not remembered; the choice still applies until the window closes
  }
}

/** Golden Gate shape: edge-to-edge sidebar, unified toolbar over the content column. */
export default function AppLayout() {
  const [collapsed, setCollapsed] = useState(false); // hidden entirely (⌥⌘S)
  const [pref, setPref] = useState<SidebarPref>(readPref);
  const narrow = useMediaQuery("(max-width: 999px)");
  const iconsOnly = pref === "auto" ? narrow : pref === "icons";
  const history = useHistoryNav();
  const [switcherOpen, setSwitcherOpen] = useState(false);
  const [startOpen, setStartOpen] = useState(false);
  const [updateVersion, setUpdateVersion] = useState<string | null>(null);
  useUpdateCheckAtLaunch(setUpdateVersion);
  const navigate = useNavigate();
  const { data: recorder } = useRecorderStatus();
  const { stop } = useStopAndOpen();

  function toggleIcons() {
    const next = iconsOnly ? "expanded" : "icons";
    // Choosing what the window width would pick anyway goes back to following it.
    const chosen: SidebarPref = next === (narrow ? "icons" : "expanded") ? "auto" : next;
    setPref(chosen);
    savePref(chosen);
  }

  function toggleRecording() {
    if (!recorder || recorder.phase === "idle") setStartOpen(true);
    else if (recorder.phase !== "stopping") stop();
  }

  useWorkspacesSync();
  useBrandSync();
  useTranscriptionSync();
  useRecorderSync();
  useSummarySync();
  useWorkspaceShortcuts();
  useMenuAction((action) => {
    switch (action) {
      case "toggle_sidebar":
        setCollapsed((was) => !was);
        break;
      case "switch_workspace":
        setCollapsed(false);
        setSwitcherOpen(true);
        break;
      case "toggle_recording":
        toggleRecording();
        break;
      case "icon_sidebar":
        if (collapsed) setCollapsed(false);
        else toggleIcons();
        break;
      // ⌘[ / ⌘] fire even while typing; leaving the page then would drop an unsaved edit.
      case "go_back":
        if (!typing()) history.back();
        break;
      case "go_forward":
        if (!typing()) history.forward();
        break;
      case "import_audio":
        void navigate({ to: "/meetings", search: { import: true } });
        break;
      case "open_tenant_setup":
        void navigate({ to: "/setup", search: { step: "identity" } });
        break;
      case "open_workspace":
        void navigate({ to: "/workspace", search: { tab: "overview" } });
        break;
      case "run_setup":
        // The welcome page has no recorder controls: never leave a running recording behind.
        if (!recorder || recorder.phase === "idle") void navigate({ to: "/welcome", search: { step: "welcome" } });
        break;
    }
  });

  return (
    <div className={`${styles.layout} ${collapsed ? styles.collapsed : iconsOnly ? styles.icons : ""}`}>
      <Sidebar
        hidden={collapsed}
        iconsOnly={iconsOnly && !collapsed}
        onToggleIcons={toggleIcons}
        switcherOpen={switcherOpen}
        onSwitcherOpenChange={setSwitcherOpen}
      />
      <div className={styles.main}>
        <Toolbar sidebarHidden={collapsed} onStartRecording={() => setStartOpen(true)} />
        <ContentRoot>
          {updateVersion && (
            <div className={styles.updateBanner} role="status">
              <span>New Dawn {updateVersion} is available.</span>
              <button type="button" className="btn btnSecondary btnCompact" onClick={() => void openSettings("about")}>
                See what's new
              </button>
              <button type="button" className="btn btnGhost btnCompact" aria-label="Dismiss" onClick={() => setUpdateVersion(null)}>
                Later
              </button>
            </div>
          )}
          <RecorderBanner />
          <Outlet />
        </ContentRoot>
      </div>
      {startOpen && <StartRecordingDialog onClose={() => setStartOpen(false)} />}
    </div>
  );
}
