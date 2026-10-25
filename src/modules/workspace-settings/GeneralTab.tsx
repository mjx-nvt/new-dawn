import { useEffect, useState } from "react";
import { useGlassStatus } from "../../kernel/bridge/glass";
import { useProfile, useSetProfile } from "../../kernel/bridge/profile";
import { openPrivacySettings, useMicrophonePermission, useRequestMicrophone, type PermissionState } from "../../kernel/bridge/recorder";
import { useCurrentWorkspace } from "../../kernel/bridge/workspaces";
import styles from "./Settings.module.css";

const MIC_LABEL: Record<PermissionState, string> = {
  granted: "Allowed",
  denied: "Denied",
  not_determined: "Not asked yet",
  restricted: "Restricted",
};

const GLASS_LABELS = { liquid_glass: "Liquid Glass", vibrancy: "Vibrancy (older macOS)", none: "None" } as const;

/** Settings → General: who you are, who attends your meetings, where you are, what macOS has
 *  allowed, how the window is drawn. */
export default function GeneralTab() {
  const { data: workspace } = useCurrentWorkspace();
  const { data: glass } = useGlassStatus();
  const mic = useMicrophonePermission();
  const requestMic = useRequestMicrophone();
  const { data: profile } = useProfile();
  const setProfile = useSetProfile();
  const name = profile?.name ?? "";
  const [draft, setDraft] = useState(name);
  useEffect(() => setDraft(name), [name]);

  return (
    <>
      <p className="groupTitle">Profile</p>
      <div className="group">
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">Your name</p>
            <p className="rowHint">The minute-taker in your minutes and the name in your email sign-off. Each workspace adds your title.</p>
          </div>
          <div className="rowControl">
            <input
              className={`input ${styles.rowInput}`}
              value={draft}
              maxLength={120}
              placeholder="Not yet captured"
              aria-label="Your name"
              disabled={!profile}
              onChange={(e) => setDraft(e.target.value)}
              onBlur={() => draft.trim() !== name && setProfile.mutate({ name: draft.trim() })}
              onKeyDown={(e) => e.key === "Enter" && !e.nativeEvent.isComposing && e.currentTarget.blur()}
            />
          </div>
        </div>
      </div>

      <p className="groupTitle">Meetings</p>
      <div className="group">
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">Everyone involved attends</p>
            <p className="rowHint">On: new summaries put everyone the meeting involves in the attendees, and you remove who wasn't there. Off: only you and the presenter — you add the rest.</p>
          </div>
          <div className="rowControl">
            <div className="segmented" role="group" aria-label="Everyone involved attends">
              {(
                [
                  [true, "On"],
                  [false, "Off"],
                ] as const
              ).map(([value, label]) => (
                <button key={label} type="button" aria-pressed={profile?.everyone_attends === value} disabled={!profile || setProfile.isPending} onClick={() => setProfile.mutate({ everyone_attends: value })}>
                  {label}
                </button>
              ))}
            </div>
          </div>
        </div>
      </div>
      {setProfile.error && (
        <p className="formError" role="alert">
          {setProfile.error.message}
        </p>
      )}

      <p className="groupTitle">Workspace</p>
      <div className="group">
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">Current workspace</p>
            <p className="rowHint">{workspace?.folder_path ?? "Not yet captured"}</p>
          </div>
          <div className="rowControl">{workspace ? <span className="chip chipPrimary">{workspace.name}</span> : <span className="ghostText">Not yet captured</span>}</div>
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
            <p className="rowHint">The other side. macOS asks the first time you record.</p>
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
