import { useGlassStatus } from "../../kernel/bridge/glass";
import { openPrivacySettings, useMicrophonePermission, useRequestMicrophone, type PermissionState } from "../../kernel/bridge/recorder";
import { useCurrentWorkspace } from "../../kernel/bridge/workspaces";

const MIC_LABEL: Record<PermissionState, string> = {
  granted: "Allowed",
  denied: "Denied",
  not_determined: "Not asked yet",
  restricted: "Restricted",
};

const GLASS_LABELS = { liquid_glass: "Liquid Glass", vibrancy: "Vibrancy (older macOS)", none: "None" } as const;

/** Settings → General: where you are, what macOS has allowed, how the window is drawn. */
export default function GeneralTab() {
  const { data: workspace } = useCurrentWorkspace();
  const { data: glass } = useGlassStatus();
  const mic = useMicrophonePermission();
  const requestMic = useRequestMicrophone();

  return (
    <>
      <p className="groupTitle">Workspace</p>
      <div className="group">
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">Current workspace</p>
            <p className="rowHint">{workspace?.folder_path ?? "Not yet captured"}</p>
          </div>
          <div className="rowControl">{workspace ? <span className="chip chipBrand">{workspace.name}</span> : <span className="ghostText">Not yet captured</span>}</div>
        </div>
      </div>

      <p className="groupTitle">Permissions</p>
      <div className="group">
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">Microphone</p>
            <p className="rowHint">Your side of the call.</p>
          </div>
          <div className="rowControl">
            {mic.data ? <span className="chip">{MIC_LABEL[mic.data]}</span> : <span className="ghostText">Not yet captured</span>}
            {mic.data === "not_determined" && (
              <button type="button" className="btn btnSecondary btnCompact" onClick={() => requestMic.mutate()}>
                Allow…
              </button>
            )}
            {(mic.data === "denied" || mic.data === "restricted") && (
              <button type="button" className="btn btnGhost btnCompact" onClick={() => openPrivacySettings(false)}>
                Open System Settings
              </button>
            )}
          </div>
        </div>
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">Call audio</p>
            <p className="rowHint">The other side. macOS asks the first time you record; a banner appears if it stays silent.</p>
          </div>
          <div className="rowControl">
            <button type="button" className="btn btnGhost btnCompact" onClick={() => openPrivacySettings(true)}>
              Open System Settings
            </button>
          </div>
        </div>
      </div>

      <p className="groupTitle">Window</p>
      <div className="group">
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">Glass</p>
            <p className="rowHint">Appearance follows macOS: light or dark, accent colour, Reduce Transparency.</p>
          </div>
          <div className="rowControl">{glass ? <span className="chip">{GLASS_LABELS[glass]}</span> : <span className="ghostText">Not yet captured</span>}</div>
        </div>
      </div>
    </>
  );
}
