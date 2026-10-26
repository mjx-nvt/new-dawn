import type { MeetingStatus } from "../../kernel/bridge/meetings";
import { STATUS_LABEL } from "./format";
import styles from "./StatusChip.module.css";

/** Done = green, the app working on it = yellow, failed = red; Recording and Not transcribed stay neutral. */
const TONE: Partial<Record<MeetingStatus, string>> = {
  transcribing: styles.pending,
  transcribed: styles.positive,
  summarising: styles.pending,
  summarised: styles.positive,
  failed: styles.negative,
};

export default function StatusChip({ status }: { status: MeetingStatus }) {
  return <span className={`chip ${TONE[status] ?? ""}`}>{STATUS_LABEL[status]}</span>;
}
