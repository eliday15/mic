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

/// Plazo máximo de la inspección completa. Muy holgado (la inspección real
/// tarda segundos); es la última red: pase lo que pase adentro, el usuario
/// SIEMPRE recibe un error en vez de un spinner infinito.
const PLAZO_INSPECCION: std::time::Duration = std::time::Duration::from_secs(180);

/// Plazo máximo de la migración completa.
const PLAZO_MIGRACION: std::time::Duration = std::time::Duration::from_secs(1800);

/// Mensaje de timeout con el último paso registrado y la ruta de la bitácora.
fn mensaje_plazo(operacion: &str, plazo: std::time::Duration) -> String {
    format!(
        "{operacion} tardó más de {} s y se canceló. Último paso: '{}'. \
         Envíe la bitácora de diagnóstico para revisarlo: {}",
        plazo.as_secs(),
        mic_migrator::diag::paso_actual(),
        mic_migrator::diag::ruta_log().display()
    )
}

/// Vigía de pasos: mientras la operación corre, emite el paso actual del
/// diagnóstico como evento `migracion-progreso` (cada ~600 ms, solo si cambió).
/// Así el diálogo muestra EN VIVO en qué paso va — y si algo se atora, la
/// pantalla dice exactamente dónde. Se detiene al soltar el `Sender` devuelto.
fn vigia_pasos(app: AppHandle) -> tokio::sync::oneshot::Sender<()> {
    let (tx, mut rx) = tokio::sync::oneshot::channel::<()>();
    tauri::async_runtime::spawn(async move {
        let mut ultimo = String::new();
        loop {
            match rx.try_recv() {
                Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {}
                _ => break, // el Sender se soltó: la operación terminó
            }
            let paso = mic_migrator::diag::paso_actual();
            if !paso.is_empty() && paso != ultimo {
                let _ = app.emit(
                    EVENTO_PROGRESO,
                    &ProgresoEvento {
                        fase: paso.clone(),
                        hechas: 0,
                        total: 0,
                    },
                );
                ultimo = paso;
            }
            tokio::time::sleep(std::time::Duration::from_millis(600)).await;
        }
    });
    tx
}

/// Inspecciona un `.mdb` sin migrarlo: tablas, campos, total estimado y si tiene
/// variantes. Sirve para la vista previa del asistente de migración.
///
/// Con plazo máximo y pasos en vivo: nunca puede dejar al usuario ante un
/// spinner mudo (la lección de v3.0.1–v3.0.4).
#[tauri::command]
pub async fn migracion_inspeccionar(
    app: AppHandle,
    ruta_mdb: String,
) -> Result<MdbInspeccion, String> {
    let ruta = PathBuf::from(ruta_mdb);
    let _vigia = vigia_pasos(app);
    let tarea = tokio::task::spawn_blocking(move || mic_migrator::inspeccionar(&ruta));
    match tokio::time::timeout(PLAZO_INSPECCION, tarea).await {
        Ok(res) => res
            .map_err(|e| format!("error interno al inspeccionar: {e}"))?
            .map_err(a_string),
        Err(_) => Err(mensaje_plazo("La inspección", PLAZO_INSPECCION)),
    }
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

    let _vigia = vigia_pasos(app);
    let tarea =
        tokio::task::spawn_blocking(move || mic_migrator::migrar(&origen, &destino, progreso));
    match tokio::time::timeout(PLAZO_MIGRACION, tarea).await {
        Ok(res) => res
            .map_err(|e| format!("error interno durante la migración: {e}"))?
            .map_err(a_string),
        Err(_) => Err(mensaje_plazo("La migración", PLAZO_MIGRACION)),
    }
}

