import { useState, type ReactNode } from 'react';

import { Icon } from '../components/Icons';
import { Sidebar } from './Sidebar';

interface AppLayoutProps {
  title: string;
  subtitle?: string;
  actions?: ReactNode;
  children: ReactNode;
}

export function AppLayout({ title, subtitle, actions, children }: AppLayoutProps) {
  const [menuOpen, setMenuOpen] = useState(false);

  return (
    <div className="app">
      <Sidebar open={menuOpen} onNavigate={() => setMenuOpen(false)} />
      {menuOpen ? (
        <div
          className="overlay"
          style={{ padding: 0, zIndex: 65 }}
          onClick={() => setMenuOpen(false)}
          role="presentation"
        />
      ) : null}
      <div className="main">
        <header className="topbar">
          <div className="topbar-title">
            <div className="row" style={{ gap: 8 }}>
              <button
                type="button"
                className="btn btn-icon mobile-only"
                onClick={() => setMenuOpen((value) => !value)}
                aria-label="Abrir navegación"
              >
                <Icon name="menu" size={16} />
              </button>
              <h1>{title}</h1>
            </div>
            {subtitle ? <p>{subtitle}</p> : null}
          </div>
          {actions ? <div className="page-actions">{actions}</div> : null}
        </header>
        <main className="content">
          <div className="content-inner">{children}</div>
        </main>
      </div>
    </div>
  );
}
