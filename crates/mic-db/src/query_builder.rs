//! Construcción segura de `WHERE` / `ORDER BY` desde un [`QueryReq`].
//!
//! Reglas de seguridad (inviolables):
//! - Los valores del usuario SIEMPRE viajan como parámetros `?` (nunca se
//!   interpolan en el SQL).
//! - Los nombres de columna provienen EXCLUSIVAMENTE de `CampoDef.col_fisica`,
//!   resueltos a partir del nombre visible. El input del usuario nunca se
//!   concatena como identificador SQL.
//! - Un nombre de campo desconocido se ignora silenciosamente (no rompe la
//!   consulta), igual de seguro que el original que solo aceptaba campos reales.
//!
//! Precedencia de operadores: como el original (frmFA), las condiciones se
//! evalúan de izquierda a derecha sin precedencia especial entre `Y` / `O`. Para
//! reproducir esa semántica plana en SQL (que sí da precedencia a `AND` sobre
//! `OR`), envolvemos el acumulado en paréntesis en cada paso:
//! `((((c1) op c2) op c3) …)`.

use std::collections::HashMap;

use mic_core::model::{CampoDef, Direccion, OpComp, OpRel, QueryReq, Tabla, TipoCampo};
use rusqlite::types::Value as SqlValue;

/// Fragmento de SQL construido con sus parámetros posicionales.
pub struct Construido {
    /// Cláusula `WHERE` completa (sin la palabra `WHERE`), o vacía.
    pub where_sql: String,
    /// Cláusula `ORDER BY` completa (sin las palabras `ORDER BY`).
    pub order_sql: String,
    /// `JOIN` extra para la búsqueda FTS (vacío si no hay búsqueda).
    pub join_sql: String,
    /// Parámetros en orden de aparición (`?`).
    pub params: Vec<SqlValue>,
}

/// Índice de campos por nombre visible → definición, para resolver columnas.
struct Indice<'a> {
    por_nombre: HashMap<&'a str, &'a CampoDef>,
}

impl<'a> Indice<'a> {
    fn nuevo(campos: &'a [CampoDef]) -> Self {
        let mut por_nombre = HashMap::with_capacity(campos.len());
        for c in campos {
            por_nombre.insert(c.nombre.as_str(), c);
        }
        Self { por_nombre }
    }

    fn buscar(&self, nombre: &str) -> Option<&'a CampoDef> {
        self.por_nombre.get(nombre).copied()
    }
}

/// Construye `WHERE`/`ORDER BY`/`JOIN` y los parámetros para `req`.
///
/// `tabla` es la tabla base (`principal` o `variantes`); las referencias a la
/// tabla FTS usan el alias `fts`.
///
/// `niveles_grupo` son los pares `(campo, valor)` de cada nivel seleccionado del
/// grupo activo, ya resueltos por `repo_grupos` (que conoce qué campos del álbum
/// corresponden a `por`/`luego1`/`luego2`). Aquí solo generamos las igualdades.
pub fn construir(
    campos: &[CampoDef],
    req: &QueryReq,
    niveles_grupo: &[(&CampoDef, String)],
) -> Construido {
    let tabla = req.tabla;
    // El índice solo contiene campos de la tabla consultada: una condición u
    // orden que referencie un campo de la otra tabla (p. ej. un orden guardado
    // con un campo de variantes al consultar principal) se ignora en lugar de
    // generar SQL sobre una columna inexistente.
    let campos_tabla: Vec<CampoDef> = campos
        .iter()
        .filter(|c| c.tabla == tabla)
        .cloned()
        .collect();
    let idx = Indice::nuevo(&campos_tabla);
    let alias = tabla.nombre();

    let mut clausulas: Vec<String> = Vec::new();
    let mut params: Vec<SqlValue> = Vec::new();
    let mut join_sql = String::new();

    // Registros ocultos (`_auxiliar_ = 1`): se excluyen salvo petición expresa.
    if !req.incluir_ocultos {
        clausulas.push(format!("{alias}._auxiliar_ = 0"));
    }

    // Variantes de un principal concreto.
    if matches!(tabla, Tabla::Variantes) {
        if let Some(idp) = req.id_principal {
            clausulas.push(format!("{alias}._idprincipal_ = ?"));
            params.push(SqlValue::Integer(idp));
        }
    }

    // Filtro rápido del panel lateral.
    if let Some(fr) = &req.filtro_rapido {
        if let Some(campo) = idx.buscar(&fr.campo) {
            empujar_filtro_rapido(&mut clausulas, &mut params, alias, campo, &fr.valor);
        }
    }

    // Grupo jerárquico ya resuelto: igualdad por cada nivel.
    if !niveles_grupo.is_empty() {
        let grupo_clausula = condiciones_grupo(alias, niveles_grupo, &mut params);
        if !grupo_clausula.is_empty() {
            clausulas.push(grupo_clausula);
        }
    }

    // Condiciones de filtro avanzado (izquierda a derecha, sin precedencia).
    if let Some(cond_sql) = clausula_condiciones(&idx, alias, &req.condiciones, &mut params) {
        if !cond_sql.is_empty() {
            clausulas.push(cond_sql);
        }
    }

    // Búsqueda libre FTS5.
    if let Some(busq) = &req.busqueda {
        let busq = busq.trim();
        if !busq.is_empty() && matches!(tabla, Tabla::Principal) {
            join_sql = format!(
                " JOIN principal_fts AS fts ON fts.rowid = {alias}._id_ \
                 AND principal_fts MATCH ?"
            );
            params.push(SqlValue::Text(consulta_fts(busq)));
        }
    }

    let where_sql = if clausulas.is_empty() {
        String::new()
    } else {
        clausulas.join(" AND ")
    };

    let order_sql = clausula_orden(&idx, alias, &req.orden);

    Construido {
        where_sql,
        order_sql,
        join_sql,
        params,
    }
}

/// Empuja la condición de filtro rápido: `=` para campos escalares, `EXISTS`
/// sobre `multidatos` para campos de tipo Multidato.
fn empujar_filtro_rapido(
    clausulas: &mut Vec<String>,
    params: &mut Vec<SqlValue>,
    alias: &str,
    campo: &CampoDef,
    valor: &str,
) {
    let principal = (alias == "principal") as i64;
    if matches!(campo.tipo, TipoCampo::Multidato) {
        clausulas.push(format!(
            "EXISTS (SELECT 1 FROM multidatos m WHERE m.reg_id = {alias}._id_ \
             AND m.campo_id = ? AND m.principal = ? AND m.valor = ?)"
        ));
        params.push(SqlValue::Integer(campo.id));
        params.push(SqlValue::Integer(principal));
        params.push(SqlValue::Text(valor.to_string()));
    } else {
        clausulas.push(format!("{alias}.{} = ?", campo.col_fisica));
        params.push(valor_para_campo(campo, valor));
    }
}

/// Aplica un grupo ya resuelto a pares `(CampoDef, valor)` por nivel.
///
/// El repositorio de grupos resuelve los nombres de campo del grupo y llama a
/// esta función con los niveles seleccionados. Genera igualdades `col = ?` por
/// cada nivel con valor presente.
pub fn condiciones_grupo(
    alias: &str,
    niveles: &[(&CampoDef, String)],
    params: &mut Vec<SqlValue>,
) -> String {
    let mut partes = Vec::new();
    for (campo, valor) in niveles {
        if matches!(campo.tipo, TipoCampo::Multidato) {
            let principal = (alias == "principal") as i64;
            partes.push(format!(
                "EXISTS (SELECT 1 FROM multidatos m WHERE m.reg_id = {alias}._id_ \
                 AND m.campo_id = ? AND m.principal = ? AND m.valor = ?)"
            ));
            params.push(SqlValue::Integer(campo.id));
            params.push(SqlValue::Integer(principal));
            params.push(SqlValue::Text(valor.clone()));
        } else {
            partes.push(format!("{alias}.{} = ?", campo.col_fisica));
            params.push(valor_para_campo(campo, valor));
        }
    }
    partes.join(" AND ")
}

/// Construye la cláusula de las condiciones avanzadas respetando el orden
/// izquierda-a-derecha del original (sin precedencia AND/OR).
fn clausula_condiciones(
    idx: &Indice,
    alias: &str,
    condiciones: &[mic_core::model::CondicionFiltro],
    params: &mut Vec<SqlValue>,
) -> Option<String> {
    let mut acumulado = String::new();
    for cond in condiciones {
        let campo = match idx.buscar(&cond.campo) {
            Some(c) => c,
            None => continue, // campo desconocido → se ignora (seguro)
        };
        let predicado = predicado_comparacion(alias, campo, cond.op_comp, &cond.valor, params);

        if acumulado.is_empty() {
            acumulado = predicado;
        } else {
            // Envolvemos el acumulado para forzar evaluación plana izq→der.
            let conector = match cond.op_rel {
                Some(OpRel::O) => "OR",
                _ => "AND", // None o Y → AND
            };
            acumulado = format!("({acumulado} {conector} {predicado})");
        }
    }
    if acumulado.is_empty() {
        None
    } else {
        Some(acumulado)
    }
}

/// Predicado SQL para una comparación individual, con su parámetro.
fn predicado_comparacion(
    alias: &str,
    campo: &CampoDef,
    op: OpComp,
    valor: &str,
    params: &mut Vec<SqlValue>,
) -> String {
    // Multidato: la comparación se hace contra los valores de la tabla.
    if matches!(campo.tipo, TipoCampo::Multidato) {
        let principal = (alias == "principal") as i64;
        let (cmp, val) = match op {
            OpComp::Contiene => ("LIKE", SqlValue::Text(format!("%{valor}%"))),
            OpComp::Empieza => ("LIKE", SqlValue::Text(format!("{valor}%"))),
            OpComp::Distinto => ("<>", SqlValue::Text(valor.to_string())),
            _ => ("=", SqlValue::Text(valor.to_string())),
        };
        let predicado = format!(
            "EXISTS (SELECT 1 FROM multidatos m WHERE m.reg_id = {alias}._id_ \
             AND m.campo_id = ? AND m.principal = ? AND m.valor {cmp} ?)"
        );
        params.push(SqlValue::Integer(campo.id));
        params.push(SqlValue::Integer(principal));
        params.push(val);
        return predicado;
    }

    let col = format!("{alias}.{}", campo.col_fisica);
    match op {
        OpComp::Igual => {
            params.push(valor_para_campo(campo, valor));
            format!("{col} = ?")
        }
        OpComp::Distinto => {
            params.push(valor_para_campo(campo, valor));
            format!("{col} <> ?")
        }
        OpComp::Mayor => {
            params.push(valor_para_campo(campo, valor));
            format!("{col} > ?")
        }
        OpComp::Menor => {
            params.push(valor_para_campo(campo, valor));
            format!("{col} < ?")
        }
        OpComp::MayorIgual => {
            params.push(valor_para_campo(campo, valor));
            format!("{col} >= ?")
        }
        OpComp::MenorIgual => {
            params.push(valor_para_campo(campo, valor));
            format!("{col} <= ?")
        }
        OpComp::Contiene => {
            params.push(SqlValue::Text(format!("%{valor}%")));
            format!("CAST({col} AS TEXT) LIKE ?")
        }
        OpComp::Empieza => {
            params.push(SqlValue::Text(format!("{valor}%")));
            format!("CAST({col} AS TEXT) LIKE ?")
        }
    }
}

/// `ORDER BY` con hasta 3 niveles + `_id_` como desempate estable.
fn clausula_orden(idx: &Indice, alias: &str, orden: &[mic_core::model::OrdenCampo]) -> String {
    let mut partes = Vec::new();
    for nivel in orden.iter().take(3) {
        if let Some(campo) = idx.buscar(&nivel.campo) {
            let dir = match nivel.direccion {
                Direccion::Asc => "ASC",
                Direccion::Desc => "DESC",
            };
            // Multidato ordena por su conteo (la columna física), que es lo
            // razonable cuando no hay un valor escalar único.
            partes.push(format!("{alias}.{} {dir}", campo.col_fisica));
        }
    }
    // Desempate determinista por id.
    partes.push(format!("{alias}._id_ ASC"));
    partes.join(", ")
}

/// Convierte el valor textual del usuario al [`SqlValue`] adecuado al tipo del
/// campo, para que las comparaciones numéricas/fecha usen el orden correcto.
fn valor_para_campo(campo: &CampoDef, valor: &str) -> SqlValue {
    match campo.tipo {
        TipoCampo::Numerico | TipoCampo::Moneda | TipoCampo::Calculado => {
            match valor.trim().replace(',', ".").parse::<f64>() {
                Ok(n) => SqlValue::Real(n),
                Err(_) => SqlValue::Text(valor.to_string()),
            }
        }
        _ => SqlValue::Text(valor.to_string()),
    }
}

/// Sanea la cadena de búsqueda para FTS5: prefija cada token con comillas y le
/// añade `*` para búsqueda por prefijo, evitando que caracteres especiales de la
/// sintaxis FTS rompan la consulta.
fn consulta_fts(entrada: &str) -> String {
    let tokens: Vec<String> = entrada
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|t| {
            let limpio = t.replace('"', "");
            format!("\"{limpio}\"*")
        })
        .collect();
    tokens.join(" ")
}
