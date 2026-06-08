//! Índice de búsqueda libre FTS5 (`principal_fts`).
//!
//! La tabla guarda su propio texto (no contentless), lo que permite borrar por
//! `rowid` sin conocer el contenido previo —imprescindible para el reindexado
//! incremental—. El `rowid` de cada fila FTS coincide con `_id_` del registro en
//! `principal`, de modo que el `JOIN` del query_builder
//! (`fts.rowid = principal._id_`) es directo.
//!
//! Tokenizer `unicode61 remove_diacritics 2`: "cafe" encuentra "café".
//!
//! Solo se indexa la tabla `principal` (la búsqueda libre opera sobre el
//! catálogo principal, igual que el buscador del original).

use mic_core::error::MicError;
use mic_core::model::{CampoDef, Tabla, TipoCampo};
use rusqlite::params;

use crate::pool::{err_sql, Conn};

/// Concatena el texto indexable de un registro principal: todos los campos de
/// texto/fecha/calculado de la tabla principal + los valores de sus multidatos.
///
/// Los campos numéricos/moneda se incluyen como su representación textual para
/// permitir buscar también por números sueltos.
fn texto_indexable(conn: &Conn, campos: &[CampoDef], id: i64) -> Result<String, MicError> {
    let principales: Vec<&CampoDef> = campos
        .iter()
        .filter(|c| matches!(c.tabla, Tabla::Principal))
        .collect();

    let mut partes: Vec<String> = Vec::new();

    // Columnas escalares de la fila principal.
    let columnas: Vec<&str> = principales
        .iter()
        .filter(|c| !matches!(c.tipo, TipoCampo::Multidato))
        .map(|c| c.col_fisica.as_str())
        .collect();

    if !columnas.is_empty() {
        let select_cols = columnas
            .iter()
            .map(|c| format!("CAST({c} AS TEXT)"))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!("SELECT {select_cols} FROM principal WHERE _id_ = ?1");
        let mut stmt = conn.prepare(&sql).map_err(err_sql)?;
        let fila: Option<Vec<Option<String>>> = stmt
            .query_row(params![id], |row| {
                let mut v = Vec::with_capacity(columnas.len());
                for i in 0..columnas.len() {
                    v.push(row.get::<_, Option<String>>(i)?);
                }
                Ok(v)
            })
            .ok();
        if let Some(vals) = fila {
            for v in vals.into_iter().flatten() {
                if !v.trim().is_empty() {
                    partes.push(v);
                }
            }
        }
    }

    // Valores de multidatos del registro (campos de la principal).
    let multi_ids: Vec<i64> = principales
        .iter()
        .filter(|c| matches!(c.tipo, TipoCampo::Multidato))
        .map(|c| c.id)
        .collect();
    if !multi_ids.is_empty() {
        let mut stmt = conn
            .prepare(
                "SELECT valor FROM multidatos \
                 WHERE reg_id = ?1 AND principal = 1 AND campo_id = ?2",
            )
            .map_err(err_sql)?;
        for cid in multi_ids {
            let filas = stmt
                .query_map(params![id, cid], |row| row.get::<_, String>(0))
                .map_err(err_sql)?;
            for v in filas {
                let v = v.map_err(err_sql)?;
                if !v.trim().is_empty() {
                    partes.push(v);
                }
            }
        }
    }

    Ok(partes.join(" "))
}

/// Actualiza (o crea) la entrada FTS del registro principal `id`.
///
/// Borra la fila anterior con ese `rowid` y reinserta el texto recalculado.
/// No-op para tablas que no sean principal (la búsqueda libre es solo principal).
pub fn actualizar_registro(conn: &Conn, campos: &[CampoDef], id: i64) -> Result<(), MicError> {
    let texto = texto_indexable(conn, campos, id)?;
    // Borrado por rowid (la tabla guarda su texto, así que no requiere el
    // contenido original) e inserción del texto recalculado.
    conn.execute("DELETE FROM principal_fts WHERE rowid = ?1", params![id])
        .map_err(err_sql)?;
    conn.execute(
        "INSERT INTO principal_fts (rowid, texto) VALUES (?1, ?2)",
        params![id, texto],
    )
    .map_err(err_sql)?;
    Ok(())
}

/// Elimina la entrada FTS de un registro principal.
pub fn eliminar_registro(conn: &Conn, id: i64) -> Result<(), MicError> {
    conn.execute("DELETE FROM principal_fts WHERE rowid = ?1", params![id])
        .map_err(err_sql)?;
    Ok(())
}

/// Reconstruye el índice FTS completo desde cero (tras migración o cambio de
/// estructura). Vacía la tabla y reindexa todos los registros principales.
pub fn reindexar(conn: &Conn, campos: &[CampoDef]) -> Result<(), MicError> {
    conn.execute_batch("DELETE FROM principal_fts;")
        .map_err(err_sql)?;
    let ids: Vec<i64> = {
        let mut stmt = conn
            .prepare("SELECT _id_ FROM principal ORDER BY _id_")
            .map_err(err_sql)?;
        let filas = stmt
            .query_map([], |row| row.get::<_, i64>(0))
            .map_err(err_sql)?;
        filas.collect::<Result<Vec<_>, _>>().map_err(err_sql)?
    };
    for id in ids {
        actualizar_registro(conn, campos, id)?;
    }
    Ok(())
}
