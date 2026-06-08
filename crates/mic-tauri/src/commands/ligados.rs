//! Comandos de álbumes ligados (ex-frmAlbumsL/frmEdligado/frmstligas del VB6).
//!
//! Una "liga" sincroniza datos DESDE otro álbum `.micdb` HACIA el actual: se
//! elige un campo llave presente en ambos; para cada registro del álbum actual
//! se busca el registro del álbum ligado con la misma llave y se copian los
//! valores de los campos cuyo NOMBRE coincide en ambos (excepto la llave y los
//! calculados del destino, que se recalculan). Opcionalmente, si una llave del
//! ligado no existe en el actual, se da de alta el registro. Solo tabla
//! principal.
//!
//! `liga_actualizar` emite el evento `liga-progreso` cada ~50 registros para
//! que el frontend muestre una barra de progreso (patrón de `migracion.rs`).

use std::collections::HashMap;

use mic_core::calc::MotorCalculo;
use mic_core::error::MicError;
use mic_core::model::{CampoDef, QueryReq, Tabla, TipoCampo, Valor, Valores};
use mic_db::repo_ligados::Liga;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::commands::{en_db, handle};
use crate::state::{AlbumHandle, AppState};

/// Nombre del evento de progreso de actualización de ligas.
const EVENTO_PROGRESO: &str = "liga-progreso";

/// Cada cuántos registros procesados se emite un evento de progreso.
const PASO_PROGRESO: u64 = 50;

/// Payload del evento `liga-progreso`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgresoEvento {
    hechas: u64,
    total: u64,
}

/// Resultado de actualizar una liga.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultadoLiga {
    /// Registros del álbum actual cuyos campos se sincronizaron.
    pub actualizados: u64,
    /// Registros dados de alta porque la llave no existía (si `crear_faltantes`).
    pub creados: u64,
    /// Llaves del álbum ligado sin coincidencia en el actual (no creadas).
    pub sin_coincidencia: u64,
}

/// Lista las ligas definidas en el álbum.
#[tauri::command]
pub async fn ligados_listar(
    state: State<'_, AppState>,
    album_id: u64,
) -> Result<Vec<Liga>, String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let conn = h.db.conn()?;
        mic_db::repo_ligados::listar(&conn)
    })
    .await
}

/// Guarda una liga (`id == 0` crea, en otro caso edita). Devuelve el id.
#[tauri::command]
pub async fn liga_guardar(
    state: State<'_, AppState>,
    album_id: u64,
    liga: Liga,
) -> Result<i64, String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let conn = h.db.conn()?;
        mic_db::repo_ligados::guardar(&conn, &liga)
    })
    .await
}

/// Elimina una liga por id.
#[tauri::command]
pub async fn liga_eliminar(
    state: State<'_, AppState>,
    album_id: u64,
    liga_id: i64,
) -> Result<(), String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let conn = h.db.conn()?;
        mic_db::repo_ligados::eliminar(&conn, liga_id)
    })
    .await
}

/// Actualiza una liga: sincroniza los datos del álbum ligado al actual.
///
/// Emite `liga-progreso` durante el proceso. Devuelve el [`ResultadoLiga`].
#[tauri::command]
pub async fn liga_actualizar(
    app: AppHandle,
    state: State<'_, AppState>,
    album_id: u64,
    liga_id: i64,
) -> Result<ResultadoLiga, String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| sincronizar_liga(&app, h, liga_id)).await
}

/// Actualiza todas las ligas del álbum, en orden. Devuelve un resultado por liga.
#[tauri::command]
pub async fn ligas_actualizar_todas(
    app: AppHandle,
    state: State<'_, AppState>,
    album_id: u64,
) -> Result<Vec<ResultadoLiga>, String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let ligas = {
            let conn = h.db.conn()?;
            mic_db::repo_ligados::listar(&conn)?
        };
        let mut resultados = Vec::with_capacity(ligas.len());
        for liga in &ligas {
            resultados.push(sincronizar_liga(&app, h, liga.id)?);
        }
        Ok(resultados)
    })
    .await
}

/// Lee todos los registros principales de un álbum (sin paginar).
fn leer_todos_principales(
    db: &mic_db::AlbumDb,
    campos: &[CampoDef],
) -> Result<Vec<Valores>, MicError> {
    let req = QueryReq {
        tabla: Tabla::Principal,
        id_principal: None,
        grupo: None,
        filtro_rapido: None,
        condiciones: Vec::new(),
        busqueda: None,
        orden: Vec::new(),
        incluir_ocultos: true,
        offset: 0,
        limit: u32::MAX,
    };
    let conn = db.conn()?;
    let pagina = mic_db::repo_registros::query(&conn, campos, &req)?;
    Ok(pagina.registros.into_iter().map(|r| r.valores).collect())
}

/// Clave textual de un valor para indexar/comparar (vacía = sin llave útil).
fn clave_de(valores: &Valores, llave: &str) -> Option<String> {
    let v = valores.get(llave)?;
    if v.es_nulo() {
        return None;
    }
    let txt = v.como_texto();
    let txt = txt.trim();
    if txt.is_empty() {
        None
    } else {
        Some(txt.to_string())
    }
}

/// Sincroniza una liga del álbum (corre dentro del pool de bloqueo).
///
/// Carga la configuración, los campos y el motor desde el handle de Tauri y
/// delega en [`sincronizar_liga_nucleo`], que es la lógica pura testeable. El
/// único cometido del `app` aquí es emitir el evento de progreso.
fn sincronizar_liga(
    app: &AppHandle,
    h: &AlbumHandle,
    liga_id: i64,
) -> Result<ResultadoLiga, MicError> {
    // Configuración de la liga.
    let liga = {
        let conn = h.db.conn()?;
        mic_db::repo_ligados::obtener(&conn, liga_id)?
    };

    let campos_actual = h.campos();
    let motor = h.motor.read().unwrap_or_else(|e| e.into_inner());
    sincronizar_liga_nucleo(
        &h.db,
        &campos_actual,
        motor.as_ref(),
        &liga,
        |hechas, total| emitir_progreso(app, hechas, total),
    )
}

/// Núcleo puro de la sincronización de una liga, sin dependencias de Tauri.
///
/// Recibe la base del álbum actual, sus campos, el motor de cálculo y la
/// configuración de la liga, más un callback `progreso(hechas, total)` que se
/// invoca periódicamente. Esto permite probar la sincronización directamente,
/// sin `AppHandle` (mismo patrón que el núcleo puro de `importar.rs`).
fn sincronizar_liga_nucleo(
    db: &mic_db::AlbumDb,
    campos_actual: &[CampoDef],
    motor: Option<&MotorCalculo>,
    liga: &Liga,
    mut progreso: impl FnMut(u64, u64),
) -> Result<ResultadoLiga, MicError> {
    // Abrimos el álbum ligado (solo lectura lógica) y leemos su estructura.
    let db_ligado = mic_db::AlbumDb::abrir(std::path::Path::new(&liga.ruta_album))?;
    let campos_ligado = {
        let conn = db_ligado.conn()?;
        mic_db::repo_campos::listar(&conn)?
    };

    // La llave debe existir en ambos álbumes (tabla principal).
    let existe_en = |campos: &[CampoDef], nombre: &str| {
        campos
            .iter()
            .any(|c| c.tabla == Tabla::Principal && c.nombre == nombre)
    };
    if !existe_en(campos_actual, &liga.llave) {
        return Err(MicError::Invalido(format!(
            "el campo llave '{}' no existe en el álbum actual",
            liga.llave
        )));
    }
    if !existe_en(&campos_ligado, &liga.llave) {
        return Err(MicError::Invalido(format!(
            "el campo llave '{}' no existe en el álbum ligado",
            liga.llave
        )));
    }

    // Campos a copiar: nombre coincidente en ambos, de la principal, salvo la
    // llave y los calculados del destino (que se recalculan al editar/crear).
    let nombres_ligado: std::collections::HashSet<&str> = campos_ligado
        .iter()
        .filter(|c| c.tabla == Tabla::Principal)
        .map(|c| c.nombre.as_str())
        .collect();
    let campos_copia: Vec<&CampoDef> = campos_actual
        .iter()
        .filter(|c| {
            c.tabla == Tabla::Principal
                && c.nombre != liga.llave
                && !matches!(c.tipo, TipoCampo::Calculado)
                && nombres_ligado.contains(c.nombre.as_str())
        })
        .collect();

    // Índice del álbum ligado por valor de llave (último gana en duplicados).
    let registros_ligado = leer_todos_principales(&db_ligado, &campos_ligado)?;
    let mut indice: HashMap<String, Valores> = HashMap::with_capacity(registros_ligado.len());
    for valores in registros_ligado {
        if let Some(clave) = clave_de(&valores, &liga.llave) {
            indice.insert(clave, valores);
        }
    }

    // Registros del álbum actual, indexados por su llave para detectar faltantes.
    let registros_actual = leer_todos_principales(db, campos_actual)?;
    let claves_actual: std::collections::HashSet<String> = registros_actual
        .iter()
        .filter_map(|v| clave_de(v, &liga.llave))
        .collect();

    let total = registros_actual.len() as u64
        + if liga.crear_faltantes {
            indice.keys().filter(|k| !claves_actual.contains(*k)).count() as u64
        } else {
            0
        };

    let mut resultado = ResultadoLiga::default();
    let mut hechas = 0u64;
    progreso(hechas, total);

    // 1) Recorre los registros del álbum actual: copia valores del ligado.
    {
        let mut conn = db.conn()?;
        for valores_actual in &registros_actual {
            if let Some(clave) = clave_de(valores_actual, &liga.llave) {
                if let Some(origen) = indice.get(&clave) {
                    // Localiza el id del registro actual por su llave.
                    let id = id_por_llave(&conn, campos_actual, &liga.llave, &clave)?;
                    if let Some(id) = id {
                        // Cargamos el registro ACTUAL completo y sobreponemos solo
                        // los campos a copiar. Pasar un mapa parcial a `editar`
                        // anularía la columna llave y toda columna escalar sin
                        // homónimo en el ligado (UPDATE de todas las columnas);
                        // mismo merge que `importar.rs::aplicar_existente`.
                        let actual = mic_db::repo_registros::obtener(
                            &conn,
                            campos_actual,
                            Tabla::Principal,
                            id,
                        )?;
                        let mut nuevos: Valores = actual.valores.clone();
                        for c in &campos_copia {
                            let v = origen.get(&c.nombre).cloned().unwrap_or(Valor::Nulo(None));
                            nuevos.insert(c.nombre.clone(), v);
                        }
                        mic_db::repo_registros::editar(
                            &mut conn,
                            campos_actual,
                            motor,
                            Tabla::Principal,
                            id,
                            &nuevos,
                            None,
                        )?;
                        resultado.actualizados += 1;
                    }
                }
            }
            hechas += 1;
            if hechas.is_multiple_of(PASO_PROGRESO) {
                progreso(hechas, total);
            }
        }

        // 2) Da de alta las llaves del ligado que no existen en el actual.
        if liga.crear_faltantes {
            for (clave, origen) in &indice {
                if claves_actual.contains(clave) {
                    continue;
                }
                // Mapa parcial al CREAR es legítimo (las columnas ausentes nacen
                // NULL), pero la llave SÍ debe incluirse o el registro nacería sin
                // llave.
                let mut nuevos: Valores = HashMap::new();
                nuevos.insert(liga.llave.clone(), Valor::Texto(clave.clone()));
                for c in &campos_copia {
                    let v = origen.get(&c.nombre).cloned().unwrap_or(Valor::Nulo(None));
                    nuevos.insert(c.nombre.clone(), v);
                }
                mic_db::repo_registros::crear(
                    &mut conn,
                    campos_actual,
                    motor,
                    Tabla::Principal,
                    &nuevos,
                    &HashMap::new(),
                    None,
                    None,
                    None,
                )?;
                resultado.creados += 1;
                hechas += 1;
                if hechas.is_multiple_of(PASO_PROGRESO) {
                    progreso(hechas, total);
                }
            }
        } else {
            resultado.sin_coincidencia =
                indice.keys().filter(|k| !claves_actual.contains(*k)).count() as u64;
        }
    }

    progreso(total, total);
    Ok(resultado)
}

/// Localiza el id del registro principal cuyo valor de llave coincide.
fn id_por_llave(
    conn: &mic_db::pool::Conn,
    campos: &[CampoDef],
    llave: &str,
    valor: &str,
) -> Result<Option<i64>, MicError> {
    let campo = campos
        .iter()
        .find(|c| c.tabla == Tabla::Principal && c.nombre == llave)
        .ok_or_else(|| MicError::Invalido(format!("campo llave '{llave}' no encontrado")))?;
    // `ORDER BY _id_` hace determinista "la primera coincidencia" (id ascendente)
    // cuando hay llaves duplicadas en el álbum actual.
    let sql = format!(
        "SELECT _id_ FROM principal WHERE CAST({} AS TEXT) = ?1 ORDER BY _id_ LIMIT 1",
        campo.col_fisica
    );
    match conn.query_row(&sql, rusqlite::params![valor], |row| row.get(0)) {
        Ok(id) => Ok(Some(id)),
        // Sin coincidencia: no es un error, simplemente no hay registro con esa llave.
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        // Cualquier otro error SQL (p. ej. columna inexistente) sí se propaga.
        Err(e) => Err(MicError::Db(e.to_string())),
    }
}

/// Emite el evento de progreso, registrando (sin propagar) cualquier fallo.
fn emitir_progreso(app: &AppHandle, hechas: u64, total: u64) {
    let payload = ProgresoEvento { hechas, total };
    if let Err(e) = app.emit(EVENTO_PROGRESO, &payload) {
        tracing::warn!(error = %e, "no se pudo emitir progreso de liga");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mic_core::model::CampoNuevo;
    use mic_db::{repo_campos, repo_registros, AlbumDb};

    // --- helpers -----------------------------------------------------------

    /// Ruta temporal única para un `.micdb` de pruebas.
    fn ruta_tmp(nombre: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "mic_lig_{}_{}_{nombre}.micdb",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn nuevo(nombre: &str, tipo: TipoCampo) -> CampoNuevo {
        CampoNuevo {
            nombre: nombre.to_string(),
            tabla: Tabla::Principal,
            tipo,
            decimales: 2,
            totalizable: false,
            formula: None,
            visible: true,
            modificable: true,
            orden_visible: 0,
            formato: None,
        }
    }

    /// Inserta un registro con los `valores` dados (recalcula con el motor).
    fn sembrar(db: &AlbumDb, campos: &[CampoDef], motor: Option<&MotorCalculo>, valores: Valores) {
        let mut conn = db.conn().unwrap();
        repo_registros::crear(
            &mut conn,
            campos,
            motor,
            Tabla::Principal,
            &valores,
            &HashMap::new(),
            None,
            None,
            None,
        )
        .unwrap();
    }

    /// Lee el registro principal cuyo campo `Clave` == `clave`, completo.
    fn por_clave(db: &AlbumDb, campos: &[CampoDef], clave: &str) -> Option<Valores> {
        let conn = db.conn().unwrap();
        let req = QueryReq {
            tabla: Tabla::Principal,
            id_principal: None,
            grupo: None,
            filtro_rapido: None,
            condiciones: vec![],
            busqueda: None,
            orden: vec![],
            incluir_ocultos: true,
            offset: 0,
            limit: u32::MAX,
        };
        let pagina = repo_registros::query(&conn, campos, &req).unwrap();
        for reg in pagina.registros {
            if reg.valores.get("Clave").map(|v| v.como_texto()) == Some(clave.to_string()) {
                let completo =
                    repo_registros::obtener(&conn, campos, Tabla::Principal, reg.id).unwrap();
                return Some(completo.valores);
            }
        }
        None
    }

    fn txt(v: &str) -> Valor {
        Valor::Texto(v.to_string())
    }

    /// Liga "en seco": apunta al `.micdb` ligado por su ruta, llave `Clave`.
    fn liga_para(ruta_ligado: &std::path::Path, crear_faltantes: bool) -> Liga {
        Liga {
            id: 1,
            ruta_album: ruta_ligado.to_string_lossy().into_owned(),
            llave: "Clave".to_string(),
            crear_faltantes,
        }
    }

    // --- regresión: la liga NO debe perder datos ---------------------------

    /// Tras sincronizar, los campos comunes se actualizan desde el ligado, pero
    /// la columna LLAVE y las columnas escalares sin homónimo en el ligado deben
    /// quedar INTACTAS.
    ///
    /// Este test FALLA con el código viejo (que pasaba un mapa parcial a
    /// `editar`, anulando llave y `Notas`) y PASA con el merge sobre el registro
    /// actual completo.
    #[test]
    fn liga_preserva_llave_y_columnas_no_comunes() {
        // Álbum actual: Clave, Nombre, Notas (SIN homónimo en el ligado), Precio.
        let ruta_actual = ruta_tmp("actual");
        let db = AlbumDb::crear(&ruta_actual, "Actual").unwrap();
        {
            let conn = db.conn().unwrap();
            repo_campos::crear(&conn, &nuevo("Clave", TipoCampo::Texto)).unwrap();
            repo_campos::crear(&conn, &nuevo("Nombre", TipoCampo::Texto)).unwrap();
            repo_campos::crear(&conn, &nuevo("Notas", TipoCampo::Texto)).unwrap();
            repo_campos::crear(&conn, &nuevo("Precio", TipoCampo::Numerico)).unwrap();
        }
        let campos = {
            let conn = db.conn().unwrap();
            repo_campos::listar(&conn).unwrap()
        };

        // Álbum ligado: Clave, Nombre, Precio (sin Notas).
        let ruta_ligado = ruta_tmp("ligado");
        let db_lig = AlbumDb::crear(&ruta_ligado, "Ligado").unwrap();
        {
            let conn = db_lig.conn().unwrap();
            repo_campos::crear(&conn, &nuevo("Clave", TipoCampo::Texto)).unwrap();
            repo_campos::crear(&conn, &nuevo("Nombre", TipoCampo::Texto)).unwrap();
            repo_campos::crear(&conn, &nuevo("Precio", TipoCampo::Numerico)).unwrap();
        }
        let campos_lig = {
            let conn = db_lig.conn().unwrap();
            repo_campos::listar(&conn).unwrap()
        };

        // Registro actual: Clave="A-1", Notas="local"; Nombre/Precio vacíos.
        let mut v: Valores = HashMap::new();
        v.insert("Clave".into(), txt("A-1"));
        v.insert("Notas".into(), txt("local"));
        sembrar(&db, &campos, None, v);

        // Registro ligado con la misma llave: trae Nombre y Precio.
        let mut vl: Valores = HashMap::new();
        vl.insert("Clave".into(), txt("A-1"));
        vl.insert("Nombre".into(), txt("Jarrón chino"));
        vl.insert("Precio".into(), Valor::Numero(99.0));
        sembrar(&db_lig, &campos_lig, None, vl);

        // Sincroniza la liga (núcleo puro, sin Tauri).
        let liga = liga_para(&ruta_ligado, false);
        let r = sincronizar_liga_nucleo(&db, &campos, None, &liga, |_, _| {}).unwrap();
        assert_eq!(r.actualizados, 1, "debe actualizar el único registro casado");

        // Tras sincronizar: Nombre/Precio del ligado; Clave y Notas INTACTOS.
        let reg = por_clave(&db, &campos, "A-1").expect("el registro no debe perder su llave");
        assert_eq!(
            reg.get("Clave").map(|v| v.como_texto()),
            Some("A-1".to_string()),
            "la columna llave debe conservarse"
        );
        assert_eq!(
            reg.get("Notas").map(|v| v.como_texto()),
            Some("local".to_string()),
            "la columna sin homónimo en el ligado debe conservarse"
        );
        assert_eq!(
            reg.get("Nombre").map(|v| v.como_texto()),
            Some("Jarrón chino".to_string()),
            "el campo común debe actualizarse desde el ligado"
        );
        assert_eq!(
            reg.get("Precio").and_then(|v| v.como_f64()),
            Some(99.0),
            "el campo común numérico debe actualizarse desde el ligado"
        );

        std::fs::remove_file(&ruta_actual).ok();
        std::fs::remove_file(&ruta_ligado).ok();
    }

    /// Con `crear_faltantes`, el registro dado de alta lleva la llave puesta.
    #[test]
    fn liga_crear_faltantes_incluye_llave() {
        let ruta_actual = ruta_tmp("actual");
        let db = AlbumDb::crear(&ruta_actual, "Actual").unwrap();
        {
            let conn = db.conn().unwrap();
            repo_campos::crear(&conn, &nuevo("Clave", TipoCampo::Texto)).unwrap();
            repo_campos::crear(&conn, &nuevo("Nombre", TipoCampo::Texto)).unwrap();
        }
        let campos = {
            let conn = db.conn().unwrap();
            repo_campos::listar(&conn).unwrap()
        };

        let ruta_ligado = ruta_tmp("ligado");
        let db_lig = AlbumDb::crear(&ruta_ligado, "Ligado").unwrap();
        {
            let conn = db_lig.conn().unwrap();
            repo_campos::crear(&conn, &nuevo("Clave", TipoCampo::Texto)).unwrap();
            repo_campos::crear(&conn, &nuevo("Nombre", TipoCampo::Texto)).unwrap();
        }
        let campos_lig = {
            let conn = db_lig.conn().unwrap();
            repo_campos::listar(&conn).unwrap()
        };

        // El ligado tiene una llave "B-2" que no existe en el actual.
        let mut vl: Valores = HashMap::new();
        vl.insert("Clave".into(), txt("B-2"));
        vl.insert("Nombre".into(), txt("Nuevo"));
        sembrar(&db_lig, &campos_lig, None, vl);

        let liga = liga_para(&ruta_ligado, true);
        let r = sincronizar_liga_nucleo(&db, &campos, None, &liga, |_, _| {}).unwrap();
        assert_eq!(r.creados, 1, "debe dar de alta la llave faltante");

        let reg = por_clave(&db, &campos, "B-2").expect("el registro creado debe existir");
        assert_eq!(
            reg.get("Clave").map(|v| v.como_texto()),
            Some("B-2".to_string()),
            "el registro creado debe llevar la llave puesta"
        );
        assert_eq!(
            reg.get("Nombre").map(|v| v.como_texto()),
            Some("Nuevo".to_string()),
            "el campo común debe copiarse al crear"
        );

        std::fs::remove_file(&ruta_actual).ok();
        std::fs::remove_file(&ruta_ligado).ok();
    }

    /// Un error SQL real en `id_por_llave` (columna física inexistente) debe
    /// propagarse como `MicError`, no tragarse como `Ok(None)`.
    #[test]
    fn id_por_llave_error_sql_se_propaga() {
        let ruta_actual = ruta_tmp("actual");
        let db = AlbumDb::crear(&ruta_actual, "Actual").unwrap();
        {
            let conn = db.conn().unwrap();
            repo_campos::crear(&conn, &nuevo("Clave", TipoCampo::Texto)).unwrap();
        }
        let mut campos = {
            let conn = db.conn().unwrap();
            repo_campos::listar(&conn).unwrap()
        };
        // Forzamos un nombre de columna física inexistente para la llave.
        for c in &mut campos {
            if c.nombre == "Clave" {
                c.col_fisica = "columna_que_no_existe".to_string();
            }
        }

        let conn = db.conn().unwrap();
        let res = id_por_llave(&conn, &campos, "Clave", "A-1");
        assert!(
            matches!(res, Err(MicError::Db(_))),
            "un error SQL real debe propagarse, no convertirse en Ok(None): {res:?}"
        );

        std::fs::remove_file(&ruta_actual).ok();
    }
}
