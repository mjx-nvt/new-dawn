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
}

/** Workspace · meeting type · language, shared by Import audio and New recording.
 *  Changing workspace clears the type (types belong to one tenant); picking a type adopts its language. */
export default function MeetingFields(props: Props) {
  const { data: workspaces } = useWorkspaces();
  const types = useMeetingTypes(props.slug);

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
      </div>
    </>
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
