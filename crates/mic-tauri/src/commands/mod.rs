//! Comandos Tauri de MIC (ver `CONTRACT.md` en la raíz del repo).
//!
//! Cada submódulo agrupa los comandos de un área del contrato. Todos los
//! comandos devuelven `Result<T, String>`: el `String` es un mensaje en
//! español listo para mostrar en el frontend. Las operaciones de base de datos
//! (rusqlite es síncrono) se ejecutan en `tokio::task::spawn_blocking` para no
//! bloquear el hilo del runtime async.

pub mod album;
pub mod campos;
pub mod exportar;
pub mod filtros;
pub mod grupos;
pub mod importar;
pub mod ligados;
pub mod migracion;
pub mod multidatos;
pub mod registros;
pub mod reportes;
pub mod thumbs;

use std::sync::Arc;

use mic_core::error::MicError;

use crate::state::{AlbumHandle, AppState};

/// Convierte un [`MicError`] en `String`, registrándolo antes vía `tracing`.
///
/// Centraliza el logging de errores del backend: cada fallo que cruza la
/// frontera hacia el frontend queda trazado con su contexto.
pub(crate) fn a_string(err: MicError) -> String {
    tracing::error!(error = %err, "error en comando");
    err.to_string()
}

/// Recupera el handle de un álbum abierto desde el estado, o devuelve un error.
pub(crate) fn handle(state: &AppState, album_id: u64) -> Result<Arc<AlbumHandle>, String> {
    state.obtener(album_id)
}

/// Ejecuta una operación de base de datos (síncrona) en el pool de hilos de
/// bloqueo de tokio, propagando su resultado.
///
/// `f` recibe el handle del álbum (clonado) y devuelve un `Result<T, MicError>`.
/// El error se traza y se convierte a `String` para el frontend.
pub(crate) async fn en_db<T, F>(handle: Arc<AlbumHandle>, f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&AlbumHandle) -> Result<T, MicError> + Send + 'static,
{
    tokio::task::spawn_blocking(move || f(&handle))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "tarea de base de datos cancelada o en pánico");
            format!("error interno: {e}")
        })?
        .map_err(a_string)
}
