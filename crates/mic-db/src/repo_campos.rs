//! Repositorio de campos configurables (ex tabla `propiedades` del original).
//!
//! Crear/editar/eliminar un campo implica DDL dinámico sobre `principal` o
//! `variantes` (`ALTER TABLE ADD/DROP COLUMN`). El cambio de tipo replica la
//! conversión de datos de `CvrteCmps` (Module5.bas), simplificada a MIC 3.0:
//! ya no existe el límite `longitud`, así que las reglas de "no cabe / muy
//! grande" desaparecen y solo queda la conversión de valor entre tipos.

use mic_core::error::MicError;
use mic_core::model::{CampoDef, CampoNuevo, Tabla, TipoCampo};
use rusqlite::{params, OptionalExtension};

use crate::pool::{err_sql, Conn};
use crate::schema::col_fisica;

/// Lee un [`CampoDef`] desde una fila de la tabla `campos`.
fn map_campo(row: &rusqlite::Row) -> rusqlite::Result<CampoDef> {
    let tabla_txt: String = row.get("tabla")?;
    let tabla = match tabla_txt.as_str() {
        "variantes" => Tabla::Variantes,
        _ => Tabla::Principal,
    };
    let tipo_n: i64 = row.get("tipo")?;
    let tipo = TipoCampo::from_jet(tipo_n).unwrap_or(TipoCampo::Texto);
    Ok(CampoDef {
        id: row.get("id")?,
        nombre: row.get("nombre")?,
        col_fisica: row.get("col_fisica")?,
        tabla,
        tipo,
        decimales: row.get::<_, i64>("decimales")? as u8,
        totalizable: row.get::<_, i64>("totalizable")? != 0,
        formula: row.get("formula")?,
        visible: row.get::<_, i64>("visible")? != 0,
        modificable: row.get::<_, i64>("modificable")? != 0,
        orden_visible: row.get::<_, i64>("orden_visible")? as i32,
        formato: row.get("formato")?,
    })
}

/// Byte de tipo tal como lo persiste el original (TipoCar de Jet).
fn tipo_a_byte(t: TipoCampo) -> i64 {
    match t {
        TipoCampo::Texto => 0,
        TipoCampo::Numerico => 1,
        TipoCampo::Moneda => 2,
        TipoCampo::Fecha => 3,
        TipoCampo::Calculado => 4,
        TipoCampo::Multidato => 5,
    }
}

/// ¿El tipo admite índice B-tree directo y útil para orden/filtro?
///
/// Multidato guarda solo un conteo en la columna (los valores van a la tabla
/// `multidatos`), así que no se indexa la columna física.
fn es_ordenable(t: TipoCampo) -> bool {
    !matches!(t, TipoCampo::Multidato)
}

/// Lista todos los campos del álbum ordenados por `orden_visible` y luego `id`.
pub fn listar(conn: &Conn) -> Result<Vec<CampoDef>, MicError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, nombre, col_fisica, tabla, tipo, decimales, totalizable, \
             formula, visible, modificable, orden_visible, formato \
             FROM campos ORDER BY orden_visible, id",
        )
        .map_err(err_sql)?;
    let filas = stmt
        .query_map([], map_campo)
        .map_err(err_sql)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err_sql)?;
    Ok(filas)
}

/// Obtiene un campo por id.
fn obtener(conn: &Conn, id: i64) -> Result<CampoDef, MicError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, nombre, col_fisica, tabla, tipo, decimales, totalizable, \
             formula, visible, modificable, orden_visible, formato \
             FROM campos WHERE id = ?1",
        )
        .map_err(err_sql)?;
    stmt.query_row([id], map_campo)
        .optional()
        .map_err(err_sql)?
        .ok_or_else(|| MicError::NoEncontrado(format!("campo id={id}")))
}

/// Crea un campo: INSERT en `campos` + `ALTER TABLE ADD COLUMN` con el tipo
/// SQLite correspondiente + índice si el tipo es ordenable.
pub fn crear(conn: &Conn, def: &CampoNuevo) -> Result<CampoDef, MicError> {
    let tabla = def.tabla.nombre();

    conn.execute(
        "INSERT INTO campos \
         (nombre, col_fisica, tabla, tipo, decimales, totalizable, formula, \
          visible, modificable, orden_visible, formato) \
         VALUES (?1, '', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            def.nombre,
            tabla,
            tipo_a_byte(def.tipo),
            def.decimales as i64,
            def.totalizable as i64,
            def.formula,
            def.visible as i64,
            def.modificable as i64,
            def.orden_visible as i64,
            def.formato,
        ],
    )
    .map_err(err_sql)?;

    let id = conn.last_insert_rowid();
    let col = col_fisica(id);
    conn.execute("UPDATE campos SET col_fisica = ?1 WHERE id = ?2", params![col, id])
        .map_err(err_sql)?;

    // ALTER TABLE: nombre de columna seguro (f_<id>, id entero de la base).
    let sql_tipo = def.tipo.sqlite_type();
    conn.execute_batch(&format!(
        "ALTER TABLE {tabla} ADD COLUMN {col} {sql_tipo};"
    ))
    .map_err(err_sql)?;

    if es_ordenable(def.tipo) {
        conn.execute_batch(&format!(
            "CREATE INDEX IF NOT EXISTS ix_{tabla}_{col} ON {tabla} ({col});"
        ))
        .map_err(err_sql)?;
    }

    obtener(conn, id)
}

/// Edita un campo. Si cambia el tipo, convierte los datos existentes con las
/// reglas portadas de `CvrteCmps` (ver módulo). El nombre físico (`f_<id>`) no
/// cambia nunca; solo se reescribe el contenido y los metadatos.
pub fn editar(conn: &Conn, id: i64, def: &CampoNuevo) -> Result<CampoDef, MicError> {
    let actual = obtener(conn, id)?;
    let col = &actual.col_fisica;
    let tabla = actual.tabla.nombre();

    if actual.tipo != def.tipo {
        convertir_columna(conn, &actual, def.tipo)?;

        // Reajusta el índice según el nuevo tipo.
        conn.execute_batch(&format!("DROP INDEX IF EXISTS ix_{tabla}_{col};"))
            .map_err(err_sql)?;
        if es_ordenable(def.tipo) {
            conn.execute_batch(&format!(
                "CREATE INDEX IF NOT EXISTS ix_{tabla}_{col} ON {tabla} ({col});"
            ))
            .map_err(err_sql)?;
        }
    }

    conn.execute(
        "UPDATE campos SET nombre = ?1, tipo = ?2, decimales = ?3, \
         totalizable = ?4, formula = ?5, visible = ?6, modificable = ?7, \
         orden_visible = ?8, formato = ?9 WHERE id = ?10",
        params![
            def.nombre,
            tipo_a_byte(def.tipo),
            def.decimales as i64,
            def.totalizable as i64,
            def.formula,
            def.visible as i64,
            def.modificable as i64,
            def.orden_visible as i64,
            def.formato,
            id,
        ],
    )
    .map_err(err_sql)?;

    obtener(conn, id)
}

/// Convierte el contenido de la columna física de `campo` al `nuevo` tipo,
/// mediante `UPDATE ... CAST`. Port simplificado de `CvrteCmps` (Module5.bas):
/// sin `longitud`, las reglas de truncado/«no cabe» desaparecen.
///
/// Reglas conservadas:
/// - texto → número/moneda/calculado: `CAST` a REAL (no numérico → 0, como el
///   original que ponía 0 cuando `IsNumeric` fallaba).
/// - texto → fecha: si no es fecha ISO válida, se anula (el original ponía la
///   fecha actual; aquí preferimos NULL para no inventar datos — ver nota).
/// - número/moneda/calculado → texto/fecha: `CAST` a TEXT.
/// - número ↔ moneda ↔ calculado: REAL → REAL, sin pérdida.
/// - cualquiera → multidato: se vuelca el valor escalar como primer multidato y
///   la columna pasa a guardar el conteo (lo hace [`a_multidato`]).
/// - multidato → escalar: se toma el primer valor del multidato.
fn convertir_columna(conn: &Conn, campo: &CampoDef, nuevo: TipoCampo) -> Result<(), MicError> {
    let col = &campo.col_fisica;
    let tabla = campo.tabla.nombre();
    let viejo = campo.tipo;

    // Caso especial: el nuevo tipo es Multidato. Volcamos el escalar a la tabla
    // multidatos y dejamos en la columna el conteo (0 o 1).
    if nuevo == TipoCampo::Multidato {
        a_multidato(conn, campo)?;
        return Ok(());
    }
    // Caso especial: el viejo tipo era Multidato. Tomamos el primer valor.
    if viejo == TipoCampo::Multidato {
        desde_multidato(conn, campo, nuevo)?;
        return Ok(());
    }

    let es_principal = matches!(campo.tabla, Tabla::Principal);
    let principal_flag = if es_principal { 1 } else { 0 };
    let _ = principal_flag; // documentación: el tablero principal/variantes se decide por `tabla`

    // Conversión escalar→escalar mediante CAST.
    let sql = match nuevo {
        TipoCampo::Numerico | TipoCampo::Moneda | TipoCampo::Calculado => {
            // No numérico → 0 (paridad con el original). CAST de texto no
            // numérico devuelve 0.0 en SQLite, lo que coincide.
            format!(
                "UPDATE {tabla} SET {col} = CAST({col} AS REAL) \
                 WHERE {col} IS NOT NULL;"
            )
        }
        TipoCampo::Texto => {
            format!(
                "UPDATE {tabla} SET {col} = CAST({col} AS TEXT) \
                 WHERE {col} IS NOT NULL;"
            )
        }
        TipoCampo::Fecha => {
            // Solo conservamos textos que parezcan fecha ISO (YYYY-MM-DD…);
            // el resto se anula. SQLite `date()` devuelve NULL para entradas no
            // reconocibles, así que filtramos por ese resultado.
            format!(
                "UPDATE {tabla} SET {col} = CASE \
                   WHEN date({col}) IS NOT NULL THEN substr(CAST({col} AS TEXT), 1, 10) \
                   ELSE NULL END \
                 WHERE {col} IS NOT NULL;"
            )
        }
        TipoCampo::Multidato => unreachable!("manejado arriba"),
    };

    conn.execute_batch(&sql).map_err(err_sql)?;
    Ok(())
}

/// Vuelca el valor escalar de la columna a la tabla `multidatos` (un registro
/// por fila no nula) y deja la columna con el conteo (0 o 1).
fn a_multidato(conn: &Conn, campo: &CampoDef) -> Result<(), MicError> {
    let col = &campo.col_fisica;
    let tabla = campo.tabla.nombre();
    let principal = matches!(campo.tabla, Tabla::Principal) as i64;

    // Limpia multidatos previos de este campo (no debería haber, pero por idempotencia).
    conn.execute(
        "DELETE FROM multidatos WHERE campo_id = ?1 AND principal = ?2",
        params![campo.id, principal],
    )
    .map_err(err_sql)?;

    // Inserta un multidato por cada valor escalar no vacío.
    conn.execute(
        &format!(
            "INSERT INTO multidatos (reg_id, principal, campo_id, valor) \
             SELECT _id_, ?1, ?2, CAST({col} AS TEXT) FROM {tabla} \
             WHERE {col} IS NOT NULL AND TRIM(CAST({col} AS TEXT)) <> '';"
        ),
        params![principal, campo.id],
    )
    .map_err(err_sql)?;

    // La columna pasa a guardar el conteo (1 si había valor, 0 si no).
    conn.execute_batch(&format!(
        "UPDATE {tabla} SET {col} = CASE \
           WHEN {col} IS NOT NULL AND TRIM(CAST({col} AS TEXT)) <> '' THEN 1 ELSE 0 END;"
    ))
    .map_err(err_sql)?;
    Ok(())
}

/// Convierte de Multidato a un tipo escalar: toma el primer valor de cada
/// registro y lo coloca en la columna; luego limpia la tabla `multidatos`.
fn desde_multidato(conn: &Conn, campo: &CampoDef, nuevo: TipoCampo) -> Result<(), MicError> {
    let col = &campo.col_fisica;
    let tabla = campo.tabla.nombre();
    let principal = matches!(campo.tabla, Tabla::Principal) as i64;

    // Primer valor por registro (rowid mínimo) — equivale al arrm2(1) del original.
    let cast_expr = match nuevo {
        TipoCampo::Numerico | TipoCampo::Moneda | TipoCampo::Calculado => "CAST(m.valor AS REAL)",
        TipoCampo::Fecha => "substr(m.valor, 1, 10)",
        _ => "m.valor",
    };
    conn.execute_batch(&format!(
        "UPDATE {tabla} SET {col} = ( \
            SELECT {cast_expr} FROM multidatos m \
            WHERE m.reg_id = {tabla}._id_ AND m.campo_id = {cid} AND m.principal = {pr} \
            ORDER BY m.rowid LIMIT 1 );",
        cid = campo.id,
        pr = principal
    ))
    .map_err(err_sql)?;

    conn.execute(
        "DELETE FROM multidatos WHERE campo_id = ?1 AND principal = ?2",
        params![campo.id, principal],
    )
    .map_err(err_sql)?;
    Ok(())
}

/// Elimina un campo: `ALTER TABLE DROP COLUMN` + limpieza de `multidatos` y
/// `categorias` asociadas + borrado de la fila de metadatos.
pub fn eliminar(conn: &Conn, id: i64) -> Result<(), MicError> {
    let campo = obtener(conn, id)?;
    let col = &campo.col_fisica;
    let tabla = campo.tabla.nombre();
    let principal = matches!(campo.tabla, Tabla::Principal) as i64;

    conn.execute_batch(&format!("DROP INDEX IF EXISTS ix_{tabla}_{col};"))
        .map_err(err_sql)?;
    conn.execute_batch(&format!("ALTER TABLE {tabla} DROP COLUMN {col};"))
        .map_err(err_sql)?;

    conn.execute(
        "DELETE FROM multidatos WHERE campo_id = ?1 AND principal = ?2",
        params![id, principal],
    )
    .map_err(err_sql)?;
    conn.execute(
        "DELETE FROM categorias WHERE campo_id = ?1 AND principal = ?2",
        params![id, principal],
    )
    .map_err(err_sql)?;
    conn.execute("DELETE FROM campos WHERE id = ?1", params![id])
        .map_err(err_sql)?;
    Ok(())
}

/// Reordena los campos: `orden` lista los ids en el orden visible deseado.
pub fn reordenar(conn: &Conn, orden: &[i64]) -> Result<(), MicError> {
    for (pos, id) in orden.iter().enumerate() {
        conn.execute(
            "UPDATE campos SET orden_visible = ?1 WHERE id = ?2",
            params![pos as i64, id],
        )
        .map_err(err_sql)?;
    }
    Ok(())
}
