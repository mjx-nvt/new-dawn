import type { MeetingStatus } from "../../kernel/bridge/meetings";
import { STATUS_LABEL } from "./format";

/** Done = positive, the app working on it = warning, failed = negative; Recording and Not
 *  transcribed stay neutral. The one chip set (D-090). */
const TONE: Partial<Record<MeetingStatus, string>> = {
  transcribing: "chipWarning",
  transcribed: "chipPositive",
  summarising: "chipWarning",
  summarised: "chipPositive",
  failed: "chipNegative",
};

export default function StatusChip({ status }: { status: MeetingStatus }) {
  return <span className={`chip ${TONE[status] ?? ""}`}>{STATUS_LABEL[status]}</span>;
}
