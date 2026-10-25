import type { SVGProps } from "react";

/** The shell's few icons, drawn inline (no icon library): 16 pt, 1.6 stroke, currentColor. */
function Icon({ children, ...props }: SVGProps<SVGSVGElement>) {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
      {...props}
    >
      {children}
    </svg>
  );
}

export const MeetingsIcon = () => (
  <Icon>
    <rect x="2" y="3" width="12" height="11" rx="2" />
    <path d="M2 6.5h12M5.5 1.5v3M10.5 1.5v3" />
  </Icon>
);

export const RecordIcon = () => (
  <Icon>
    <circle cx="8" cy="8" r="6" />
    <circle cx="8" cy="8" r="2.6" fill="currentColor" stroke="none" />
  </Icon>
);

export const WorkspaceIcon = () => (
  <Icon>
    <path d="M2.5 14V3.5a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1V14M10.5 7h2a1 1 0 0 1 1 1v6M2 14h12M5 5.5h2M5 8.5h2M5 11.5h2" />
  </Icon>
);

export const SettingsIcon = () => (
  <Icon>
    <circle cx="8" cy="8" r="2.2" />
    <path d="M8 1.5v1.8M8 12.7v1.8M1.5 8h1.8M12.7 8h1.8M3.4 3.4l1.3 1.3M11.3 11.3l1.3 1.3M3.4 12.6l1.3-1.3M11.3 4.7l1.3-1.3" />
  </Icon>
);

export const CodeIcon = () => (
  <Icon>
    <path d="M5.5 4.5 2 8l3.5 3.5M10.5 4.5 14 8l-3.5 3.5" />
  </Icon>
);

export const BackIcon = () => (
  <Icon>
    <path d="M10 3 5 8l5 5" />
  </Icon>
);

export const ForwardIcon = () => (
  <Icon>
    <path d="m6 3 5 5-5 5" />
  </Icon>
);

/** ⟨⟨ collapse to icons / ⟩⟩ expand (flip with `expand`). */
export const CollapseIcon = ({ expand = false }: { expand?: boolean }) => (
  <Icon style={expand ? { transform: "scaleX(-1)" } : undefined}>
    <path d="M8 3 3 8l5 5M13 3 8 8l5 5" />
  </Icon>
);
