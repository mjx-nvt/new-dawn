import type { MeetingStatus } from "../../kernel/bridge/meetings";
import { STATUS_LABEL } from "./format";
import styles from "./StatusChip.module.css";

export default function StatusChip({ status }: { status: MeetingStatus }) {
  const tone = status === "failed" ? styles.negative : status === "transcribed" || status === "summarised" ? styles.positive : "";
  return <span className={`chip ${tone}`}>{STATUS_LABEL[status]}</span>;
}
