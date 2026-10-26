/** The Live page's pure parts (0.7.5): the words for a session's languages and format, how its
 *  lines sit on the page — who starts speaking, a presenter's paragraphs — Copy all, and the
 *  choices remembered on this Mac. `scripts/check-live.ts` runs them (hence the `.ts` import). */
import type { Clarity, Language, LiveDirection, MeetingFormat, Speaker } from "../../bindings";
import { formatClock } from "../meetings/format.ts";

export const DIRECTION_LABEL: Record<LiveDirection, string> = {
  vi_en: "Tiếng Việt → English",
  en_vi: "English → Tiếng Việt",
  both: "Both ways",
};

/** A saved live meeting's direction: its language says it (both ways is auto-detect). */
export function directionOf(language: Language): LiveDirection {
  return language === "en" ? "en_vi" : language === "auto" ? "both" : "vi_en";
}

/** The two column heads: what was said | its translation. */
export function columnTitles(direction: LiveDirection): [string, string] {
  if (direction === "both") return ["Said", "Translation"];
  return direction === "en_vi" ? ["English", "Tiếng Việt"] : ["Tiếng Việt", "English"];
}

/** "Discussion", or "One presenter · Minh". */
export function formatLabel(format: MeetingFormat, presenter: string | null): string {
  if (format === "discussion") return "Discussion";
  return presenter ? `One presenter · ${presenter}` : "One presenter";
}

/** What the layout and Copy all read of a line (a saved one or a quick translation's). */
interface Line {
  start_ms: number;
  end_ms: number;
  text: string;
  translation?: string | null;
  speaker?: Speaker | null;
  clarity?: Clarity | null;
}

export type SpeakerLabel = "You" | "Them";

/** A presenter's line that starts this soon after the last one ended carries on its paragraph. */
const PARAGRAPH_GAP_MS = 1_500;

/** How a line sits on the page: `label` — who starts speaking at it (at each change of speaker;
 *  only a discussion's lines have speakers); `continues` — it carries on a presenter's paragraph
 *  (no gap above, no time). */
export interface LineLayout {
  label: SpeakerLabel | null;
  continues: boolean;
}

export function layoutLines(lines: Line[], format: MeetingFormat): LineLayout[] {
  return lines.map((line, i) => {
    const prev = i > 0 ? lines[i - 1] : null;
    const speaker = line.speaker ?? null;
    const label = speaker && speaker !== (prev?.speaker ?? null) ? (speaker === "you" ? "You" : "Them") : null;
    const continues = format === "presentation" && !!prev && !label && line.start_ms - prev.end_ms < PARAGRAPH_GAP_MS;
    return { label, continues };
  });
}

/** The last line translated so far (-1 before the first). Lines are translated in the order they
 *  arrived (`seq`), not always the order they're read in: an owner's line that waited for the
 *  call's reply reads before it but comes after it. */
export function lastTranslated(lines: { seq: number; translation?: string | null }[]): number {
  return lines.reduce((last, line) => (line.translation ? Math.max(last, line.seq) : last), -1);
}

/** Copy all, as plain text: a block per line (per paragraph, for a presenter) — its time and who
 *  starts speaking, what was said, then the translation (left out when there is none at all). */
export function copyText(lines: Line[], format: MeetingFormat): string {
  const layout = layoutLines(lines, format);
  const translated = lines.some((line) => line.translation);
  const blocks: { head: string; said: string[]; translation: string[] }[] = [];
  lines.forEach((line, i) => {
    const { label, continues } = layout[i];
    if (!continues) blocks.push({ head: label ? `${formatClock(line.start_ms)} · ${label}` : formatClock(line.start_ms), said: [], translation: [] });
    const block = blocks[blocks.length - 1];
    block.said.push(line.clarity === "unclear" ? `${line.text} [unclear]` : line.text);
    block.translation.push(!line.translation ? "[not translated]" : line.clarity === "uncertain" ? `${line.translation} [may be inaccurate]` : line.translation);
  });
  return blocks.map((b) => [b.head, b.said.join(" "), ...(translated ? [b.translation.join(" ")] : [])].join("\n")).join("\n\n");
}

/** A choice remembered on this Mac, else the first of `options`. Storage may be unavailable, so
 *  every access is guarded: it's a convenience, never state that must survive. */
export function remembered<T extends string>(key: string, options: readonly T[]): T {
  try {
    const saved = localStorage.getItem(key);
    return options.find((option) => option === saved) ?? options[0];
  } catch {
    return options[0];
  }
}

export function remember(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    // not remembered; the choice still applies this time
  }
}
