//! Comandos de estructura del álbum: campos configurables y prueba de fórmulas.
//!
//! Cualquier cambio de estructura (crear/editar/eliminar/reordenar campo)
//! refresca la lista de campos cacheada del handle y recompila el
//! [`MotorCalculo`](mic_core::calc::MotorCalculo) para que los recálculos y las
//! consultas posteriores reflejen la nueva forma del álbum.

use mic_core::calc::MotorCalculo;
use mic_core::error::MicError;
use mic_core::model::{CampoDef, CampoNuevo, TipoCampo, Valor, Valores};
use tauri::State;

use crate::commands::{a_string, en_db, handle};
use crate::state::{AlbumHandle, AppState};

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
        recalcular_si_calculado(h, &def)?;
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
        recalcular_si_calculado(h, &def)?;
        Ok(editado)
    })
    .await
}

/// Tras crear/editar un calculado (fórmula o tipo de resultado nuevos),
/// recalcula el álbum para que los valores guardados reflejen la definición.
fn recalcular_si_calculado(h: &AlbumHandle, def: &CampoNuevo) -> Result<(), MicError> {
    if def.tipo != TipoCampo::Calculado {
        return Ok(());
    }
    let campos = h.campos();
    let motor = h.motor.read().unwrap_or_else(|e| e.into_inner());
    let mut conn = h.db.conn()?;
    mic_db::repo_registros::recalcular_todo(&mut conn, &campos, motor.as_ref())?;
    Ok(())
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
/// de fórmulas). No persiste nada. `formato` es el del campo en edición:
/// `"fecha"` devuelve el resultado como fecha ISO.
#[tauri::command]
pub async fn formula_probar(
    state: State<'_, AppState>,
    album_id: u64,
    formula: String,
    valores: Valores,
    formato: Option<String>,
) -> Result<Valor, String> {
    let h = handle(&state, album_id)?;
    let campos = h.campos();
    tokio::task::spawn_blocking(move || {
        MotorCalculo::evaluar_formula_libre(&campos, &formula, &valores, formato.as_deref())
    })
    .await
    .map_err(|e| format!("error interno al evaluar fórmula: {e}"))?
    .map_err(a_string)
}
