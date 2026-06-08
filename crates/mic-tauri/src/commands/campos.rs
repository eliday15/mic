//! Comandos de estructura del álbum: campos configurables y prueba de fórmulas.
//!
//! Cualquier cambio de estructura (crear/editar/eliminar/reordenar campo)
//! refresca la lista de campos cacheada del handle y recompila el
//! [`MotorCalculo`](mic_core::calc::MotorCalculo) para que los recálculos y las
//! consultas posteriores reflejen la nueva forma del álbum.

use mic_core::calc::MotorCalculo;
use mic_core::model::{CampoDef, CampoNuevo, Valor, Valores};
use tauri::State;

use crate::commands::{a_string, en_db, handle};
use crate::state::AppState;

/// Lista los campos del álbum (orden visible).
#[tauri::command]
pub async fn campos_listar(
    state: State<'_, AppState>,
    album_id: u64,
) -> Result<Vec<CampoDef>, String> {
    let h = handle(&state, album_id)?;
    en_db(h, |h| {
        let conn = h.db.conn()?;
        mic_db::repo_campos::listar(&conn)
    })
    .await
}

/// Crea un campo nuevo. Aplica DDL dinámico y refresca campos + motor.
#[tauri::command]
pub async fn campo_crear(
    state: State<'_, AppState>,
    album_id: u64,
    def: CampoNuevo,
) -> Result<CampoDef, String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let creado = {
            let conn = h.db.conn()?;
            mic_db::repo_campos::crear(&conn, &def)?
        };
        h.refrescar_campos()?;
        Ok(creado)
    })
    .await
}

/// Edita un campo existente (puede cambiar tipo, convirtiendo datos). Refresca
/// campos + motor.
#[tauri::command]
pub async fn campo_editar(
    state: State<'_, AppState>,
    album_id: u64,
    campo_id: i64,
    def: CampoNuevo,
) -> Result<CampoDef, String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let editado = {
            let conn = h.db.conn()?;
            mic_db::repo_campos::editar(&conn, campo_id, &def)?
        };
        h.refrescar_campos()?;
        Ok(editado)
    })
    .await
}

/// Elimina un campo (DROP COLUMN + limpieza). Refresca campos + motor.
#[tauri::command]
pub async fn campo_eliminar(
    state: State<'_, AppState>,
    album_id: u64,
    campo_id: i64,
) -> Result<(), String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        {
            let conn = h.db.conn()?;
            mic_db::repo_campos::eliminar(&conn, campo_id)?;
        }
        h.refrescar_campos()
    })
    .await
}

/// Reordena los campos según `orden` (ids en el orden visible deseado).
#[tauri::command]
pub async fn campos_reordenar(
    state: State<'_, AppState>,
    album_id: u64,
    orden: Vec<i64>,
) -> Result<(), String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        {
            let conn = h.db.conn()?;
            mic_db::repo_campos::reordenar(&conn, &orden)?;
        }
        h.refrescar_campos()
    })
    .await
}

/// Evalúa una fórmula suelta contra valores de prueba (vista previa del editor
/// de fórmulas). No persiste nada.
#[tauri::command]
pub async fn formula_probar(
    state: State<'_, AppState>,
    album_id: u64,
    formula: String,
    valores: Valores,
) -> Result<Valor, String> {
    let h = handle(&state, album_id)?;
    let campos = h.campos();
    tokio::task::spawn_blocking(move || {
        MotorCalculo::evaluar_formula_libre(&campos, &formula, &valores)
    })
    .await
    .map_err(|e| format!("error interno al evaluar fórmula: {e}"))?
    .map_err(a_string)
}
