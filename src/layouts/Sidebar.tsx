import { useEffect, useState } from 'react';
import { NavLink, useLocation } from 'react-router-dom';

import { Icon, type IconName } from '../components/Icons';
import { api } from '../services/api';

interface NavEntry {
  to: string;
  label: string;
  icon: IconName;
  badge?: number;
}

const GROUPS: { title: string; items: Omit<NavEntry, 'badge'>[] }[] = [
  {
    title: 'Resumen',
    items: [
      { to: '/', label: 'Inicio', icon: 'home' },
      { to: '/movimientos', label: 'Movimientos', icon: 'arrows' },
      { to: '/historial', label: 'Historial y reportes', icon: 'chart' },
      { to: '/calendario', label: 'Calendario', icon: 'calendar' },
    ],
  },
  {
    title: 'Patrimonio',
    items: [
      { to: '/cuentas', label: 'Cuentas', icon: 'wallet' },
      { to: '/presupuestos', label: 'Presupuestos', icon: 'target' },
      { to: '/deseos', label: 'Lista de deseos', icon: 'heart' },
    ],
  },
  {
    title: 'Compromisos',
    items: [
      { to: '/compras', label: 'Compras y cuotas', icon: 'bag' },
      { to: '/suscripciones', label: 'Suscripciones', icon: 'repeat' },
      { to: '/por-cobrar', label: 'Por cobrar', icon: 'incoming' },
      { to: '/por-pagar', label: 'Por pagar', icon: 'outgoing' },
      { to: '/depositos', label: 'Depósitos pendientes', icon: 'send' },
    ],
  },
];

export function Sidebar({
  open,
  onNavigate,
}: {
  open: boolean;
  onNavigate: () => void;
}) {
  const location = useLocation();
  const [counts, setCounts] = useState<{ overdue: number; deposits: number }>({
    overdue: 0,
    deposits: 0,
  });

  useEffect(() => {
    let active = true;
    const load = async () => {
      try {
        const [overdue, deposits] = await Promise.all([
          api.upcomingCommitments(200),
          api.listDeposits('pending'),
        ]);
        if (!active) return;
        const overdueCount = overdue.filter((item) => item.status === 'overdue').length;
        setCounts({
          overdue: overdueCount,
          deposits: deposits.length,
        });
      } catch {
        if (active) setCounts({ overdue: 0, deposits: 0 });
      }
    };
    void load();
    return () => {
      active = false;
    };
  }, [location.pathname]);

  const badgeFor = (to: string): number | undefined => {
    if (to === '/depositos' && counts.deposits > 0) return counts.deposits;
    if (
      ['/compras', '/por-pagar', '/suscripciones'].includes(to) &&
      counts.overdue > 0
    ) {
      return counts.overdue;
    }
    return undefined;
  };

  return (
    <aside className={`sidebar${open ? ' is-open' : ''}`} aria-label="Navegación principal">
      <div className="sidebar-brand">
        <span className="brand-mark" aria-hidden="true">
          A
        </span>
        <span>
          <span className="brand-name">Aureon</span>
          <br />
          <span className="brand-sub">Finanzas personales</span>
        </span>
      </div>

      <nav className="sidebar-nav">
        {GROUPS.map((group) => (
          <div className="nav-group" key={group.title}>
            <p className="nav-group-title">{group.title}</p>
            {group.items.map((item) => (
              <NavLink
                key={item.to}
                to={item.to}
                end={item.to === '/'}
                className={({ isActive }) => `nav-item${isActive ? ' is-active' : ''}`}
                onClick={onNavigate}
              >
                <Icon name={item.icon} />
                <span className="grow">{item.label}</span>
                {badgeFor(item.to) ? (
                  <span className="nav-badge">{badgeFor(item.to)}</span>
                ) : null}
              </NavLink>
            ))}
          </div>
        ))}
      </nav>

      <div className="sidebar-footer">
        <NavLink
          to="/configuracion"
          className={({ isActive }) => `nav-item${isActive ? ' is-active' : ''}`}
          onClick={onNavigate}
        >
          <Icon name="settings" />
          <span className="grow">Configuración</span>
        </NavLink>
      </div>
    </aside>
  );
}
