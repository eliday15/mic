//! Repositorio de campos multi-valor (tabla `multidatos` del original).
//!
//! Cada registro puede tener N valores para un campo de tipo Multidato. La
//! columna física del campo en `principal`/`variantes` guarda el conteo (para
//! orden/filtro baratos); los valores reales viven aquí. El patrón de guardado
//! replica `GuardaMultid` (db.bas): borrar todos los valores previos del par
//! (registro, campo) y reinsertar la lista nueva.

use mic_core::error::MicError;
use rusqlite::params;

use crate::pool::{err_sql, Conn};

/// Lista los valores de un campo multidato para un registro concreto.
///
/// `principal = true` → tabla principal; `false` → variantes.
pub fn listar(
    conn: &Conn,
    reg_id: i64,
    campo_id: i64,
    principal: bool,
) -> Result<Vec<String>, MicError> {
    let pr = principal as i64;
    let mut stmt = conn
        .prepare(
            "SELECT valor FROM multidatos \
             WHERE reg_id = ?1 AND campo_id = ?2 AND principal = ?3 \
             ORDER BY rowid",
        )
        .map_err(err_sql)?;
    let filas = stmt
        .query_map(params![reg_id, campo_id, pr], |row| row.get::<_, String>(0))
        .map_err(err_sql)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err_sql)?;
    Ok(filas)
}

/// Reemplaza por completo los valores de un campo multidato para un registro:
/// borra los previos y reinserta `valores` (filtrando vacíos). Devuelve el
/// conteo resultante (para actualizar la columna física del campo).
///
/// No abre transacción propia: se espera ser llamada dentro de la transacción de
/// `repo_registros` (crear/editar), que también persiste el conteo y reindexa FTS.
pub fn guardar(
    conn: &Conn,
    reg_id: i64,
    campo_id: i64,
    principal: bool,
    valores: &[String],
) -> Result<usize, MicError> {
    let pr = principal as i64;
    conn.execute(
        "DELETE FROM multidatos WHERE reg_id = ?1 AND campo_id = ?2 AND principal = ?3",
        params![reg_id, campo_id, pr],
    )
    .map_err(err_sql)?;

    let mut conteo = 0usize;
    {
        let mut stmt = conn
            .prepare(
                "INSERT INTO multidatos (reg_id, principal, campo_id, valor) \
                 VALUES (?1, ?2, ?3, ?4)",
            )
            .map_err(err_sql)?;
        for v in valores {
            let v = v.trim();
            if v.is_empty() {
                continue;
            }
            stmt.execute(params![reg_id, pr, campo_id, v])
                .map_err(err_sql)?;
            conteo += 1;
        }
    }
    Ok(conteo)
}

/// Borra todos los multidatos de un registro (todas sus campos multidato).
/// Usada al eliminar un registro.
pub fn borrar_registro(conn: &Conn, reg_id: i64, principal: bool) -> Result<(), MicError> {
    let pr = principal as i64;
    conn.execute(
        "DELETE FROM multidatos WHERE reg_id = ?1 AND principal = ?2",
        params![reg_id, pr],
    )
    .map_err(err_sql)?;
    Ok(())
}
