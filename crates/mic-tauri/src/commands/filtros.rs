//! Comandos de filtros avanzados guardados (listas de condiciones con nombre).

use mic_core::model::CondicionFiltro;
use tauri::State;

use crate::commands::{en_db, handle};
use crate::state::AppState;

/// Lista los nombres de los filtros guardados (orden alfabético).
#[tauri::command]
pub async fn filtros_listar(
    state: State<'_, AppState>,
    album_id: u64,
) -> Result<Vec<String>, String> {
    let h = handle(&state, album_id)?;
    en_db(h, |h| {
        let conn = h.db.conn()?;
        mic_db::repo_filtros::listar(&conn)
    })
    .await
}

/// Obtiene las condiciones de un filtro guardado, en orden.
#[tauri::command]
pub async fn filtro_obtener(
    state: State<'_, AppState>,
    album_id: u64,
    nombre: String,
) -> Result<Vec<CondicionFiltro>, String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let conn = h.db.conn()?;
        mic_db::repo_filtros::obtener(&conn, &nombre)
    })
    .await
}

/// Guarda (reemplaza) un filtro con nombre `nombre` y sus condiciones.
#[tauri::command]
pub async fn filtro_guardar(
    state: State<'_, AppState>,
    album_id: u64,
    nombre: String,
    condiciones: Vec<CondicionFiltro>,
) -> Result<(), String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let mut conn = h.db.conn()?;
        mic_db::repo_filtros::guardar(&mut conn, &nombre, &condiciones)
    })
    .await
}

/// Elimina un filtro guardado por nombre.
#[tauri::command]
pub async fn filtro_eliminar(
    state: State<'_, AppState>,
    album_id: u64,
    nombre: String,
) -> Result<(), String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let conn = h.db.conn()?;
        mic_db::repo_filtros::eliminar(&conn, &nombre)
    })
    .await
}
