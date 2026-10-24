import { getRouteApi, useNavigate } from "@tanstack/react-router";
import BrandStep from "./BrandStep";
import IdentityStep from "./IdentityStep";
import MeetingTypesStep from "./MeetingTypesStep";
import ReviewStep from "./ReviewStep";
import styles from "./TenantSetupPage.module.css";

const route = getRouteApi("/shell/setup");

export const SETUP_STEPS = [
  { id: "identity", label: "Identity" },
  { id: "brand", label: "Brand" },
  { id: "meeting_types", label: "Meeting types" },
  { id: "review", label: "Review" },
] as const;

export type SetupStep = (typeof SETUP_STEPS)[number]["id"];

const CONTINUE_LABEL: Record<SetupStep, string> = {
  identity: "Create workspace",
  brand: "Continue",
  meeting_types: "Continue",
  review: "Save setup",
};

/** Tenant setup wizard: Identity → Brand → Meeting types → Review. Step is URL state (?step=). */
export default function TenantSetupPage() {
  const { step } = route.useSearch();
  const navigate = useNavigate();
  const index = SETUP_STEPS.findIndex((s) => s.id === step);
  const go = (to: SetupStep) => void navigate({ to: "/setup", search: { step: to } });

  return (
    <div className={`page ${styles.page}`}>
      <p className="eyebrow">Tenant setup</p>
      <h1 className="pageTitle">New workspace</h1>

      <ol className={styles.stepper}>
        {SETUP_STEPS.map((s, i) => {
          const state = i < index ? styles.done : i === index ? styles.current : "";
          return (
            <li key={s.id} className={`${styles.step} ${state}`}>
              <button type="button" disabled={i > index} onClick={() => go(s.id)}>
                <span className={styles.number}>{i < index ? "✓" : i + 1}</span>
                {s.label}
              </button>
            </li>
          );
        })}
      </ol>

      <div className={styles.body}>
        {step === "identity" && <IdentityStep />}
        {step === "brand" && <BrandStep />}
        {step === "meeting_types" && <MeetingTypesStep />}
        {step === "review" && <ReviewStep />}
      </div>

      <footer className={styles.footer}>
        {index > 0 && (
          <button type="button" className="btn btnGhost" onClick={() => go(SETUP_STEPS[index - 1].id)}>
            Back
          </button>
        )}
        <span className={styles.spacer} />
        <button
          type="button"
          className="btn btnPrimary"
          onClick={() => index < SETUP_STEPS.length - 1 && go(SETUP_STEPS[index + 1].id)}
        >
          {CONTINUE_LABEL[step]}
        </button>
      </footer>
    </div>
  );
}
