import { useState } from "react";
import { useAppInfo } from "../../kernel/bridge/app";
import { autoCheckEnabled, setAutoCheck, useUpdater } from "../../kernel/bridge/updater";

export default function AboutTab() {
  const { data } = useAppInfo();
  const updater = useUpdater();
  const [auto, setAuto] = useState(autoCheckEnabled);

  const status = (() => {
    switch (updater.phase) {
      case "checking":
        return "Checking…";
      case "none":
        return "You're up to date.";
      case "available":
        return `Version ${updater.version} is available.`;
      case "downloading":
        return `Downloading… ${updater.percent ?? 0} %`;
      case "ready":
        return "Installed. Restarting…";
      case "error":
        return updater.error ?? "The check failed.";
      default:
        return "Updates come from the New Dawn releases page; nothing else leaves this Mac.";
    }
  })();

  return (
    <>
      <div className="group">
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">New Dawn {data?.version ?? ""}</p>
            <p className="rowHint">{status}</p>
            {updater.phase === "available" && updater.notes && <p className="rowHint">{updater.notes}</p>}
          </div>
          <div className="rowControl">
            {updater.phase === "available" ? (
              <button type="button" className="btn btnPrimary btnCompact" onClick={() => void updater.install()}>
                Install and restart
              </button>
            ) : (
              <button type="button" className="btn btnSecondary btnCompact" disabled={updater.phase === "checking" || updater.phase === "downloading"} onClick={() => void updater.checkNow()}>
                Check for updates
              </button>
            )}
          </div>
        </div>
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">Check automatically</p>
            <p className="rowHint">Shortly after launch and once a day while the app runs. You still choose when to install.</p>
          </div>
          <div className="rowControl">
            <div className="segmented" role="group" aria-label="Check automatically">
              {(
                [
                  [true, "On"],
                  [false, "Off"],
                ] as const
              ).map(([value, label]) => (
                <button
                  key={String(value)}
                  type="button"
                  aria-pressed={auto === value}
                  onClick={() => {
                    setAuto(value);
                    setAutoCheck(value);
                  }}
                >
                  {label}
                </button>
              ))}
            </div>
          </div>
        </div>
        <div className="groupRow">
          <p className="rowLabel">Bundle identifier</p>
          <div className="rowControl">{data?.identifier ? <span className="fieldValue">{data.identifier}</span> : <span className="ghostText">Not yet captured</span>}</div>
        </div>
      </div>
      <p className="fieldHint" style={{ marginTop: "var(--space-3)" }}>
        Everything stays on this Mac: speech and text AI run locally.
      </p>
    </>
  );
}
