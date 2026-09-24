interface Option<V extends string> {
  value: V;
  label: string;
}

interface Props<V extends string> {
  options: readonly Option<V>[];
  value: V;
  /** Название выбора — для экранного диктора. */
  label: string;
  disabled?: boolean;
  onChange: (next: V) => void;
}

/** Выбор одного из нескольких вариантов кнопками в одну строку — язык
 *  интерфейса и язык видео (спека этапа 6 §5.2). */
export default function Segmented<V extends string>({ options, value, label, disabled = false, onChange }: Props<V>) {
  return (
    <div className="segmented" role="radiogroup" aria-label={label}>
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          role="radio"
          aria-checked={o.value === value}
          className={o.value === value ? "segmented-option active" : "segmented-option"}
          disabled={disabled}
          onClick={() => {
            if (o.value !== value) onChange(o.value);
          }}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}
