interface Props {
  onClose: () => void;
}

/** Заглушка: настоящий экран собирает задача 8. */
export default function Settings({ onClose }: Props) {
  return (
    <div className="settings">
      <button type="button" onClick={onClose}>
        ← Назад
      </button>
    </div>
  );
}
