import { useEffect, useRef, useState, type ReactNode } from "react";
import {
  useBrand,
  useResetBrand,
  useSetBrandLogo,
  useTranslateSlogan,
  useUpdateBrand,
  type BrandPatch,
  type BrandView,
} from "../../kernel/bridge/brand";
import {
  useGenerateTemplate,
  useMeetingTypeTemplates,
  useOpenTypeTemplate,
  useRedrawTemplatePreview,
  useTemplatePreview,
} from "../../kernel/bridge/templates";
import { BridgeError } from "../../kernel/bridge/unwrap";
import DangerButton from "../../kernel/shell/DangerButton";
import { SparkleIcon } from "../../kernel/shell/icons";
import { logoUrl } from "../../kernel/theme/brandLogo";
import { convertFileSrc } from "@tauri-apps/api/core";
import styles from "./BrandPanel.module.css";

/** Workspace → Brand: the logo, colour, slogan and fonts of one company's documents, saved
 *  to its brand/tokens.json, and the minutes template made from them. The app's own look
 *  never changes with it (D-033). */
export default function BrandPanel({ slug }: { slug: string }) {
  const brand = useBrand(slug);
  const update = useUpdateBrand(slug);
  const setLogo = useSetBrandLogo(slug);
  const reset = useResetBrand(slug);
  const redraw = useRedrawTemplatePreview(slug);
  const error = update.error ?? setLogo.error ?? reset.error ?? brand.error;
  const b = brand.data;
  const save = (patch: BrandPatch) => update.mutate(patch);
  // The minutes preview shows the slogans (they fill in at export): draw it again once they change.
  const saveSlogan = (patch: BrandPatch) => update.mutate(patch, { onSuccess: redraw });

  return (
    <>
      <p className="fieldHint">For documents only — minutes, PDFs and decks carry this brand; the app keeps its own look.</p>

      <p className="groupTitle">Logo</p>
      <div className="group">
        <LogoRow label="Logo" hint="SVG or PNG. We make the white version from it." path={b?.logo_primary ?? null} onPick={() => setLogo.mutate({ onDark: false })} onClear={() => setLogo.mutate({ onDark: false, clear: true })} />
        <LogoRow label="White logo" hint="Made from your logo — replace it if you have an official one." path={b?.logo_on_dark ?? null} dark onPick={() => setLogo.mutate({ onDark: true })} onClear={() => setLogo.mutate({ onDark: true, clear: true })} />
      </div>

      <p className="groupTitle">Colour</p>
      <div className="group">
        <ColourRow label="Primary" hint="Headings, rules and accents." value={b?.primary ?? ""} onSave={(v) => save({ primary: v })} />
      </div>

      <p className="groupTitle">Words and fonts</p>
      <div className="group">
        <SloganRows slug={slug} brand={b} onSave={saveSlogan} />
        <TextRow label="Heading font" hint="Installed on this Mac, or bundled in the brand folder." value={b?.document_fonts.heading ?? ""} onSave={(v) => save({ font_heading: v })} />
        <TextRow label="Body font" value={b?.document_fonts.body ?? ""} onSave={(v) => save({ font_body: v })} />
      </div>

      <MinutesTemplate slug={slug} />

      <div className={styles.footer}>
        <span className="fieldHint">Started from the {b?.starter === "default" ? "neutral" : b?.starter.toUpperCase()} pack · saved in the workspace's brand folder.</span>
        <DangerButton confirm="Replace with the starter pack?" onConfirm={() => reset.mutate(undefined, { onSuccess: redraw })} disabled={reset.isPending}>
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

/** The default meeting type's minutes template, drawn with a made-up meeting; made again from
 *  the brand on request. Logo, colour and fonts go into the file when it's made; slogans, names
 *  and the meeting fill in at export — unless the file has its own header text (a starter
 *  pack's layout, an older copy or your own file), which its fields tell. */
function MinutesTemplate({ slug }: { slug: string }) {
  const types = useMeetingTypeTemplates(slug);
  const generate = useGenerateTemplate(slug);
  const open = useOpenTypeTemplate(slug);
  const main = types.data?.find((t) => t.is_default) ?? types.data?.[0];
  const error = generate.error ?? open.error;
  const fields = main?.template.check?.fields;
  const slogans = !fields || fields.includes("slogan_en");
  const names = !fields || fields.includes("minute_taker");
  return (
    <>
      <p className="groupTitle">Minutes template</p>
      <div className="group">
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">{main ? `${main.name} (default type)` : "Default meeting type"}</p>
            <p className="rowHint">
              Logo, colour and fonts are set in the file — update it after you change them.
              {slogans && names && " Slogans and names fill in at export."}
              {!slogans && " This file has its own header text, so slogan changes don't reach it."}
              {!names && " Your name fills in as minute-taker after Reset template in Meeting types."}
              {(types.data?.length ?? 0) > 1 && " Other meeting types keep their own templates — remake them with Reset template in Meeting types."}
            </p>
          </div>
          <div className="rowControl">
            <button type="button" className="btn btnSecondary btnCompact" disabled={!main || open.isPending} onClick={() => main && open.mutate({ id: main.id, reveal: false })}>
              Open in Word
            </button>
            <DangerButton
              confirm={slogans ? "Make the minutes template again from this brand? The current one goes to the Trash." : "Replace this layout with the standard one made from your brand? The current file goes to the Trash."}
              onConfirm={() => generate.mutate()}
              disabled={generate.isPending}
            >
              {generate.isPending ? "Updating…" : "Update from brand"}
            </DangerButton>
          </div>
        </div>
        <MinutesPreview slug={slug} />
      </div>
      {error && (
        <p className="formError" role="alert">
          {error.message}
        </p>
      )}
    </>
  );
}

/** The minutes' first page as Quick Look draws it (a ghost while it draws), as a group row. */
export function MinutesPreview({ slug }: { slug: string }) {
  const preview = useTemplatePreview(slug, true);
  return (
    <div className={`groupRow ${styles.pageRow}`}>
      {preview.data ? (
        <img className={styles.page} src={logoUrl(preview.data.path, preview.data.version)} alt="First page of the minutes, filled with a made-up meeting" />
      ) : (
        <div className={`${styles.page} ghostCard`} />
      )}
      {preview.error && <p className="rowHint">{preview.error.message}</p>}
    </div>
  );
}

/** The slogan in English and Vietnamese, as two rows. Translate (the Translation agent) writes
 *  the Vietnamese from the English; `autoTranslate` also does it once, when you leave a new
 *  English slogan while there is no Vietnamese yet. */
export function SloganRows({ slug, brand, autoTranslate, onSave }: { slug: string; brand: BrandView | undefined; autoTranslate?: boolean; onSave: (patch: BrandPatch) => void }) {
  const translate = useTranslateSlogan(slug);
  // The English to translate: what was last typed (clicking Translate blurs the field and saves
  // it, before this re-renders), else the saved slogan whenever that changes.
  const english = useRef("");
  const savedEnglish = brand?.slogan.en ?? "";
  useEffect(() => {
    english.current = savedEnglish;
  }, [savedEnglish]);
  const tried = useRef(false);
  // Saved from the promise, not a mutate callback: it still lands if this view is left meanwhile.
  const run = () =>
    void translate.mutateAsync(english.current).then(
      (vi) => onSave({ slogan_vi: vi }),
      () => undefined, // shown below from translate.error
    );
  const error =
    translate.error instanceof BridgeError && translate.error.code === "ollama_unavailable"
      ? "Ollama isn't running — type the Vietnamese yourself."
      : translate.error?.message;
  return (
    <>
      <TextRow
        label="Slogan (English)"
        hint="One line — about 80 characters fit the header."
        value={savedEnglish}
        onSave={(v) => {
          english.current = v;
          onSave({ slogan_en: v });
          if (autoTranslate && !tried.current && v.trim() && !brand?.slogan.vi) {
            tried.current = true;
            run();
          }
        }}
      />
      <TextRow
        label="Slogan (Tiếng Việt)"
        hint={
          error && (
            <span className="formError" role="alert">
              {error}
            </span>
          )
        }
        value={brand?.slogan.vi ?? ""}
        disabled={translate.isPending}
        onSave={(v) => onSave({ slogan_vi: v })}
      >
        <button
          type="button"
          className="btn btnGhost btnCompact btnAi"
          aria-busy={translate.isPending || undefined}
          disabled={!brand || translate.isPending}
          title="The Translation agent writes it from the English slogan"
          onClick={run}
        >
          <SparkleIcon />
          {translate.isPending ? "Translating…" : "Translate"}
        </button>
      </TextRow>
    </>
  );
}

/** A logo slot: the image (on a dark box for the white logo), Choose… and Remove. */
export function LogoRow({ label, hint, path, dark, onPick, onClear }: { label: string; hint: string; path: string | null; dark?: boolean; onPick: () => void; onClear: () => void }) {
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
        <button type="button" className="btn btnSecondary btnCompact" aria-label={`Choose the ${label.toLowerCase()}`} onClick={onPick}>
          Choose…
        </button>
        {path && (
          <button type="button" className="btn btnGhost btnCompact" aria-label={`Remove the ${label.toLowerCase()}`} onClick={onClear}>
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
export function ColourRow({ label, hint, value, onSave }: { label: string; hint?: string; value: string; onSave: (hex: string) => void }) {
  const [draft, setDraft] = useState(value);
  const [picked, setPicked] = useState<string | null>(null);
  useEffect(() => setDraft(value), [value]);
  // One save per pick: once handed over it's forgotten, so a later change (typed hex, reset)
  // isn't undone by an old pick.
  useEffect(() => {
    if (picked === null) return;
    const t = setTimeout(() => {
      setPicked(null);
      if (picked !== value) onSave(picked);
    }, 300);
    return () => clearTimeout(t);
  }, [picked, value, onSave]);
  const commit = (v: string) => {
    const next = v.trim().toUpperCase();
    if (next === value) return;
    if (HEX.test(next)) onSave(next);
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
          placeholder="#000000"
          aria-label={`${label} colour, hex`}
          spellCheck={false}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={(e) => commit(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && !e.nativeEvent.isComposing && e.currentTarget.blur()}
        />
      </div>
    </div>
  );
}

/** A one-line text field that saves when you leave it (or press Enter); `children` sit beside
 *  the field (an action such as Translate, which can lock it with `disabled` while it runs). */
export function TextRow({ label, hint, value, disabled, onSave, children }: { label: string; hint?: ReactNode; value: string; disabled?: boolean; onSave: (v: string) => void; children?: ReactNode }) {
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
          aria-label={label}
          disabled={disabled}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={() => draft.trim() !== value && onSave(draft)}
          onKeyDown={(e) => e.key === "Enter" && !e.nativeEvent.isComposing && e.currentTarget.blur()}
        />
        {children}
      </div>
    </div>
  );
}
