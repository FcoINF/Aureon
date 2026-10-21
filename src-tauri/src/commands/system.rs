//! Comandos de sistema: ajustes, copias de seguridad, exportaciones y archivos.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager, State};

use crate::core::dates;
use crate::core::error::AppError;
use crate::core::models::{AppInfo, Attachment, BackupResult, TransactionFilter};
use crate::core::repos;
use crate::core::services::{backup, excel, export, settings};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Ajustes
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<settings::AppSettings, String> {
    state
        .read_or(settings::load)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_settings(
    state: State<'_, AppState>,
    settings: settings::AppSettings,
) -> Result<settings::AppSettings, String> {
    state
        .write(|conn| settings::save(conn, &settings))
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Información de la aplicación
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn app_info(state: State<'_, AppState>) -> AppInfo {
    let db_path = state.db_path();
    let schema_version = state
        .read(crate::core::migrations::current_version)
        .ok()
        .flatten()
        .unwrap_or(0);
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        data_dir: state.data_dir().to_string_lossy().to_string(),
        database_path: db_path.to_string_lossy().to_string(),
        database_exists: db_path.exists(),
        schema_version,
        has_data: state.has_db(),
    }
}

/// Actualiza los estados vencidos de cuotas, obligaciones y suscripciones.
/// No crea la base de datos: si todavía no hay información, no hace nada.
#[tauri::command]
pub fn refresh_overdue_status(state: State<'_, AppState>) -> Result<(), String> {
    let today = dates::today_str();
    state
        .write_existing(|conn| {
            repos::subscriptions::refresh_overdue(conn, &today)?;
            repos::maintenance::refresh(conn, &today)
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Exportaciones
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn export_dataset(
    state: State<'_, AppState>,
    dataset: String,
    filter: Option<TransactionFilter>,
) -> Result<String, String> {
    state
        .read(|conn| {
            let content = match dataset.as_str() {
                "transactions" => export::transactions_csv(conn, &filter.unwrap_or_default())?,
                "purchases" => export::purchases_csv(conn)?,
                "installments" => export::installments_csv(conn)?,
                "subscriptions" => export::subscriptions_csv(conn)?,
                "receivables" => export::receivables_csv(conn)?,
                "payables" => export::payables_csv(conn)?,
                "deposits" => export::deposits_csv(conn)?,
                "wishlist" => export::wishlist_csv(conn)?,
                other => {
                    return Err(AppError::validation(format!(
                        "No se reconoce el conjunto de datos «{other}»."
                    )))
                }
            };
            Ok(content)
        })?
        .ok_or_else(|| "Todavía no hay información para exportar.".to_string())
}

/// Genera y guarda el reporte financiero en formato Excel (.xlsx).
///
/// El libro se construye en Rust para poder incluir estilos, filtros y
/// gráficos reales, algo que un CSV no puede representar.
#[tauri::command]
pub fn export_excel_report(
    state: State<'_, AppState>,
    path: String,
    filter: TransactionFilter,
    currency: String,
    period_label: Option<String>,
) -> Result<String, String> {
    let target = PathBuf::from(&path);
    let extension = target
        .extension()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if extension != "xlsx" {
        return Err("El reporte debe guardarse con extensión .xlsx.".to_string());
    }

    state
        .read(|conn| {
            excel::report(
                conn,
                &target,
                &filter,
                &currency,
                period_label.as_deref().unwrap_or("Período"),
            )
        })
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Todavía no hay información para exportar.".to_string())?;

    notify_shell_file_created(&target);
    Ok(target.to_string_lossy().to_string())
}

/// Escribe un archivo de texto en la ruta indicada (usado por las exportaciones).
#[tauri::command]
pub fn write_text_file(path: String, contents: String) -> Result<String, String> {
    let target = PathBuf::from(&path);
    let extension = target
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if !["csv", "txt", "json", "pdf"].contains(&extension.as_str()) {
        return Err("El archivo debe ser un documento CSV, PDF o de texto.".to_string());
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("No se pudo crear la carpeta de destino: {e}"))?;
    }
    std::fs::write(&target, contents.as_bytes())
        .map_err(|e| format!("No se pudo guardar el archivo: {e}"))?;
    notify_shell_file_created(&target);
    Ok(target.to_string_lossy().to_string())
}

/// Escribe un archivo binario a partir de su contenido en base64.
/// Se usa para guardar los reportes en PDF generados en la interfaz.
#[tauri::command]
pub fn write_base64_file(path: String, contents: String) -> Result<String, String> {
    let target = PathBuf::from(&path);
    let extension = target
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if !["pdf", "csv", "bin"].contains(&extension.as_str()) {
        return Err("El archivo debe ser un documento PDF.".to_string());
    }
    let bytes = decode_base64(&contents)?;
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("No se pudo crear la carpeta de destino: {e}"))?;
    }
    std::fs::write(&target, bytes)
        .map_err(|e| format!("No se pudo guardar el archivo: {e}"))?;
    notify_shell_file_created(&target);
    Ok(target.to_string_lossy().to_string())
}

/// Avisa a Explorer que se creó un archivo para que actualice la carpeta.
///
/// Sin este aviso, al guardar en el escritorio (especialmente si está
/// sincronizado con OneDrive) el archivo puede tardar en aparecer hasta que
/// el usuario actualiza manualmente la vista.
#[cfg(target_os = "windows")]
fn notify_shell_file_created(path: &Path) {
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "shell32")]
    extern "system" {
        fn SHChangeNotify(
            event_id: i32,
            flags: u32,
            item1: *const std::ffi::c_void,
            item2: *const std::ffi::c_void,
        );
    }

    const SHCNE_CREATE: i32 = 0x0000_0002;
    const SHCNE_UPDATEDIR: i32 = 0x0000_1000;
    const SHCNF_PATHW: u32 = 0x0005;

    unsafe {
        if let Some(parent) = path.parent() {
            let folder: Vec<u16> = parent.as_os_str().encode_wide().chain(Some(0)).collect();
            SHChangeNotify(
                SHCNE_UPDATEDIR,
                SHCNF_PATHW,
                folder.as_ptr() as *const std::ffi::c_void,
                std::ptr::null(),
            );
        }
        let file: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        SHChangeNotify(
            SHCNE_CREATE,
            SHCNF_PATHW,
            file.as_ptr() as *const std::ffi::c_void,
            std::ptr::null(),
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn notify_shell_file_created(_path: &Path) {}

/// Decodifica texto en base64 estándar.
fn decode_base64(input: &str) -> Result<Vec<u8>, String> {
    const TABLA: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let limpio: Vec<u8> = input
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();
    let mut salida = Vec::with_capacity(limpio.len() * 3 / 4);
    let mut buffer: u32 = 0;
    let mut bits: u32 = 0;
    for byte in limpio {
        if byte == b'=' {
            break;
        }
        let valor = TABLA
            .iter()
            .position(|c| *c == byte)
            .ok_or_else(|| "El contenido del archivo no es válido.".to_string())?
            as u32;
        buffer = (buffer << 6) | valor;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            salida.push(((buffer >> bits) & 0xFF) as u8);
        }
    }
    Ok(salida)
}

// ---------------------------------------------------------------------------
// Copias de seguridad
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn create_backup(
    state: State<'_, AppState>,
    path: String,
) -> Result<BackupResult, String> {
    let target = PathBuf::from(&path);
    let result = state
        .write(|conn| backup::create_snapshot(conn, &target))
        .map_err(|e| e.to_string())?;
    notify_shell_file_created(&target);
    Ok(result)
}

#[tauri::command]
pub fn validate_backup_file(path: String) -> Result<i64, String> {
    backup::validate_source(Path::new(&path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn restore_backup(state: State<'_, AppState>, path: String) -> Result<i64, String> {
    state
        .restore_from(Path::new(&path))
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Archivos adjuntos y enlaces externos
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn import_attachment(
    state: State<'_, AppState>,
    source_path: String,
) -> Result<Attachment, String> {
    let source = PathBuf::from(&source_path);
    if !source.is_file() {
        return Err("El archivo seleccionado no existe.".to_string());
    }
    let file_name = source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "documento".to_string());
    let extension = source
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();
    let stored_name = format!("{}.{}", uuid::Uuid::new_v4(), extension);
    let size = source.metadata().map(|m| m.len() as i64).unwrap_or(0);

    let attachments_dir = state.data_dir().join("attachments");
    std::fs::create_dir_all(&attachments_dir)
        .map_err(|e| format!("No se pudo crear la carpeta de adjuntos: {e}"))?;
    let destination = attachments_dir.join(&stored_name);
    std::fs::copy(&source, &destination)
        .map_err(|e| format!("No se pudo guardar el documento: {e}"))?;

    let attachment = Attachment {
        id: uuid::Uuid::new_v4().to_string(),
        file_name,
        stored_name: stored_name.clone(),
        mime_type: None,
        size_bytes: size,
        created_at: dates::now_str(),
    };
    Ok(attachment)
}

/// Abre un documento guardado por el usuario.
#[tauri::command]
pub fn open_attachment(state: State<'_, AppState>, stored_name: String) -> Result<(), String> {
    let path = state.data_dir().join("attachments").join(&stored_name);
    if !path.is_file() {
        return Err("El documento ya no está disponible.".to_string());
    }
    open_path(&path).map_err(|e| e.to_string())
}

/// Abre un enlace externo. Sólo se permiten http y https.
#[tauri::command]
pub fn open_external_url(app: AppHandle, url: String) -> Result<(), String> {
    let trimmed = url.trim();
    let lower = trimmed.to_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return Err("Sólo se pueden abrir enlaces que comienzan con http:// o https://.".to_string());
    }
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(trimmed, None::<&str>)
        .map_err(|_| "No se pudo abrir el enlace en el navegador.".to_string())
}

fn open_path(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("No se pudo abrir el documento: {e}"))
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("No se pudo abrir el documento: {e}"))
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("No se pudo abrir el documento: {e}"))
    }
}

/// Ruta sugerida para guardar una copia de seguridad. Respeta la carpeta
/// configurada por el usuario en los ajustes.
#[tauri::command]
pub fn default_backup_path(state: State<'_, AppState>) -> String {
    let nombre = format!("aureon-copia-{}.db", dates::today_str());
    let carpeta = state
        .read(crate::core::services::settings::load)
        .ok()
        .flatten()
        .and_then(|settings| settings.backup_folder)
        .filter(|value| !value.trim().is_empty());
    match carpeta {
        Some(carpeta) => PathBuf::from(carpeta).join(nombre).to_string_lossy().to_string(),
        None => state.data_dir().join(nombre).to_string_lossy().to_string(),
    }
}

/// Datos de la aplicación para la sección de información.
#[tauri::command]
pub fn app_data_dir(app: AppHandle) -> Result<String, String> {
    let dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("No se pudo obtener la carpeta de datos: {e}"))?;
    Ok(dir.to_string_lossy().to_string())
}
