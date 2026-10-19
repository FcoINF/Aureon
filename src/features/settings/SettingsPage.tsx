import { useEffect, useState } from 'react';
import { open, save } from '@tauri-apps/plugin-dialog';

import { Icon } from '../../components/Icons';
import { ConfirmDialog } from '../../components/Modal';
import { EmptyState, Field, PageHeader } from '../../components/Primitives';
import { useToast } from '../../components/Toast';
import { useAsync } from '../../hooks/useAsync';
import { api } from '../../services/api';
import { useAppData } from '../../services/store';
import type { AppSettings, Category, CategoryKind, DateFormat } from '../../types';
import { todayIso } from '../../utils/format';
const DATE_FORMATS = [
  { value: 'DD/MM/AAAA', label: 'DD/MM/AAAA (31/12/2026)' },
  { value: 'AAAA-MM-DD', label: 'AAAA-MM-DD (2026-12-31)' },
];

const SEPARATORS = [
  { value: ';', label: 'Punto y coma (;)' },
  { value: ',', label: 'Coma (,)' },
  { value: '\t', label: 'Tabulación (tab)' },
];

const DATASETS: { id: string; label: string; file: string }[] = [
  { id: 'transactions', label: 'Movimientos', file: 'aureon-movimientos' },
  { id: 'purchases', label: 'Compras', file: 'aureon-compras' },
  { id: 'subscriptions', label: 'Suscripciones', file: 'aureon-suscripciones' },
  { id: 'receivables', label: 'Cuentas por cobrar', file: 'aureon-por-cobrar' },
  { id: 'payables', label: 'Cuentas por pagar', file: 'aureon-por-pagar' },
  { id: 'deposits', label: 'Depósitos pendientes', file: 'aureon-depositos' },
];

const CATEGORY_COLORS = ['#b08b4a', '#4c7ba8', '#3f8a7a', '#b0574b', '#7a62a8', '#c2703d'];

export function SettingsPage() {
  const { settings, currencies, setSettings, refresh } = useAppData();
  // En una instalacion recien estrenada la base de datos no existe todavia,
  // asi que las monedas vienen del catalogo de referencia del backend.
  const catalogoMonedas =
    currencies.length > 0
      ? currencies
      : [
          { code: 'CLP', name: 'Peso chileno', symbol: '$', exponent: 0 },
          { code: 'USD', name: 'Dolar estadounidense', symbol: 'US$', exponent: 2 },
          { code: 'EUR', name: 'Euro', symbol: 'EUR', exponent: 2 },
        ];
  const toast = useToast();

  const [form, setForm] = useState<AppSettings>(settings);
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [categoryName, setCategoryName] = useState('');
  const [categoryKind, setCategoryKind] = useState<CategoryKind>('expense');
  const [categoryColor, setCategoryColor] = useState(CATEGORY_COLORS[0]);
  const [categoryError, setCategoryError] = useState<string | null>(null);
  const [categoryBusy, setCategoryBusy] = useState(false);

  const [backupPath, setBackupPath] = useState('');
  const [backupBusy, setBackupBusy] = useState(false);
  const [restorePath, setRestorePath] = useState<string | null>(null);
  const [restoreVersion, setRestoreVersion] = useState<number | null>(null);
  const [restoreBusy, setRestoreBusy] = useState(false);
  const [exportBusy, setExportBusy] = useState<string | null>(null);

  const categories = useAsync(() => api.listCategories(undefined, true), []);
  const info = useAsync(() => api.appInfo(), []);

  useEffect(() => {
    if (!dirty) setForm(settings);
  }, [settings, dirty]);

  const update = <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => {
    setForm((current) => ({ ...current, [key]: value }));
    setDirty(true);
    setSaved(false);
  };

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    setSaved(false);
    try {
      await setSettings(form);
      setDirty(false);
      setSaved(true);
      toast.success('Cambios guardados.');
    } catch (err) {
      const message = err instanceof Error ? err.message : 'No se pudieron guardar los cambios.';
      setError(message);
      toast.error(message);
    } finally {
      setSaving(false);
    }
  };

  const handleCreateCategory = async () => {
    const name = categoryName.trim();
    if (!name) {
      setCategoryError('Escribe el nombre de la categoría.');
      return;
    }
    setCategoryBusy(true);
    setCategoryError(null);
    try {
      await api.createCategory({ name, kind: categoryKind, color: categoryColor });
      toast.success('Categoría creada.');
      setCategoryName('');
      categories.reload();
      await refresh();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'No se pudo crear la categoría.';
      setCategoryError(message);
      toast.error(message);
    } finally {
      setCategoryBusy(false);
    }
  };

  const handleToggleArchive = async (category: Category) => {
    try {
      await api.setCategoryArchived(category.id, !category.archived);
      toast.success(category.archived ? 'Categoría restaurada.' : 'Categoría archivada.');
      categories.reload();
      await refresh();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'No se pudo actualizar la categoría.');
    }
  };

  const handleSuggestBackupPath = async () => {
    try {
      setBackupPath(await api.defaultBackupPath());
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'No se pudo obtener la ruta sugerida.');
    }
  };

  const handleCreateBackup = async () => {
    setBackupBusy(true);
    try {
      const suggested = backupPath.trim() || (await api.defaultBackupPath());
      const path = await save({
        defaultPath: suggested,
        filters: [{ name: 'Copia de Aureon', extensions: ['db'] }],
      });
      if (!path) return;
      const result = await api.createBackup(path);
      setBackupPath(result.path);
      toast.success(`Copia de seguridad creada en ${result.path}.`);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'No se pudo crear la copia de seguridad.');
    } finally {
      setBackupBusy(false);
    }
  };

  const handlePickBackup = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'Copia de Aureon', extensions: ['db', 'sqlite', 'aureon'] }],
      });
      if (typeof selected !== 'string') return;
      const version = await api.validateBackupFile(selected);
      setRestorePath(selected);
      setRestoreVersion(version);
    } catch (err) {
      toast.error(
        err instanceof Error
          ? err.message
          : 'El archivo seleccionado no es una copia de seguridad válida.',
      );
    }
  };

  const handleRestore = async () => {
    if (!restorePath) return;
    setRestoreBusy(true);
    try {
      await api.restoreBackup(restorePath);
      toast.success('Copia restaurada. Recargando Aureon…');
      window.setTimeout(() => window.location.reload(), 700);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'No se pudo restaurar la copia.');
      setRestoreBusy(false);
    }
  };

  const handleExport = async (dataset: { id: string; label: string; file: string }) => {
    setExportBusy(dataset.id);
    try {
      const path = await save({
        defaultPath: `${dataset.file}-${todayIso()}.csv`,
        filters: [{ name: 'CSV', extensions: ['csv'] }],
      });
      if (!path) return;
      const contents = await api.exportDataset(dataset.id);
      await api.writeTextFile(path, contents);
      toast.success(`${dataset.label} exportados en ${path}.`);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'No se pudo exportar el archivo.');
    } finally {
      setExportBusy(null);
    }
  };

  const list = categories.data ?? [];

  return (
    <>
      <PageHeader
        title="Configuración"
        subtitle="Preferencias, categorías, copias de seguridad y exportaciones de Aureon."
        actions={
          <button
            type="button"
            className="btn btn-primary"
            onClick={() => void handleSave()}
            disabled={saving}
          >
            <Icon name="check" size={14} />
            {saving ? 'Guardando…' : 'Guardar cambios'}
          </button>
        }
      />

      <section className="card">
        <header className="card-header">
          <h2 className="card-title">Preferencias</h2>
        </header>
        <div className="card-body">
          {error ? (
            <div className="alert alert-error" style={{ marginBottom: 'var(--space-4)' }}>
              {error}
            </div>
          ) : null}
          {saved ? (
            <div className="alert alert-success" style={{ marginBottom: 'var(--space-4)' }}>
              <Icon name="check" size={15} />
              <span>Cambios guardados.</span>
            </div>
          ) : null}

          <div className="form-grid">
            <Field label="Moneda por defecto">
              {(id) => (
                <select
                  id={id}
                  className="select"
                  value={form.defaultCurrency}
                  onChange={(event) => update('defaultCurrency', event.target.value)}
                >
                  {catalogoMonedas.map((currency) => (
                    <option key={currency.code} value={currency.code}>
                      {currency.code} - {currency.name}
                    </option>
                  ))}
                </select>
              )}
            </Field>

            <Field label="Formato de fecha">
              {(id) => (
                <select
                  id={id}
                  className="select"
                  value={form.dateFormat}
                  onChange={(event) => update('dateFormat', event.target.value as DateFormat)}
                >
                  {DATE_FORMATS.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
              )}
            </Field>

            <div className="field">
              <span className="field-label">Tema</span>
              <div className="segmented" role="group" aria-label="Tema de la aplicación">
                <button
                  type="button"
                  aria-pressed={form.theme === 'light'}
                  onClick={() => update('theme', 'light')}
                >
                  <Icon name="sun" size={13} />
                  Claro
                </button>
                <button
                  type="button"
                  aria-pressed={form.theme === 'dark'}
                  onClick={() => update('theme', 'dark')}
                >
                  <Icon name="moon" size={13} />
                  Oscuro
                </button>
              </div>
            </div>

            <Field label="Separador de exportación">
              {(id) => (
                <select
                  id={id}
                  className="select"
                  value={form.exportSeparator}
                  onChange={(event) => update('exportSeparator', event.target.value)}
                >
                  {SEPARATORS.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
              )}
            </Field>

            <Field
              label="Carpeta de copias de seguridad"
              className="span-2"
              hint="Se usa como destino sugerido al crear una copia."
            >
              {(id) => (
                <input
                  id={id}
                  className="input"
                  value={form.backupFolder ?? ''}
                  placeholder="Dejar vacío para usar la carpeta de datos de Aureon"
                  onChange={(event) =>
                    update('backupFolder', event.target.value.trim() || null)
                  }
                />
              )}
            </Field>
          </div>

          <div className="row" style={{ marginTop: 'var(--space-5)' }}>
            <button
              type="button"
              className="btn btn-primary"
              onClick={() => void handleSave()}
              disabled={saving}
            >
              {saving ? 'Guardando…' : 'Guardar cambios'}
            </button>
          </div>
        </div>
      </section>

      <section className="card">
        <header className="card-header">
          <h2 className="card-title">Categorías</h2>
          <span className="text-sm text-muted">
            {list.filter((item) => !item.archived).length} activas
          </span>
        </header>
        <div className="card-body">
          {categoryError ? (
            <div className="alert alert-error" style={{ marginBottom: 'var(--space-4)' }}>
              {categoryError}
            </div>
          ) : null}

          <div className="form-grid" style={{ marginBottom: 'var(--space-5)' }}>
            <Field label="Nombre de la categoría">
              {(id) => (
                <input
                  id={id}
                  className="input"
                  value={categoryName}
                  placeholder="Ej.: Supermercado"
                  onChange={(event) => setCategoryName(event.target.value)}
                />
              )}
            </Field>

            <Field label="Tipo">
              {(id) => (
                <select
                  id={id}
                  className="select"
                  value={categoryKind}
                  onChange={(event) => setCategoryKind(event.target.value as CategoryKind)}
                >
                  <option value="expense">Gasto</option>
                  <option value="income">Ingreso</option>
                </select>
              )}
            </Field>

            <Field label="Color">
              {(id) => (
                <div className="row" style={{ gap: 8 }}>
                  <input
                    id={id}
                    type="color"
                    value={categoryColor}
                    aria-label="Color de la categoría"
                    onChange={(event) => setCategoryColor(event.target.value)}
                    style={{
                      width: 42,
                      height: 34,
                      padding: 2,
                      border: '1px solid var(--border-strong)',
                      borderRadius: 'var(--radius-sm)',
                      background: 'var(--surface)',
                    }}
                  />
                  {CATEGORY_COLORS.map((color) => (
                    <button
                      key={color}
                      type="button"
                      className="btn btn-icon"
                      title={`Usar el color ${color}`}
                      aria-label={`Usar el color ${color}`}
                      onClick={() => setCategoryColor(color)}
                      style={{ background: color }}
                    />
                  ))}
                </div>
              )}
            </Field>

            <div className="field">
              <span className="field-label">Crear</span>
              <button
                type="button"
                className="btn btn-primary"
                onClick={() => void handleCreateCategory()}
                disabled={categoryBusy}
              >
                <Icon name="plus" size={14} />
                {categoryBusy ? 'Creando…' : 'Crear categoría'}
              </button>
            </div>
          </div>

          {categories.loading ? (
            <div className="loading">
              <span className="spinner" />
            </div>
          ) : categories.error ? (
            <div className="alert alert-error">{categories.error}</div>
          ) : list.length === 0 ? (
            <EmptyState
              icon="target"
              title="Todavía no hay categorías"
              text="Crea la primera categoría para ordenar tus ingresos y gastos."
            />
          ) : (
            <div className="table-wrap">
              <table className="table">
                <thead>
                  <tr>
                    <th>Categoría</th>
                    <th>Tipo</th>
                    <th>Estado</th>
                    <th />
                  </tr>
                </thead>
                <tbody>
                  {list.map((category) => (
                    <tr key={category.id}>
                      <td>
                        <div className="row" style={{ gap: 8 }}>
                          <span
                            aria-hidden="true"
                            style={{
                              width: 12,
                              height: 12,
                              borderRadius: 3,
                              background: category.color ?? 'var(--text-soft)',
                            }}
                          />
                          <span className={category.archived ? 'text-soft' : ''}>
                            {category.name}
                          </span>
                        </div>
                      </td>
                      <td className="text-sm text-muted">
                        {category.kind === 'income' ? 'Ingreso' : 'Gasto'}
                      </td>
                      <td>
                        <span
                          className={
                            category.archived ? 'badge badge-warning' : 'badge badge-income'
                          }
                        >
                          {category.archived ? 'Archivada' : 'Activa'}
                        </span>
                      </td>
                      <td>
                        <div className="table-actions">
                          <button
                            type="button"
                            className="btn btn-icon"
                            title={category.archived ? 'Restaurar categoría' : 'Archivar categoría'}
                            aria-label={
                              category.archived
                                ? `Restaurar ${category.name}`
                                : `Archivar ${category.name}`
                            }
                            onClick={() => void handleToggleArchive(category)}
                          >
                            <Icon name={category.archived ? 'unlock' : 'close'} size={14} />
                          </button>
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      </section>

      <section className="card">
        <header className="card-header">
          <h2 className="card-title">Información de Aureon</h2>
        </header>
        <div className="card-body">
          {info.loading ? (
            <div className="loading">
              <span className="spinner" />
            </div>
          ) : info.error ? (
            <div className="alert alert-error">{info.error}</div>
          ) : info.data ? (
            <div className="detail-list">
              <div className="detail-row">
                <span className="detail-key">Versión</span>
                <span className="detail-value">{info.data.version}</span>
              </div>
              <div className="detail-row">
                <span className="detail-key">Carpeta de datos</span>
                <span className="detail-value">{info.data.dataDir}</span>
              </div>
              <div className="detail-row">
                <span className="detail-key">Base de datos</span>
                <span className="detail-value">{info.data.databasePath}</span>
              </div>
              <div className="detail-row">
                <span className="detail-key">Estado de la base</span>
                <span className="detail-value">
                  {info.data.databaseExists ? 'Creada' : 'Todavía no creada'}
                </span>
              </div>
              <div className="detail-row">
                <span className="detail-key">Versión del esquema</span>
                <span className="detail-value">{info.data.schemaVersion}</span>
              </div>
              <div className="detail-row">
                <span className="detail-key">Datos registrados</span>
                <span className="detail-value">
                  {info.data.hasData ? 'Sí' : 'Todavía no'}
                </span>
              </div>
            </div>
          ) : null}

          <div className="alert alert-info" style={{ marginTop: 'var(--space-4)' }}>
            <Icon name="info" size={15} />
            <span>
              Tus datos se guardan en tu equipo, dentro de la carpeta de datos que aparece
              arriba. Desinstalar Aureon NO borra esa carpeta: tus movimientos siguen ahí
              después de desinstalar. Si quieres empezar de cero, borra la carpeta a mano.
            </span>
          </div>
        </div>
      </section>

      <section className="card">
        <header className="card-header">
          <h2 className="card-title">Copias de seguridad</h2>
        </header>
        <div className="card-body">
          <div className="form-section">
            <p className="form-section-title">Crear una copia</p>
            <div className="form-grid">
              <Field
                label="Ruta del archivo"
                className="span-2"
                hint="Puedes editarla antes de confirmar la ubicación."
              >
                {(id) => (
                  <div className="row" style={{ gap: 8 }}>
                    <input
                      id={id}
                      className="input"
                      value={backupPath}
                      placeholder="Elige la carpeta y el nombre del archivo"
                      onChange={(event) => setBackupPath(event.target.value)}
                    />
                    <button
                      type="button"
                      className="btn btn-icon"
                      title="Usar la ruta sugerida"
                      aria-label="Usar la ruta sugerida"
                      onClick={() => void handleSuggestBackupPath()}
                    >
                      <Icon name="repeat" size={14} />
                    </button>
                  </div>
                )}
              </Field>
            </div>
            <div className="row">
              <button
                type="button"
                className="btn btn-primary"
                onClick={() => void handleCreateBackup()}
                disabled={backupBusy}
              >
                <Icon name="download" size={14} />
                {backupBusy ? 'Creando…' : 'Crear copia de seguridad'}
              </button>
            </div>
          </div>

          <div className="form-section">
            <p className="form-section-title">Restaurar una copia</p>
            <p className="text-sm text-muted">
              Elige el archivo de una copia anterior. Aureon comprobará que sea válido y te
              pedirá confirmación antes de reemplazar tus datos actuales.
            </p>
            <div className="row">
              <button
                type="button"
                className="btn"
                onClick={() => void handlePickBackup()}
                disabled={restoreBusy}
              >
                <Icon name="upload" size={14} />
                Elegir copia de seguridad
              </button>
              {restorePath ? (
                <span className="text-sm text-soft truncate">{restorePath}</span>
              ) : null}
            </div>
          </div>
        </div>
      </section>

      <section className="card">
        <header className="card-header">
          <h2 className="card-title">Exportaciones</h2>
        </header>
        <div className="card-body">
          <p className="text-sm text-muted" style={{ marginBottom: 'var(--space-4)' }}>
            Cada botón abre un diálogo para elegir dónde guardar el archivo CSV.
          </p>
          <div className="grid-3">
            {DATASETS.map((dataset) => (
              <button
                key={dataset.id}
                type="button"
                className="btn"
                onClick={() => void handleExport(dataset)}
                disabled={exportBusy !== null}
              >
                <Icon name="download" size={14} />
                {exportBusy === dataset.id ? 'Exportando…' : dataset.label}
              </button>
            ))}
          </div>
        </div>
      </section>

      <ConfirmDialog
        open={restorePath !== null}
        title="Restaurar copia de seguridad"
        message={
          restorePath
            ? `Se reemplazarán todos tus datos actuales por los de «${restorePath}»${
                restoreVersion !== null ? ` (versión de esquema ${restoreVersion})` : ''
              }. Antes de restaurar, Aureon creará automáticamente una copia preventiva de
              los datos actuales en la carpeta de datos, por si necesitas volver atrás.`
            : ''
        }
        confirmLabel="Restaurar"
        danger
        busy={restoreBusy}
        onConfirm={handleRestore}
        onCancel={() => {
          setRestorePath(null);
          setRestoreVersion(null);
        }}
      />
    </>
  );
}
