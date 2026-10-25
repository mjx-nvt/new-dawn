import { useEffect, useState } from "react";
import { useBrand, useResetBrand, useSetBrandLogo, useUpdateBrand, type BrandPatch, type BrandView } from "../../kernel/bridge/brand";
import DangerButton from "../../kernel/shell/DangerButton";
import { useMediaQuery } from "../../kernel/shell/useMediaQuery";
import { brandProps, useBrandLogo } from "../../kernel/theme/applyBrand";
import { convertFileSrc } from "@tauri-apps/api/core";
import styles from "./BrandPanel.module.css";

const RADIUS = [
  ["sharp", "Sharp"],
  ["soft", "Soft"],
  ["round", "Round"],
] as const;
const SHAPES = [
  ["square", "Square"],
  ["rounded", "Rounded"],
  ["pill", "Pill"],
] as const;
const PRIMARY = ["filled", "soft", "outline"] as const;
const SECONDARY = ["soft", "outline", "ghost"] as const;

/** Settings → Brand: the colours, logo, corners and document details of one workspace,
 *  saved to its brand/tokens.json and applied to the content area straight away. */
export default function BrandPanel({ slug }: { slug: string }) {
  const brand = useBrand(slug);
  const update = useUpdateBrand(slug);
  const setLogo = useSetBrandLogo(slug);
  const reset = useResetBrand(slug);
  const dark = useMediaQuery("(prefers-color-scheme: dark)");
  const error = update.error ?? setLogo.error ?? reset.error ?? brand.error;
  const b = brand.data;
  const save = (patch: BrandPatch) => update.mutate(patch);

  return (
    <>
      {b ? <Preview brand={b} dark={dark} slug={slug} /> : <div className={`${styles.preview} ghostCard`} />}

      <p className="groupTitle">Logo</p>
      <div className="group">
        <LogoRow label="Logo" hint="SVG or PNG, shown in the sidebar and on documents." path={b?.logo_primary ?? null} onPick={() => setLogo.mutate({ onDark: false })} onClear={() => setLogo.mutate({ onDark: false, clear: true })} />
        <LogoRow label="Logo on dark backgrounds" hint="Optional. Used in dark mode." path={b?.logo_on_dark ?? null} dark onPick={() => setLogo.mutate({ onDark: true })} onClear={() => setLogo.mutate({ onDark: true, clear: true })} />
      </div>

      <p className="groupTitle">Colours</p>
      <div className="group">
        <ColourRow label="Primary" hint="Buttons, chips, section eyebrows. Changing it rebuilds the tints and shades." value={b?.primary ?? ""} onSave={(v) => save({ primary: v })} />
        <ColourRow label="Secondary" hint="Optional." value={b?.secondary ?? ""} optional onSave={(v) => save({ secondary: v })} />
        <ColourRow label="Accent" hint="Optional." value={b?.accent ?? ""} optional onSave={(v) => save({ accent: v })} />
        <ColourRow label="Canvas" hint="The reading surface (light mode)." value={b?.canvas ?? ""} onSave={(v) => save({ canvas: v })} />
      </div>

      <p className="groupTitle">Status colours</p>
      <div className="group">
        <ColourRow label="Positive" value={b?.status.positive ?? ""} onSave={(v) => save({ positive: v })} />
        <ColourRow label="Warning" value={b?.status.warning ?? ""} onSave={(v) => save({ warning: v })} />
        <ColourRow label="Negative" value={b?.status.negative ?? ""} onSave={(v) => save({ negative: v })} />
      </div>

      <p className="groupTitle">Shape</p>
      <div className="group">
        <Choice label="Corners" value={b?.radius.preset ?? "soft"} options={RADIUS} onChange={(v) => save({ radius_preset: v })} />
        <Choice label="Buttons" value={b?.button.shape ?? "rounded"} options={SHAPES} onChange={(v) => save({ button_shape: v })} />
        <Choice label="Primary button" value={b?.button.primary ?? "filled"} options={PRIMARY.map((v) => [v, v[0].toUpperCase() + v.slice(1)] as const)} onChange={(v) => save({ button_primary: v })} />
        <Choice label="Secondary button" value={b?.button.secondary ?? "soft"} options={SECONDARY.map((v) => [v, v[0].toUpperCase() + v.slice(1)] as const)} onChange={(v) => save({ button_secondary: v })} />
      </div>

      <p className="groupTitle">Documents</p>
      <div className="group">
        <ColourRow label="Text" hint="Exported documents only; the app keeps its own text colour." value={b?.text ?? ""} onSave={(v) => save({ text: v })} />
        <TextRow label="Slogan (English)" value={b?.slogan.en ?? ""} onSave={(v) => save({ slogan_en: v })} />
        <TextRow label="Slogan (Tiếng Việt)" value={b?.slogan.vi ?? ""} onSave={(v) => save({ slogan_vi: v })} />
        <TextRow label="Heading font" hint="Exported documents only; the app keeps Be Vietnam Pro." value={b?.document_fonts.heading ?? ""} onSave={(v) => save({ font_heading: v })} />
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

/** A small content panel in the workspace's brand, so a change is seen before it's trusted. */
function Preview({ brand, dark, slug }: { brand: BrandView; dark: boolean; slug: string }) {
  const logo = useBrandLogo(slug);
  return (
    <div className={styles.preview} {...brandProps(brand, dark)}>
      <div className={styles.previewHead}>
        {logo ? <img className={styles.previewLogo} src={logo} alt="" /> : <span className={styles.previewMark}>{brand.name.charAt(0)}</span>}
        <span className="eyebrow">{brand.name}</span>
      </div>
      <p className={styles.previewTitle}>Quarterly partner review</p>
      <p className={styles.previewRow}>
        <span className="chip chipBrand">Summarised</span>
        <span className="chip">Weekly meeting</span>
      </p>
      <p className={styles.previewRow}>
        <span className="btn btnPrimary btnCompact">Export minutes</span>
        <span className="btn btnSecondary btnCompact">Copy as email</span>
        <span className="btn btnGhost btnCompact">Re-run</span>
      </p>
    </div>
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

function Choice<V extends string>({ label, value, options, onChange }: { label: string; value: V | string; options: readonly (readonly [V, string])[]; onChange: (v: V) => void }) {
  return (
    <div className="groupRow">
      <p className="rowLabel">{label}</p>
      <div className="rowControl">
        <div className="segmented" role="group" aria-label={label}>
          {options.map(([v, text]) => (
            <button key={v} type="button" aria-pressed={value === v} onClick={() => onChange(v)}>
              {text}
            </button>
          ))}
        </div>
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
