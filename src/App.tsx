import { Navigate, Route, Routes } from 'react-router-dom';

import { AppLayout } from './layouts/AppLayout';
import { useAppData } from './services/store';
import { DashboardPage } from './features/dashboard/DashboardPage';
import { TransactionsPage } from './features/transactions/TransactionsPage';
import { AccountsPage } from './features/accounts/AccountsPage';
import { BudgetsPage } from './features/budgets/BudgetsPage';
import { PurchasesPage } from './features/purchases/PurchasesPage';
import { SubscriptionsPage } from './features/subscriptions/SubscriptionsPage';
import { ReceivablesPage } from './features/receivables/ReceivablesPage';
import { PayablesPage } from './features/payables/PayablesPage';
import { DepositsPage } from './features/deposits/DepositsPage';
import { WishlistPage } from './features/wishlist/WishlistPage';
import { HistoryPage } from './features/history/HistoryPage';
import { CalendarPage } from './features/calendar/CalendarPage';
import { SettingsPage } from './features/settings/SettingsPage';

export default function App() {
  const { ready, error } = useAppData();

  if (!ready) {
    return (
      <div className="loading" style={{ height: '100vh' }}>
        <div className="row">
          <span className="spinner" />
          <span>Preparando Aureon…</span>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="loading" style={{ height: '100vh' }}>
        <p>{error}</p>
      </div>
    );
  }

  return (
    <Routes>
      <Route
        path="/"
        element={
          <AppLayout title="Inicio" subtitle="Tu situación financiera de un vistazo">
            <DashboardPage />
          </AppLayout>
        }
      />
      <Route
        path="/movimientos"
        element={
          <AppLayout title="Movimientos" subtitle="Ingresos, gastos y transferencias">
            <TransactionsPage />
          </AppLayout>
        }
      />
      <Route
        path="/cuentas"
        element={
          <AppLayout title="Cuentas" subtitle="Efectivo, bancos y tarjetas">
            <AccountsPage />
          </AppLayout>
        }
      />
      <Route
        path="/presupuestos"
        element={
          <AppLayout title="Presupuestos" subtitle="Límites mensuales por categoría">
            <BudgetsPage />
          </AppLayout>
        }
      />
      <Route
        path="/compras"
        element={
          <AppLayout title="Compras y cuotas" subtitle="Compras al contado y compras en cuotas">
            <PurchasesPage />
          </AppLayout>
        }
      />
      <Route
        path="/suscripciones"
        element={
          <AppLayout title="Suscripciones" subtitle="Servicios recurrentes y su historial">
            <SubscriptionsPage />
          </AppLayout>
        }
      />
      <Route
        path="/por-cobrar"
        element={
          <AppLayout title="Por cobrar" subtitle="Dinero que otras personas te deben">
            <ReceivablesPage />
          </AppLayout>
        }
      />
      <Route
        path="/por-pagar"
        element={
          <AppLayout title="Por pagar" subtitle="Obligaciones que tú debes pagar">
            <PayablesPage />
          </AppLayout>
        }
      />
      <Route
        path="/depositos"
        element={
          <AppLayout title="Depósitos pendientes" subtitle="Transferencias y depósitos por realizar">
            <DepositsPage />
          </AppLayout>
        }
      />
      <Route
        path="/deseos"
        element={
          <AppLayout title="Lista de deseos" subtitle="Productos que quieres comprar">
            <WishlistPage />
          </AppLayout>
        }
      />
      <Route
        path="/historial"
        element={
          <AppLayout title="Historial y reportes" subtitle="Consulta tus finanzas por período">
            <HistoryPage />
          </AppLayout>
        }
      />
      <Route
        path="/calendario"
        element={
          <AppLayout title="Calendario" subtitle="Vencimientos y cobros próximos">
            <CalendarPage />
          </AppLayout>
        }
      />
      <Route
        path="/configuracion"
        element={
          <AppLayout title="Configuración" subtitle="Preferencias de Aureon">
            <SettingsPage />
          </AppLayout>
        }
      />
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}
