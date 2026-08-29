import type { Page } from "../types";

const ITEMS: { id: Page; label: string; icon: string }[] = [
  { id: "games", label: "Игры", icon: "🎮" },
  { id: "news", label: "Новости", icon: "📰" },
  { id: "promo", label: "Промокоды", icon: "🎁" },
  { id: "guides", label: "Гайды", icon: "📖" },
  { id: "settings", label: "Настройки", icon: "⚙️" },
];

export default function Sidebar({
  page,
  onNavigate,
}: {
  page: Page;
  onNavigate: (p: Page) => void;
}) {
  return (
    <aside className="sidebar">
      <div className="logo">
        Gacha<span>Hub</span>
      </div>
      {ITEMS.map((it) => (
        <button
          key={it.id}
          className={`nav-item${page === it.id ? " active" : ""}`}
          onClick={() => onNavigate(it.id)}
        >
          <span className="nav-icon">{it.icon}</span>
          {it.label}
        </button>
      ))}
      <div className="sidebar-footer">v0.1.0 · MVP</div>
    </aside>
  );
}
