//! Comandos de migración de álbumes Access/Jet (`.mdb`) a SQLite (`.micdb`).
//!
//! Delegan en `mic-migrator`, que lee los `.mdb` en proceso con el crate
//! pure-Rust `jetdb` (sin binarios externos). `migracion_ejecutar` emite el
//! evento `migracion-progreso` por cada fase reportada por el migrador, para
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

/// Comprueba si el lector de `.mdb` está disponible.
///
/// Desde MIC 3.0.4 la lectura de Access es **en proceso** (crate pure-Rust
/// `jetdb`, compilado dentro de la app): ya no hay binarios externos que puedan
/// faltar, así que siempre está disponible. El comando se conserva porque el
/// frontend lo invoca antes de mostrar el asistente de migración; devolver
/// `true` mantiene esa comprobación inofensiva sin romper el contrato.
#[tauri::command]
pub async fn migracion_verificar_mdbtools() -> Result<bool, String> {
    Ok(true)
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

