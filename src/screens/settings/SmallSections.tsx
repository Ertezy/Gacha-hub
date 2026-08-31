interface Props {
  tab: "behaviour" | "data" | "about";
}

/** Заглушка: настоящие разделы собирает задача 12. */
export default function SmallSections({ tab }: Props) {
  return <p className="settings-empty">Раздел {tab}</p>;
}
