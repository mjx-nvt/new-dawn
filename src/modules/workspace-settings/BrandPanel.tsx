import { useEffect, useState } from "react";
import { useBrand, useResetBrand, useSetBrandLogo, useUpdateBrand, type BrandPatch } from "../../kernel/bridge/brand";
import DangerButton from "../../kernel/shell/DangerButton";
import { logoUrl } from "../../kernel/theme/brandLogo";
import { convertFileSrc } from "@tauri-apps/api/core";
import styles from "./BrandPanel.module.css";
import DocumentPreview from "./DocumentPreview";

/** Workspace → Brand: the logo, colours, slogan and fonts of one company's documents, saved
 *  to its brand/tokens.json. The app's own look never changes with it (D-033). */
export default function BrandPanel({ slug }: { slug: string }) {
  const brand = useBrand(slug);
  const update = useUpdateBrand(slug);
  const setLogo = useSetBrandLogo(slug);
  const reset = useResetBrand(slug);
  const error = update.error ?? setLogo.error ?? reset.error ?? brand.error;
  const b = brand.data;
  const save = (patch: BrandPatch) => update.mutate(patch);

  return (
    <>
      {b ? (
        <DocumentPreview name={b.name} primary={b.primary} text={b.text} headingFont={b.document_fonts.heading} bodyFont={b.document_fonts.body} slogan={b.slogan.en} logo={b.logo_primary ? logoUrl(b.logo_primary, b.logo_version) : null} />
      ) : (
        <div className={`${styles.preview} ghostCard`} />
      )}
      <p className="fieldHint" style={{ marginTop: "var(--space-3)" }}>
        For documents only — minutes, PDFs and decks carry this brand; the app keeps its own look.
      </p>

      <p className="groupTitle">Logo</p>
      <div className="group">
        <LogoRow label="Logo" hint="SVG or PNG, shown in the sidebar and on documents." path={b?.logo_primary ?? null} onPick={() => setLogo.mutate({ onDark: false })} onClear={() => setLogo.mutate({ onDark: false, clear: true })} />
        <LogoRow label="Logo on dark backgrounds" hint="Optional. Used in dark mode." path={b?.logo_on_dark ?? null} dark onPick={() => setLogo.mutate({ onDark: true })} onClear={() => setLogo.mutate({ onDark: true, clear: true })} />
      </div>

      <p className="groupTitle">Colours</p>
      <div className="group">
        <ColourRow label="Primary" hint="Headings, rules and accents." value={b?.primary ?? ""} onSave={(v) => save({ primary: v })} />
        <ColourRow label="Secondary" hint="Optional." value={b?.secondary ?? ""} optional onSave={(v) => save({ secondary: v })} />
        <ColourRow label="Accent" hint="Optional." value={b?.accent ?? ""} optional onSave={(v) => save({ accent: v })} />
        <ColourRow label="Text" value={b?.text ?? ""} onSave={(v) => save({ text: v })} />
      </div>

      <p className="groupTitle">Words and fonts</p>
      <div className="group">
        <TextRow label="Slogan (English)" value={b?.slogan.en ?? ""} onSave={(v) => save({ slogan_en: v })} />
        <TextRow label="Slogan (Tiếng Việt)" value={b?.slogan.vi ?? ""} onSave={(v) => save({ slogan_vi: v })} />
        <TextRow label="Heading font" hint="Installed on this Mac, or bundled in the brand folder." value={b?.document_fonts.heading ?? ""} onSave={(v) => save({ font_heading: v })} />
        <TextRow label="Body font" value={b?.document_fonts.body ?? ""} onSave={(v) => save({ font_body: v })} />
      </div>

      <div className={styles.footer}>
        <span className="fieldHint">Started from the {b?.starter === "default" ? "neutral" : b?.starter.toUpperCase()} pack · saved in the workspace's brand folder.</span>
        <DangerButton confirm="Replace with the starter pack?" onConfirm={() => reset.mutate()} disabled={reset.isPending}>
          Reset to starter
        </DangerButton>
      </div>
      {error && (
        <p className="formError" role="alert">
          {error.message}
        </p>
      )}
    </>
  );
}

function LogoRow({ label, hint, path, dark, onPick, onClear }: { label: string; hint: string; path: string | null; dark?: boolean; onPick: () => void; onClear: () => void }) {
  return (
    <div className="groupRow">
      <div className="rowText">
        <p className="rowLabel">{label}</p>
        <p className="rowHint">{hint}</p>
      </div>
      <div className="rowControl">
        {path ? (
          <span className={dark ? `${styles.logoBox} ${styles.logoBoxDark}` : styles.logoBox}>
            <LogoImage path={path} />
          </span>
        ) : (
          <span className={`${styles.logoBox} ghostCard`} />
        )}
        <button type="button" className="btn btnSecondary btnCompact" onClick={onPick}>
          Choose…
        </button>
        {path && (
          <button type="button" className="btn btnGhost btnCompact" onClick={onClear}>
            Remove
          </button>
        )}
      </div>
    </div>
  );
}

function LogoImage({ path }: { path: string }) {
  // The asset protocol serves the workspace's brand folder (tauri.conf.json scope).
  return <img src={convertFileSrc(path)} alt="" />;
}

const HEX = /^#[0-9A-F]{6}$/;

/** A hex field with the system colour picker beside it. The field saves when you leave it;
 *  the picker saves as you pick (debounced, the colour panel keeps keyboard focus). */
function ColourRow({ label, hint, value, optional, onSave }: { label: string; hint?: string; value: string; optional?: boolean; onSave: (hex: string) => void }) {
  const [draft, setDraft] = useState(value);
  const [picked, setPicked] = useState<string | null>(null);
  useEffect(() => setDraft(value), [value]);
  useEffect(() => {
    if (picked === null || picked === value) return;
    const t = setTimeout(() => onSave(picked), 300);
    return () => clearTimeout(t);
  }, [picked, value, onSave]);
  const commit = (v: string) => {
    const next = v.trim().toUpperCase();
    if (next === value) return;
    if (HEX.test(next) || (optional && next === "")) onSave(next);
    else setDraft(value);
  };
  return (
    <div className="groupRow">
      <div className="rowText">
        <p className="rowLabel">{label}</p>
        {hint && <p className="rowHint">{hint}</p>}
      </div>
      <div className="rowControl">
        <input
          type="color"
          className={styles.swatch}
          aria-label={`${label} colour`}
          value={HEX.test(draft) ? draft : "#888888"}
          onChange={(e) => {
            setDraft(e.target.value.toUpperCase());
            setPicked(e.target.value.toUpperCase());
          }}
        />
        <input
          className={`input ${styles.hex}`}
          value={draft}
          placeholder={optional ? "None" : "#000000"}
          spellCheck={false}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={(e) => commit(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && e.currentTarget.blur()}
        />
      </div>
    </div>
  );
}

function TextRow({ label, hint, value, onSave }: { label: string; hint?: string; value: string; onSave: (v: string) => void }) {
  const [draft, setDraft] = useState(value);
  useEffect(() => setDraft(value), [value]);
  return (
    <div className="groupRow">
      <div className="rowText">
        <p className="rowLabel">{label}</p>
        {hint && <p className="rowHint">{hint}</p>}
      </div>
      <div className="rowControl">
        <input
          className={`input ${styles.text}`}
          value={draft}
          maxLength={200}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={() => draft.trim() !== value && onSave(draft)}
          onKeyDown={(e) => e.key === "Enter" && !e.nativeEvent.isComposing && e.currentTarget.blur()}
        />
      </div>
    </div>
  );
}
