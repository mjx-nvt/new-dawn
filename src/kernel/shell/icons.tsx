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

/** Material Symbols "auto_awesome" (Apache-2.0), the AI marker. Filled, so it reads at 14 px. */
export const SparkleIcon = ({ size = 14 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" aria-hidden>
    <path d="M19 9l1.25-2.75L23 5l-2.75-1.25L19 1l-1.25 2.75L15 5l2.75 1.25L19 9zm-7.5.5L9 4 6.5 9.5 1 12l5.5 2.5L9 20l2.5-5.5L17 12l-5.5-2.5zM19 15l-1.25 2.75L15 19l2.75 1.25L19 23l1.25-2.75L23 19l-2.75-1.25L19 15z" />
  </svg>
);

/** Material Symbols for the player (Apache-2.0): play_arrow, pause, replay (+ its mirror). */
const Material = ({ d, flip = false, size = 18 }: { d: string; flip?: boolean; size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" aria-hidden>
    <path d={d} transform={flip ? "matrix(-1 0 0 1 24 0)" : undefined} />
  </svg>
);
export const PlayIcon = () => <Material d="M8 6.82v10.36c0 .79.87 1.27 1.54.84l8.14-5.18a1 1 0 0 0 0-1.69L9.54 5.98A.998.998 0 0 0 8 6.82z" />;
export const PauseIcon = () => <Material d="M8 19c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2s-2 .9-2 2v10c0 1.1.9 2 2 2zm6-12v10c0 1.1.9 2 2 2s2-.9 2-2V7c0-1.1-.9-2-2-2s-2 .9-2 2z" />;
const REPLAY = "M12 5V2.21c0-.45-.54-.67-.85-.35l-3.8 3.79c-.2.2-.2.51 0 .71l3.79 3.79c.32.31.86.09.86-.36V7c3.73 0 6.68 3.42 5.86 7.29-.47 2.27-2.31 4.1-4.57 4.57-3.57.75-6.75-1.7-7.23-5.01-.07-.48-.49-.85-.98-.85-.6 0-1.08.53-1 1.13.62 4.39 4.8 7.64 9.53 6.72 3.12-.61 5.63-3.12 6.24-6.24C20.84 9.48 16.94 5 12 5z";
export const SkipBackIcon = () => <Material d={REPLAY} size={16} />;
export const SkipForwardIcon = () => <Material d={REPLAY} size={16} flip />;

export const MeetingsIcon = () => (
  <Icon>
    <rect x="2" y="3" width="12" height="11" rx="2" />
    <path d="M2 6.5h12M5.5 1.5v3M10.5 1.5v3" />
  </Icon>
);

/** A speech bubble with lines: captions as people speak (Live, D-041). */
export const LiveIcon = () => (
  <Icon>
    <path d="M3 2.5h10A1.5 1.5 0 0 1 14.5 4v6a1.5 1.5 0 0 1-1.5 1.5H8.5L5 14v-2.5H3A1.5 1.5 0 0 1 1.5 10V4A1.5 1.5 0 0 1 3 2.5z" />
    <path d="M4.5 5.5h7M4.5 8.5h4.5" />
  </Icon>
);

/** A checklist: two ticks, two lines (Tasks, D-042). */
export const TasksIcon = () => (
  <Icon>
    <path d="M2 5l1.5 1.5 3-3M2 11l1.5 1.5 3-3M9 5h5M9 11h5" />
  </Icon>
);

/** A tick ("In Tasks"). */
export const CheckIcon = () => (
  <Icon>
    <path d="M3 8.5 6.5 12 13 4.5" />
  </Icon>
);

export const MailIcon = () => (
  <Icon>
    <rect x="1.5" y="3" width="13" height="10" rx="1.5" />
    <path d="M2 4l6 4.5L14 4" />
  </Icon>
);

export const PeopleIcon = () => (
  <Icon>
    <circle cx="6" cy="5.5" r="2.5" />
    <path d="M1.5 13.5c.6-2.3 2.3-3.5 4.5-3.5s3.9 1.2 4.5 3.5M10.5 3.2a2.5 2.5 0 0 1 0 4.6M12 10.3c1.2.5 2 1.6 2.5 3.2" />
  </Icon>
);

export const ProjectsIcon = () => (
  <Icon>
    <path d="M1.5 4.5a1 1 0 0 1 1-1h3.6l1.4 1.5h6a1 1 0 0 1 1 1v6.5a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1z" />
    <path d="M1.5 7h13" />
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
