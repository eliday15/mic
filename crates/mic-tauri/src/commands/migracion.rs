//! Comandos de migración de álbumes Access/Jet (`.mdb`) a SQLite (`.micdb`).
//!
//! Delegan en `mic-migrator` (shell-out a mdbtools). `migracion_ejecutar` emite
//! el evento `migracion-progreso` por cada fase reportada por el migrador, para
//! que el frontend muestre una barra de progreso.

use std::path::PathBuf;

use mic_migrator::{MdbInspeccion, MigracionReporte};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::commands::a_string;

/// Nombre del evento de progreso de migración (ver CONTRACT.md).
const EVENTO_PROGRESO: &str = "migracion-progreso";

/// Payload del evento `migracion-progreso` (ver tabla de eventos del contrato).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgresoEvento {
    fase: String,
    hechas: u64,
    total: u64,
}

/// Nombres de los binarios de mdbtools que la migración necesita.
const BINARIOS_MDBTOOLS: &[&str] = &["mdb-tables", "mdb-export", "mdb-schema"];

/// Comprueba si mdbtools está disponible en el `PATH` del sistema.
///
/// Devuelve `true` solo si los tres binarios usados por el migrador existen y
/// son ejecutables. La búsqueda recorre las entradas de `PATH` (compatible con
/// macOS/Linux y, en Windows, con los `.exe` instalados en el `PATH`).
#[tauri::command]
pub async fn migracion_verificar_mdbtools() -> Result<bool, String> {
    let disponible = tokio::task::spawn_blocking(|| {
        BINARIOS_MDBTOOLS.iter().all(|bin| binario_en_path(bin))
    })
    .await
    .map_err(|e| format!("error interno al verificar mdbtools: {e}"))?;
    Ok(disponible)
}

/// Inspecciona un `.mdb` sin migrarlo: tablas, campos, total estimado y si tiene
/// variantes. Sirve para la vista previa del asistente de migración.
#[tauri::command]
pub async fn migracion_inspeccionar(ruta_mdb: String) -> Result<MdbInspeccion, String> {
    let ruta = PathBuf::from(ruta_mdb);
    tokio::task::spawn_blocking(move || mic_migrator::inspeccionar(&ruta))
        .await
        .map_err(|e| format!("error interno al inspeccionar: {e}"))?
        .map_err(a_string)
}

/// Ejecuta la migración completa de `ruta_mdb` a `ruta_destino` (`.micdb`),
/// emitiendo `migracion-progreso` en cada fase. Devuelve el reporte de paridad.
#[tauri::command]
pub async fn migracion_ejecutar(
    app: AppHandle,
    ruta_mdb: String,
    ruta_destino: String,
) -> Result<MigracionReporte, String> {
    let origen = PathBuf::from(ruta_mdb);
    let destino = PathBuf::from(ruta_destino);
    let app_emisor = app.clone();

    // El migrador invoca `progreso(fase, hechas, total)` a lo largo del proceso;
    // reenviamos cada paso como evento `migracion-progreso` al frontend.
    let progreso: mic_migrator::ProgresoMigracion =
        Box::new(move |fase: &str, hechas: u64, total: u64| {
            let payload = ProgresoEvento {
                fase: fase.to_string(),
                hechas,
                total,
            };
            if let Err(e) = app_emisor.emit(EVENTO_PROGRESO, &payload) {
                tracing::warn!(error = %e, "no se pudo emitir progreso de migración");
            }
        });

    tokio::task::spawn_blocking(move || mic_migrator::migrar(&origen, &destino, progreso))
        .await
        .map_err(|e| format!("error interno durante la migración: {e}"))?
        .map_err(a_string)
}

/// ¿Existe `nombre` (o `nombre.exe` en Windows) como ejecutable en el `PATH`?
fn binario_en_path(nombre: &str) -> bool {
    let path = match std::env::var_os("PATH") {
        Some(p) => p,
        None => return false,
    };
    let candidatos: &[String] = &candidatos_nombre(nombre);
    for dir in std::env::split_paths(&path) {
        for cand in candidatos {
            let completo = dir.join(cand);
            if completo.is_file() {
                return true;
            }
        }
    }
    false
}

/// Nombres de archivo candidatos para un binario, según la plataforma.
fn candidatos_nombre(nombre: &str) -> Vec<String> {
    if cfg!(windows) {
        vec![format!("{nombre}.exe"), nombre.to_string()]
    } else {
        vec![nombre.to_string()]
    }
}
