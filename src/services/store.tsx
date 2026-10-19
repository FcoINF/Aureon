import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from 'react';

import { api } from './api';
import type { Account, AppSettings, Category, Currency } from '../types';

interface AppData {
  settings: AppSettings;
  currencies: Currency[];
  accounts: Account[];
  expenseCategories: Category[];
  incomeCategories: Category[];
  loading: boolean;
  ready: boolean;
  error: string | null;
  refresh: () => Promise<void>;
  setSettings: (settings: AppSettings) => Promise<void>;
}

const DEFAULT_SETTINGS: AppSettings = {
  defaultCurrency: 'CLP',
  dateFormat: 'DD/MM/AAAA',
  theme: 'light',
  exportSeparator: ';',
  backupFolder: null,
  weekStartsOnMonday: true,
};

const AppDataContext = createContext<AppData | null>(null);

/**
 * Carga los datos de referencia que se usan en casi todas las pantallas.
 * Se mantiene en memoria durante la sesión de la aplicación.
 */
export function AppDataProvider({ children }: { children: ReactNode }) {
  const [settings, setSettingsState] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [currencies, setCurrencies] = useState<Currency[]>([]);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [expenseCategories, setExpenseCategories] = useState<Category[]>([]);
  const [incomeCategories, setIncomeCategories] = useState<Category[]>([]);
  const [loading, setLoading] = useState(true);
  const [ready, setReady] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [loadedSettings, loadedCurrencies, loadedAccounts, loadedExpense, loadedIncome] =
        await Promise.all([
          api.getSettings(),
          api.listCurrencies(),
          api.listAccounts(false),
          api.listCategories('expense', false),
          api.listCategories('income', false),
        ]);
      setSettingsState(loadedSettings);
      setCurrencies(loadedCurrencies);
      setAccounts(loadedAccounts);
      setExpenseCategories(loadedExpense);
      setIncomeCategories(loadedIncome);
      setReady(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'No se pudieron cargar los datos.');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', settings.theme);
  }, [settings.theme]);

  const setSettings = useCallback(
    async (next: AppSettings) => {
      const saved = await api.saveSettings(next);
      setSettingsState(saved);
    },
    [],
  );

  const value = useMemo<AppData>(
    () => ({
      settings,
      currencies,
      accounts,
      expenseCategories,
      incomeCategories,
      loading,
      ready,
      error,
      refresh,
      setSettings,
    }),
    [
      settings,
      currencies,
      accounts,
      expenseCategories,
      incomeCategories,
      loading,
      ready,
      error,
      refresh,
      setSettings,
    ],
  );

  return <AppDataContext.Provider value={value}>{children}</AppDataContext.Provider>;
}

export function useAppData(): AppData {
  const context = useContext(AppDataContext);
  if (!context) {
    throw new Error('useAppData debe usarse dentro de AppDataProvider');
  }
  return context;
}
