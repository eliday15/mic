//! Repositorio de categorías para autocomplete de multidatos (tabla
//! `Categorias` del original). Por campo y por tabla (principal/variantes) se
//! guardan valores sugeridos, uno de ellos marcable como predeterminado.

use mic_core::error::MicError;
use mic_core::model::CategoriaVal;
use rusqlite::params;

use crate::pool::{err_sql, Conn};

/// Sugiere valores de categoría cuyo texto empieza por `prefijo` (case-insensitive),
/// hasta `limit` resultados. Si `prefijo` está vacío, devuelve los primeros valores.
pub fn sugerir(
    conn: &Conn,
    campo_id: i64,
    principal: bool,
    prefijo: &str,
    limit: u32,
) -> Result<Vec<String>, MicError> {
    let pr = principal as i64;
    let patron = format!("{}%", prefijo.trim());
    let mut stmt = conn
        .prepare(
            "SELECT valor FROM categorias \
             WHERE campo_id = ?1 AND principal = ?2 AND valor LIKE ?3 COLLATE NOCASE \
             ORDER BY es_default DESC, valor COLLATE NOCASE LIMIT ?4",
        )
        .map_err(err_sql)?;
    let filas = stmt
        .query_map(params![campo_id, pr, patron, limit], |row| {
            row.get::<_, String>(0)
        })
        .map_err(err_sql)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err_sql)?;
    Ok(filas)
}

/// Lista todas las categorías de un campo (con su marca de predeterminado).
pub fn listar(
    conn: &Conn,
    campo_id: i64,
    principal: bool,
) -> Result<Vec<CategoriaVal>, MicError> {
    let pr = principal as i64;
    let mut stmt = conn
        .prepare(
            "SELECT valor, es_default FROM categorias \
             WHERE campo_id = ?1 AND principal = ?2 \
             ORDER BY es_default DESC, valor COLLATE NOCASE",
        )
        .map_err(err_sql)?;
    let filas = stmt
        .query_map(params![campo_id, pr], |row| {
            Ok(CategoriaVal {
                valor: row.get(0)?,
                es_default: row.get::<_, i64>(1)? != 0,
            })
        })
        .map_err(err_sql)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err_sql)?;
    Ok(filas)
}

/// Reemplaza el conjunto de categorías de un campo por `valores`.
/// (Borrado total + reinserción; transacción simple.)
pub fn actualizar(
    conn: &mut Conn,
    campo_id: i64,
    principal: bool,
    valores: &[CategoriaVal],
) -> Result<(), MicError> {
    let pr = principal as i64;
    let tx = conn.transaction().map_err(err_sql)?;
    tx.execute(
        "DELETE FROM categorias WHERE campo_id = ?1 AND principal = ?2",
        params![campo_id, pr],
    )
    .map_err(err_sql)?;
    {
        let mut stmt = tx
            .prepare(
                "INSERT INTO categorias (campo_id, principal, valor, es_default) \
                 VALUES (?1, ?2, ?3, ?4)",
            )
            .map_err(err_sql)?;
        for c in valores {
            let v = c.valor.trim();
            if v.is_empty() {
                continue;
            }
            stmt.execute(params![campo_id, pr, v, c.es_default as i64])
                .map_err(err_sql)?;
        }
    }
    tx.commit().map_err(err_sql)?;
    Ok(())
}

/// Registra (si no existe) un valor como categoría para un campo. Útil al guardar
/// un registro con un multidato nuevo, para alimentar el autocomplete.
pub fn asegurar_valor(
    conn: &Conn,
    campo_id: i64,
    principal: bool,
    valor: &str,
) -> Result<(), MicError> {
    let v = valor.trim();
    if v.is_empty() {
        return Ok(());
    }
    let pr = principal as i64;
    let existe: bool = conn
        .query_row(
            "SELECT 1 FROM categorias \
             WHERE campo_id = ?1 AND principal = ?2 AND valor = ?3 LIMIT 1",
            params![campo_id, pr, v],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if !existe {
        conn.execute(
            "INSERT INTO categorias (campo_id, principal, valor, es_default) \
             VALUES (?1, ?2, ?3, 0)",
            params![campo_id, pr, v],
        )
        .map_err(err_sql)?;
    }
    Ok(())
}
