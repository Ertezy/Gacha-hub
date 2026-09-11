import { useState } from "react";
import GamesSection from "./settings/GamesSection";
import LookSection from "./settings/LookSection";
import SmallSections from "./settings/SmallSections";

type Tab = "games" | "look" | "behaviour" | "data" | "about";

const TABS: { id: Tab; label: string }[] = [
  { id: "games", label: "Игры" },
  { id: "look", label: "Вид" },
  { id: "behaviour", label: "Поведение" },
  { id: "data", label: "Данные" },
  { id: "about", label: "О программе" },
];

interface Props {
  onClose: () => void;
}

export default function Settings({ onClose }: Props) {
  const [tab, setTab] = useState<Tab>("games");

  return (
    <div className="settings">
      <nav className="settings-nav">
        <button type="button" className="settings-back" onClick={onClose}>
          ← Назад
        </button>
        {TABS.map((t) => (
          <button
            key={t.id}
            type="button"
            className={t.id === tab ? "settings-tab active" : "settings-tab"}
            onClick={() => setTab(t.id)}
          >
            {t.label}
          </button>
        ))}
      </nav>

      <div className="settings-body">
        {tab === "games" && <GamesSection />}
        {tab === "look" && <LookSection />}
        {tab !== "games" && tab !== "look" && <SmallSections tab={tab} />}
      </div>
    </div>
  );
}
