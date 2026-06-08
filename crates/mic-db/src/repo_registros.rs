//! Repositorio de registros (tablas `principal` / `variantes`).
//!
//! CRUD tipado sobre columnas reales, consulta paginada para el scroll virtual,
//! variantes ligadas a su principal, recálculo de campos calculados con
//! [`MotorCalculo`], sincronización de multidatos (con conteo en la columna del
//! campo) y del índice FTS. Reemplaza la paginación manual `clsPaginas` y el
//! acceso DAO/Jet del original.

use std::collections::HashMap;

use mic_core::calc::MotorCalculo;
use mic_core::error::MicError;
use mic_core::model::{
    CampoDef, Grupo, QueryPage, QueryReq, RegistroCompleto, RegistroLigero, Tabla, TipoCampo,
    Valor, Valores,
};
use rusqlite::types::{Value as SqlValue, ValueRef};
use rusqlite::{params, ToSql};

use crate::pool::{err_sql, Conn};
use crate::{fts, query_builder, repo_grupos, repo_multidatos};

/// Campos visibles de la tabla indicada (ordenados por orden_visible).
fn campos_de(campos: &[CampoDef], tabla: Tabla) -> Vec<&CampoDef> {
    campos.iter().filter(|c| c.tabla == tabla).collect()
}

/// Convierte un [`ValueRef`] de SQLite en un [`Valor`] del dominio según el tipo.
fn valor_desde_sql(vr: ValueRef, tipo: TipoCampo) -> Valor {
    match vr {
        ValueRef::Null => Valor::Nulo(None),
        ValueRef::Integer(i) => match tipo {
            TipoCampo::Multidato => Valor::Entero(i),
            TipoCampo::Numerico | TipoCampo::Moneda | TipoCampo::Calculado => Valor::Numero(i as f64),
            _ => Valor::Entero(i),
        },
        ValueRef::Real(r) => Valor::Numero(r),
        ValueRef::Text(t) => Valor::Texto(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(_) => Valor::Nulo(None),
    }
}

/// Convierte un [`Valor`] del dominio al [`SqlValue`] para persistir, según el
/// tipo del campo destino. Reglas suaves (paridad relajada con `CvrteCmps`):
/// - numérico/moneda/calculado: intenta f64; texto no numérico → NULL.
/// - fecha: guarda la fecha ISO (primeros 10 caracteres del texto).
/// - texto: representación textual.
fn valor_a_sql(valor: &Valor, tipo: TipoCampo) -> SqlValue {
    if valor.es_nulo() {
        return SqlValue::Null;
    }
    match tipo {
        TipoCampo::Numerico | TipoCampo::Moneda | TipoCampo::Calculado => {
            match valor.como_f64() {
                Some(n) => SqlValue::Real(n),
                None => SqlValue::Null,
            }
        }
        TipoCampo::Fecha => {
            let t = valor.como_texto();
            let t = t.trim();
            if t.is_empty() {
                SqlValue::Null
            } else {
                SqlValue::Text(t.chars().take(10).collect())
            }
        }
        TipoCampo::Multidato => {
            // El conteo se gestiona aparte; aquí solo respetamos un entero si llega.
            match valor {
                Valor::Entero(i) => SqlValue::Integer(*i),
                _ => SqlValue::Integer(valor.como_f64().unwrap_or(0.0) as i64),
            }
        }
        TipoCampo::Texto => SqlValue::Text(valor.como_texto()),
    }
}

/// mtime del archivo de imagen (epoch segundos) si existe, para versionar la URL.
fn imagen_version(dir_imagenes: Option<&std::path::Path>, ruta_rel: &str) -> Option<i64> {
    let dir = dir_imagenes?;
    let ruta = dir.join(ruta_rel.trim_start_matches("imagenes/").trim_start_matches('/'));
    let meta = std::fs::metadata(&ruta).ok()?;
    let modif = meta.modified().ok()?;
    let dur = modif.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(dur.as_secs() as i64)
}

/// Total de registros principales del álbum (para la barra de estado).
pub fn total(conn: &Conn) -> Result<u64, MicError> {
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM principal", [], |row| row.get(0))
        .map_err(err_sql)?;
    Ok(n as u64)
}

/// Resuelve los niveles seleccionados de un grupo a pares `(CampoDef, valor)`.
fn niveles_grupo<'a>(
    conn: &Conn,
    campos: &'a [CampoDef],
    req: &QueryReq,
) -> Result<Vec<(&'a CampoDef, String)>, MicError> {
    let sel = match &req.grupo {
        Some(s) => s,
        None => return Ok(Vec::new()),
    };
    let grupo: Grupo = repo_grupos::obtener(conn, sel.grupo_id)?;
    let nombres: Vec<Option<String>> = vec![
        Some(grupo.por.clone()),
        grupo.luego1.clone(),
        grupo.luego2.clone(),
    ];
    let mut out = Vec::new();
    for (i, nombre_opt) in nombres.iter().enumerate() {
        let valor = match sel.valores.get(i) {
            Some(Some(v)) => v.clone(),
            _ => continue, // nivel no seleccionado
        };
        if let Some(nombre) = nombre_opt {
            if let Some(campo) = campos.iter().find(|c| &c.nombre == nombre) {
                out.push((campo, valor));
            }
        }
    }
    Ok(out)
}

/// Consulta paginada de registros para el scroll virtual.
///
/// Devuelve el total que cumple el filtro y la ventana `[offset, offset+limit)`.
pub fn query(
    conn: &Conn,
    campos: &[CampoDef],
    req: &QueryReq,
) -> Result<QueryPage, MicError> {
    let tabla = req.tabla;
    let alias = tabla.nombre();
    let niveles = niveles_grupo(conn, campos, req)?;
    let construido = query_builder::construir(campos, req, &niveles);

    let where_clause = if construido.where_sql.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", construido.where_sql)
    };

    // Conteo total con los mismos filtros.
    let sql_total = format!(
        "SELECT COUNT(*) FROM {alias}{join}{where_clause}",
        join = construido.join_sql
    );
    let param_refs: Vec<&dyn ToSql> = construido
        .params
        .iter()
        .map(|v| v as &dyn ToSql)
        .collect();
    let total: i64 = conn
        .query_row(&sql_total, param_refs.as_slice(), |row| row.get(0))
        .map_err(err_sql)?;

    // Columnas a leer: fijas + físicas de los campos de la tabla.
    let campos_tabla = campos_de(campos, tabla);
    let mut select_cols = vec![
        format!("{alias}._id_"),
        format!("{alias}._imagen_"),
        format!("{alias}._imagen_version_"),
    ];
    if matches!(tabla, Tabla::Principal) {
        select_cols.push(format!("{alias}._variantes_"));
    } else {
        select_cols.push("0".to_string());
    }
    select_cols.push(format!("{alias}._auxiliar_"));
    for c in &campos_tabla {
        if matches!(c.tipo, TipoCampo::Multidato) {
            // Etiquetas: la grilla muestra los valores reales ("a · b"), no el
            // conteo. `c.id` y `principal` son enteros propios (no input del
            // usuario), seguros de interpolar.
            let principal = matches!(tabla, Tabla::Principal) as i64;
            select_cols.push(format!(
                "(SELECT group_concat(m.valor, ' · ') FROM multidatos m \
                 WHERE m.reg_id = {alias}._id_ AND m.campo_id = {id} \
                 AND m.principal = {principal})",
                id = c.id
            ));
        } else {
            select_cols.push(format!("{alias}.{}", c.col_fisica));
        }
    }
    let cols_sql = select_cols.join(", ");

    let sql = format!(
        "SELECT {cols_sql} FROM {alias}{join}{where_clause} ORDER BY {order} LIMIT ? OFFSET ?",
        join = construido.join_sql,
        order = construido.order_sql
    );

    let mut params_pag = construido.params.clone();
    params_pag.push(SqlValue::Integer(req.limit as i64));
    params_pag.push(SqlValue::Integer(req.offset as i64));
    let param_refs2: Vec<&dyn ToSql> = params_pag.iter().map(|v| v as &dyn ToSql).collect();

    let mut stmt = conn.prepare(&sql).map_err(err_sql)?;
    let n_fijas = 5usize; // _id_, _imagen_, _imagen_version_, _variantes_, _auxiliar_
    let registros = stmt
        .query_map(param_refs2.as_slice(), |row| {
            let id: i64 = row.get(0)?;
            let imagen: Option<String> = row.get(1)?;
            let imagen_version: Option<i64> = row.get(2)?;
            let tiene_variantes: bool = row.get::<_, i64>(3)? != 0;
            let oculto: bool = row.get::<_, i64>(4)? != 0;
            let mut valores: Valores = HashMap::with_capacity(campos_tabla.len());
            for (i, c) in campos_tabla.iter().enumerate() {
                let vr = row.get_ref(n_fijas + i)?;
                valores.insert(c.nombre.clone(), valor_desde_sql(vr, c.tipo));
            }
            Ok(RegistroLigero {
                id,
                imagen,
                imagen_version,
                tiene_variantes,
                oculto,
                valores,
            })
        })
        .map_err(err_sql)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err_sql)?;

    Ok(QueryPage {
        total: total as u64,
        offset: req.offset,
        registros,
    })
}

/// Obtiene un registro completo (valores + multidatos) por id.
pub fn obtener(
    conn: &Conn,
    campos: &[CampoDef],
    tabla: Tabla,
    id: i64,
) -> Result<RegistroCompleto, MicError> {
    let alias = tabla.nombre();
    let campos_tabla = campos_de(campos, tabla);

    let mut cols = vec![
        format!("{alias}._imagen_"),
        format!("{alias}._imagen_version_"),
    ];
    for c in &campos_tabla {
        cols.push(format!("{alias}.{}", c.col_fisica));
    }
    let cols_sql = cols.join(", ");
    let sql = format!("SELECT {cols_sql} FROM {alias} WHERE _id_ = ?1");

    let mut stmt = conn.prepare(&sql).map_err(err_sql)?;
    let (imagen, imagen_version, mut valores) = stmt
        .query_row(params![id], |row| {
            let imagen: Option<String> = row.get(0)?;
            let imagen_version: Option<i64> = row.get(1)?;
            let mut valores: Valores = HashMap::with_capacity(campos_tabla.len());
            for (i, c) in campos_tabla.iter().enumerate() {
                let vr = row.get_ref(2 + i)?;
                valores.insert(c.nombre.clone(), valor_desde_sql(vr, c.tipo));
            }
            Ok((imagen, imagen_version, valores))
        })
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                MicError::NoEncontrado(format!("registro {alias} id={id}"))
            }
            otro => err_sql(otro),
        })?;

    // Multidatos del registro.
    let principal = matches!(tabla, Tabla::Principal);
    let mut multidatos: HashMap<String, Vec<String>> = HashMap::new();
    for c in &campos_tabla {
        if matches!(c.tipo, TipoCampo::Multidato) {
            let vals = repo_multidatos::listar(conn, id, c.id, principal)?;
            multidatos.insert(c.nombre.clone(), vals);
        }
    }
    // Para multidatos, el "valor" en el mapa es el conteo (number).
    for c in &campos_tabla {
        if matches!(c.tipo, TipoCampo::Multidato) {
            let n = multidatos.get(&c.nombre).map(|v| v.len()).unwrap_or(0);
            valores.insert(c.nombre.clone(), Valor::Entero(n as i64));
        }
    }

    Ok(RegistroCompleto {
        id,
        tabla,
        imagen,
        imagen_version,
        valores,
        multidatos,
    })
}

/// Recalcula los campos calculados (en `orden_recalculo`) y los inserta en `valores`.
fn recalcular(
    motor: Option<&MotorCalculo>,
    campos_tabla: &[&CampoDef],
    valores: &mut Valores,
) -> Result<(), MicError> {
    let motor = match motor {
        Some(m) => m,
        None => return Ok(()),
    };
    // Solo recalculamos los calculados que pertenecen a esta tabla.
    let nombres_tabla: std::collections::HashSet<&str> = campos_tabla
        .iter()
        .filter(|c| matches!(c.tipo, TipoCampo::Calculado))
        .map(|c| c.nombre.as_str())
        .collect();
    for nombre in motor.orden_recalculo() {
        if nombres_tabla.contains(nombre.as_str()) {
            let v = motor.evaluar(nombre, valores)?;
            valores.insert(nombre.clone(), v);
        }
    }
    Ok(())
}

/// Crea un registro en `tabla`. Aplica recálculo, persiste columnas, guarda
/// multidatos (con conteo en columna), alimenta categorías, mantiene
/// `_variantes_` en el principal e indexa FTS. Todo en una transacción.
///
/// Devuelve el id del nuevo registro.
#[allow(clippy::too_many_arguments)]
pub fn crear(
    conn: &mut Conn,
    campos: &[CampoDef],
    motor: Option<&MotorCalculo>,
    tabla: Tabla,
    valores: &Valores,
    multidatos: &HashMap<String, Vec<String>>,
    imagen: Option<&str>,
    id_principal: Option<i64>,
    dir_imagenes: Option<&std::path::Path>,
) -> Result<i64, MicError> {
    let campos_tabla = campos_de(campos, tabla);
    let principal = matches!(tabla, Tabla::Principal);
    let alias = tabla.nombre();

    let mut vals = valores.clone();
    recalcular(motor, &campos_tabla, &mut vals)?;

    let tx = conn.transaction().map_err(err_sql)?;

    // Columnas escalares + conteo de multidatos.
    let mut col_names: Vec<String> = Vec::new();
    let mut col_vals: Vec<SqlValue> = Vec::new();
    for c in &campos_tabla {
        col_names.push(c.col_fisica.clone());
        if matches!(c.tipo, TipoCampo::Multidato) {
            let n = multidatos.get(&c.nombre).map(|v| v.len()).unwrap_or(0);
            col_vals.push(SqlValue::Integer(n as i64));
        } else {
            let v = vals.get(&c.nombre).cloned().unwrap_or_default();
            col_vals.push(valor_a_sql(&v, c.tipo));
        }
    }

    // Imagen y versión.
    let img_ver = imagen.and_then(|r| imagen_version(dir_imagenes, r));
    col_names.push("_imagen_".to_string());
    col_vals.push(match imagen {
        Some(r) => SqlValue::Text(r.to_string()),
        None => SqlValue::Null,
    });
    col_names.push("_imagen_version_".to_string());
    col_vals.push(match img_ver {
        Some(v) => SqlValue::Integer(v),
        None => SqlValue::Null,
    });

    if !principal {
        let idp = id_principal
            .ok_or_else(|| MicError::Invalido("variante sin id_principal".into()))?;
        col_names.push("_idprincipal_".to_string());
        col_vals.push(SqlValue::Integer(idp));
    }

    let placeholders = (1..=col_names.len())
        .map(|i| format!("?{i}"))
        .collect::<Vec<_>>()
        .join(", ");
    let cols_sql = col_names.join(", ");
    let sql = format!("INSERT INTO {alias} ({cols_sql}) VALUES ({placeholders})");
    let refs: Vec<&dyn ToSql> = col_vals.iter().map(|v| v as &dyn ToSql).collect();
    tx.execute(&sql, refs.as_slice()).map_err(err_sql)?;
    let nuevo_id = tx.last_insert_rowid();

    // Multidatos.
    for c in &campos_tabla {
        if matches!(c.tipo, TipoCampo::Multidato) {
            if let Some(lista) = multidatos.get(&c.nombre) {
                guardar_multidato_tx(&tx, nuevo_id, c.id, principal, lista)?;
                for v in lista {
                    asegurar_categoria_tx(&tx, c.id, principal, v)?;
                }
            }
        }
    }

    // Marca de variantes en el principal.
    if !principal {
        if let Some(idp) = id_principal {
            tx.execute(
                "UPDATE principal SET _variantes_ = 1 WHERE _id_ = ?1",
                params![idp],
            )
            .map_err(err_sql)?;
        }
    }

    tx.commit().map_err(err_sql)?;

    // FTS fuera de la transacción (solo principal).
    if principal {
        let conn_ref: &Conn = conn;
        fts::actualizar_registro(conn_ref, campos, nuevo_id)?;
    }

    Ok(nuevo_id)
}

/// Edita un registro. Recalcula calculados, persiste columnas, opcionalmente
/// reemplaza multidatos, reindexa FTS, y devuelve el registro completo.
#[allow(clippy::too_many_arguments)]
pub fn editar(
    conn: &mut Conn,
    campos: &[CampoDef],
    motor: Option<&MotorCalculo>,
    tabla: Tabla,
    id: i64,
    valores: &Valores,
    multidatos: Option<&HashMap<String, Vec<String>>>,
) -> Result<RegistroCompleto, MicError> {
    let campos_tabla = campos_de(campos, tabla);
    let principal = matches!(tabla, Tabla::Principal);
    let alias = tabla.nombre();

    let mut vals = valores.clone();
    // Para recalcular bien necesitamos también el conteo de multidatos actual.
    for c in &campos_tabla {
        if matches!(c.tipo, TipoCampo::Multidato) {
            let n = match multidatos {
                Some(m) => m.get(&c.nombre).map(|v| v.len()).unwrap_or(0),
                None => repo_multidatos::listar(conn, id, c.id, principal)?.len(),
            };
            vals.insert(c.nombre.clone(), Valor::Entero(n as i64));
        }
    }
    recalcular(motor, &campos_tabla, &mut vals)?;

    let tx = conn.transaction().map_err(err_sql)?;

    // SET de columnas escalares y conteo de multidatos.
    let mut sets: Vec<String> = Vec::new();
    let mut set_vals: Vec<SqlValue> = Vec::new();
    let mut idx = 1;
    for c in &campos_tabla {
        sets.push(format!("{} = ?{idx}", c.col_fisica));
        if matches!(c.tipo, TipoCampo::Multidato) {
            // El conteo ya quedó en `vals` (lo calculamos arriba antes del
            // recálculo, sea desde `multidatos` o desde la tabla actual).
            let n = vals
                .get(&c.nombre)
                .and_then(|v| v.como_f64())
                .map(|f| f as i64)
                .unwrap_or(0);
            set_vals.push(SqlValue::Integer(n));
        } else {
            let v = vals.get(&c.nombre).cloned().unwrap_or_default();
            set_vals.push(valor_a_sql(&v, c.tipo));
        }
        idx += 1;
    }

    if !sets.is_empty() {
        let sql = format!(
            "UPDATE {alias} SET {} WHERE _id_ = ?{idx}",
            sets.join(", ")
        );
        set_vals.push(SqlValue::Integer(id));
        let refs: Vec<&dyn ToSql> = set_vals.iter().map(|v| v as &dyn ToSql).collect();
        tx.execute(&sql, refs.as_slice()).map_err(err_sql)?;
    }

    // Multidatos (solo si llegaron).
    if let Some(m) = multidatos {
        for c in &campos_tabla {
            if matches!(c.tipo, TipoCampo::Multidato) {
                if let Some(lista) = m.get(&c.nombre) {
                    guardar_multidato_tx(&tx, id, c.id, principal, lista)?;
                    for v in lista {
                        asegurar_categoria_tx(&tx, c.id, principal, v)?;
                    }
                }
            }
        }
    }

    tx.commit().map_err(err_sql)?;

    if principal {
        let conn_ref: &Conn = conn;
        fts::actualizar_registro(conn_ref, campos, id)?;
    }

    obtener(conn, campos, tabla, id)
}

/// Edición en lote: aplica los mismos `valores` a varios registros (inspector
/// multi-selección). No recalcula multidatos. Recalcula calculados por registro
/// si hay motor.
pub fn editar_lote(
    conn: &mut Conn,
    campos: &[CampoDef],
    motor: Option<&MotorCalculo>,
    tabla: Tabla,
    ids: &[i64],
    valores: &Valores,
) -> Result<(), MicError> {
    for &id in ids {
        // Cargamos el registro para tener todos los campos al recalcular.
        let actual = obtener(conn, campos, tabla, id)?;
        let mut vals = actual.valores.clone();
        for (k, v) in valores {
            vals.insert(k.clone(), v.clone());
        }
        editar(conn, campos, motor, tabla, id, &vals, None)?;
    }
    Ok(())
}

/// Elimina registros por id. En `principal`, la cascada FK borra sus variantes;
/// limpiamos sus multidatos y entradas FTS. Transacción única.
pub fn eliminar(conn: &mut Conn, tabla: Tabla, ids: &[i64]) -> Result<(), MicError> {
    if ids.is_empty() {
        return Ok(());
    }
    let alias = tabla.nombre();
    let principal = matches!(tabla, Tabla::Principal);

    let tx = conn.transaction().map_err(err_sql)?;
    for &id in ids {
        // Variantes que se irán por cascada: limpiamos sus multidatos primero.
        if principal {
            let mut stmt = tx
                .prepare("SELECT _id_ FROM variantes WHERE _idprincipal_ = ?1")
                .map_err(err_sql)?;
            let var_ids: Vec<i64> = stmt
                .query_map(params![id], |row| row.get(0))
                .map_err(err_sql)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(err_sql)?;
            drop(stmt);
            for vid in var_ids {
                tx.execute(
                    "DELETE FROM multidatos WHERE reg_id = ?1 AND principal = 0",
                    params![vid],
                )
                .map_err(err_sql)?;
            }
        }
        // Multidatos del propio registro.
        tx.execute(
            "DELETE FROM multidatos WHERE reg_id = ?1 AND principal = ?2",
            params![id, principal as i64],
        )
        .map_err(err_sql)?;
        // Borrado de la fila (cascada de variantes si es principal).
        tx.execute(
            &format!("DELETE FROM {alias} WHERE _id_ = ?1"),
            params![id],
        )
        .map_err(err_sql)?;
    }
    tx.commit().map_err(err_sql)?;

    // FTS: borramos entradas de los principales eliminados.
    if principal {
        for &id in ids {
            fts::eliminar_registro(conn, id)?;
        }
    }
    Ok(())
}

/// Asigna la imagen de un registro y devuelve su nueva versión (mtime).
pub fn set_imagen(
    conn: &Conn,
    tabla: Tabla,
    id: i64,
    ruta_rel: &str,
    dir_imagenes: Option<&std::path::Path>,
) -> Result<i64, MicError> {
    let alias = tabla.nombre();
    let ver = imagen_version(dir_imagenes, ruta_rel).unwrap_or(0);
    conn.execute(
        &format!("UPDATE {alias} SET _imagen_ = ?1, _imagen_version_ = ?2 WHERE _id_ = ?3"),
        params![ruta_rel, ver, id],
    )
    .map_err(err_sql)?;
    Ok(ver)
}

/// Lista las variantes de un principal como registros ligeros.
pub fn variantes_de(
    conn: &Conn,
    campos: &[CampoDef],
    id_principal: i64,
) -> Result<Vec<RegistroLigero>, MicError> {
    let req = QueryReq {
        tabla: Tabla::Variantes,
        id_principal: Some(id_principal),
        grupo: None,
        filtro_rapido: None,
        condiciones: Vec::new(),
        busqueda: None,
        orden: Vec::new(),
        // El strip de variantes del editor muestra también las ocultas.
        incluir_ocultos: true,
        offset: 0,
        limit: u32::MAX,
    };
    Ok(query(conn, campos, &req)?.registros)
}

/// Marca u oculta registros (`_auxiliar_`), el "Ocultar" del original: el
/// registro deja de aparecer en consultas normales sin eliminarse.
pub fn set_auxiliar(
    conn: &Conn,
    tabla: Tabla,
    ids: &[i64],
    oculto: bool,
) -> Result<(), MicError> {
    if ids.is_empty() {
        return Ok(());
    }
    let alias = tabla.nombre();
    let marcas = vec!["?"; ids.len()].join(", ");
    let sql =
        format!("UPDATE {alias} SET _auxiliar_ = ?1 WHERE _id_ IN ({marcas})");
    let mut params_v: Vec<SqlValue> = Vec::with_capacity(ids.len() + 1);
    params_v.push(SqlValue::Integer(oculto as i64));
    for &id in ids {
        params_v.push(SqlValue::Integer(id));
    }
    // Los placeholders posicionales continúan tras ?1 en orden de aparición.
    let refs: Vec<&dyn ToSql> = params_v.iter().map(|v| v as &dyn ToSql).collect();
    conn.execute(&sql, refs.as_slice()).map_err(err_sql)?;
    Ok(())
}

/// Ids de todos los registros que cumplen el filtro de `req` (sin paginar).
/// Base de la actualización masiva (ex-frmActGrlDat) con alcance "filtrados".
pub fn ids_de_query(
    conn: &Conn,
    campos: &[CampoDef],
    req: &QueryReq,
) -> Result<Vec<i64>, MicError> {
    let alias = req.tabla.nombre();
    let niveles = niveles_grupo(conn, campos, req)?;
    let construido = query_builder::construir(campos, req, &niveles);
    let where_clause = if construido.where_sql.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", construido.where_sql)
    };
    let sql = format!(
        "SELECT {alias}._id_ FROM {alias}{join}{where_clause} ORDER BY {order}",
        join = construido.join_sql,
        order = construido.order_sql
    );
    let refs: Vec<&dyn ToSql> = construido.params.iter().map(|v| v as &dyn ToSql).collect();
    let mut stmt = conn.prepare(&sql).map_err(err_sql)?;
    let ids = stmt
        .query_map(refs.as_slice(), |row| row.get(0))
        .map_err(err_sql)?
        .collect::<Result<Vec<i64>, _>>()
        .map_err(err_sql)?;
    Ok(ids)
}

/// Suma los campos `totalizable` del conjunto que cumple el filtro de `req`
/// (ex-frmTotalizar). Devuelve también el conteo de registros.
pub fn totalizar(
    conn: &Conn,
    campos: &[CampoDef],
    req: &QueryReq,
) -> Result<mic_core::model::Totales, MicError> {
    let tabla = req.tabla;
    let alias = tabla.nombre();
    let totalizables: Vec<&CampoDef> = campos_de(campos, tabla)
        .into_iter()
        .filter(|c| c.totalizable && !matches!(c.tipo, TipoCampo::Multidato | TipoCampo::Texto))
        .collect();

    let niveles = niveles_grupo(conn, campos, req)?;
    let construido = query_builder::construir(campos, req, &niveles);
    let where_clause = if construido.where_sql.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", construido.where_sql)
    };

    let mut select_cols = vec!["COUNT(*)".to_string()];
    for c in &totalizables {
        select_cols.push(format!("COALESCE(SUM({alias}.{}), 0)", c.col_fisica));
    }
    let sql = format!(
        "SELECT {cols} FROM {alias}{join}{where_clause}",
        cols = select_cols.join(", "),
        join = construido.join_sql
    );
    let refs: Vec<&dyn ToSql> = construido.params.iter().map(|v| v as &dyn ToSql).collect();
    let (registros, sumas): (i64, Vec<f64>) = conn
        .query_row(&sql, refs.as_slice(), |row| {
            let n: i64 = row.get(0)?;
            let mut sumas = Vec::with_capacity(totalizables.len());
            for i in 0..totalizables.len() {
                sumas.push(row.get::<_, f64>(1 + i)?);
            }
            Ok((n, sumas))
        })
        .map_err(err_sql)?;

    Ok(mic_core::model::Totales {
        registros: registros as u64,
        totales: totalizables
            .iter()
            .zip(sumas)
            .map(|(c, suma)| mic_core::model::TotalCampo {
                campo: c.nombre.clone(),
                suma,
            })
            .collect(),
    })
}

/// Estadísticas (ex-Totalizar ampliado): cuenta, suma, media, mediana, moda,
/// mínimo y máximo de los campos numéricos pedidos, sobre el conjunto que
/// cumple el filtro de `req`. Los nombres desconocidos o no numéricos se
/// ignoran. La mediana y la moda se calculan en SQL (orden + offset / group by).
pub fn estadisticas(
    conn: &Conn,
    campos: &[CampoDef],
    req: &QueryReq,
    seleccion: &[String],
) -> Result<mic_core::model::Estadisticas, MicError> {
    use mic_core::model::{Estadisticas, EstadisticaCampo};

    let tabla = req.tabla;
    let alias = tabla.nombre();
    let niveles = niveles_grupo(conn, campos, req)?;
    let construido = query_builder::construir(campos, req, &niveles);
    let where_clause = if construido.where_sql.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", construido.where_sql)
    };
    let join = &construido.join_sql;
    let refs: Vec<&dyn ToSql> = construido.params.iter().map(|v| v as &dyn ToSql).collect();

    // Registros del conjunto (para el encabezado del panel).
    let registros: i64 = conn
        .query_row(
            &format!("SELECT COUNT(*) FROM {alias}{join}{where_clause}"),
            refs.as_slice(),
            |row| row.get(0),
        )
        .map_err(err_sql)?;

    let numericos: Vec<&CampoDef> = seleccion
        .iter()
        .filter_map(|n| campos.iter().find(|c| &c.nombre == n))
        .filter(|c| {
            c.tabla == tabla
                && matches!(
                    c.tipo,
                    TipoCampo::Numerico | TipoCampo::Moneda | TipoCampo::Calculado
                )
        })
        .collect();

    let mut resultado = Vec::with_capacity(numericos.len());
    for c in numericos {
        let col = format!("{alias}.{}", c.col_fisica);
        let no_nulo = if construido.where_sql.is_empty() {
            format!(" WHERE {col} IS NOT NULL")
        } else {
            format!("{where_clause} AND {col} IS NOT NULL")
        };

        // Agregados básicos en una sola pasada.
        let (cuenta, suma, media, minimo, maximo): (i64, f64, Option<f64>, Option<f64>, Option<f64>) =
            conn.query_row(
                &format!(
                    "SELECT COUNT({col}), COALESCE(SUM({col}), 0), AVG({col}), \
                     MIN({col}), MAX({col}) FROM {alias}{join}{where_clause}"
                ),
                refs.as_slice(),
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .map_err(err_sql)?;

        // Mediana: promedio de los 1-2 valores centrales del orden.
        let mediana: Option<f64> = if cuenta == 0 {
            None
        } else {
            conn.query_row(
                &format!(
                    "SELECT AVG(v) FROM (SELECT {col} AS v FROM {alias}{join}{no_nulo} \
                     ORDER BY {col} LIMIT {lim} OFFSET {off})",
                    lim = 2 - (cuenta % 2),
                    off = (cuenta - 1) / 2
                ),
                refs.as_slice(),
                |row| row.get(0),
            )
            .map_err(err_sql)?
        };

        // Moda: el valor más repetido (desempate por el menor valor).
        let (moda, moda_conteo): (Option<f64>, i64) = if cuenta == 0 {
            (None, 0)
        } else {
            conn.query_row(
                &format!(
                    "SELECT {col}, COUNT(*) AS c FROM {alias}{join}{no_nulo} \
                     GROUP BY {col} ORDER BY c DESC, {col} ASC LIMIT 1"
                ),
                refs.as_slice(),
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(err_sql)?
        };

        resultado.push(EstadisticaCampo {
            campo: c.nombre.clone(),
            cuenta: cuenta as u64,
            suma,
            media,
            mediana,
            moda,
            moda_conteo: moda_conteo as u64,
            minimo,
            maximo,
        });
    }

    Ok(Estadisticas {
        registros: registros as u64,
        campos: resultado,
    })
}

/// Actualización masiva (ex-frmActGrlDat): aplica `valores` a TODOS los
/// registros que cumplen el filtro de `req` (sin paginar), recalculando los
/// campos calculados de cada registro. Devuelve cuántos registros tocó.
pub fn actualizar_masivo(
    conn: &mut Conn,
    campos: &[CampoDef],
    motor: Option<&MotorCalculo>,
    req: &QueryReq,
    valores: &Valores,
) -> Result<u64, MicError> {
    let ids = ids_de_query(conn, campos, req)?;
    editar_lote(conn, campos, motor, req.tabla, &ids, valores)?;
    Ok(ids.len() as u64)
}

/// Recalcula los campos calculados de TODOS los registros del álbum (ambas
/// tablas) y reindexa FTS. Devuelve cuántos registros tocó (ex "Act. Calculados").
pub fn recalcular_todo(
    conn: &mut Conn,
    campos: &[CampoDef],
    motor: Option<&MotorCalculo>,
) -> Result<u64, MicError> {
    if motor.is_none() {
        return Ok(0);
    }
    let mut tocados = 0u64;
    for tabla in [Tabla::Principal, Tabla::Variantes] {
        let alias = tabla.nombre();
        let ids: Vec<i64> = {
            let mut stmt = conn
                .prepare(&format!("SELECT _id_ FROM {alias}"))
                .map_err(err_sql)?;
            let ids = stmt
                .query_map([], |row| row.get(0))
                .map_err(err_sql)?
                .collect::<Result<Vec<i64>, _>>()
                .map_err(err_sql)?;
            ids
        };
        for id in ids {
            // `editar` con los valores actuales fuerza el recálculo y la
            // sincronización FTS del registro.
            let actual = obtener(conn, campos, tabla, id)?;
            editar(conn, campos, motor, tabla, id, &actual.valores, None)?;
            tocados += 1;
        }
    }
    Ok(tocados)
}

// --- Helpers dentro de transacción --------------------------------------------

/// Guarda multidatos usando la conexión de la transacción (borra+reinserta).
fn guardar_multidato_tx(
    tx: &rusqlite::Transaction,
    reg_id: i64,
    campo_id: i64,
    principal: bool,
    valores: &[String],
) -> Result<(), MicError> {
    let pr = principal as i64;
    tx.execute(
        "DELETE FROM multidatos WHERE reg_id = ?1 AND campo_id = ?2 AND principal = ?3",
        params![reg_id, campo_id, pr],
    )
    .map_err(err_sql)?;
    let mut stmt = tx
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
    }
    Ok(())
}

/// Asegura una categoría (autocomplete) usando la conexión de la transacción.
fn asegurar_categoria_tx(
    tx: &rusqlite::Transaction,
    campo_id: i64,
    principal: bool,
    valor: &str,
) -> Result<(), MicError> {
    let v = valor.trim();
    if v.is_empty() {
        return Ok(());
    }
    let pr = principal as i64;
    let existe: bool = tx
        .query_row(
            "SELECT 1 FROM categorias \
             WHERE campo_id = ?1 AND principal = ?2 AND valor = ?3 LIMIT 1",
            params![campo_id, pr, v],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if !existe {
        tx.execute(
            "INSERT INTO categorias (campo_id, principal, valor, es_default) \
             VALUES (?1, ?2, ?3, 0)",
            params![campo_id, pr, v],
        )
        .map_err(err_sql)?;
    }
    Ok(())
}

