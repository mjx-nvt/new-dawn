import { getRouteApi, useNavigate } from "@tanstack/react-router";
import { useRef, type ReactNode } from "react";
import BrandStep from "./BrandStep";
import IdentityStep from "./IdentityStep";
import TemplateStep from "./TemplateStep";
import styles from "./TenantSetupPage.module.css";

const route = getRouteApi("/shell/setup");

export const SETUP_STEPS = [
  { id: "identity", label: "Workspace" },
  { id: "brand", label: "Brand" },
  { id: "template", label: "Minutes template" },
] as const;

export type SetupStep = (typeof SETUP_STEPS)[number]["id"];

/** `slug`: the workspace being set up, once the first step has made it. */
export type SetupSearch = { step: SetupStep; slug?: string };

/** A step's primary action, in the footer. */
export type StepAction = { label: string; run: () => void; pending: boolean; disabled?: boolean; error: Error | null };

/** Lays out a step: its content, then the footer with Back and the step's action. */
export type StepFrame = (content: ReactNode, action: StepAction) => ReactNode;

/** New workspace: Workspace → Brand → Minutes template. Each step's action saves what it shows;
 *  the step and the workspace being set up are URL state (?step=&slug=). */
export default function TenantSetupPage() {
  const { step, slug } = route.useSearch();
  const navigate = useNavigate();
  const index = SETUP_STEPS.findIndex((s) => s.id === step);
  // Without a slug the workspace isn't made yet: that entry is replaced, so going back in history
  // never shows an empty first step for a workspace that exists.
  const go = (to: SetupStep, target = slug) => void navigate({ to: "/setup", search: { step: to, slug: target }, replace: !slug });
  const madeFor = useRef<string | null>(null); // see BrandStep

  const frame: StepFrame = (content, action) => (
    <>
      <div className={styles.body}>{content}</div>
      <footer className={styles.footer}>
        {index > 0 && (
          <button type="button" className="btn btnGhost" onClick={() => go(SETUP_STEPS[index - 1].id)}>
            Back
          </button>
        )}
        <span className={styles.spacer} />
        {action.error && (
          <p className="formError" role="alert">
            {action.error.message}
          </p>
        )}
        <button type="button" className="btn btnPrimary" disabled={action.pending || action.disabled} onClick={action.run}>
          {action.label}
        </button>
      </footer>
    </>
  );

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

      {/* Keyed so "New workspace…" from a finished first step starts empty. */}
      {step === "identity" && <IdentityStep key={slug ?? ""} slug={slug} frame={frame} onDone={(made) => go("brand", made)} />}
      {step === "brand" && slug && <BrandStep slug={slug} madeFor={madeFor} frame={frame} onDone={() => go("template")} />}
      {step === "template" && slug && <TemplateStep slug={slug} frame={frame} onDone={() => void navigate({ to: "/meetings", search: {} })} />}
    </div>
  );
}
