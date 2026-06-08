//! Comandos de registros (tablas `principal` / `variantes`): consulta paginada
//! para el scroll virtual, obtención, alta, edición, borrado, asignación de
//! imagen y listado de variantes.
//!
//! Las operaciones de escritura toman el motor de cálculo del handle (bajo
//! `RwLock`) y lo pasan a `mic-db`, que recalcula los campos calculados dentro
//! de su transacción. El alta con imagen copia el archivo de origen a la
//! carpeta `imagenes/` del álbum con un nombre único (evita colisiones) y
//! persiste la ruta relativa.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use mic_core::error::MicError;
use mic_core::model::{
    QueryPage, QueryReq, RegistroCompleto, RegistroLigero, Tabla, Valores,
};
use serde::Serialize;
use tauri::State;

use crate::commands::{en_db, handle};
use crate::state::{AlbumHandle, AppState};

/// Contador de proceso para desambiguar nombres de imagen copiados.
static CONTADOR_IMG: AtomicU64 = AtomicU64::new(0);

/// Resultado de asignar una imagen: ruta relativa persistida y su versión (mtime).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImagenSet {
    pub imagen: String,
    pub imagen_version: i64,
}

/// Copia `origen` a la carpeta de imágenes del álbum con un nombre único y
/// devuelve la ruta relativa (`imagenes/<archivo>`) lista para persistir.
///
/// El nombre se construye a partir del nombre original más un sufijo único
/// (epoch en milisegundos + contador) para evitar colisiones entre archivos
/// homónimos provenientes de carpetas distintas.
fn copiar_imagen(handle: &AlbumHandle, origen: &str) -> Result<String, MicError> {
    let origen_pb = PathBuf::from(origen);
    if !origen_pb.exists() {
        return Err(MicError::NoEncontrado(format!(
            "imagen de origen no encontrada: {origen}"
        )));
    }

    let dir = handle.dir_imagenes();
    std::fs::create_dir_all(&dir)?;

    let nombre_unico = nombre_destino_unico(&dir, &origen_pb);
    let destino = dir.join(&nombre_unico);
    std::fs::copy(&origen_pb, &destino).map_err(|e| {
        MicError::Io(format!(
            "no se pudo copiar la imagen a '{}': {e}",
            destino.display()
        ))
    })?;

    Ok(format!("imagenes/{nombre_unico}"))
}

/// Genera un nombre de archivo libre dentro de `dir`, preservando la extensión
/// del `origen` y añadiendo un sufijo único si ya existiera uno homónimo.
fn nombre_destino_unico(dir: &Path, origen: &Path) -> String {
    let stem = origen
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("imagen");
    let ext = origen.extension().and_then(|s| s.to_str());

    let sufijo = {
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let n = CONTADOR_IMG.fetch_add(1, Ordering::Relaxed);
        format!("{ms:x}{n:x}")
    };

    let candidato = match ext {
        Some(e) => format!("{stem}_{sufijo}.{e}"),
        None => format!("{stem}_{sufijo}"),
    };

    // En el caso (muy improbable) de colisión, reintenta con otro contador.
    if dir.join(&candidato).exists() {
        let n2 = CONTADOR_IMG.fetch_add(1, Ordering::Relaxed);
        return match ext {
            Some(e) => format!("{stem}_{sufijo}_{n2:x}.{e}"),
            None => format!("{stem}_{sufijo}_{n2:x}"),
        };
    }
    candidato
}

/// Consulta paginada de registros (ventana para el scroll virtual).
#[tauri::command]
pub async fn registros_query(
    state: State<'_, AppState>,
    album_id: u64,
    req: QueryReq,
) -> Result<QueryPage, String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let campos = h.campos();
        let conn = h.db.conn()?;
        mic_db::repo_registros::query(&conn, &campos, &req)
    })
    .await
}

/// Obtiene un registro completo (valores + multidatos) por id.
#[tauri::command]
pub async fn registro_obtener(
    state: State<'_, AppState>,
    album_id: u64,
    id: i64,
    tabla: Tabla,
) -> Result<RegistroCompleto, String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let campos = h.campos();
        let conn = h.db.conn()?;
        mic_db::repo_registros::obtener(&conn, &campos, tabla, id)
    })
    .await
}

/// Crea un registro. Si llega `imagen_origen`, copia el archivo a `imagenes/`
/// con nombre único y persiste su ruta relativa. Devuelve el id del nuevo
/// registro.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn registro_crear(
    state: State<'_, AppState>,
    album_id: u64,
    tabla: Tabla,
    valores: Valores,
    multidatos: std::collections::HashMap<String, Vec<String>>,
    imagen_origen: Option<String>,
    id_principal: Option<i64>,
) -> Result<i64, String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        // Copia de imagen (si procede) antes de abrir la conexión de escritura.
        let imagen_rel = match imagen_origen {
            Some(ref o) if !o.trim().is_empty() => Some(copiar_imagen(h, o)?),
            _ => None,
        };
        let dir_img = h.dir_imagenes();
        let campos = h.campos();
        let motor = h.motor.read().unwrap_or_else(|e| e.into_inner());
        let mut conn = h.db.conn()?;
        mic_db::repo_registros::crear(
            &mut conn,
            &campos,
            motor.as_ref(),
            tabla,
            &valores,
            &multidatos,
            imagen_rel.as_deref(),
            id_principal,
            Some(dir_img.as_path()),
        )
    })
    .await
}

/// Edita un registro. Recalcula calculados y devuelve el registro completo.
#[tauri::command]
pub async fn registro_editar(
    state: State<'_, AppState>,
    album_id: u64,
    id: i64,
    tabla: Tabla,
    valores: Valores,
    multidatos: Option<std::collections::HashMap<String, Vec<String>>>,
) -> Result<RegistroCompleto, String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let campos = h.campos();
        let motor = h.motor.read().unwrap_or_else(|e| e.into_inner());
        let mut conn = h.db.conn()?;
        mic_db::repo_registros::editar(
            &mut conn,
            &campos,
            motor.as_ref(),
            tabla,
            id,
            &valores,
            multidatos.as_ref(),
        )
    })
    .await
}

/// Elimina registros por id (cascada de variantes si la tabla es principal).
#[tauri::command]
pub async fn registros_eliminar(
    state: State<'_, AppState>,
    album_id: u64,
    ids: Vec<i64>,
    tabla: Tabla,
) -> Result<(), String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let mut conn = h.db.conn()?;
        mic_db::repo_registros::eliminar(&mut conn, tabla, &ids)
    })
    .await
}

/// Asigna una imagen a un registro: copia `ruta_origen` a `imagenes/` con
/// nombre único, actualiza la fila y devuelve la ruta relativa + versión (mtime).
#[tauri::command]
pub async fn registro_imagen_set(
    state: State<'_, AppState>,
    album_id: u64,
    id: i64,
    tabla: Tabla,
    ruta_origen: String,
) -> Result<ImagenSet, String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let imagen_rel = copiar_imagen(h, &ruta_origen)?;
        let dir_img = h.dir_imagenes();
        let conn = h.db.conn()?;
        let version = mic_db::repo_registros::set_imagen(
            &conn,
            tabla,
            id,
            &imagen_rel,
            Some(dir_img.as_path()),
        )?;
        Ok(ImagenSet {
            imagen: imagen_rel,
            imagen_version: version,
        })
    })
    .await
}

/// Edición en lote: aplica los mismos `valores` a varios registros (inspector
/// multi-selección).
#[tauri::command]
pub async fn registros_editar_lote(
    state: State<'_, AppState>,
    album_id: u64,
    ids: Vec<i64>,
    tabla: Tabla,
    valores: Valores,
) -> Result<(), String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let campos = h.campos();
        let motor = h.motor.read().unwrap_or_else(|e| e.into_inner());
        let mut conn = h.db.conn()?;
        mic_db::repo_registros::editar_lote(&mut conn, &campos, motor.as_ref(), tabla, &ids, &valores)
    })
    .await
}

/// Oculta o muestra registros (`_auxiliar_`): el "Ocultar" del original.
#[tauri::command]
pub async fn registros_set_auxiliar(
    state: State<'_, AppState>,
    album_id: u64,
    ids: Vec<i64>,
    tabla: Tabla,
    oculto: bool,
) -> Result<(), String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let conn = h.db.conn()?;
        mic_db::repo_registros::set_auxiliar(&conn, tabla, &ids, oculto)
    })
    .await
}

/// Suma los campos totalizables del conjunto filtrado actual (ex-frmTotalizar).
#[tauri::command]
pub async fn registros_totalizar(
    state: State<'_, AppState>,
    album_id: u64,
    req: QueryReq,
) -> Result<mic_core::model::Totales, String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let campos = h.campos();
        let conn = h.db.conn()?;
        mic_db::repo_registros::totalizar(&conn, &campos, &req)
    })
    .await
}

/// Estadísticas de campos numéricos (cuenta, suma, media, mediana, moda,
/// mín/máx) sobre el conjunto filtrado actual. Panel "Totalizar" ampliado.
#[tauri::command]
pub async fn registros_estadisticas(
    state: State<'_, AppState>,
    album_id: u64,
    req: QueryReq,
    campos: Vec<String>,
) -> Result<mic_core::model::Estadisticas, String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let defs = h.campos();
        let conn = h.db.conn()?;
        mic_db::repo_registros::estadisticas(&conn, &defs, &req, &campos)
    })
    .await
}

/// Actualización masiva (ex-frmActGrlDat): aplica `valores` a todos los
/// registros que cumplen el filtro de `req`. Devuelve cuántos tocó.
#[tauri::command]
pub async fn registros_actualizar_masivo(
    state: State<'_, AppState>,
    album_id: u64,
    req: QueryReq,
    valores: Valores,
) -> Result<u64, String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let campos = h.campos();
        let motor = h.motor.read().unwrap_or_else(|e| e.into_inner());
        let mut conn = h.db.conn()?;
        mic_db::repo_registros::actualizar_masivo(
            &mut conn,
            &campos,
            motor.as_ref(),
            &req,
            &valores,
        )
    })
    .await
}

/// Lista las variantes de un principal como registros ligeros.
#[tauri::command]
pub async fn variantes_listar(
    state: State<'_, AppState>,
    album_id: u64,
    id_principal: i64,
) -> Result<Vec<RegistroLigero>, String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let campos = h.campos();
        let conn = h.db.conn()?;
        mic_db::repo_registros::variantes_de(&conn, &campos, id_principal)
    })
    .await
}

/// Extensiones de imagen aceptadas por el alta masiva desde carpeta.
const EXT_IMAGEN: [&str; 8] = ["jpg", "jpeg", "png", "gif", "webp", "bmp", "tiff", "tif"];

/// Alta masiva desde carpeta (ex "Imagenes de Dir"): crea un registro por cada
/// imagen encontrada en `carpeta` (no recursivo). Emite `carpeta-progreso`
/// `{hechas, total}` y devuelve cuántos registros creó.
#[tauri::command]
pub async fn registros_crear_desde_carpeta(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    album_id: u64,
    carpeta: String,
) -> Result<u64, String> {
    use tauri::Emitter;

    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let dir = PathBuf::from(&carpeta);
        if !dir.is_dir() {
            return Err(MicError::Invalido(format!(
                "no es una carpeta válida: {carpeta}"
            )));
        }
        // Solo archivos de imagen del primer nivel, en orden por nombre.
        let mut imagenes: Vec<PathBuf> = std::fs::read_dir(&dir)
            .map_err(|e| MicError::Io(format!("no se pudo leer la carpeta: {e}")))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.is_file()
                    && p.extension()
                        .and_then(|x| x.to_str())
                        .map(|x| EXT_IMAGEN.contains(&x.to_lowercase().as_str()))
                        .unwrap_or(false)
            })
            .collect();
        imagenes.sort();

        let total = imagenes.len();
        let campos = h.campos();
        let motor = h.motor.read().unwrap_or_else(|e| e.into_inner());
        let dir_img = h.dir_imagenes();
        let mut conn = h.db.conn()?;
        let vacios: Valores = Valores::new();
        let sin_multidatos = std::collections::HashMap::new();

        let mut creados = 0u64;
        for (i, origen) in imagenes.iter().enumerate() {
            let rel = copiar_imagen(h, &origen.to_string_lossy())?;
            mic_db::repo_registros::crear(
                &mut conn,
                &campos,
                motor.as_ref(),
                Tabla::Principal,
                &vacios,
                &sin_multidatos,
                Some(&rel),
                None,
                Some(dir_img.as_path()),
            )?;
            creados += 1;
            if i % 10 == 0 || i + 1 == total {
                let _ = app.emit(
                    "carpeta-progreso",
                    serde_json::json!({ "hechas": i + 1, "total": total }),
                );
            }
        }
        Ok(creados)
    })
    .await
}
