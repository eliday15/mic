//! Orquestación de la migración de un álbum `.mdb` (Access/Jet) a `.micdb`
//! (SQLite de MIC 3.0), y del importador de plantillas `.xms`.
//!
//! # Flujo de [`migrar`]
//! 1. Crea el `.micdb` ([`mic_db::AlbumDb::crear`]).
//! 2. Lee `propiedades` → crea los campos con [`mic_db::repo_campos`].
//! 3. Vuelca `Principal` → tabla `principal` (INSERTs directos en transacción,
//!    mapeando `_imagen_`, `_id_`, `_auxiliar_`, `_variantes_` y las columnas
//!    dinámicas por nombre).
//! 4. `Variantes` (si existe), enlazando `_idprincipal_`.
//! 5. `Multidatos` (resuelve `Campo_n` → `campo_id`).
//! 6. `Categorias`, `Grupos`, `FiltrosAv` (avisos si algún operador no mapea).
//! 7. Copia la carpeta `imagenes/` junto al `.mdb` si existe.
//! 8. Reindexa FTS y recalcula los campos calculados (las discrepancias se
//!    reportan como advertencias, nunca como error).
//!
//! Todas las cadenas se decodifican Windows-1252 → UTF-8 en [`crate::csv_parser`].

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use mic_core::error::MicError;
use mic_core::model::{CampoDef, CampoNuevo, TipoCampo, Valor, Valores};
use serde::{Deserialize, Serialize};

use mic_db::pool::Conn;
use mic_db::AlbumDb;
use mic_db::{fts, repo_campos, repo_filtros, repo_grupos};
use rusqlite::params;

use crate::csv_parser::TablaCsv;
use crate::jet;
use crate::paths;
use crate::type_map::{self, parse_numero};

/// Inspección previa de un `.mdb`: qué contiene, sin migrar nada.
///
/// Alimenta el diálogo de migración (`migracion_inspeccionar`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdbInspeccion {
    /// Nombres de las tablas presentes en el `.mdb`.
    pub tablas: Vec<String>,
    /// Campos de usuario detectados: `(nombre, tipo legible)`.
    pub campos: Vec<(String, String)>,
    /// Número estimado de registros en `Principal`.
    pub total_estimado: u64,
    /// `true` si hay tabla `Variantes` con filas.
    pub tiene_variantes: bool,
}

/// Reporte del resultado de una migración.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigracionReporte {
    pub filas_principal: u64,
    pub filas_variantes: u64,
    pub filas_multidatos: u64,
    pub imagenes_encontradas: u64,
    /// Rutas de imágenes referenciadas que no existen en disco.
    pub imagenes_faltantes: Vec<String>,
    /// Avisos no fatales (operadores de filtro no mapeables, discrepancias de
    /// calculados, tablas opcionales ausentes, …).
    pub advertencias: Vec<String>,
}

/// Callback de progreso: `(fase, hechas, total)`.
///
/// La capa Tauri lo usa para emitir el evento `migracion-progreso`.
pub type ProgresoMigracion = Box<dyn Fn(&str, u64, u64) + Send>;

/// Nombre legible de un [`TipoCampo`] para la inspección.
fn tipo_legible(t: TipoCampo) -> &'static str {
    match t {
        TipoCampo::Texto => "texto",
        TipoCampo::Numerico => "numérico",
        TipoCampo::Moneda => "moneda",
        TipoCampo::Fecha => "fecha",
        TipoCampo::Calculado => "calculado",
        TipoCampo::Multidato => "multidato",
    }
}

/// Busca una tabla por nombre (case-insensitive) en una lista.
fn existe_tabla(tablas: &[String], nombre: &str) -> bool {
    tablas.iter().any(|t| t.eq_ignore_ascii_case(nombre))
}

/// Resuelve el nombre real de una tabla (case-insensitive) → nombre tal cual.
fn nombre_tabla<'a>(tablas: &'a [String], nombre: &str) -> Option<&'a str> {
    tablas
        .iter()
        .find(|t| t.eq_ignore_ascii_case(nombre))
        .map(|s| s.as_str())
}

/// Lee una tabla del `.mdb` con [`jet::leer_tabla`] y, si jetdb tuvo que omitir
/// filas por errores de parseo, lo registra como advertencia en el reporte.
///
/// Las filas omitidas NUNCA son un error de la migración: un álbum con una fila
/// corrupta debe migrar el resto y avisar de las que faltan.
fn leer_para_migrar(
    ruta_mdb: &Path,
    tabla: &str,
    reporte: &mut MigracionReporte,
) -> Result<TablaCsv, MicError> {
    let leida = jet::leer_tabla(ruta_mdb, tabla)?;
    if leida.omitidas > 0 {
        reporte.advertencias.push(format!(
            "la tabla '{tabla}' tenía {} fila(s) que no se pudieron leer y se omitieron",
            leida.omitidas
        ));
    }
    Ok(leida.csv)
}

// ---------------------------------------------------------------------------
// Copia de trabajo local
// ---------------------------------------------------------------------------

/// Copia de trabajo local del `.mdb` de origen (se borra al soltar el guard).
///
/// Los álbumes del MIC clásico suelen vivir en carpetas de red de un servidor
/// (o en unidades en la nube). jetdb lee el archivo en proceso, pero hace muchas
/// lecturas aleatorias, que sobre SMB/nube son lentísimas o se atascan. Copiar
/// el archivo UNA sola vez a disco local (lectura secuencial, lo que las redes
/// sí hacen bien) y trabajar sobre la copia es más rápido y a prueba de cuelgues.
struct CopiaLocal {
    ruta: PathBuf,
}

impl Drop for CopiaLocal {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.ruta);
    }
}

/// Plazo máximo para copiar el `.mdb` a disco local.
const PLAZO_COPIA: std::time::Duration = std::time::Duration::from_secs(300);

/// Copia `origen` a un temporal local único y devuelve su guard.
///
/// La copia corre en un hilo vigilado por un plazo: una unidad de red muerta
/// puede atascar hasta un simple `fs::copy`, y la app no debe colgarse nunca.
fn copia_local(origen: &Path) -> Result<CopiaLocal, MicError> {
    static CONSECUTIVO: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = CONSECUTIVO.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let destino = std::env::temp_dir().join(format!(
        "mic-migracion-{}-{n}.mdb",
        std::process::id()
    ));

    let (tx, rx) = std::sync::mpsc::channel();
    let origen_hilo = origen.to_path_buf();
    let destino_hilo = destino.clone();
    std::thread::spawn(move || {
        let _ = tx.send(std::fs::copy(&origen_hilo, &destino_hilo));
    });

    match rx.recv_timeout(PLAZO_COPIA) {
        Ok(Ok(_)) => Ok(CopiaLocal { ruta: destino }),
        Ok(Err(e)) => {
            let _ = std::fs::remove_file(&destino);
            Err(MicError::Migracion(format!(
                "no se pudo copiar el .mdb a disco local para procesarlo: {e}"
            )))
        }
        Err(_) => Err(MicError::Migracion(format!(
            "la lectura del .mdb tardó más de {} s y se canceló. La carpeta \
             de origen (¿red o nube?) no responde; copie el archivo a esta \
             computadora e inténtelo de nuevo desde ahí.",
            PLAZO_COPIA.as_secs()
        ))),
    }
}

// ---------------------------------------------------------------------------
// Inspección
// ---------------------------------------------------------------------------

/// Inspecciona un `.mdb` sin migrarlo: lista tablas, campos de usuario, total
/// estimado y si tiene variantes.
pub fn inspeccionar(ruta_mdb: &Path) -> Result<MdbInspeccion, MicError> {
    if !ruta_mdb.exists() {
        return Err(MicError::NoEncontrado(format!(
            "archivo .mdb no encontrado: {}",
            ruta_mdb.display()
        )));
    }

    // Trabaja sobre una copia local: el origen suele estar en una carpeta de
    // red y jetdb hace lecturas aleatorias que sobre SMB son lentas (copiar a
    // local una vez, secuencialmente, es lo que las redes sí hacen bien).
    let local = copia_local(ruta_mdb)?;
    let ruta_mdb = local.ruta.as_path();

    let tablas = jet::tablas(ruta_mdb)?;

    // Campos de usuario desde propiedades.
    let campos = if existe_tabla(&tablas, "propiedades") {
        let nom = nombre_tabla(&tablas, "propiedades").unwrap();
        let props = jet::leer_tabla(ruta_mdb, nom)?.csv;
        type_map::mapear_campos(&props)
            .into_iter()
            .map(|c| (c.def.nombre, tipo_legible(c.def.tipo).to_string()))
            .collect()
    } else {
        Vec::new()
    };

    // Total estimado de Principal.
    let total_estimado = if existe_tabla(&tablas, "Principal") {
        let nom = nombre_tabla(&tablas, "Principal").unwrap();
        jet::leer_tabla(ruta_mdb, nom)?.csv.len() as u64
    } else {
        0
    };

    // Variantes con filas.
    let tiene_variantes = if existe_tabla(&tablas, "Variantes") {
        let nom = nombre_tabla(&tablas, "Variantes").unwrap();
        !jet::leer_tabla(ruta_mdb, nom)?.csv.is_empty()
    } else {
        false
    };

    Ok(MdbInspeccion {
        tablas,
        campos,
        total_estimado,
        tiene_variantes,
    })
}

// ---------------------------------------------------------------------------
// Migración
// ---------------------------------------------------------------------------

/// Contexto compartido durante la migración (campos creados, índices).
struct Contexto {
    /// Todos los campos del álbum nuevo (con su id asignado).
    campos: Vec<CampoDef>,
    /// nombre de campo → id (para multidatos y resolución por nombre).
    por_nombre: HashMap<String, i64>,
}

impl Contexto {
    fn campo_por_nombre(&self, nombre: &str) -> Option<&CampoDef> {
        self.por_nombre
            .get(nombre)
            .and_then(|id| self.campos.iter().find(|c| c.id == *id))
    }
}

/// Migra un `.mdb` a un `.micdb` nuevo en `destino`.
///
/// `progreso(fase, hechas, total)` se invoca a lo largo del proceso. Devuelve el
/// [`MigracionReporte`] con conteos, imágenes y advertencias.
pub fn migrar(
    ruta_mdb: &Path,
    destino: &Path,
    progreso: ProgresoMigracion,
) -> Result<MigracionReporte, MicError> {
    if !ruta_mdb.exists() {
        return Err(MicError::NoEncontrado(format!(
            "archivo .mdb no encontrado: {}",
            ruta_mdb.display()
        )));
    }

    // OJO: las imágenes del álbum viven JUNTO AL .mdb ORIGINAL (carpeta del
    // servidor); este directorio debe calcularse antes de cambiar a la copia.
    let dir_mdb = ruta_mdb.parent().unwrap_or_else(|| Path::new("."));

    // Los datos, en cambio, se leen de una copia local: jetdb hace muchas
    // lecturas aleatorias y sobre una carpeta de red eso es lentísimo; copiar
    // el archivo una vez (lectura secuencial) y leer la copia es más rápido.
    let local = copia_local(ruta_mdb)?;
    let ruta_mdb = local.ruta.as_path();

    let mut reporte = MigracionReporte::default();

    let nombre_album = destino
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Álbum migrado".to_string());

    progreso("Preparando", 0, 0);
    let db = AlbumDb::crear(destino, &nombre_album)?;
    let dir_imagenes_destino = db.dir_imagenes();

    let tablas = jet::tablas(ruta_mdb)?;

    // --- 1) Campos (propiedades) -------------------------------------------
    progreso("Campos", 0, 0);
    let ctx = migrar_campos(ruta_mdb, &tablas, &db, &mut reporte)?;

    // --- 2) Principal ------------------------------------------------------
    if existe_tabla(&tablas, "Principal") {
        let nom = nombre_tabla(&tablas, "Principal").unwrap().to_string();
        let datos = leer_para_migrar(ruta_mdb, &nom, &mut reporte)?;
        reporte.filas_principal =
            migrar_registros(&db, &ctx, &datos, true, dir_mdb, &progreso, &mut reporte)?;
    }

    // --- 3) Variantes ------------------------------------------------------
    if existe_tabla(&tablas, "Variantes") {
        let nom = nombre_tabla(&tablas, "Variantes").unwrap().to_string();
        let datos = leer_para_migrar(ruta_mdb, &nom, &mut reporte)?;
        if !datos.is_empty() {
            reporte.filas_variantes = migrar_registros(
                &db, &ctx, &datos, false, dir_mdb, &progreso, &mut reporte,
            )?;
        }
    }

    // --- 4) Multidatos -----------------------------------------------------
    if existe_tabla(&tablas, "Multidatos") {
        let nom = nombre_tabla(&tablas, "Multidatos").unwrap().to_string();
        let datos = leer_para_migrar(ruta_mdb, &nom, &mut reporte)?;
        reporte.filas_multidatos = migrar_multidatos(&db, &ctx, &datos, &mut reporte)?;
    }

    // --- 5) Categorias -----------------------------------------------------
    if existe_tabla(&tablas, "Categorias") {
        let nom = nombre_tabla(&tablas, "Categorias").unwrap().to_string();
        let datos = leer_para_migrar(ruta_mdb, &nom, &mut reporte)?;
        migrar_categorias(&db, &ctx, &datos, &mut reporte)?;
    }

    // --- 6) Grupos ---------------------------------------------------------
    if existe_tabla(&tablas, "Grupos") {
        let nom = nombre_tabla(&tablas, "Grupos").unwrap().to_string();
        let datos = leer_para_migrar(ruta_mdb, &nom, &mut reporte)?;
        migrar_grupos(&db, &datos, &mut reporte)?;
    }

    // --- 7) FiltrosAv ------------------------------------------------------
    if existe_tabla(&tablas, "FiltrosAv") {
        let nom = nombre_tabla(&tablas, "FiltrosAv").unwrap().to_string();
        let datos = leer_para_migrar(ruta_mdb, &nom, &mut reporte)?;
        migrar_filtros(&db, &datos, &mut reporte)?;
    }

    // --- 8) Imágenes -------------------------------------------------------
    progreso("Imágenes", 0, 0);
    copiar_imagenes(dir_mdb, &dir_imagenes_destino, &mut reporte);
    verificar_imagenes(&db, &dir_imagenes_destino, &mut reporte)?;

    // --- 9) FTS + recálculo ------------------------------------------------
    progreso("Indexando", 0, 0);
    {
        let conn = db.conn()?;
        fts::reindexar(&conn, &ctx.campos)?;
    }
    recalcular_calculados(&db, &ctx, &mut reporte)?;

    progreso("Completado", reporte.filas_principal, reporte.filas_principal);
    Ok(reporte)
}

/// Lee `propiedades` y crea los campos con `repo_campos::crear`.
fn migrar_campos(
    ruta_mdb: &Path,
    tablas: &[String],
    db: &AlbumDb,
    reporte: &mut MigracionReporte,
) -> Result<Contexto, MicError> {
    let mut campos = Vec::new();
    let mut por_nombre = HashMap::new();

    if let Some(nom) = nombre_tabla(tablas, "propiedades").map(|s| s.to_string()) {
        let props = leer_para_migrar(ruta_mdb, &nom, reporte)?;
        let origen = type_map::mapear_campos(&props);
        let conn = db.conn()?;
        for c in origen {
            match repo_campos::crear(&conn, &c.def) {
                Ok(def) => {
                    por_nombre.insert(def.nombre.clone(), def.id);
                    campos.push(def);
                }
                Err(e) => reporte.advertencias.push(format!(
                    "no se pudo crear el campo '{}': {e}",
                    c.def.nombre
                )),
            }
        }
    } else {
        reporte
            .advertencias
            .push("el .mdb no tiene tabla 'propiedades'; sin campos de usuario".into());
    }

    Ok(Contexto { campos, por_nombre })
}

/// Inserta las filas de Principal/Variantes con INSERTs directos en una sola
/// transacción (rápido para datos masivos). Devuelve el número de filas.
fn migrar_registros(
    db: &AlbumDb,
    ctx: &Contexto,
    datos: &TablaCsv,
    principal: bool,
    dir_mdb: &Path,
    progreso: &ProgresoMigracion,
    reporte: &mut MigracionReporte,
) -> Result<u64, MicError> {
    let tabla_sql = if principal { "principal" } else { "variantes" };
    let fase = if principal { "Principal" } else { "Variantes" };
    let total = datos.len() as u64;

    // Campos de esta tabla, mapeados a su índice de columna en el CSV por nombre.
    let campos_tabla: Vec<&CampoDef> = ctx
        .campos
        .iter()
        .filter(|c| {
            let es_ppal = matches!(c.tabla, mic_core::model::Tabla::Principal);
            es_ppal == principal
        })
        .collect();

    // Índices de columnas de sistema en el CSV.
    let i_id = datos.indice("_id_").or_else(|| datos.indice("Id"));
    let i_imagen = datos.indice("_imagen_");
    let i_aux = datos.indice("_auxiliar_");
    let i_variantes = datos.indice("_variantes_");
    let i_idprincipal = datos
        .indice("_idprincipal_")
        .or_else(|| datos.indice("_idprincipal"));

    // Por cada campo de usuario: nombre original → índice en el CSV.
    // El CSV usa el nombre tal cual de Access (igual al nombre del campo nuevo).
    let mut idx_campo: Vec<(usize, &CampoDef)> = Vec::new();
    for c in &campos_tabla {
        if let Some(i) = datos.indice(&c.nombre) {
            idx_campo.push((i, c));
        }
    }

    let mut conn = db.conn()?;
    let tx = conn.transaction().map_err(err_sql)?;
    let mut insertadas = 0u64;

    for (n, fila) in datos.filas.iter().enumerate() {
        let mut cols: Vec<String> = Vec::new();
        let mut vals: Vec<rusqlite::types::Value> = Vec::new();

        // _id_ (preservamos el id original si existe).
        if let Some(idp) = i_id.and_then(|i| fila.get(i)).and_then(|s| s.trim().parse::<i64>().ok())
        {
            cols.push("_id_".into());
            vals.push(rusqlite::types::Value::Integer(idp));
        }

        // _imagen_ + versión.
        if let Some(raw) = i_imagen.and_then(|i| fila.get(i)) {
            let rel = paths::normalizar_imagen(raw, dir_mdb);
            if !rel.is_empty() {
                cols.push("_imagen_".into());
                vals.push(rusqlite::types::Value::Text(rel));
            }
        }

        // _auxiliar_.
        let aux = i_aux
            .and_then(|i| fila.get(i))
            .map(|s| type_map::jet_bool(s))
            .unwrap_or(false);
        cols.push("_auxiliar_".into());
        vals.push(rusqlite::types::Value::Integer(aux as i64));

        if principal {
            // _variantes_.
            let tv = i_variantes
                .and_then(|i| fila.get(i))
                .map(|s| type_map::jet_bool(s))
                .unwrap_or(false);
            cols.push("_variantes_".into());
            vals.push(rusqlite::types::Value::Integer(tv as i64));
        } else {
            // _idprincipal_ obligatorio en variantes.
            let idp = i_idprincipal
                .and_then(|i| fila.get(i))
                .and_then(|s| s.trim().parse::<i64>().ok());
            match idp {
                Some(v) => {
                    cols.push("_idprincipal_".into());
                    vals.push(rusqlite::types::Value::Integer(v));
                }
                None => {
                    reporte.advertencias.push(format!(
                        "variante en fila {} sin _idprincipal_ válido; omitida",
                        n + 1
                    ));
                    continue;
                }
            }
        }

        // Columnas dinámicas por campo.
        for (i, c) in &idx_campo {
            let crudo = fila.get(*i).map(|s| s.as_str()).unwrap_or("");
            // Los multidatos guardan conteo; se rellenan al migrar Multidatos.
            if matches!(c.tipo, TipoCampo::Multidato) {
                continue;
            }
            let v = convertir_valor_sql(crudo, c.tipo);
            cols.push(c.col_fisica.clone());
            vals.push(v);
        }

        let placeholders = (1..=cols.len())
            .map(|i| format!("?{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "INSERT INTO {tabla_sql} ({}) VALUES ({placeholders})",
            cols.join(", ")
        );
        let refs: Vec<&dyn rusqlite::ToSql> =
            vals.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
        tx.execute(&sql, refs.as_slice()).map_err(err_sql)?;
        insertadas += 1;

        if (n as u64).is_multiple_of(500) {
            progreso(fase, n as u64, total);
        }
    }

    tx.commit().map_err(err_sql)?;
    progreso(fase, total, total);
    Ok(insertadas)
}

/// Convierte un valor crudo del CSV al `rusqlite::Value` adecuado al tipo.
fn convertir_valor_sql(crudo: &str, tipo: TipoCampo) -> rusqlite::types::Value {
    use rusqlite::types::Value;
    let t = crudo.trim();
    if t.is_empty() {
        return Value::Null;
    }
    match tipo {
        TipoCampo::Numerico | TipoCampo::Moneda | TipoCampo::Calculado => {
            match parse_numero(t) {
                Some(n) => Value::Real(n),
                None => Value::Null,
            }
        }
        TipoCampo::Fecha => match normalizar_fecha_iso(t) {
            Some(iso) => Value::Text(iso),
            None => Value::Null,
        },
        // Texto y multidato (este último no llega aquí).
        _ => Value::Text(t.to_string()),
    }
}

/// Normaliza una fecha a ISO `YYYY-MM-DD`.
///
/// `mdb-export` ya emite ISO gracias a `-D '%Y-%m-%d'`, pero por robustez también
/// aceptamos `DD/MM/YYYY` y `MM/DD/YYYY` (volcados antiguos).
fn normalizar_fecha_iso(s: &str) -> Option<String> {
    let t = s.trim();
    // Ya ISO (posible parte de hora que recortamos).
    let solo_fecha = t.split([' ', 'T']).next().unwrap_or(t);
    if let Ok(d) = chrono::NaiveDate::parse_from_str(solo_fecha, "%Y-%m-%d") {
        return Some(d.format("%Y-%m-%d").to_string());
    }
    for fmt in ["%d/%m/%Y", "%m/%d/%Y", "%d-%m-%Y"] {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(solo_fecha, fmt) {
            return Some(d.format("%Y-%m-%d").to_string());
        }
    }
    None
}

/// Migra la tabla Multidatos: resuelve `Campo_n` → `campo_id` y rellena la tabla
/// `multidatos` + el conteo en la columna física del campo.
fn migrar_multidatos(
    db: &AlbumDb,
    ctx: &Contexto,
    datos: &TablaCsv,
    reporte: &mut MigracionReporte,
) -> Result<u64, MicError> {
    let i_id = datos.indice("Id").or_else(|| datos.indice("_id_"));
    let i_principal = datos.indice("Principal");
    let i_campo = datos.indice("Campo_n").or_else(|| datos.indice("Campo_N"));
    let i_valor = datos.indice("Valor");

    let (i_id, i_campo, i_valor) = match (i_id, i_campo, i_valor) {
        (Some(a), Some(b), Some(c)) => (a, b, c),
        _ => {
            reporte
                .advertencias
                .push("tabla Multidatos con columnas inesperadas; omitida".into());
            return Ok(0);
        }
    };

    // Agrupa (reg_id, principal, campo_id) → valores, contando conteos por
    // (tabla, reg_id, campo_id) para actualizar la columna física.
    let mut grupos: HashMap<(i64, bool, i64), Vec<String>> = HashMap::new();
    let mut insertadas = 0u64;
    let mut campos_no_encontrados: Vec<String> = Vec::new();

    for fila in &datos.filas {
        let id = fila
            .get(i_id)
            .and_then(|s| s.trim().parse::<i64>().ok());
        let id = match id {
            Some(v) => v,
            None => continue,
        };
        let principal = i_principal
            .and_then(|i| fila.get(i))
            .map(|s| type_map::jet_bool(s))
            .unwrap_or(true);
        let nombre_campo = fila.get(i_campo).map(|s| s.trim()).unwrap_or("");
        let valor = fila.get(i_valor).map(|s| s.trim()).unwrap_or("");
        if nombre_campo.is_empty() || valor.is_empty() {
            continue;
        }

        let campo_id = match ctx.campo_por_nombre(nombre_campo) {
            Some(c) => c.id,
            None => {
                if !campos_no_encontrados.iter().any(|n| n == nombre_campo) {
                    campos_no_encontrados.push(nombre_campo.to_string());
                }
                continue;
            }
        };

        grupos
            .entry((id, principal, campo_id))
            .or_default()
            .push(valor.to_string());
    }

    for n in campos_no_encontrados {
        reporte
            .advertencias
            .push(format!("multidato de campo desconocido '{n}'; omitido"));
    }

    // Inserta en una transacción + actualiza conteos en columna física.
    let mut conn = db.conn()?;
    let tx = conn.transaction().map_err(err_sql)?;
    for ((reg_id, principal, campo_id), valores) in &grupos {
        let pr = *principal as i64;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO multidatos (reg_id, principal, campo_id, valor) \
                     VALUES (?1, ?2, ?3, ?4)",
                )
                .map_err(err_sql)?;
            for v in valores {
                stmt.execute(params![reg_id, pr, campo_id, v])
                    .map_err(err_sql)?;
                insertadas += 1;
            }
        }
        // Conteo en la columna física del campo.
        if let Some(campo) = ctx.campos.iter().find(|c| c.id == *campo_id) {
            let tabla_sql = if *principal { "principal" } else { "variantes" };
            let _ = tx.execute(
                &format!(
                    "UPDATE {tabla_sql} SET {} = ?1 WHERE _id_ = ?2",
                    campo.col_fisica
                ),
                params![valores.len() as i64, reg_id],
            );
        }
    }
    tx.commit().map_err(err_sql)?;
    Ok(insertadas)
}

/// Migra la tabla Categorias → `categorias` (autocomplete de multidatos).
fn migrar_categorias(
    db: &AlbumDb,
    ctx: &Contexto,
    datos: &TablaCsv,
    reporte: &mut MigracionReporte,
) -> Result<(), MicError> {
    let i_campo = datos.indice("Campo_n").or_else(|| datos.indice("Campo_N"));
    let i_principal = datos.indice("Principal");
    let i_valor = datos.indice("Valor");
    let i_default = datos.indice("Default");

    let (i_campo, i_valor) = match (i_campo, i_valor) {
        (Some(a), Some(b)) => (a, b),
        _ => {
            reporte
                .advertencias
                .push("tabla Categorias con columnas inesperadas; omitida".into());
            return Ok(());
        }
    };

    let conn = db.conn()?;
    for fila in &datos.filas {
        let nombre_campo = fila.get(i_campo).map(|s| s.trim()).unwrap_or("");
        let valor = fila.get(i_valor).map(|s| s.trim()).unwrap_or("");
        if nombre_campo.is_empty() || valor.is_empty() {
            continue;
        }
        let principal = i_principal
            .and_then(|i| fila.get(i))
            .map(|s| type_map::jet_bool(s))
            .unwrap_or(true);
        let es_default = i_default
            .and_then(|i| fila.get(i))
            .map(|s| type_map::jet_bool(s))
            .unwrap_or(false);

        let campo_id = match ctx.campo_por_nombre(nombre_campo) {
            Some(c) => c.id,
            None => continue,
        };
        conn.execute(
            "INSERT INTO categorias (campo_id, principal, valor, es_default) \
             VALUES (?1, ?2, ?3, ?4)",
            params![campo_id, principal as i64, valor, es_default as i64],
        )
        .map_err(err_sql)?;
    }
    Ok(())
}

/// Migra la tabla Grupos → `grupos` vía `repo_grupos::guardar`.
fn migrar_grupos(
    db: &AlbumDb,
    datos: &TablaCsv,
    reporte: &mut MigracionReporte,
) -> Result<(), MicError> {
    let i_nombre = datos.indice("Nombre");
    let i_por = datos.indice("Por");
    let i_luego1 = datos.indice("Luego1");
    let i_luego2 = datos.indice("Luego2");

    let (i_nombre, i_por) = match (i_nombre, i_por) {
        (Some(a), Some(b)) => (a, b),
        _ => {
            reporte
                .advertencias
                .push("tabla Grupos con columnas inesperadas; omitida".into());
            return Ok(());
        }
    };

    let conn = db.conn()?;
    let limpia = |s: Option<&String>| -> Option<String> {
        s.map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty() && !v.eq_ignore_ascii_case("(Ninguno)"))
    };

    for fila in &datos.filas {
        let nombre = fila.get(i_nombre).map(|s| s.trim()).unwrap_or("");
        let por = fila.get(i_por).map(|s| s.trim()).unwrap_or("");
        if nombre.is_empty() || por.is_empty() {
            continue;
        }
        let grupo = mic_core::model::Grupo {
            id: 0,
            nombre: nombre.to_string(),
            por: por.to_string(),
            luego1: limpia(i_luego1.and_then(|i| fila.get(i))),
            luego2: limpia(i_luego2.and_then(|i| fila.get(i))),
        };
        if let Err(e) = repo_grupos::guardar(&conn, &grupo) {
            reporte
                .advertencias
                .push(format!("no se pudo migrar el grupo '{nombre}': {e}"));
        }
    }
    Ok(())
}

/// Migra la tabla FiltrosAv → `filtros_av`. Los operadores SQL del original se
/// mapean a [`OpComp`]; los no mapeables (Is null, like negados…) generan aviso.
fn migrar_filtros(
    db: &AlbumDb,
    datos: &TablaCsv,
    reporte: &mut MigracionReporte,
) -> Result<(), MicError> {
    use mic_core::model::{CondicionFiltro, OpRel};

    let i_nombre = datos.indice("Nombre");
    let i_opr = datos.indice("OpR");
    let i_campo = datos.indice("Campo");
    let i_opc = datos.indice("OpC");
    let i_valor = datos.indice("Valor");

    let (i_nombre, i_campo, i_opc, i_valor) = match (i_nombre, i_campo, i_opc, i_valor) {
        (Some(a), Some(c), Some(d), Some(e)) => (a, c, d, e),
        _ => {
            reporte
                .advertencias
                .push("tabla FiltrosAv con columnas inesperadas; omitida".into());
            return Ok(());
        }
    };

    // Agrupa por nombre, preservando el orden de aparición.
    let mut orden_nombres: Vec<String> = Vec::new();
    let mut por_nombre: HashMap<String, Vec<CondicionFiltro>> = HashMap::new();

    for fila in &datos.filas {
        let nombre = fila.get(i_nombre).map(|s| s.trim()).unwrap_or("");
        if nombre.is_empty() {
            continue;
        }
        let campo = fila.get(i_campo).map(|s| s.trim()).unwrap_or("");
        let op_sql = fila.get(i_opc).map(|s| s.trim()).unwrap_or("");
        let valor = fila.get(i_valor).map(|s| s.trim()).unwrap_or("");
        let op_rel_txt = i_opr.and_then(|i| fila.get(i)).map(|s| s.trim()).unwrap_or("");

        let op_comp = match mapear_op_comp(op_sql) {
            Some(o) => o,
            None => {
                reporte.advertencias.push(format!(
                    "filtro '{nombre}': operador '{op_sql}' no soportado; condición omitida"
                ));
                continue;
            }
        };

        let es_primera = !por_nombre.contains_key(nombre)
            || por_nombre.get(nombre).map(|v| v.is_empty()).unwrap_or(true);
        let op_rel = if es_primera {
            None
        } else {
            mapear_op_rel(op_rel_txt)
        };

        if !por_nombre.contains_key(nombre) {
            orden_nombres.push(nombre.to_string());
        }
        por_nombre.entry(nombre.to_string()).or_default().push(
            CondicionFiltro {
                op_rel: op_rel.or(if es_primera { None } else { Some(OpRel::Y) }),
                campo: campo.to_string(),
                op_comp,
                valor: valor.to_string(),
            },
        );
    }

    let mut conn = db.conn()?;
    for nombre in orden_nombres {
        if let Some(conds) = por_nombre.get(&nombre) {
            if conds.is_empty() {
                continue;
            }
            if let Err(e) = repo_filtros::guardar(&mut conn, &nombre, conds) {
                reporte
                    .advertencias
                    .push(format!("no se pudo migrar el filtro '{nombre}': {e}"));
            }
        }
    }
    Ok(())
}

/// Mapea el operador SQL almacenado en `FiltrosAv.OpC` a [`OpComp`].
///
/// Valores del original (frmFA): `=`, `<>`, `>`, `>=`, `<`, `<=`, `Is null`,
/// `Not Is Null`, `like`, `Not like`. Los nulos y negaciones de `like` no tienen
/// equivalente directo en el modelo nuevo → `None` (aviso).
fn mapear_op_comp(op: &str) -> Option<mic_core::model::OpComp> {
    use mic_core::model::OpComp;
    let o = op.trim().to_lowercase();
    match o.as_str() {
        "=" | "es igual" | "igual" => Some(OpComp::Igual),
        "<>" | "!=" | "no es igual" | "distinto" => Some(OpComp::Distinto),
        ">" | "mayor que" | "mayor" => Some(OpComp::Mayor),
        ">=" | "mayor o igual que" | "mayor_igual" => Some(OpComp::MayorIgual),
        "<" | "menor que" | "menor" => Some(OpComp::Menor),
        "<=" | "menor o igual que" | "menor_igual" => Some(OpComp::MenorIgual),
        "like" | "contiene" => Some(OpComp::Contiene),
        // No mapeables: Is null, Not Is Null, Not like.
        _ => None,
    }
}

/// Mapea el conector lógico almacenado (`Y`/`O` o `AND`/`OR`).
fn mapear_op_rel(op: &str) -> Option<mic_core::model::OpRel> {
    use mic_core::model::OpRel;
    match op.trim().to_uppercase().as_str() {
        "Y" | "AND" => Some(OpRel::Y),
        "O" | "OR" => Some(OpRel::O),
        _ => None,
    }
}

/// Copia recursivamente la carpeta `imagenes/` que esté junto al `.mdb`.
fn copiar_imagenes(dir_mdb: &Path, destino: &Path, reporte: &mut MigracionReporte) {
    let origen = dir_mdb.join(paths::DIR_IMAGENES);
    if !origen.is_dir() {
        return;
    }
    if let Err(e) = copiar_dir(&origen, destino) {
        reporte
            .advertencias
            .push(format!("no se pudo copiar la carpeta de imágenes: {e}"));
    }
}

/// Copia recursiva sencilla de archivos (sin symlinks).
fn copiar_dir(origen: &Path, destino: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(destino)?;
    for entrada in std::fs::read_dir(origen)? {
        let entrada = entrada?;
        let ruta = entrada.path();
        let dst = destino.join(entrada.file_name());
        if ruta.is_dir() {
            copiar_dir(&ruta, &dst)?;
        } else if ruta.is_file() {
            std::fs::copy(&ruta, &dst)?;
        }
    }
    Ok(())
}

/// Recorre las imágenes referenciadas y cuenta presentes/faltantes.
fn verificar_imagenes(
    db: &AlbumDb,
    dir_imagenes: &Path,
    reporte: &mut MigracionReporte,
) -> Result<(), MicError> {
    let conn = db.conn()?;
    let mut faltantes = Vec::new();
    let mut encontradas = 0u64;

    for tabla in ["principal", "variantes"] {
        // variantes podría no tener filas; la consulta es válida igualmente.
        let mut stmt = match conn.prepare(&format!(
            "SELECT _imagen_ FROM {tabla} WHERE _imagen_ IS NOT NULL AND _imagen_ <> ''"
        )) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let filas = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(err_sql)?;
        for r in filas {
            let rel = r.map_err(err_sql)?;
            let nombre = paths::nombre_archivo(&rel);
            let abs = dir_imagenes.join(nombre);
            if abs.is_file() {
                encontradas += 1;
            } else {
                faltantes.push(rel);
            }
        }
    }

    reporte.imagenes_encontradas = encontradas;
    reporte.imagenes_faltantes = faltantes;
    Ok(())
}

/// Recalcula los campos calculados con el motor nuevo y reporta discrepancias
/// frente al valor migrado (nunca falla la migración por esto).
fn recalcular_calculados(
    db: &AlbumDb,
    ctx: &Contexto,
    reporte: &mut MigracionReporte,
) -> Result<(), MicError> {
    let hay_calculados = ctx
        .campos
        .iter()
        .any(|c| matches!(c.tipo, TipoCampo::Calculado));
    if !hay_calculados {
        return Ok(());
    }

    let motor = match mic_core::calc::MotorCalculo::new(&ctx.campos) {
        Ok(m) => m,
        Err(e) => {
            reporte.advertencias.push(format!(
                "no se pudo compilar el motor de calculados: {e}; se conservan los valores migrados"
            ));
            return Ok(());
        }
    };

    let mut discrepancias = 0u64;
    for principal in [true, false] {
        let tabla_sql = if principal { "principal" } else { "variantes" };
        let campos_tabla: Vec<&CampoDef> = ctx
            .campos
            .iter()
            .filter(|c| {
                matches!(c.tabla, mic_core::model::Tabla::Principal) == principal
            })
            .collect();
        if campos_tabla.is_empty() {
            continue;
        }

        let conn = db.conn()?;
        recalcular_tabla(
            &conn,
            tabla_sql,
            &campos_tabla,
            &motor,
            &mut discrepancias,
        )?;
    }

    if discrepancias > 0 {
        reporte.advertencias.push(format!(
            "{discrepancias} valor(es) calculado(s) difieren del original; se usó el resultado del motor nuevo"
        ));
    }
    Ok(())
}

/// Recalcula los calculados de una tabla, fila a fila.
fn recalcular_tabla(
    conn: &Conn,
    tabla_sql: &str,
    campos_tabla: &[&CampoDef],
    motor: &mic_core::calc::MotorCalculo,
    discrepancias: &mut u64,
) -> Result<(), MicError> {
    let calculados: Vec<&CampoDef> = campos_tabla
        .iter()
        .filter(|c| matches!(c.tipo, TipoCampo::Calculado))
        .copied()
        .collect();
    if calculados.is_empty() {
        return Ok(());
    }

    // Lee todas las columnas de usuario de la tabla.
    let mut cols_sql = vec!["_id_".to_string()];
    for c in campos_tabla {
        cols_sql.push(c.col_fisica.clone());
    }
    let sql = format!("SELECT {} FROM {tabla_sql}", cols_sql.join(", "));
    let mut stmt = conn.prepare(&sql).map_err(err_sql)?;

    let filas: Vec<(i64, Valores)> = stmt
        .query_map([], |row| {
            let id: i64 = row.get(0)?;
            let mut valores: Valores = Valores::new();
            for (i, c) in campos_tabla.iter().enumerate() {
                let valor = leer_valor(row, i + 1, c.tipo);
                valores.insert(c.nombre.clone(), valor);
            }
            Ok((id, valores))
        })
        .map_err(err_sql)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err_sql)?;

    for (id, mut valores) in filas {
        for c in motor.orden_recalculo() {
            // Solo recalculamos los calculados de esta tabla.
            if !calculados.iter().any(|k| &k.nombre == c) {
                continue;
            }
            if let Ok(nuevo) = motor.evaluar(c, &valores) {
                let anterior = valores.get(c).cloned().unwrap_or_default();
                if difieren(&anterior, &nuevo) {
                    *discrepancias += 1;
                }
                valores.insert(c.clone(), nuevo.clone());
                if let Some(campo) = calculados.iter().find(|k| &k.nombre == c) {
                    let v = valor_calc_a_sql(&nuevo);
                    let _ = conn.execute(
                        &format!(
                            "UPDATE {tabla_sql} SET {} = ?1 WHERE _id_ = ?2",
                            campo.col_fisica
                        ),
                        params![v, id],
                    );
                }
            }
        }
    }
    Ok(())
}

/// Lee un valor de una fila SQLite según el tipo del campo.
fn leer_valor(row: &rusqlite::Row, idx: usize, tipo: TipoCampo) -> Valor {
    match tipo {
        TipoCampo::Numerico | TipoCampo::Moneda | TipoCampo::Calculado => {
            row.get::<_, Option<f64>>(idx)
                .ok()
                .flatten()
                .map(Valor::Numero)
                .unwrap_or_default()
        }
        TipoCampo::Multidato => row
            .get::<_, Option<i64>>(idx)
            .ok()
            .flatten()
            .map(Valor::Entero)
            .unwrap_or_default(),
        _ => row
            .get::<_, Option<String>>(idx)
            .ok()
            .flatten()
            .map(Valor::Texto)
            .unwrap_or_default(),
    }
}

/// Convierte el resultado de un calculado a `Value` para persistir en columna REAL.
fn valor_calc_a_sql(v: &Valor) -> rusqlite::types::Value {
    use rusqlite::types::Value;
    match v {
        Valor::Numero(n) => Value::Real(*n),
        Valor::Entero(n) => Value::Real(*n as f64),
        // @FECHA passthrough devuelve Texto ISO; se guarda como texto.
        Valor::Texto(s) => Value::Text(s.clone()),
        Valor::Bool(b) => Value::Real(if *b { 1.0 } else { 0.0 }),
        Valor::Nulo(_) => Value::Null,
    }
}

/// Indica si dos valores calculados difieren (con tolerancia numérica).
fn difieren(a: &Valor, b: &Valor) -> bool {
    match (a.como_f64(), b.como_f64()) {
        (Some(x), Some(y)) => (x - y).abs() > 1e-6,
        _ => a.como_texto() != b.como_texto(),
    }
}

/// Atajo al conversor de error SQL de mic-db (no es público; lo reimplementamos).
fn err_sql(e: rusqlite::Error) -> MicError {
    MicError::Db(e.to_string())
}

// ---------------------------------------------------------------------------
// Importador de plantillas .xms
// ---------------------------------------------------------------------------

/// Parsea una plantilla XML `.xms` del original y devuelve sus campos.
///
/// Formato (ver `Module1.bas`, `DeCodiNodo`/`CrearCNodo`):
/// ```xml
/// <mic Version="2.0" Plantilla="normal">
///   <Registro>
///     <Descripcion Tipo="0" Longitud="50"/>
///     <Precio Tipo="2" Longitud="0" Decimales="2" Totalizable="True"/>
///     <Total Tipo="4" ... Formula="Precio*Cantidad" TipoSalida="1"/>
///     <Tallas Tipo="5"/>
///     <_variante_>
///       <Registro> ... campos de variantes ... </Registro>
///     </_variante_>
///   </Registro>
/// </mic>
/// ```
/// - El **nombre** del campo es el nombre del nodo, con la codificación
///   espacio↔underscore del original (`__` = `_` literal, `_` = espacio).
/// - Atributos: `Tipo` (0..5), `Longitud` (descartada), `Decimales`,
///   `Totalizable` (`True`/`False`), `Formula`, `TipoSalida`.
/// - Nodos de sistema (`_imagen_`, `_id_`, …) y `_variante_` se omiten como
///   campos de usuario; los hijos de `_variante_` se devuelven como campos de la
///   tabla de variantes.
pub fn parse_xms(ruta: &Path) -> Result<Vec<CampoNuevo>, MicError> {
    let bytes = std::fs::read(ruta).map_err(|e| {
        MicError::Migracion(format!("no se pudo leer la plantilla '{}': {e}", ruta.display()))
    })?;
    // Las plantillas se guardan en iso-8859-1/Windows-1252.
    let texto = crate::csv_parser::decodificar_cp1252(&bytes);
    parse_xms_texto(&texto)
}

/// Igual que [`parse_xms`] pero sobre el XML ya decodificado (para tests).
pub fn parse_xms_texto(xml: &str) -> Result<Vec<CampoNuevo>, MicError> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut lector = Reader::from_str(xml);
    lector.config_mut().trim_text(true);

    let mut campos: Vec<CampoNuevo> = Vec::new();
    let mut orden: i32 = 0;
    // Pila de contextos: true = dentro de _variante_ (campos de variantes).
    let mut en_variantes = false;
    // Profundidad para distinguir nodos campo de los estructurales.
    let mut dentro_registro = false;

    let mut buf = Vec::new();
    loop {
        match lector.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let nombre_nodo = String::from_utf8_lossy(e.name().as_ref()).into_owned();

                match nombre_nodo.as_str() {
                    "mic" => {}
                    "Registro" => dentro_registro = true,
                    "_variante_" => {
                        en_variantes = true;
                    }
                    _ if nombre_nodo.starts_with('_') => {
                        // Campo de sistema (_imagen_, _id_, _auxiliar_, …): se ignora.
                    }
                    _ if dentro_registro => {
                        // Nodo de campo de usuario.
                        if let Some(campo) =
                            nodo_a_campo(&nombre_nodo, &e, en_variantes, orden)?
                        {
                            campos.push(campo);
                            orden += 1;
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let nombre_nodo = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                match nombre_nodo.as_str() {
                    "_variante_" => en_variantes = false,
                    "Registro" => {
                        // El cierre del Registro de variantes no debe apagar el de
                        // principal; pero como _variante_ envuelve su Registro,
                        // basta con el flag en_variantes ya gestionado.
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(MicError::Migracion(format!(
                    "plantilla .xms mal formada: {e}"
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(campos)
}

/// Convierte un nodo de campo `.xms` a [`CampoNuevo`].
fn nodo_a_campo(
    nombre_nodo: &str,
    e: &quick_xml::events::BytesStart,
    en_variantes: bool,
    orden: i32,
) -> Result<Option<CampoNuevo>, MicError> {
    use mic_core::model::Tabla;

    let nombre = cambia_und_esp(nombre_nodo);
    if nombre.is_empty() {
        return Ok(None);
    }

    let mut tipo_byte: i64 = 0;
    let mut decimales: u8 = 0;
    let mut totalizable = false;
    let mut formula: Option<String> = None;

    for attr in e.attributes().flatten() {
        let clave = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
        let valor = attr
            .unescape_value()
            .map(|c| c.into_owned())
            .unwrap_or_else(|_| String::from_utf8_lossy(&attr.value).into_owned());
        match clave.as_str() {
            "Tipo" => tipo_byte = valor.trim().parse().unwrap_or(0),
            "Decimales" => decimales = valor.trim().parse().unwrap_or(0),
            "Totalizable" => totalizable = type_map::jet_bool(&valor),
            "Formula" => {
                let f = valor.trim();
                if !f.is_empty() {
                    formula = Some(f.to_string());
                }
            }
            // "Longitud" y "TipoSalida" se descartan (sin restricción de longitud;
            // el tipo de salida no existe en el modelo nuevo).
            _ => {}
        }
    }

    let tipo = TipoCampo::from_jet(tipo_byte).unwrap_or(TipoCampo::Texto);
    let formula = if matches!(tipo, TipoCampo::Calculado) {
        formula
    } else {
        None
    };

    Ok(Some(CampoNuevo {
        nombre,
        tabla: if en_variantes {
            Tabla::Variantes
        } else {
            Tabla::Principal
        },
        tipo,
        decimales,
        totalizable,
        formula,
        visible: true,
        modificable: true,
        orden_visible: orden,
        formato: None,
    }))
}

/// Reimplementa `CambiaUndEsp` (Module1.bas): underscores → espacios.
///
/// Regla del original:
/// - Nombres que empiezan por `_` son de sistema: se devuelven tal cual.
/// - `__` (doble) representa un `_` literal del nombre.
/// - `_` (simple) representa un espacio.
///
/// Implementación equivalente al `Replace` triple del VB:
/// `__`→`  `(2 espacios), `_`→` `, `  `→`_`.
pub fn cambia_und_esp(nombre: &str) -> String {
    if nombre.starts_with('_') {
        return nombre.to_string();
    }
    let paso1 = nombre.replace("__", "  ");
    let paso2 = paso1.replace('_', " ");
    paso2.replace("  ", "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cambia_und_esp_espacios() {
        // "Precio_Venta" (un underscore) → "Precio Venta".
        assert_eq!(cambia_und_esp("Precio_Venta"), "Precio Venta");
    }

    #[test]
    fn cambia_und_esp_literal() {
        // "Campo__especial" (doble) → "Campo_especial".
        assert_eq!(cambia_und_esp("Campo__especial"), "Campo_especial");
    }

    #[test]
    fn cambia_und_esp_sistema() {
        assert_eq!(cambia_und_esp("_imagen_"), "_imagen_");
        assert_eq!(cambia_und_esp("_id_"), "_id_");
    }

    #[test]
    fn parse_xms_basico() {
        let xml = r#"<?xml version="1.0" encoding="iso-8859-1"?>
<mic Version="2.0" Plantilla="normal">
  <Registro>
    <_imagen_ Tipo="0" Longitud="255"/>
    <Descripcion Tipo="0" Longitud="50"/>
    <Precio Tipo="2" Longitud="0" Decimales="2" Totalizable="True"/>
    <Total Tipo="4" Longitud="0" Decimales="2" Totalizable="False" Formula="Precio*Cantidad" TipoSalida="1"/>
    <Tallas Tipo="5"/>
  </Registro>
</mic>"#;
        let campos = parse_xms_texto(xml).unwrap();
        assert_eq!(campos.len(), 4); // _imagen_ se omite
        assert_eq!(campos[0].nombre, "Descripcion");
        assert_eq!(campos[0].tipo, TipoCampo::Texto);
        assert_eq!(campos[1].nombre, "Precio");
        assert_eq!(campos[1].tipo, TipoCampo::Moneda);
        assert_eq!(campos[1].decimales, 2);
        assert!(campos[1].totalizable);
        assert_eq!(campos[2].nombre, "Total");
        assert_eq!(campos[2].tipo, TipoCampo::Calculado);
        assert_eq!(campos[2].formula.as_deref(), Some("Precio*Cantidad"));
        assert_eq!(campos[3].tipo, TipoCampo::Multidato);
    }

    #[test]
    fn parse_xms_nombre_con_espacio() {
        let xml = r#"<mic Version="2.0" Plantilla="x">
  <Registro>
    <Precio_Venta Tipo="2" Decimales="2"/>
  </Registro>
</mic>"#;
        let campos = parse_xms_texto(xml).unwrap();
        assert_eq!(campos.len(), 1);
        assert_eq!(campos[0].nombre, "Precio Venta");
    }

    #[test]
    fn parse_xms_variantes() {
        let xml = r#"<mic Version="2.0" Plantilla="x">
  <Registro>
    <Modelo Tipo="0"/>
    <_variante_>
      <Registro>
        <Color Tipo="0"/>
        <Talla Tipo="0"/>
      </Registro>
    </_variante_>
  </Registro>
</mic>"#;
        let campos = parse_xms_texto(xml).unwrap();
        assert_eq!(campos.len(), 3);
        let modelo = campos.iter().find(|c| c.nombre == "Modelo").unwrap();
        assert_eq!(modelo.tabla, mic_core::model::Tabla::Principal);
        let color = campos.iter().find(|c| c.nombre == "Color").unwrap();
        assert_eq!(color.tabla, mic_core::model::Tabla::Variantes);
        let talla = campos.iter().find(|c| c.nombre == "Talla").unwrap();
        assert_eq!(talla.tabla, mic_core::model::Tabla::Variantes);
    }

    #[test]
    fn mapear_op_comp_variantes() {
        use mic_core::model::OpComp;
        assert_eq!(mapear_op_comp("="), Some(OpComp::Igual));
        assert_eq!(mapear_op_comp("<>"), Some(OpComp::Distinto));
        assert_eq!(mapear_op_comp(">="), Some(OpComp::MayorIgual));
        assert_eq!(mapear_op_comp("like"), Some(OpComp::Contiene));
        assert_eq!(mapear_op_comp("Is null"), None);
        assert_eq!(mapear_op_comp("Not like"), None);
    }

    #[test]
    fn normaliza_fecha_variantes() {
        assert_eq!(normalizar_fecha_iso("2007-10-29"), Some("2007-10-29".into()));
        assert_eq!(normalizar_fecha_iso("29/10/2007"), Some("2007-10-29".into()));
        assert_eq!(
            normalizar_fecha_iso("2007-10-29 00:00:00"),
            Some("2007-10-29".into())
        );
        assert_eq!(normalizar_fecha_iso("basura"), None);
    }
}
