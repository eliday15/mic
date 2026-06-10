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

/// Comprueba si mdbtools está disponible para el migrador.
///
/// Delega en `mic_migrator::mdbtools::disponible()`, la MISMA lógica de
/// localización que usan inspeccionar/migrar: binarios empaquetados con la app
/// (Windows), rutas típicas del sistema (Homebrew…) y `PATH`. Importante: este
/// comando no debe tener una búsqueda propia — tener dos fuentes de verdad fue
/// justo el bug que bloqueaba la importación en Windows aunque los binarios
/// embebidos estuvieran instalados.
#[tauri::command]
pub async fn migracion_verificar_mdbtools() -> Result<bool, String> {
    tokio::task::spawn_blocking(mic_migrator::mdbtools::disponible)
        .await
        .map_err(|e| format!("error interno al verificar mdbtools: {e}"))
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

