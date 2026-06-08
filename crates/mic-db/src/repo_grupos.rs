//! Repositorio de grupos jerárquicos (tabla `Grupos` del original).
//!
//! Un grupo define hasta 3 niveles de agrupación (`por`, `luego1`, `luego2`),
//! cada uno el nombre visible de un campo. El árbol resuelto (`arbol`) devuelve
//! los valores distintos por nivel con su conteo de registros, anidados.

use mic_core::error::MicError;
use mic_core::model::{CampoDef, Grupo, NodoGrupo, TipoCampo};
use rusqlite::{params, OptionalExtension};

use crate::pool::{err_sql, Conn};

const NINGUNO: &str = "(Ninguno)";

/// Mapea una fila de `grupos` a [`Grupo`], normalizando "(Ninguno)" a `None`.
fn map_grupo(row: &rusqlite::Row) -> rusqlite::Result<Grupo> {
    let norm = |s: Option<String>| -> Option<String> {
        match s {
            Some(v) if v != NINGUNO && !v.is_empty() => Some(v),
            _ => None,
        }
    };
    Ok(Grupo {
        id: row.get("id")?,
        nombre: row.get("nombre")?,
        por: row.get("por")?,
        luego1: norm(row.get("luego1")?),
        luego2: norm(row.get("luego2")?),
    })
}

/// Lista todos los grupos definidos.
pub fn listar(conn: &Conn) -> Result<Vec<Grupo>, MicError> {
    let mut stmt = conn
        .prepare("SELECT id, nombre, por, luego1, luego2 FROM grupos ORDER BY id")
        .map_err(err_sql)?;
    let filas = stmt
        .query_map([], map_grupo)
        .map_err(err_sql)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err_sql)?;
    Ok(filas)
}

/// Obtiene un grupo por id.
pub fn obtener(conn: &Conn, grupo_id: i64) -> Result<Grupo, MicError> {
    let mut stmt = conn
        .prepare("SELECT id, nombre, por, luego1, luego2 FROM grupos WHERE id = ?1")
        .map_err(err_sql)?;
    stmt.query_row(params![grupo_id], map_grupo)
        .optional()
        .map_err(err_sql)?
        .ok_or_else(|| MicError::NoEncontrado(format!("grupo id={grupo_id}")))
}

/// Guarda un grupo: `id == 0` crea uno nuevo, en otro caso edita. Devuelve el id.
pub fn guardar(conn: &Conn, grupo: &Grupo) -> Result<i64, MicError> {
    let luego1 = grupo.luego1.clone().unwrap_or_else(|| NINGUNO.to_string());
    let luego2 = grupo.luego2.clone().unwrap_or_else(|| NINGUNO.to_string());
    if grupo.id == 0 {
        conn.execute(
            "INSERT INTO grupos (nombre, por, luego1, luego2, status) \
             VALUES (?1, ?2, ?3, ?4, 0)",
            params![grupo.nombre, grupo.por, luego1, luego2],
        )
        .map_err(err_sql)?;
        Ok(conn.last_insert_rowid())
    } else {
        conn.execute(
            "UPDATE grupos SET nombre = ?1, por = ?2, luego1 = ?3, luego2 = ?4 \
             WHERE id = ?5",
            params![grupo.nombre, grupo.por, luego1, luego2, grupo.id],
        )
        .map_err(err_sql)?;
        Ok(grupo.id)
    }
}

/// Elimina un grupo por id.
pub fn eliminar(conn: &Conn, grupo_id: i64) -> Result<(), MicError> {
    conn.execute("DELETE FROM grupos WHERE id = ?1", params![grupo_id])
        .map_err(err_sql)?;
    Ok(())
}

/// Resuelve el nombre visible de un nivel a su [`CampoDef`] (de la principal).
fn resolver<'a>(campos: &'a [CampoDef], nombre: &str) -> Option<&'a CampoDef> {
    campos.iter().find(|c| c.nombre == nombre)
}

/// Construye el árbol completo del grupo: valores distintos por nivel con sus
/// conteos, anidados hasta 3 niveles. Árbol completo (no lazy), suficiente para
/// los tamaños de catálogo objetivo.
///
/// Para cada nivel, los multidatos se resuelven uniendo la tabla `multidatos`;
/// los campos escalares agrupan directamente por su columna física.
pub fn arbol(
    conn: &Conn,
    campos: &[CampoDef],
    grupo_id: i64,
) -> Result<Vec<NodoGrupo>, MicError> {
    let grupo = obtener(conn, grupo_id)?;

    // Resolvemos los campos de cada nivel definido.
    let mut niveles: Vec<&CampoDef> = Vec::new();
    if let Some(c) = resolver(campos, &grupo.por) {
        niveles.push(c);
    }
    if let Some(l1) = &grupo.luego1 {
        if let Some(c) = resolver(campos, l1) {
            niveles.push(c);
        }
    }
    if let Some(l2) = &grupo.luego2 {
        if let Some(c) = resolver(campos, l2) {
            niveles.push(c);
        }
    }

    if niveles.is_empty() {
        return Ok(Vec::new());
    }

    construir_nivel(conn, &niveles, 0, &[])
}

/// Construye recursivamente los nodos de `niveles[profundidad]`, filtrando por
/// los valores ya elegidos en los niveles superiores (`filtros`).
fn construir_nivel(
    conn: &Conn,
    niveles: &[&CampoDef],
    profundidad: usize,
    filtros: &[(&CampoDef, String)],
) -> Result<Vec<NodoGrupo>, MicError> {
    if profundidad >= niveles.len() {
        return Ok(Vec::new());
    }
    let campo = niveles[profundidad];

    // Expresión de valor para este nivel.
    let (from_extra, valor_expr) = if matches!(campo.tipo, TipoCampo::Multidato) {
        (
            format!(
                " JOIN multidatos mg ON mg.reg_id = principal._id_ \
                 AND mg.campo_id = {cid} AND mg.principal = 1",
                cid = campo.id
            ),
            "mg.valor".to_string(),
        )
    } else {
        (
            String::new(),
            format!("CAST(principal.{} AS TEXT)", campo.col_fisica),
        )
    };

    // WHERE de los niveles superiores ya fijados.
    let mut where_partes: Vec<String> = Vec::new();
    let mut binds: Vec<String> = Vec::new();
    for (c, v) in filtros {
        if matches!(c.tipo, TipoCampo::Multidato) {
            where_partes.push(format!(
                "EXISTS (SELECT 1 FROM multidatos mf WHERE mf.reg_id = principal._id_ \
                 AND mf.campo_id = {cid} AND mf.principal = 1 AND mf.valor = ?)",
                cid = c.id
            ));
        } else {
            where_partes.push(format!("CAST(principal.{} AS TEXT) = ?", c.col_fisica));
        }
        binds.push(v.clone());
    }
    let where_sql = if where_partes.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", where_partes.join(" AND "))
    };

    let sql = format!(
        "SELECT {valor_expr} AS v, COUNT(DISTINCT principal._id_) AS n \
         FROM principal{from_extra}{where_sql} \
         GROUP BY v ORDER BY v COLLATE NOCASE"
    );

    let mut stmt = conn.prepare(&sql).map_err(err_sql)?;
    let bind_refs: Vec<&dyn rusqlite::ToSql> =
        binds.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
    let filas: Vec<(Option<String>, i64)> = stmt
        .query_map(bind_refs.as_slice(), |row| {
            Ok((row.get::<_, Option<String>>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(err_sql)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err_sql)?;

    let mut nodos = Vec::with_capacity(filas.len());
    for (valor_opt, conteo) in filas {
        let valor = valor_opt.unwrap_or_default();
        // Hijos: bajamos un nivel fijando el valor de este.
        let mut filtros_hijo = filtros.to_vec();
        filtros_hijo.push((campo, valor.clone()));
        let hijos = construir_nivel(conn, niveles, profundidad + 1, &filtros_hijo)?;
        nodos.push(NodoGrupo {
            valor,
            conteo: conteo as u64,
            hijos,
        });
    }
    Ok(nodos)
}
