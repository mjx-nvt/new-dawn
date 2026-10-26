import { useEffect, useId, useRef } from "react";
import { useMeetingTypes, type Language } from "../../kernel/bridge/meetings";
import { useWorkspaces } from "../../kernel/bridge/workspaces";
import { LANGUAGE_LABEL } from "./format";

const LANGUAGES: Language[] = ["en", "vi", "auto"];

interface Props {
  slug: string;
  onSlugChange: (slug: string) => void;
  typeId: string;
  onTypeChange: (typeId: string) => void;
  language: Language;
  onLanguageChange: (language: Language) => void;
  /** False hides the meeting type: a quick translation makes no meeting. */
  withType?: boolean;
}

/** Workspace · meeting type · language, shared by Import audio and New recording.
 *  Changing workspace clears the type (types belong to one tenant); picking a type adopts its language. */
export default function MeetingFields(props: Props) {
  const { data: workspaces } = useWorkspaces();
  const types = useMeetingTypes(props.slug);
  const { typeId, onTypeChange, onLanguageChange } = props;

  // A new meeting starts with the workspace's default type (once per workspace; "None" stays).
  const preset = useRef("");
  useEffect(() => {
    const fallback = types.data?.find((t) => t.is_default);
    if (!fallback || preset.current === props.slug) return;
    preset.current = props.slug;
    if (!typeId) {
      onTypeChange(fallback.id);
      onLanguageChange(fallback.language);
    }
  }, [types.data, props.slug, typeId, onTypeChange, onLanguageChange]);

  return (
    <>
      <div className="fields">
        <label className="field">
          <span className="fieldLabel">Workspace</span>
          <select
            className="input"
            value={props.slug}
            onChange={(e) => {
              props.onSlugChange(e.target.value);
              props.onTypeChange("");
            }}
          >
            {workspaces
              ?.filter((w) => !w.archived)
              .map((w) => (
                <option key={w.slug} value={w.slug}>
                  {w.name}
                </option>
              ))}
          </select>
        </label>
        {props.withType !== false && (
          <label className="field">
            <span className="fieldLabel">Meeting type</span>
            <select
              className="input"
              value={props.typeId}
              onChange={(e) => {
                props.onTypeChange(e.target.value);
                const type = types.data?.find((t) => t.id === e.target.value);
                if (type) props.onLanguageChange(type.language);
              }}
            >
              <option value="">None</option>
              {types.data?.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.name}
                </option>
              ))}
            </select>
          </label>
        )}
      </div>
    </>
  );
}

/** Discussion, or one main presenter — with their name when the recording may never say it (D-038).
 *  `hint`: what the choice does (the live sheet's). */
export function FormatField({
  presentation,
  presenter,
  onChange,
  hint,
}: {
  presentation: boolean;
  presenter: string;
  onChange: (presentation: boolean, presenter: string) => void;
  hint?: string;
}) {
  const id = useId();
  return (
    <div className="field">
      <span className="fieldLabel" id={id}>
        Format
      </span>
      <div className="segmented" role="group" aria-labelledby={id}>
        <button type="button" aria-pressed={!presentation} onClick={() => onChange(false, presenter)}>
          Discussion
        </button>
        <button type="button" aria-pressed={presentation} onClick={() => onChange(true, presenter)}>
          One presenter
        </button>
      </div>
      {presentation && (
        <input
          className="input"
          aria-label="Presenter"
          value={presenter}
          placeholder="Presenter's name — if the recording may not say it"
          onChange={(e) => onChange(true, e.target.value)}
        />
      )}
      {hint && <p className="fieldHint">{hint}</p>}
    </div>
  );
}

/** The spoken-language segmented control. */
export function LanguageField({ id, value, onChange }: { id: string; value: Language; onChange: (l: Language) => void }) {
  return (
    <div className="field">
      <span className="fieldLabel" id={id}>
        Spoken language
      </span>
      <div className="segmented" role="group" aria-labelledby={id}>
        {LANGUAGES.map((l) => (
          <button key={l} type="button" aria-pressed={value === l} onClick={() => onChange(l)}>
            {LANGUAGE_LABEL[l]}
          </button>
        ))}
      </div>
    </div>
  );
}
