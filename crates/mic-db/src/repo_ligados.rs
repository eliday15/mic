//! Repositorio de álbumes ligados (tabla `ligados` del esquema base).
//!
//! Una "liga" sincroniza datos DESDE otro álbum `.micdb` HACIA el actual usando
//! un campo llave común (ex-frmAlbumsL/frmEdligado/frmstligas del VB6). El DDL de
//! `ligados` es genérico (`id`, `nombre`, `config_json`), así que la
//! configuración completa de la liga (`ruta_album`, `llave`, `crear_faltantes`,
//! `fotos`, `importar_campos_nuevos`) se serializa como JSON en `config_json`;
//! `nombre` guarda la ruta del álbum ligado para facilitar listados sin
//! deserializar. Las opciones añadidas después llevan `#[serde(default)]` para
//! que las ligas guardadas antes sigan cargando.

use mic_core::error::MicError;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::pool::{err_sql, Conn};

/// Qué foto conservar cuando el registro actual y el ligado tienen imagen.
///
/// Si el actual no tiene foto, la del ligado se importa siempre.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FuenteFotos {
    /// Conservar la foto del álbum actual (por defecto).
    #[default]
    Actual,
    /// Reemplazarla por la foto del álbum ligado.
    Ligado,
}

/// Una liga: sincroniza datos desde `ruta_album` usando el campo `llave`.
///
/// `crear_faltantes`: si una llave del álbum ligado no existe en el actual,
/// crea el registro ("dar de alta si no existe").
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Liga {
    /// Id en la tabla `ligados`. `0` al crear (lo asigna la base).
    #[serde(default)]
    pub id: i64,
    /// Ruta absoluta del álbum `.micdb` del que se copian los datos.
    pub ruta_album: String,
    /// Nombre visible del campo llave común a ambos álbumes.
    pub llave: String,
    /// Si la llave del ligado no existe en el actual, ¿dar de alta el registro?
    #[serde(default)]
    pub crear_faltantes: bool,
    /// Foto a conservar cuando ambos registros tienen imagen.
    #[serde(default)]
    pub fotos: FuenteFotos,
    /// Crear en el álbum actual los campos que solo existen en el ligado (mismo
    /// tipo y decimales) y copiar sus valores.
    #[serde(default)]
    pub importar_campos_nuevos: bool,
}

/// Forma serializada en `config_json` (sin el `id`, que vive en la columna).
#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfigLiga {
    #[serde(default)]
    ruta_album: String,
    #[serde(default)]
    llave: String,
    #[serde(default)]
    crear_faltantes: bool,
    #[serde(default)]
    fotos: FuenteFotos,
    #[serde(default)]
    importar_campos_nuevos: bool,
}

/// Mapea una fila de `ligados` a [`Liga`], deserializando `config_json`.
fn map_liga(row: &rusqlite::Row) -> rusqlite::Result<Liga> {
    let id: i64 = row.get("id")?;
    let config_json: Option<String> = row.get("config_json")?;
    let cfg: ConfigLiga = config_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    Ok(Liga {
        id,
        ruta_album: cfg.ruta_album,
        llave: cfg.llave,
        crear_faltantes: cfg.crear_faltantes,
        fotos: cfg.fotos,
        importar_campos_nuevos: cfg.importar_campos_nuevos,
    })
}

/// Lista todas las ligas definidas en el álbum, en orden de id.
pub fn listar(conn: &Conn) -> Result<Vec<Liga>, MicError> {
    let mut stmt = conn
        .prepare("SELECT id, nombre, config_json FROM ligados ORDER BY id")
        .map_err(err_sql)?;
    let filas = stmt
        .query_map([], map_liga)
        .map_err(err_sql)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err_sql)?;
    Ok(filas)
}

/// Obtiene una liga por id.
pub fn obtener(conn: &Conn, liga_id: i64) -> Result<Liga, MicError> {
    let mut stmt = conn
        .prepare("SELECT id, nombre, config_json FROM ligados WHERE id = ?1")
        .map_err(err_sql)?;
    stmt.query_row(params![liga_id], map_liga)
        .optional()
        .map_err(err_sql)?
        .ok_or_else(|| MicError::NoEncontrado(format!("liga id={liga_id}")))
}

/// Guarda una liga: `id == 0` crea una nueva, en otro caso edita. Devuelve el id.
///
/// `nombre` replica la ruta del álbum (búsquedas rápidas); `config_json` lleva
/// la configuración completa serializada.
pub fn guardar(conn: &Conn, liga: &Liga) -> Result<i64, MicError> {
    let cfg = ConfigLiga {
        ruta_album: liga.ruta_album.clone(),
        llave: liga.llave.clone(),
        crear_faltantes: liga.crear_faltantes,
        fotos: liga.fotos,
        importar_campos_nuevos: liga.importar_campos_nuevos,
    };
    let json = serde_json::to_string(&cfg)
        .map_err(|e| MicError::Invalido(format!("no se pudo serializar la liga: {e}")))?;

    if liga.id == 0 {
        conn.execute(
            "INSERT INTO ligados (nombre, config_json) VALUES (?1, ?2)",
            params![liga.ruta_album, json],
        )
        .map_err(err_sql)?;
        Ok(conn.last_insert_rowid())
    } else {
        conn.execute(
            "UPDATE ligados SET nombre = ?1, config_json = ?2 WHERE id = ?3",
            params![liga.ruta_album, json, liga.id],
        )
        .map_err(err_sql)?;
        Ok(liga.id)
    }
}

/// Elimina una liga por id.
pub fn eliminar(conn: &Conn, liga_id: i64) -> Result<(), MicError> {
    conn.execute("DELETE FROM ligados WHERE id = ?1", params![liga_id])
        .map_err(err_sql)?;
    Ok(())
}
