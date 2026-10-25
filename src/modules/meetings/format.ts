import type { Language, MeetingStatus, Stage } from "../../kernel/bridge/meetings";

/** 754_000 → "12:34"; 3_723_000 → "1:02:03". */
export function formatClock(ms: number): string {
  const total = Math.floor(ms / 1000);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const pad = (n: number) => String(n).padStart(2, "0");
  return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`;
}

/** ISO-8601 UTC → "24 Oct 2026, 14:05" in the user's time zone. */
export function formatWhen(iso: string): string {
  return new Date(iso).toLocaleString("en-GB", { day: "numeric", month: "short", year: "numeric", hour: "2-digit", minute: "2-digit" });
}

/** "Nguyễn Văn Tiến" → "nguyen van tien": case, accents and đ don't matter. */
export function foldName(name: string): string {
  return name.normalize("NFD").replace(/[\u0300-\u036f]/g, "").replace(/đ/gi, "d").toLowerCase().trim();
}

export const LANGUAGE_LABEL: Record<Language, string> = { en: "English", vi: "Tiếng Việt", auto: "Auto-detect" };

export const STATUS_LABEL: Record<MeetingStatus, string> = {
  recording: "Recording",
  recorded: "Not transcribed",
  transcribing: "Transcribing",
  transcribed: "Transcribed",
  summarising: "Summarising",
  summarised: "Summarised",
  failed: "Failed",
};

export const STAGE_LABEL: Record<Stage, string> = {
  queued: "Waiting for the previous job",
  decoding: "Reading the audio",
  detecting_speech: "Finding speech",
  transcribing: "Transcribing",
  saving: "Saving",
};
