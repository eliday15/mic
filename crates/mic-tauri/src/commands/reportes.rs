//! Comandos de configuraciones de reporte guardadas (impresión/reportes).
//!
//! Cada reporte es una configuración de impresión serializada como JSON,
//! identificada por nombre. Replica las plantillas de frmprint/frmprint2 del
//! original (ex tablas de reportes CI/SI).

use tauri::State;

use mic_db::repo_reportes::ReporteGuardado;

use crate::commands::{en_db, handle};
use crate::state::AppState;

/// Lista los reportes guardados del álbum (orden alfabético por nombre).
#[tauri::command]
pub async fn reportes_listar(
    state: State<'_, AppState>,
    album_id: u64,
) -> Result<Vec<ReporteGuardado>, String> {
    let h = handle(&state, album_id)?;
    en_db(h, |h| {
        let conn = h.db.conn()?;
        mic_db::repo_reportes::listar(&conn)
    })
    .await
}

/// Guarda (upsert por nombre) la configuración de un reporte.
#[tauri::command]
pub async fn reporte_guardar(
    state: State<'_, AppState>,
    album_id: u64,
    nombre: String,
    config: serde_json::Value,
) -> Result<(), String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let conn = h.db.conn()?;
        mic_db::repo_reportes::guardar(&conn, &nombre, &config)
    })
    .await
}

/// Elimina un reporte guardado por nombre.
#[tauri::command]
pub async fn reporte_eliminar(
    state: State<'_, AppState>,
    album_id: u64,
    nombre: String,
) -> Result<(), String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let conn = h.db.conn()?;
        mic_db::repo_reportes::eliminar(&conn, &nombre)
    })
    .await
}
