//! Comandos de categorías para autocomplete de campos multidato.
//!
//! Las categorías se gestionan por campo y por tabla (principal/variantes). El
//! parámetro `principal: bool` del contrato indica la tabla destino.

use mic_core::model::CategoriaVal;
use tauri::State;

use crate::commands::{en_db, handle};
use crate::state::AppState;

/// Límite de sugerencias devueltas por prefijo (suficiente para el desplegable).
const LIMITE_SUGERENCIAS: u32 = 50;

/// Sugiere valores de categoría cuyo texto empieza por `prefijo`.
#[tauri::command]
pub async fn categorias_sugerir(
    state: State<'_, AppState>,
    album_id: u64,
    campo_id: i64,
    principal: bool,
    prefijo: String,
) -> Result<Vec<String>, String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let conn = h.db.conn()?;
        mic_db::repo_categorias::sugerir(&conn, campo_id, principal, &prefijo, LIMITE_SUGERENCIAS)
    })
    .await
}

/// Lista todas las categorías de un campo (con su marca de predeterminado).
#[tauri::command]
pub async fn categorias_listar(
    state: State<'_, AppState>,
    album_id: u64,
    campo_id: i64,
    principal: bool,
) -> Result<Vec<CategoriaVal>, String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let conn = h.db.conn()?;
        mic_db::repo_categorias::listar(&conn, campo_id, principal)
    })
    .await
}

/// Reemplaza el conjunto de categorías de un campo por `valores`.
#[tauri::command]
pub async fn categorias_actualizar(
    state: State<'_, AppState>,
    album_id: u64,
    campo_id: i64,
    principal: bool,
    valores: Vec<CategoriaVal>,
) -> Result<(), String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let mut conn = h.db.conn()?;
        mic_db::repo_categorias::actualizar(&mut conn, campo_id, principal, &valores)
    })
    .await
}
