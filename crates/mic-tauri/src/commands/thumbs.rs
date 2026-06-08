//! Comandos de miniaturas. El fetch de miniaturas no es un comando: se sirve
//! por el protocolo custom `thumb://` (ver `lib.rs`). Aquí solo vive la
//! invalidación de la caché de un registro tras cambiar su imagen.

use mic_core::error::MicError;
use mic_core::model::Tabla;
use tauri::State;

use crate::commands::{en_db, handle};
use crate::state::{AlbumHandle, AppState};

/// Resuelve la ruta absoluta de la imagen de un registro, o `None` si no tiene.
///
/// La columna `_imagen_` guarda una ruta relativa (`imagenes/<archivo>`); se
/// combina con el directorio del álbum para obtener la ruta absoluta.
pub(crate) fn ruta_imagen_abs(
    handle: &AlbumHandle,
    tabla: Tabla,
    id: i64,
) -> Result<Option<std::path::PathBuf>, MicError> {
    let conn = handle.db.conn()?;
    let alias = tabla.nombre();
    let rel: Option<String> = conn
        .query_row(
            &format!("SELECT _imagen_ FROM {alias} WHERE _id_ = ?1"),
            [id],
            |row| row.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                MicError::NoEncontrado(format!("registro {alias} id={id}"))
            }
            otro => MicError::Db(otro.to_string()),
        })?;

    let rel = match rel {
        Some(r) if !r.trim().is_empty() => r,
        _ => return Ok(None),
    };

    let base = handle
        .db
        .ruta()
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let limpio = rel.trim_start_matches('/');
    Ok(Some(base.join(limpio)))
}

/// Invalida las miniaturas cacheadas de un registro (tras cambiar su imagen).
#[tauri::command]
pub async fn thumb_invalidar(
    state: State<'_, AppState>,
    album_id: u64,
    id: i64,
    tabla: Tabla,
) -> Result<(), String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        if let Some(abs) = ruta_imagen_abs(h, tabla, id)? {
            h.thumbs.invalidar(&abs);
        }
        Ok(())
    })
    .await
}
