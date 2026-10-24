import { Outlet, useNavigate } from "@tanstack/react-router";
import { useState } from "react";
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
import { useWorkspaceShortcuts } from "./useWorkspaceShortcuts";

/** Golden Gate shape: edge-to-edge sidebar, unified toolbar over the content column. */
export default function AppLayout() {
  const [collapsed, setCollapsed] = useState(false);
  const [switcherOpen, setSwitcherOpen] = useState(false);
  const [startOpen, setStartOpen] = useState(false);
  const navigate = useNavigate();
  const { data: recorder } = useRecorderStatus();
  const { stop } = useStopAndOpen();

  useWorkspacesSync();
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
        if (!recorder || recorder.phase === "idle") setStartOpen(true);
        else if (recorder.phase !== "stopping") stop();
        break;
      case "import_audio":
        void navigate({ to: "/meetings", search: { import: true } });
        break;
      case "open_tenant_setup":
        void navigate({ to: "/setup", search: { step: "identity" } });
        break;
      case "run_setup":
        // The welcome page has no recorder controls: never leave a running recording behind.
        if (!recorder || recorder.phase === "idle") void navigate({ to: "/welcome", search: { step: "welcome" } });
        break;
    }
  });

  return (
    <div className={collapsed ? `${styles.layout} ${styles.collapsed}` : styles.layout}>
      <Sidebar hidden={collapsed} switcherOpen={switcherOpen} onSwitcherOpenChange={setSwitcherOpen} />
      <div className={styles.main}>
        <Toolbar sidebarHidden={collapsed} onStartRecording={() => setStartOpen(true)} />
        <ContentRoot>
          <RecorderBanner />
          <Outlet />
        </ContentRoot>
      </div>
      {startOpen && <StartRecordingDialog onClose={() => setStartOpen(false)} />}
    </div>
  );
}
