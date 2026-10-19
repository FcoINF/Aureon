//! Estado de la aplicación y ciclo de vida de la base de datos.

use std::path::PathBuf;
use std::sync::Mutex;

use rusqlite::Connection;

use crate::core::db;
use crate::core::error::{AppError, AppResult};
use crate::core::services::backup;

struct Inner {
    conn: Option<Connection>,
}

pub struct AppState {
    inner: Mutex<Inner>,
    data_dir: PathBuf,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        AppState {
            inner: Mutex::new(Inner { conn: None }),
            data_dir,
        }
    }

    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    pub fn db_path(&self) -> PathBuf {
        db::db_path(&self.data_dir)
    }

    /// Abre la base de datos si ya existe. Si no existe, la aplicación puede
    /// seguir usándose en modo sin datos persistentes.
    pub fn init(&self) -> AppResult<bool> {
        let path = self.db_path();
        if !path.exists() {
            return Ok(false);
        }
        if !db::is_readable_database(&path) {
            return Err(AppError::Database(
                "La base de datos está dañada. Restaura una copia de seguridad para continuar.".to_string(),
            ));
        }
        let conn = db::open_existing(&path)?;
        let mut guard = self.inner.lock().map_err(lock_error)?;
        guard.conn = Some(conn);
        Ok(true)
    }

    /// Indica si ya existe información persistida.
    pub fn has_db(&self) -> bool {
        self.inner
            .lock()
            .map(|guard| guard.conn.is_some())
            .unwrap_or(false)
    }

    fn lock(&self) -> AppResult<std::sync::MutexGuard<'_, Inner>> {
        self.inner.lock().map_err(lock_error)
    }

    /// Acceso de lectura. No crea la base de datos: si todavía no existe,
    /// devuelve `None` y la interfaz muestra estados vacíos.
    pub fn read<T>(&self, f: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<Option<T>> {
        let guard = self.lock()?;
        match guard.conn.as_ref() {
            Some(conn) => Ok(Some(f(conn)?)),
            None => Ok(None),
        }
    }

    /// Acceso de lectura con valor por defecto cuando no hay base de datos.
    pub fn read_or<T: Default>(&self, f: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        Ok(self.read(f)?.unwrap_or_default())
    }

    /// Acceso de escritura. Crea la base de datos si es la primera operación
    /// que necesita persistencia.
    pub fn write<T>(&self, f: impl FnOnce(&mut Connection) -> AppResult<T>) -> AppResult<T> {
        let mut guard = self.lock()?;
        if guard.conn.is_none() {
            let conn = db::create_new(&self.db_path())?;
            guard.conn = Some(conn);
        }
        let conn = guard.conn.as_mut().expect("conexión recién creada");
        f(conn)
    }

    /// Acceso de escritura que no crea la base de datos. Se usa para tareas
    /// de mantenimiento que sólo tienen sentido cuando ya hay información.
    pub fn write_existing<T>(
        &self,
        f: impl FnOnce(&mut Connection) -> AppResult<T>,
    ) -> AppResult<Option<T>> {
        let mut guard = self.lock()?;
        let Some(conn) = guard.conn.as_mut() else {
            return Ok(None);
        };
        Ok(Some(f(conn)?))
    }

    /// Reemplaza la base de datos actual por una copia de seguridad.
    /// Antes de reemplazar, se crea una copia preventiva automática.
    pub fn restore_from(&self, source: &std::path::Path) -> AppResult<i64> {
        let version = backup::validate_source(source)?;
        let path = self.db_path();

        // 1. Copia preventiva de los datos actuales.
        {
            let guard = self.lock()?;
            if let Some(conn) = guard.conn.as_ref() {
                let preventive = self.data_dir.join(format!(
                    "aureon-antes-de-restaurar-{}.db",
                    crate::core::dates::now_str().replace([':', ' ', 'T'], "-")
                ));
                backup::create_snapshot(conn, &preventive)?;
            }
        }

        // 2. Se cierra la conexión actual para liberar el archivo.
        {
            let mut guard = self.lock()?;
            guard.conn = None;
        }
        backup::cleanup_wal(&path);

        // 3. Se reemplaza el archivo.
        backup::copy_file(source, &path)?;

        // 4. Se vuelve a abrir y a migrar.
        let mut conn = db::open_existing(&path)?;
        crate::core::migrations::migrate(&mut conn)?;
        let mut guard = self.lock()?;
        guard.conn = Some(conn);
        Ok(version)    }
}

fn lock_error<T>(_: T) -> AppError {
    AppError::other("La aplicación está ocupada con otra operación. Inténtalo de nuevo.")
}
