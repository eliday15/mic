//! Comandos de álbumes ligados (ex-frmAlbumsL/frmEdligado/frmstligas del VB6).
//!
//! Una "liga" sincroniza datos DESDE otro álbum `.micdb` HACIA el actual: se
//! elige un campo llave presente en ambos; para cada registro del álbum actual
//! se busca el registro del álbum ligado con la misma llave y se copian los
//! valores de los campos cuyo NOMBRE coincide en ambos (excepto la llave y los
//! calculados del destino, que se recalculan). Opciones de la liga:
//!
//! - `crear_faltantes`: si una llave del ligado no existe en el actual, se da
//!   de alta el registro.
//! - `importar_campos_nuevos`: los campos que solo existen en el ligado (salvo
//!   calculados y multidatos) se crean en el actual con el mismo tipo y
//!   decimales, y sus valores se copian como los comunes.
//! - Fotos: si el registro actual no tiene foto y el ligado sí, la foto se
//!   importa siempre (se copia a `imagenes/` del álbum actual, con lo que la
//!   miniatura se regenera). Si ambos tienen, `fotos` decide cuál queda.
//!
//! Solo tabla principal.
//!
//! `liga_actualizar` emite el evento `liga-progreso` cada ~50 registros para
//! que el frontend muestre una barra de progreso (patrón de `migracion.rs`).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use mic_core::calc::MotorCalculo;
use mic_core::error::MicError;
use mic_core::model::{
    CampoDef, CampoNuevo, QueryReq, RegistroLigero, Tabla, TipoCampo, Valor, Valores,
};
use mic_db::repo_ligados::{FuenteFotos, Liga};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::commands::registros::nombre_destino_unico;
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
    /// Fotos importadas del álbum ligado (registros sin foto o reemplazadas).
    pub fotos_copiadas: u64,
    /// Nombres de los campos creados en el álbum actual (si `importar_campos_nuevos`).
    pub campos_creados: Vec<String>,
}

/// Comparación de la estructura del álbum actual con la de un álbum a ligar.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InspeccionLiga {
    /// Campos principales presentes en ambos álbumes (se sincronizan).
    pub campos_comunes: Vec<String>,
    /// Campos que solo existen en el ligado y se pueden importar (crear aquí).
    pub campos_solo_ligado: Vec<String>,
    /// Campos que solo existen en el ligado pero no se importan (calculados y
    /// multidatos).
    pub campos_no_importables: Vec<String>,
    /// Registros principales del álbum ligado.
    pub total_registros: u64,
    /// Registros principales del álbum ligado que tienen foto.
    pub con_imagen: u64,
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

/// Compara la estructura del álbum actual con la del álbum `ruta_album` (vista
/// previa del formulario de liga: qué campos coinciden y cuáles solo existen
/// en el ligado).
#[tauri::command]
pub async fn liga_inspeccionar(
    state: State<'_, AppState>,
    album_id: u64,
    ruta_album: String,
) -> Result<InspeccionLiga, String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        inspeccionar_nucleo(&h.campos(), Path::new(&ruta_album))
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
) -> Result<Vec<RegistroLigero>, MicError> {
    let req = QueryReq {
        tabla: Tabla::Principal,
        id_principal: None,
        grupo: None,
        filtro_rapido: None,
        condiciones: Vec::new(),
        busqueda: None,
        orden: Vec::new(),
        incluir_ocultos: true,
        excluidos: Vec::new(),
        offset: 0,
        limit: u32::MAX,
    };
    let conn = db.conn()?;
    let pagina = mic_db::repo_registros::query(&conn, campos, &req)?;
    Ok(pagina.registros)
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

/// ¿El campo del ligado puede crearse en el actual? Los calculados dependen de
/// fórmulas con campos que quizá no existan aquí y los multidatos viven en otra
/// tabla, así que solo se importan los escalares de la principal.
fn es_importable(c: &CampoDef) -> bool {
    c.tabla == Tabla::Principal && !matches!(c.tipo, TipoCampo::Calculado | TipoCampo::Multidato)
}

/// Nombres de los campos de la tabla principal.
fn nombres_principales(campos: &[CampoDef]) -> HashSet<&str> {
    campos
        .iter()
        .filter(|c| c.tabla == Tabla::Principal)
        .map(|c| c.nombre.as_str())
        .collect()
}

/// Carpeta que contiene el archivo `.micdb` (base de las rutas `imagenes/...`).
fn carpeta_de(ruta_album: &Path) -> PathBuf {
    ruta_album
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Ruta absoluta de la imagen `rel` de un álbum con base en `carpeta`, solo si
/// el archivo existe (una ruta a un archivo perdido cuenta como "sin foto",
/// igual que la ve el usuario: placeholder). Mismo cálculo que
/// `thumbs::ruta_imagen_abs`.
fn imagen_existente(carpeta: &Path, rel: Option<&str>) -> Option<PathBuf> {
    let rel = rel.map(str::trim).filter(|r| !r.is_empty())?;
    let abs = carpeta.join(rel.trim_start_matches('/'));
    abs.is_file().then_some(abs)
}

/// Copia `origen` a la carpeta de imágenes `dir` con nombre único y devuelve la
/// ruta relativa (`imagenes/<archivo>`) lista para persistir.
///
/// El mtime de la copia se pone a "ahora": la versión de la imagen (mtime) entra
/// en la URL de la miniatura, y si la copia heredara el mtime del original
/// podría coincidir con la versión de la foto reemplazada y el navegador
/// seguiría mostrando la miniatura vieja.
fn copiar_imagen_a(dir: &Path, origen: &Path) -> Result<String, MicError> {
    std::fs::create_dir_all(dir)?;
    let nombre = nombre_destino_unico(dir, origen);
    let destino = dir.join(&nombre);
    std::fs::copy(origen, &destino).map_err(|e| {
        MicError::Io(format!(
            "no se pudo copiar la imagen a '{}': {e}",
            destino.display()
        ))
    })?;
    std::fs::File::options()
        .write(true)
        .open(&destino)
        .and_then(|f| f.set_modified(std::time::SystemTime::now()))?;
    Ok(format!("imagenes/{nombre}"))
}

/// Abre el álbum ligado y lista sus campos.
fn abrir_ligado(ruta: &Path) -> Result<(mic_db::AlbumDb, Vec<CampoDef>), MicError> {
    let db = mic_db::AlbumDb::abrir(ruta)?;
    let campos = {
        let conn = db.conn()?;
        mic_db::repo_campos::listar(&conn)?
    };
    Ok((db, campos))
}

/// Núcleo puro de [`liga_inspeccionar`]: compara estructuras y cuenta registros.
fn inspeccionar_nucleo(
    campos_actual: &[CampoDef],
    ruta_ligado: &Path,
) -> Result<InspeccionLiga, MicError> {
    let (db_ligado, campos_ligado) = abrir_ligado(ruta_ligado)?;
    let nombres_actual = nombres_principales(campos_actual);

    let mut ins = InspeccionLiga::default();
    for c in campos_ligado.iter().filter(|c| c.tabla == Tabla::Principal) {
        if nombres_actual.contains(c.nombre.as_str()) {
            ins.campos_comunes.push(c.nombre.clone());
        } else if es_importable(c) {
            ins.campos_solo_ligado.push(c.nombre.clone());
        } else {
            ins.campos_no_importables.push(c.nombre.clone());
        }
    }

    let conn = db_ligado.conn()?;
    ins.total_registros = mic_db::repo_registros::total(&conn)?;
    let con_imagen: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM principal WHERE TRIM(COALESCE(_imagen_, '')) <> ''",
            [],
            |row| row.get(0),
        )
        .map_err(|e| MicError::Db(e.to_string()))?;
    ins.con_imagen = con_imagen as u64;
    Ok(ins)
}

/// Sincroniza una liga del álbum (corre dentro del pool de bloqueo).
///
/// Carga la configuración, los campos y el motor desde el handle de Tauri y
/// delega en [`sincronizar_liga_nucleo`], que es la lógica pura testeable. El
/// único cometido del `app` aquí es emitir el evento de progreso. Si la liga
/// puede crear campos, refresca después la caché de campos y el motor del
/// álbum abierto (aunque la sincronización falle a medias, los campos ya
/// creados deben verse).
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
    let resultado = {
        // El guard de lectura del motor debe soltarse antes de refrescar
        // (refrescar_campos toma el lock de escritura).
        let motor = h.motor.read().unwrap_or_else(|e| e.into_inner());
        sincronizar_liga_nucleo(
            &h.db,
            &campos_actual,
            motor.as_ref(),
            &liga,
            |hechas, total| emitir_progreso(app, hechas, total),
        )
    };
    if liga.importar_campos_nuevos {
        h.refrescar_campos()?;
    }
    resultado
}

/// Crea en el álbum actual los campos importables que solo existen en el
/// ligado (mismo tipo, decimales, totalizable y formato), al final del orden
/// visible. Devuelve los nombres creados.
fn crear_campos_nuevos(
    db: &mic_db::AlbumDb,
    campos_actual: &[CampoDef],
    campos_ligado: &[CampoDef],
) -> Result<Vec<String>, MicError> {
    let nombres_actual = nombres_principales(campos_actual);
    let ultimo_orden = campos_actual
        .iter()
        .map(|c| c.orden_visible)
        .max()
        .unwrap_or(-1);
    let conn = db.conn()?;
    let mut creados = Vec::new();
    for (i, c) in campos_ligado
        .iter()
        .filter(|c| es_importable(c) && !nombres_actual.contains(c.nombre.as_str()))
        .enumerate()
    {
        mic_db::repo_campos::crear(
            &conn,
            &CampoNuevo {
                nombre: c.nombre.clone(),
                tabla: Tabla::Principal,
                tipo: c.tipo,
                decimales: c.decimales,
                totalizable: c.totalizable,
                formula: None,
                visible: c.visible,
                modificable: true,
                orden_visible: ultimo_orden + 1 + i as i32,
                formato: c.formato.clone(),
            },
        )?;
        creados.push(c.nombre.clone());
    }
    Ok(creados)
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
    let ruta_ligado = Path::new(&liga.ruta_album);
    // Ligar un álbum consigo mismo leería y escribiría el mismo archivo.
    if let (Ok(a), Ok(b)) = (db.ruta().canonicalize(), ruta_ligado.canonicalize()) {
        if a == b {
            return Err(MicError::Invalido(
                "no se puede ligar un álbum consigo mismo".to_string(),
            ));
        }
    }

    // Abrimos el álbum ligado (solo lectura lógica) y leemos su estructura.
    let (db_ligado, campos_ligado) = abrir_ligado(ruta_ligado)?;

    // La llave debe existir en ambos álbumes (tabla principal).
    if !nombres_principales(campos_actual).contains(liga.llave.as_str()) {
        return Err(MicError::Invalido(format!(
            "el campo llave '{}' no existe en el álbum actual",
            liga.llave
        )));
    }
    if !nombres_principales(&campos_ligado).contains(liga.llave.as_str()) {
        return Err(MicError::Invalido(format!(
            "el campo llave '{}' no existe en el álbum ligado",
            liga.llave
        )));
    }

    let mut resultado = ResultadoLiga::default();

    // Campos que solo existen en el ligado: se crean aquí y, a partir de ese
    // momento, son "comunes" y se copian como los demás.
    let campos_vigentes: Vec<CampoDef> = if liga.importar_campos_nuevos {
        resultado.campos_creados = crear_campos_nuevos(db, campos_actual, &campos_ligado)?;
        let conn = db.conn()?;
        mic_db::repo_campos::listar(&conn)?
    } else {
        campos_actual.to_vec()
    };
    let campos_actual = campos_vigentes.as_slice();

    // Campos a copiar: nombre coincidente en ambos, de la principal, salvo la
    // llave y los calculados del destino (que se recalculan al editar/crear).
    let nombres_ligado = nombres_principales(&campos_ligado);
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
    let mut indice: HashMap<String, RegistroLigero> =
        HashMap::with_capacity(registros_ligado.len());
    for reg in registros_ligado {
        if let Some(clave) = clave_de(&reg.valores, &liga.llave) {
            indice.insert(clave, reg);
        }
    }

    // Registros del álbum actual, indexados por su llave para detectar faltantes.
    let registros_actual = leer_todos_principales(db, campos_actual)?;
    let claves_actual: HashSet<String> = registros_actual
        .iter()
        .filter_map(|r| clave_de(&r.valores, &liga.llave))
        .collect();

    let total = registros_actual.len() as u64
        + if liga.crear_faltantes {
            indice
                .keys()
                .filter(|k| !claves_actual.contains(*k))
                .count() as u64
        } else {
            0
        };

    // Carpetas base de las imágenes de cada álbum.
    let carpeta_ligado = carpeta_de(ruta_ligado);
    let carpeta_actual = carpeta_de(db.ruta());
    let dir_imagenes = db.dir_imagenes();

    let mut hechas = 0u64;
    progreso(hechas, total);

    // 1) Recorre los registros del álbum actual: copia valores del ligado.
    {
        let mut conn = db.conn()?;
        for reg_actual in &registros_actual {
            if let Some(clave) = clave_de(&reg_actual.valores, &liga.llave) {
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
                            let v = origen
                                .valores
                                .get(&c.nombre)
                                .cloned()
                                .unwrap_or(Valor::Nulo(None));
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

                        // Foto: sin foto aquí → se importa siempre; con foto en
                        // ambos → según la opción de la liga.
                        if let Some(src) =
                            imagen_existente(&carpeta_ligado, origen.imagen.as_deref())
                        {
                            let propia =
                                imagen_existente(&carpeta_actual, actual.imagen.as_deref());
                            let copiar = match &propia {
                                None => true,
                                Some(p) => liga.fotos == FuenteFotos::Ligado && *p != src,
                            };
                            if copiar {
                                let rel = copiar_imagen_a(&dir_imagenes, &src)?;
                                mic_db::repo_registros::set_imagen(
                                    &conn,
                                    Tabla::Principal,
                                    id,
                                    &rel,
                                    Some(dir_imagenes.as_path()),
                                )?;
                                resultado.fotos_copiadas += 1;
                            }
                        }
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
                    let v = origen
                        .valores
                        .get(&c.nombre)
                        .cloned()
                        .unwrap_or(Valor::Nulo(None));
                    nuevos.insert(c.nombre.clone(), v);
                }
                // El registro nuevo nace sin foto: si el ligado tiene, se importa.
                let imagen = imagen_existente(&carpeta_ligado, origen.imagen.as_deref())
                    .map(|src| copiar_imagen_a(&dir_imagenes, &src))
                    .transpose()?;
                mic_db::repo_registros::crear(
                    &mut conn,
                    campos_actual,
                    motor,
                    Tabla::Principal,
                    &nuevos,
                    &HashMap::new(),
                    imagen.as_deref(),
                    None,
                    Some(dir_imagenes.as_path()),
                )?;
                resultado.creados += 1;
                if imagen.is_some() {
                    resultado.fotos_copiadas += 1;
                }
                hechas += 1;
                if hechas.is_multiple_of(PASO_PROGRESO) {
                    progreso(hechas, total);
                }
            }
        } else {
            resultado.sin_coincidencia = indice
                .keys()
                .filter(|k| !claves_actual.contains(*k))
                .count() as u64;
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
    use mic_db::{repo_campos, repo_registros, AlbumDb};

    // --- helpers -----------------------------------------------------------

    /// Ruta de un `.micdb` de pruebas dentro de una carpeta temporal única
    /// (cada álbum con su propia carpeta `imagenes/`).
    fn ruta_tmp(nombre: &str) -> std::path::PathBuf {
        // Contador además del reloj: los tests corren en paralelo y la
        // resolución del reloj puede repetir valores.
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "mic_lig_{}_{n}_{}_{nombre}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(format!("{nombre}.micdb"))
    }

    /// Borra la carpeta temporal de un álbum de pruebas (base + imágenes).
    fn limpiar(ruta: &std::path::Path) {
        if let Some(dir) = ruta.parent() {
            std::fs::remove_dir_all(dir).ok();
        }
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
            excluidos: Vec::new(),
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
            fotos: FuenteFotos::Actual,
            importar_campos_nuevos: false,
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
        assert_eq!(
            r.actualizados, 1,
            "debe actualizar el único registro casado"
        );

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

        limpiar(&ruta_actual);
        limpiar(&ruta_ligado);
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

        limpiar(&ruta_actual);
        limpiar(&ruta_ligado);
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

        limpiar(&ruta_actual);
    }

    // --- fotos y campos nuevos ---------------------------------------------

    /// Crea un álbum de pruebas con los campos principales dados.
    fn album_con(
        nombre: &str,
        defs: &[CampoNuevo],
    ) -> (std::path::PathBuf, AlbumDb, Vec<CampoDef>) {
        let ruta = ruta_tmp(nombre);
        let db = AlbumDb::crear(&ruta, nombre).unwrap();
        let campos = {
            let conn = db.conn().unwrap();
            for d in defs {
                repo_campos::crear(&conn, d).unwrap();
            }
            repo_campos::listar(&conn).unwrap()
        };
        (ruta, db, campos)
    }

    /// Inserta un registro `Clave = clave` (más `extra`) con una foto cuyo
    /// contenido es `foto` (si se da), escrita en la carpeta `imagenes/` del álbum.
    fn sembrar_foto(
        db: &AlbumDb,
        campos: &[CampoDef],
        clave: &str,
        extra: &[(&str, Valor)],
        foto: Option<&[u8]>,
    ) {
        let mut v: Valores = HashMap::new();
        v.insert("Clave".into(), txt(clave));
        for (k, val) in extra {
            v.insert((*k).to_string(), val.clone());
        }
        let dir = db.dir_imagenes();
        let rel = foto.map(|bytes| {
            std::fs::write(dir.join(format!("{clave}.jpg")), bytes).unwrap();
            format!("imagenes/{clave}.jpg")
        });
        let mut conn = db.conn().unwrap();
        repo_registros::crear(
            &mut conn,
            campos,
            None,
            Tabla::Principal,
            &v,
            &HashMap::new(),
            rel.as_deref(),
            None,
            Some(dir.as_path()),
        )
        .unwrap();
    }

    /// Ruta relativa y contenido de la foto del registro `Clave = clave`.
    fn foto_de(db: &AlbumDb, campos: &[CampoDef], clave: &str) -> Option<(String, Vec<u8>)> {
        let conn = db.conn().unwrap();
        let id = id_por_llave(&conn, campos, "Clave", clave).unwrap()?;
        let reg = repo_registros::obtener(&conn, campos, Tabla::Principal, id).unwrap();
        let rel = reg.imagen?;
        let abs = carpeta_de(db.ruta()).join(&rel);
        Some((rel, std::fs::read(abs).ok()?))
    }

    fn liga_con(ruta_ligado: &std::path::Path, fotos: FuenteFotos) -> Liga {
        Liga {
            fotos,
            ..liga_para(ruta_ligado, false)
        }
    }

    /// Si el registro actual no tiene foto y el ligado sí, la foto se importa
    /// (copiada a la carpeta del álbum actual), también en registros creados
    /// por `crear_faltantes`.
    #[test]
    fn liga_importa_foto_si_actual_no_tiene() {
        let defs = [nuevo("Clave", TipoCampo::Texto)];
        let (ruta_actual, db, campos) = album_con("actual", &defs);
        let (ruta_ligado, db_lig, campos_lig) = album_con("ligado", &defs);

        sembrar_foto(&db, &campos, "A-1", &[], None);
        sembrar_foto(&db_lig, &campos_lig, "A-1", &[], Some(b"LIGADO-A1"));
        sembrar_foto(&db_lig, &campos_lig, "B-2", &[], Some(b"LIGADO-B2"));

        let liga = Liga {
            crear_faltantes: true,
            ..liga_con(&ruta_ligado, FuenteFotos::Actual)
        };
        let r = sincronizar_liga_nucleo(&db, &campos, None, &liga, |_, _| {}).unwrap();
        assert_eq!(r.fotos_copiadas, 2, "foto del casado + foto del creado");
        assert_eq!(r.creados, 1);

        let (rel, bytes) = foto_de(&db, &campos, "A-1").expect("A-1 debe tener foto");
        assert_eq!(bytes, b"LIGADO-A1");
        assert!(
            rel.starts_with("imagenes/"),
            "ruta relativa al álbum actual: {rel}"
        );
        assert!(
            carpeta_de(&ruta_actual).join(&rel).is_file(),
            "la foto debe copiarse a la carpeta del álbum actual"
        );
        let (_, bytes_b) = foto_de(&db, &campos, "B-2").expect("B-2 creado con foto");
        assert_eq!(bytes_b, b"LIGADO-B2");

        limpiar(&ruta_actual);
        limpiar(&ruta_ligado);
    }

    /// Con foto en ambos: `fotos = actual` la conserva, `fotos = ligado` la reemplaza.
    #[test]
    fn liga_fotos_actual_conserva_y_ligado_reemplaza() {
        let defs = [nuevo("Clave", TipoCampo::Texto)];
        for (fuente, esperado, copiadas) in [
            (FuenteFotos::Actual, &b"ACTUAL"[..], 0),
            (FuenteFotos::Ligado, &b"LIGADO"[..], 1),
        ] {
            let (ruta_actual, db, campos) = album_con("actual", &defs);
            let (ruta_ligado, db_lig, campos_lig) = album_con("ligado", &defs);
            sembrar_foto(&db, &campos, "A-1", &[], Some(b"ACTUAL"));
            sembrar_foto(&db_lig, &campos_lig, "A-1", &[], Some(b"LIGADO"));

            let liga = liga_con(&ruta_ligado, fuente);
            let r = sincronizar_liga_nucleo(&db, &campos, None, &liga, |_, _| {}).unwrap();
            assert_eq!(r.fotos_copiadas, copiadas, "{fuente:?}");
            let (_, bytes) = foto_de(&db, &campos, "A-1").unwrap();
            assert_eq!(bytes, esperado, "{fuente:?}");

            limpiar(&ruta_actual);
            limpiar(&ruta_ligado);
        }
    }

    /// Con `importar_campos_nuevos`, los campos que solo existen en el ligado se
    /// crean (mismo tipo/decimales; calculados no) y se llenan, también en los
    /// registros creados. Sin la opción, la estructura no cambia.
    #[test]
    fn liga_importa_campos_solo_del_ligado() {
        let (ruta_actual, db, campos) = album_con(
            "actual",
            &[
                nuevo("Clave", TipoCampo::Texto),
                nuevo("Nombre", TipoCampo::Texto),
            ],
        );
        let mut peso = nuevo("Peso", TipoCampo::Numerico);
        peso.decimales = 3;
        let mut doble = nuevo("Doble", TipoCampo::Calculado);
        doble.formula = Some("Peso * 2".to_string());
        let (ruta_ligado, db_lig, campos_lig) = album_con(
            "ligado",
            &[
                nuevo("Clave", TipoCampo::Texto),
                nuevo("Nombre", TipoCampo::Texto),
                nuevo("Proveedor", TipoCampo::Texto),
                peso,
                doble,
            ],
        );
        sembrar_foto(&db, &campos, "A-1", &[], None);
        let extra = [
            ("Nombre", txt("Jarrón")),
            ("Proveedor", txt("Acme")),
            ("Peso", Valor::Numero(1.5)),
        ];
        sembrar_foto(&db_lig, &campos_lig, "A-1", &extra, None);
        sembrar_foto(
            &db_lig,
            &campos_lig,
            "C-3",
            &[("Proveedor", txt("Lumina"))],
            None,
        );

        // Sin la opción: no se crea nada.
        let liga = liga_para(&ruta_ligado, false);
        let r = sincronizar_liga_nucleo(&db, &campos, None, &liga, |_, _| {}).unwrap();
        assert!(r.campos_creados.is_empty());
        assert_eq!(
            repo_campos::listar(&db.conn().unwrap()).unwrap().len(),
            campos.len()
        );

        // Con la opción (y crear faltantes).
        let liga = Liga {
            crear_faltantes: true,
            importar_campos_nuevos: true,
            ..liga_para(&ruta_ligado, true)
        };
        let r = sincronizar_liga_nucleo(&db, &campos, None, &liga, |_, _| {}).unwrap();
        let mut creados = r.campos_creados.clone();
        creados.sort();
        assert_eq!(creados, vec!["Peso".to_string(), "Proveedor".to_string()]);

        let vigentes = repo_campos::listar(&db.conn().unwrap()).unwrap();
        let peso = vigentes
            .iter()
            .find(|c| c.nombre == "Peso")
            .expect("Peso creado");
        assert_eq!(peso.tipo, TipoCampo::Numerico);
        assert_eq!(peso.decimales, 3);
        assert!(
            !vigentes.iter().any(|c| c.nombre == "Doble"),
            "los calculados del ligado no se importan"
        );

        let a1 = por_clave(&db, &vigentes, "A-1").unwrap();
        assert_eq!(
            a1.get("Proveedor").map(|v| v.como_texto()),
            Some("Acme".into())
        );
        assert_eq!(a1.get("Peso").and_then(|v| v.como_f64()), Some(1.5));
        assert_eq!(
            a1.get("Nombre").map(|v| v.como_texto()),
            Some("Jarrón".into())
        );
        let c3 = por_clave(&db, &vigentes, "C-3").expect("C-3 creado");
        assert_eq!(
            c3.get("Proveedor").map(|v| v.como_texto()),
            Some("Lumina".into())
        );

        // Repetir la liga no duplica campos.
        let r = sincronizar_liga_nucleo(&db, &vigentes, None, &liga, |_, _| {}).unwrap();
        assert!(r.campos_creados.is_empty());

        limpiar(&ruta_actual);
        limpiar(&ruta_ligado);
    }

    /// Las ligas guardadas antes de las opciones nuevas (sin `fotos` ni
    /// `importarCamposNuevos` en `config_json`) siguen cargando con los valores
    /// por defecto, y las opciones nuevas persisten.
    #[test]
    fn liga_vieja_sin_claves_nuevas_deserializa() {
        let ruta = ruta_tmp("actual");
        let db = AlbumDb::crear(&ruta, "Actual").unwrap();
        let conn = db.conn().unwrap();
        conn.execute(
            "INSERT INTO ligados (nombre, config_json) VALUES ('/x.micdb', ?1)",
            [r#"{"rutaAlbum":"/x.micdb","llave":"Clave","crearFaltantes":true}"#],
        )
        .unwrap();
        let id = conn.last_insert_rowid();
        let vieja = mic_db::repo_ligados::obtener(&conn, id).unwrap();
        assert_eq!(vieja.llave, "Clave");
        assert!(vieja.crear_faltantes);
        assert_eq!(vieja.fotos, FuenteFotos::Actual);
        assert!(!vieja.importar_campos_nuevos);

        // El frontend viejo tampoco manda las claves nuevas.
        let del_front: Liga = serde_json::from_str(
            r#"{"id":0,"rutaAlbum":"/y.micdb","llave":"Clave","crearFaltantes":false}"#,
        )
        .unwrap();
        assert_eq!(del_front.fotos, FuenteFotos::Actual);

        // Ida y vuelta de las opciones nuevas.
        let nueva = Liga {
            id: 0,
            fotos: FuenteFotos::Ligado,
            importar_campos_nuevos: true,
            ..vieja
        };
        let id2 = mic_db::repo_ligados::guardar(&conn, &nueva).unwrap();
        let leida = mic_db::repo_ligados::obtener(&conn, id2).unwrap();
        assert_eq!(leida.fotos, FuenteFotos::Ligado);
        assert!(leida.importar_campos_nuevos);
        drop(conn);
        limpiar(&ruta);
    }

    /// La inspección clasifica los campos del ligado y cuenta registros/fotos.
    #[test]
    fn inspeccionar_clasifica_campos() {
        let (ruta_actual, _db, campos) = album_con(
            "actual",
            &[
                nuevo("Clave", TipoCampo::Texto),
                nuevo("Notas", TipoCampo::Texto),
            ],
        );
        let mut doble = nuevo("Doble", TipoCampo::Calculado);
        doble.formula = Some("1 + 1".to_string());
        let (ruta_ligado, db_lig, campos_lig) = album_con(
            "ligado",
            &[
                nuevo("Clave", TipoCampo::Texto),
                nuevo("Proveedor", TipoCampo::Texto),
                nuevo("Etiquetas", TipoCampo::Multidato),
                doble,
            ],
        );
        sembrar_foto(&db_lig, &campos_lig, "A-1", &[], Some(b"x"));
        sembrar_foto(&db_lig, &campos_lig, "B-2", &[], None);

        let ins = inspeccionar_nucleo(&campos, &ruta_ligado).unwrap();
        assert_eq!(ins.campos_comunes, vec!["Clave".to_string()]);
        assert_eq!(ins.campos_solo_ligado, vec!["Proveedor".to_string()]);
        let mut no_imp = ins.campos_no_importables.clone();
        no_imp.sort();
        assert_eq!(no_imp, vec!["Doble".to_string(), "Etiquetas".to_string()]);
        assert_eq!(ins.total_registros, 2);
        assert_eq!(ins.con_imagen, 1);

        limpiar(&ruta_actual);
        limpiar(&ruta_ligado);
    }

    /// Ligar un álbum consigo mismo se rechaza.
    #[test]
    fn liga_consigo_mismo_se_rechaza() {
        let (ruta, db, campos) = album_con("actual", &[nuevo("Clave", TipoCampo::Texto)]);
        let liga = liga_para(&ruta, false);
        let res = sincronizar_liga_nucleo(&db, &campos, None, &liga, |_, _| {});
        assert!(matches!(res, Err(MicError::Invalido(_))), "{res:?}");
        limpiar(&ruta);
    }
}
