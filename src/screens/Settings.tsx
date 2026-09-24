import { useState } from "react";
import GamesSection from "./settings/GamesSection";
import LookSection from "./settings/LookSection";
import SmallSections from "./settings/SmallSections";
import { useT } from "../i18n";

type Tab = "games" | "look" | "behaviour" | "data" | "about";

interface Props {
  onClose: () => void;
}

export default function Settings({ onClose }: Props) {
  const [tab, setTab] = useState<Tab>("games");
  const t = useT();

  const TABS: { id: Tab; label: string }[] = [
    { id: "games", label: t.settings.tabs.games },
    { id: "look", label: t.settings.tabs.look },
    { id: "behaviour", label: t.settings.tabs.behaviour },
    { id: "data", label: t.settings.tabs.data },
    { id: "about", label: t.settings.tabs.about },
  ];

  return (
    <div className="settings">
      <nav className="settings-nav">
        <button type="button" className="settings-back" onClick={onClose}>
          {t.settings.back}
        </button>
        {TABS.map((tb) => (
          <button
            key={tb.id}
            type="button"
            className={tb.id === tab ? "settings-tab active" : "settings-tab"}
            onClick={() => setTab(tb.id)}
          >
            {tb.label}
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
