import type { LucideIcon } from "lucide-react";

export interface NavItem {
  id: string;
  label: string;
  icon: LucideIcon;
}

export function Sidebar({ items, active, onSelect }: { items: NavItem[]; active: string; onSelect: (id: string) => void }) {
  return (
    <aside className="sidebar shell-card">
      <div className="brand">
        <div className="brand-mark">N</div>
        <div>
          <div className="eyebrow">system control / purple ops</div>
          <h1>Nocturne Optimizer</h1>
        </div>
      </div>
      <nav className="nav-list">
        {items.map((item) => {
          const Icon = item.icon;
          return (
            <button
              key={item.id}
              className={`nav-item ${active === item.id ? "active" : ""}`}
              onClick={() => onSelect(item.id)}
            >
              <Icon size={18} />
              <span>{item.label}</span>
            </button>
          );
        })}
      </nav>
      <div className="sidebar-footer">
        <div className="status-dot" />
        Windows-first / Tauri + Rust
      </div>
    </aside>
  );
}
