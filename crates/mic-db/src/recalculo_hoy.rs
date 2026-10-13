//! Recálculo diario de los campos calculados que usan `@HOY`.
//!
//! Un calculado como `Vencimiento - @HOY` guarda el valor del día en que se
//! calculó. Al abrir el álbum, si la fecha del último recálculo de `@HOY`
//! (clave [`CLAVE_HOY`] en `mic_album`, sin cambio de esquema) no es hoy, se
//! recalculan SOLO esos campos, por lotes de [`LOTE`] registros en
//! transacciones cortas, escribiendo solo las filas cuyo valor cambió. Coste:
//! una vez al día, y nulo para álbumes sin `@HOY`.

use chrono::NaiveDate;
use mic_core::calc::MotorCalculo;
use mic_core::error::MicError;
use mic_core::model::{CampoDef, Tabla, TipoCampo, Valores};
use rusqlite::types::Value as SqlValue;
use rusqlite::{params, OptionalExtension};

use crate::fts;
use crate::pool::{err_sql, Conn};
use crate::repo_registros::{valor_a_sql, valor_desde_sql};

/// Clave de `mic_album` con la fecha ISO del último recálculo de `@HOY`.
pub const CLAVE_HOY: &str = "hoy_recalculado";

/// Registros por transacción (acota el tiempo que se retiene la escritura).
const LOTE: usize = 500;

/// Recalcula los calculados con `@HOY` si el último recálculo no fue `hoy`.
///
/// Args:
///     conn: Conexión del álbum.
///     campos: Campos vigentes del álbum.
///     motor: Motor compilado con esos campos (`None` si no hay calculados).
///     hoy: Fecha a usar para `@HOY` (la local en producción).
///
/// Returns:
///     Cantidad de registros cuyo valor cambió y se reescribió.
pub fn recalcular_si_cambio_dia(
    conn: &Conn,
    campos: &[CampoDef],
    motor: Option<&MotorCalculo>,
    hoy: NaiveDate,
) -> Result<u64, MicError> {
    let Some(motor) = motor else { return Ok(0) };
    let con_hoy = motor.campos_con_hoy();
    if con_hoy.is_empty() {
        return Ok(0);
    }
    let hoy_iso = hoy.format("%Y-%m-%d").to_string();
    let ultimo: Option<String> = conn
        .query_row(
            "SELECT valor FROM mic_album WHERE clave = ?1",
            params![CLAVE_HOY],
            |r| r.get(0),
        )
        .optional()
        .map_err(err_sql)?;
    if ultimo.as_deref() == Some(hoy_iso.as_str()) {
        return Ok(0);
    }

    let mut cambiados = 0u64;
    for tabla in [Tabla::Principal, Tabla::Variantes] {
        let objetivos: Vec<&CampoDef> = campos
            .iter()
            .filter(|c| {
                c.tabla == tabla
                    && c.tipo == TipoCampo::Calculado
                    && con_hoy.contains(c.nombre.as_str())
            })
            .collect();
        if !objetivos.is_empty() {
            cambiados += recalcular_tabla(conn, campos, motor, tabla, &objetivos, hoy)?;
        }
    }

    conn.execute(
        "INSERT OR REPLACE INTO mic_album (clave, valor) VALUES (?1, ?2)",
        params![CLAVE_HOY, hoy_iso],
    )
    .map_err(err_sql)?;
    Ok(cambiados)
}

/// Recorre `tabla` por lotes de `_id_` y reescribe los `objetivos` que cambian.
fn recalcular_tabla(
    conn: &Conn,
    campos: &[CampoDef],
    motor: &MotorCalculo,
    tabla: Tabla,
    objetivos: &[&CampoDef],
    hoy: NaiveDate,
) -> Result<u64, MicError> {
    let alias = tabla.nombre();
    let campos_tabla: Vec<&CampoDef> = campos.iter().filter(|c| c.tabla == tabla).collect();
    // Posición de cada objetivo entre las columnas leídas.
    let pos_objetivos: Vec<usize> = objetivos
        .iter()
        .filter_map(|o| campos_tabla.iter().position(|c| c.id == o.id))
        .collect();
    let cols = campos_tabla
        .iter()
        .map(|c| c.col_fisica.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let select = format!(
        "SELECT _id_, {cols} FROM {alias} WHERE _id_ > ?1 ORDER BY _id_ LIMIT {LOTE}"
    );
    let asignaciones = objetivos
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{} = ?{}", c.col_fisica, i + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let update = format!(
        "UPDATE {alias} SET {asignaciones} WHERE _id_ = ?{}",
        objetivos.len() + 1
    );

    let mut cambiados = 0u64;
    let mut ultimo_id = 0i64;
    loop {
        // Lote: (id, valores del registro, valores SQL actuales de los objetivos).
        let lote: Vec<(i64, Valores, Vec<SqlValue>)> = {
            let mut stmt = conn.prepare_cached(&select).map_err(err_sql)?;
            let filas = stmt
                .query_map(params![ultimo_id], |row| {
                    let id: i64 = row.get(0)?;
                    let mut valores = Valores::with_capacity(campos_tabla.len());
                    for (i, c) in campos_tabla.iter().enumerate() {
                        valores.insert(c.nombre.clone(), valor_desde_sql(row.get_ref(1 + i)?, c.tipo));
                    }
                    let actuales = pos_objetivos
                        .iter()
                        .map(|i| row.get_ref(1 + i).map(SqlValue::from))
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok((id, valores, actuales))
                })
                .map_err(err_sql)?;
            filas.collect::<Result<_, _>>().map_err(err_sql)?
        };
        let Some((id_final, _, _)) = lote.last() else { break };
        ultimo_id = *id_final;

        let tx = conn.unchecked_transaction().map_err(err_sql)?;
        for (id, valores, actuales) in &lote {
            let mut nuevos = Vec::with_capacity(objetivos.len() + 1);
            let mut cambia = false;
            for (o, actual) in objetivos.iter().zip(actuales) {
                // Un error de evaluación conserva el valor guardado: abrir el
                // álbum nunca debe fallar por una fórmula.
                let nuevo = match motor.evaluar_en(&o.nombre, valores, hoy) {
                    Ok(v) => valor_a_sql(&v, o.tipo),
                    Err(_) => actual.clone(),
                };
                cambia |= nuevo != *actual;
                nuevos.push(nuevo);
            }
            if !cambia {
                continue;
            }
            nuevos.push(SqlValue::Integer(*id));
            tx.prepare_cached(&update)
                .map_err(err_sql)?
                .execute(rusqlite::params_from_iter(nuevos.iter()))
                .map_err(err_sql)?;
            if tabla == Tabla::Principal {
                fts::actualizar_registro(conn, campos, *id)?;
            }
            cambiados += 1;
        }
        tx.commit().map_err(err_sql)?;
        if lote.len() < LOTE {
            break;
        }
    }
    Ok(cambiados)
}
