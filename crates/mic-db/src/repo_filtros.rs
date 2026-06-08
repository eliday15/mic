//! Repositorio de filtros avanzados guardados (tabla `FiltrosAv` del original).
//!
//! Un filtro guardado es una lista ordenada de condiciones [`CondicionFiltro`]
//! identificada por nombre. Guardar reemplaza por completo el filtro homónimo.

use mic_core::error::MicError;
use mic_core::model::{CondicionFiltro, OpComp, OpRel};
use rusqlite::params;

use crate::pool::{err_sql, Conn};

/// Serializa un [`OpComp`] al texto persistido.
fn op_comp_a_txt(op: OpComp) -> &'static str {
    match op {
        OpComp::Igual => "igual",
        OpComp::Distinto => "distinto",
        OpComp::Mayor => "mayor",
        OpComp::Menor => "menor",
        OpComp::MayorIgual => "mayor_igual",
        OpComp::MenorIgual => "menor_igual",
        OpComp::Contiene => "contiene",
        OpComp::Empieza => "empieza",
    }
}

/// Deserializa el texto persistido a [`OpComp`] (igual por defecto).
fn txt_a_op_comp(s: &str) -> OpComp {
    match s {
        "distinto" => OpComp::Distinto,
        "mayor" => OpComp::Mayor,
        "menor" => OpComp::Menor,
        "mayor_igual" => OpComp::MayorIgual,
        "menor_igual" => OpComp::MenorIgual,
        "contiene" => OpComp::Contiene,
        "empieza" => OpComp::Empieza,
        _ => OpComp::Igual,
    }
}

/// Texto persistido de un [`OpRel`] opcional (NULL en la primera condición).
fn op_rel_a_txt(op: Option<OpRel>) -> Option<&'static str> {
    op.map(|o| match o {
        OpRel::Y => "y",
        OpRel::O => "o",
    })
}

/// Deserializa el conector lógico.
fn txt_a_op_rel(s: Option<String>) -> Option<OpRel> {
    match s.as_deref() {
        Some("y") => Some(OpRel::Y),
        Some("o") => Some(OpRel::O),
        _ => None,
    }
}

/// Lista los nombres de los filtros guardados (orden alfabético).
pub fn listar(conn: &Conn) -> Result<Vec<String>, MicError> {
    let mut stmt = conn
        .prepare("SELECT DISTINCT nombre FROM filtros_av ORDER BY nombre COLLATE NOCASE")
        .map_err(err_sql)?;
    let filas = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(err_sql)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err_sql)?;
    Ok(filas)
}

/// Obtiene las condiciones de un filtro guardado, en orden.
pub fn obtener(conn: &Conn, nombre: &str) -> Result<Vec<CondicionFiltro>, MicError> {
    let mut stmt = conn
        .prepare(
            "SELECT op_rel, campo, op_comp, valor FROM filtros_av \
             WHERE nombre = ?1 ORDER BY orden",
        )
        .map_err(err_sql)?;
    let filas = stmt
        .query_map(params![nombre], |row| {
            let op_rel: Option<String> = row.get(0)?;
            let op_comp: String = row.get(2)?;
            Ok(CondicionFiltro {
                op_rel: txt_a_op_rel(op_rel),
                campo: row.get(1)?,
                op_comp: txt_a_op_comp(&op_comp),
                valor: row.get(3)?,
            })
        })
        .map_err(err_sql)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err_sql)?;
    Ok(filas)
}

/// Guarda (reemplaza) un filtro con nombre `nombre` y sus condiciones.
pub fn guardar(
    conn: &mut Conn,
    nombre: &str,
    condiciones: &[CondicionFiltro],
) -> Result<(), MicError> {
    let tx = conn.transaction().map_err(err_sql)?;
    tx.execute("DELETE FROM filtros_av WHERE nombre = ?1", params![nombre])
        .map_err(err_sql)?;
    {
        let mut stmt = tx
            .prepare(
                "INSERT INTO filtros_av (nombre, orden, op_rel, campo, op_comp, valor) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .map_err(err_sql)?;
        for (i, c) in condiciones.iter().enumerate() {
            stmt.execute(params![
                nombre,
                i as i64,
                op_rel_a_txt(c.op_rel),
                c.campo,
                op_comp_a_txt(c.op_comp),
                c.valor,
            ])
            .map_err(err_sql)?;
        }
    }
    tx.commit().map_err(err_sql)?;
    Ok(())
}

/// Elimina un filtro guardado por nombre.
pub fn eliminar(conn: &Conn, nombre: &str) -> Result<(), MicError> {
    conn.execute("DELETE FROM filtros_av WHERE nombre = ?1", params![nombre])
        .map_err(err_sql)?;
    Ok(())
}
