import { useRef, type RefObject } from "react";
import { useBrand, useSetBrandLogo, useUpdateBrand, type BrandPatch } from "../../kernel/bridge/brand";
import { useGenerateTemplate, useRedrawTemplatePreview } from "../../kernel/bridge/templates";
import { logoUrl } from "../../kernel/theme/brandLogo";
import { ColourRow, SloganRows } from "../workspace-settings/BrandPanel";
import styles from "./BrandStep.module.css";
import type { StepFrame } from "./TenantSetupPage";

/** Step 2: the logo, slogan and primary colour — all a document needs. Each saves as you go;
 *  Continue makes the minutes template from them. `madeFor`: the workspace whose template this
 *  step last made — it's made again only after the logo or colour changed, so a look back keeps
 *  edits made in Word. */
export default function BrandStep({ slug, madeFor, frame, onDone }: { slug: string; madeFor: RefObject<string | null>; frame: StepFrame; onDone: () => void }) {
  const brand = useBrand(slug);
  const update = useUpdateBrand(slug);
  const setLogo = useSetBrandLogo(slug);
  const generate = useGenerateTemplate(slug);
  const redraw = useRedrawTemplatePreview(slug);
  const b = brand.data;
  // A field saves when you leave it — that is, as you click Continue: the template waits for it.
  // The preview shows the slogans, so it's drawn again once a save lands (the translation's can
  // land after you've moved on).
  const saving = useRef<Promise<unknown>>(Promise.resolve());
  const save = (patch: BrandPatch) => {
    saving.current = update.mutateAsync(patch).then(redraw, () => undefined); // errors shown from update.error
  };
  const changed = () => (madeFor.current = null); // the logo and colour are in the template file
  const run = () =>
    void saving.current.then(() =>
      madeFor.current === slug
        ? onDone()
        : generate.mutate(undefined, {
            onSuccess: () => {
              madeFor.current = slug;
              onDone();
            },
          }),
    );

  return frame(
    <>
      <p className="fieldHint">For documents only — your minutes carry this brand; the app keeps its own look.</p>

      <p className="groupTitle">Logo</p>
      <div className="group">
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">Logo</p>
            <p className="rowHint">SVG or PNG. We make the white version for dark backgrounds.</p>
          </div>
          <div className="rowControl">
            <button type="button" className="btn btnSecondary btnCompact" disabled={!b || setLogo.isPending} onClick={() => setLogo.mutate({ onDark: false }, { onSuccess: changed })}>
              Choose logo…
            </button>
          </div>
        </div>
        <div className={`groupRow ${styles.tiles}`}>
          {b?.logo_primary ? (
            <span className={`${styles.tile} ${styles.paper}`}>
              <img src={logoUrl(b.logo_primary, b.logo_version)} alt="Your logo" />
            </span>
          ) : (
            <span className={`${styles.tile} ghostCard`} />
          )}
          {b?.logo_on_dark ? (
            <span className={styles.tile} style={{ background: b.primary }}>
              <img src={logoUrl(b.logo_on_dark, b.logo_version)} alt="The white logo, on your colour" />
            </span>
          ) : (
            <span className={`${styles.tile} ghostCard`} />
          )}
        </div>
      </div>

      <p className="groupTitle">Slogan</p>
      <div className="group">
        <SloganRows slug={slug} brand={b} autoTranslate onSave={save} />
      </div>

      <p className="groupTitle">Colour</p>
      <div className="group">
        <ColourRow
          label="Primary"
          hint="Headings, rules and accents."
          value={b?.primary ?? ""}
          onSave={(v) => {
            changed();
            save({ primary: v });
          }}
        />
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">Shades</p>
            <p className="rowHint">Made from your colour.</p>
          </div>
          <div className={`rowControl ${styles.ramp}`}>
            {b?.ramp.map((hex, i) => <span key={i} className={styles.swatch} style={{ background: hex }} title={hex} />)}
          </div>
        </div>
      </div>
    </>,
    {
      label: generate.isPending ? "Making the template…" : "Continue",
      run,
      pending: generate.isPending,
      disabled: !b || setLogo.isPending,
      error: generate.error ?? setLogo.error ?? update.error ?? brand.error,
    },
  );
}
