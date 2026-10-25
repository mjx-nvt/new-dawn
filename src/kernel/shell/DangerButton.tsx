import { useEffect, useState } from "react";

/** A destructive action in two clicks: red text, then (for 4 s) a red fill that asks
 *  `confirm`. A click doesn't focus buttons in WebKit, so the arm times out instead of
 *  disarming on blur. */
export default function DangerButton({
  children,
  confirm,
  onConfirm,
  disabled,
  compact = true,
  title,
}: {
  children: string;
  confirm: string;
  onConfirm: () => void;
  disabled?: boolean;
  compact?: boolean;
  title?: string;
}) {
  const [armed, setArmed] = useState(false);
  useEffect(() => {
    if (!armed) return;
    const t = setTimeout(() => setArmed(false), 4000);
    return () => clearTimeout(t);
  }, [armed]);
  return (
    <button
      type="button"
      className={`btn ${compact ? "btnCompact " : ""}${armed ? "btnDangerFilled" : "btnDanger"}`}
      disabled={disabled}
      title={title}
      onClick={() => {
        if (armed) {
          setArmed(false);
          onConfirm();
        } else setArmed(true);
      }}
    >
      {armed ? confirm : children}
    </button>
  );
}
