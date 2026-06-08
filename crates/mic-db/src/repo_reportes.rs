//! Repositorio de configuraciones de reporte guardadas (tabla `reportes`).
//!
//! Un reporte guardado es una configuración de impresión (tipo, campos,
//! orientación, papel, …) serializada como JSON e identificada por nombre.
//! Guardar reemplaza por completo el reporte homónimo (upsert por nombre).

use mic_core::error::MicError;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::pool::{err_sql, Conn};

/// Una configuración de reporte guardada: nombre + JSON de configuración.
///
/// El `config` es opaco para el backend (lo interpreta el frontend): se guarda
/// y devuelve tal cual como [`serde_json::Value`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReporteGuardado {
    pub nombre: String,
    pub config: serde_json::Value,
}

/// Lista los reportes guardados (orden alfabético por nombre).
///
/// Las filas con `config_json` inválido o nulo se devuelven con `config`
/// igual a `serde_json::Value::Null` en lugar de fallar la consulta entera.
pub fn listar(conn: &Conn) -> Result<Vec<ReporteGuardado>, MicError> {
    let mut stmt = conn
        .prepare(
            "SELECT nombre, config_json FROM reportes \
             WHERE nombre IS NOT NULL ORDER BY nombre COLLATE NOCASE",
        )
        .map_err(err_sql)?;
    let filas = stmt
        .query_map([], |row| {
            let nombre: String = row.get(0)?;
            let crudo: Option<String> = row.get(1)?;
            let config = crudo
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or(serde_json::Value::Null);
            Ok(ReporteGuardado { nombre, config })
        })
        .map_err(err_sql)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err_sql)?;
    Ok(filas)
}

/// Guarda (upsert por nombre) la configuración de un reporte.
pub fn guardar(
    conn: &Conn,
    nombre: &str,
    config: &serde_json::Value,
) -> Result<(), MicError> {
    let json = serde_json::to_string(config)
        .map_err(|e| MicError::Invalido(format!("no se pudo serializar el reporte: {e}")))?;
    let filas = conn
        .execute(
            "UPDATE reportes SET config_json = ?2 WHERE nombre = ?1",
            params![nombre, json],
        )
        .map_err(err_sql)?;
    if filas == 0 {
        conn.execute(
            "INSERT INTO reportes (nombre, config_json) VALUES (?1, ?2)",
            params![nombre, json],
        )
        .map_err(err_sql)?;
    }
    Ok(())
}

/// Elimina un reporte guardado por nombre.
pub fn eliminar(conn: &Conn, nombre: &str) -> Result<(), MicError> {
    conn.execute("DELETE FROM reportes WHERE nombre = ?1", params![nombre])
        .map_err(err_sql)?;
    Ok(())
}
