/** Apple's On | Off segmented control for a settings row. */
export default function OnOff({ label, value, disabled, onChange }: { label: string; value: boolean | undefined; disabled?: boolean; onChange: (value: boolean) => void }) {
  return (
    <div className="segmented" role="group" aria-label={label}>
      {(
        [
          [true, "On"],
          [false, "Off"],
        ] as const
      ).map(([v, text]) => (
        <button key={text} type="button" aria-pressed={value === v} disabled={disabled} onClick={() => onChange(v)}>
          {text}
        </button>
      ))}
    </div>
  );
}
