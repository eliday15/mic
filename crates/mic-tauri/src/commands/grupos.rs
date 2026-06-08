//! Comandos de grupos jerárquicos (hasta 3 niveles de agrupación).
//!
//! `grupo_arbol` resuelve los valores distintos por nivel con sus conteos,
//! anidados, para alimentar el `TreeView` del panel lateral.

use mic_core::model::{Grupo, NodoGrupo};
use tauri::State;

use crate::commands::{en_db, handle};
use crate::state::AppState;

/// Lista todos los grupos definidos en el álbum.
#[tauri::command]
pub async fn grupos_listar(
    state: State<'_, AppState>,
    album_id: u64,
) -> Result<Vec<Grupo>, String> {
    let h = handle(&state, album_id)?;
    en_db(h, |h| {
        let conn = h.db.conn()?;
        mic_db::repo_grupos::listar(&conn)
    })
    .await
}

/// Guarda un grupo (`id == 0` crea uno nuevo). Devuelve el id resultante.
#[tauri::command]
pub async fn grupo_guardar(
    state: State<'_, AppState>,
    album_id: u64,
    grupo: Grupo,
) -> Result<i64, String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let conn = h.db.conn()?;
        mic_db::repo_grupos::guardar(&conn, &grupo)
    })
    .await
}

/// Elimina un grupo por id.
#[tauri::command]
pub async fn grupo_eliminar(
    state: State<'_, AppState>,
    album_id: u64,
    grupo_id: i64,
) -> Result<(), String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let conn = h.db.conn()?;
        mic_db::repo_grupos::eliminar(&conn, grupo_id)
    })
    .await
}

/// Resuelve el árbol completo de un grupo (valores distintos por nivel + conteos).
#[tauri::command]
pub async fn grupo_arbol(
    state: State<'_, AppState>,
    album_id: u64,
    grupo_id: i64,
) -> Result<Vec<NodoGrupo>, String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let campos = h.campos();
        let conn = h.db.conn()?;
        mic_db::repo_grupos::arbol(&conn, &campos, grupo_id)
    })
    .await
}
