//! Comandos de gestión de álbumes: crear, abrir, cerrar, compactar y la lista
//! de recientes (persistida como JSON en el directorio de configuración).

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use mic_core::model::{AlbumInfo, CampoNuevo};
use mic_db::AlbumDb;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::commands::{a_string, en_db, handle};
use crate::state::{AlbumHandle, AppState};

/// Entrada de la lista de álbumes recientes (ruta + nombre mostrado).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumReciente {
    pub ruta: String,
    pub nombre: String,
}

/// Nombre del archivo JSON que guarda los álbumes recientes.
const ARCHIVO_RECIENTES: &str = "recientes.json";

/// Máximo de entradas conservadas en la lista de recientes.
const MAX_RECIENTES: usize = 20;

/// Lee el `nombre` guardado en `mic_album` de un álbum abierto.
fn nombre_album(db: &AlbumDb) -> String {
    let nombre = (|| {
        let conn = db.conn().ok()?;
        conn.query_row(
            "SELECT valor FROM mic_album WHERE clave = 'nombre'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
    })();
    nombre.unwrap_or_else(|| {
        db.ruta()
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Álbum")
            .to_string()
    })
}

/// Construye el [`AlbumInfo`] de un álbum abierto a partir de su handle.
fn info_de(album_id: u64, handle: &AlbumHandle) -> Result<AlbumInfo, String> {
    let campos = handle.campos();
    let conn = handle.db.conn().map_err(a_string)?;
    let total = mic_db::repo_registros::total(&conn).map_err(a_string)?;
    let tiene_variantes: bool = conn
        .query_row("SELECT EXISTS(SELECT 1 FROM variantes LIMIT 1)", [], |row| {
            row.get::<_, i64>(0)
        })
        .map(|n| n != 0)
        .unwrap_or(false);
    drop(conn);
    Ok(AlbumInfo {
        album_id,
        ruta: handle.db.ruta().to_string_lossy().into_owned(),
        nombre: nombre_album(&handle.db),
        total_registros: total,
        tiene_variantes,
        campos,
    })
}

/// Ruta del archivo de recientes dentro del directorio de configuración de la app.
fn ruta_recientes(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("no se pudo resolver el directorio de configuración: {e}"))?;
    Ok(dir.join(ARCHIVO_RECIENTES))
}

/// Carga la lista de recientes (vacía si el archivo no existe o está corrupto).
fn cargar_recientes(app: &AppHandle) -> Vec<AlbumReciente> {
    let ruta = match ruta_recientes(app) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let datos = match std::fs::read_to_string(&ruta) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    serde_json::from_str(&datos).unwrap_or_default()
}

/// Guarda la lista de recientes en disco (best-effort: traza el error si falla).
fn guardar_recientes(app: &AppHandle, lista: &[AlbumReciente]) {
    let ruta = match ruta_recientes(app) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "no se pudo resolver ruta de recientes");
            return;
        }
    };
    if let Some(dir) = ruta.parent() {
        if let Err(e) = std::fs::create_dir_all(dir) {
            tracing::warn!(error = %e, "no se pudo crear el directorio de configuración");
            return;
        }
    }
    match serde_json::to_string_pretty(lista) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&ruta, json) {
                tracing::warn!(error = %e, "no se pudo escribir recientes.json");
            }
        }
        Err(e) => tracing::warn!(error = %e, "no se pudo serializar recientes"),
    }
}

/// Inserta (o promueve) un álbum al principio de la lista de recientes.
fn promover_reciente(app: &AppHandle, ruta: &Path, nombre: &str) {
    let ruta_txt = ruta.to_string_lossy().into_owned();
    let mut lista = cargar_recientes(app);
    lista.retain(|r| r.ruta != ruta_txt);
    lista.insert(
        0,
        AlbumReciente {
            ruta: ruta_txt,
            nombre: nombre.to_string(),
        },
    );
    lista.truncate(MAX_RECIENTES);
    guardar_recientes(app, &lista);
}

/// Crea un álbum nuevo en `ruta` con los `campos` indicados y lo deja abierto.
#[tauri::command]
pub async fn album_crear(
    app: AppHandle,
    state: State<'_, AppState>,
    ruta: String,
    nombre: String,
    campos: Vec<CampoNuevo>,
) -> Result<AlbumInfo, String> {
    let ruta_pb = PathBuf::from(&ruta);
    let nombre_c = nombre.clone();

    let nuevo_handle = tokio::task::spawn_blocking(move || -> Result<AlbumHandle, String> {
        let existia = ruta_pb.exists();
        let resultado = (|| -> Result<AlbumHandle, String> {
            let db = AlbumDb::crear(&ruta_pb, &nombre_c).map_err(a_string)?;
            {
                let conn = db.conn().map_err(a_string)?;
                for def in &campos {
                    mic_db::repo_campos::crear(&conn, def).map_err(a_string)?;
                }
            }
            AlbumHandle::nuevo(db).map_err(a_string)
        })();
        // Si la creación falló a medias, no dejar un .micdb huérfano que
        // bloquee el reintento con "el archivo ya existe". Solo se limpia si
        // el archivo NO existía antes (jamás borrar un álbum previo).
        if resultado.is_err() && !existia {
            let base = ruta_pb.to_string_lossy().into_owned();
            let _ = std::fs::remove_file(&ruta_pb);
            let _ = std::fs::remove_file(format!("{base}-wal"));
            let _ = std::fs::remove_file(format!("{base}-shm"));
        }
        resultado
    })
    .await
    .map_err(|e| format!("error interno al crear álbum: {e}"))??;

    let album_id = state.registrar(nuevo_handle);
    let h = handle(&state, album_id)?;
    let info = info_de(album_id, &h)?;
    promover_reciente(&app, Path::new(&info.ruta), &info.nombre);
    Ok(info)
}

/// Abre un álbum existente y lo registra en el estado.
#[tauri::command]
pub async fn album_abrir(
    app: AppHandle,
    state: State<'_, AppState>,
    ruta: String,
) -> Result<AlbumInfo, String> {
    let ruta_pb = PathBuf::from(&ruta);

    let nuevo_handle = tokio::task::spawn_blocking(move || -> Result<AlbumHandle, String> {
        let db = AlbumDb::abrir(&ruta_pb).map_err(a_string)?;
        AlbumHandle::nuevo(db).map_err(a_string)
    })
    .await
    .map_err(|e| format!("error interno al abrir álbum: {e}"))??;

    let album_id = state.registrar(nuevo_handle);
    let h = handle(&state, album_id)?;
    let info = info_de(album_id, &h)?;
    promover_reciente(&app, Path::new(&info.ruta), &info.nombre);
    Ok(info)
}

/// Cierra un álbum abierto (lo descarta del estado en memoria).
#[tauri::command]
pub async fn album_cerrar(state: State<'_, AppState>, album_id: u64) -> Result<(), String> {
    state.cerrar(album_id);
    Ok(())
}

/// Compacta el archivo del álbum (`VACUUM`), reclamando espacio tras borrados.
#[tauri::command]
pub async fn album_compactar(state: State<'_, AppState>, album_id: u64) -> Result<(), String> {
    let h = handle(&state, album_id)?;
    en_db(h, |h| h.db.vacuum()).await
}

/// Devuelve la lista de álbumes recientes (más reciente primero).
#[tauri::command]
pub async fn albumes_recientes(app: AppHandle) -> Result<Vec<AlbumReciente>, String> {
    Ok(cargar_recientes(&app))
}

/// Recalcula los campos calculados de todos los registros del álbum y devuelve
/// cuántos tocó (ex "Act. Calculados" del menú Herramientas).
#[tauri::command]
pub async fn album_recalcular(state: State<'_, AppState>, album_id: u64) -> Result<u64, String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        let campos = h.campos();
        let motor = h.motor.read().unwrap_or_else(|e| e.into_inner());
        let mut conn = h.db.conn()?;
        mic_db::repo_registros::recalcular_todo(&mut conn, &campos, motor.as_ref())
    })
    .await
}

/// Copia recursiva de un directorio (para la carpeta `imagenes/` del álbum).
fn copiar_dir(origen: &Path, destino: &Path) -> std::io::Result<u64> {
    let mut copiados = 0u64;
    std::fs::create_dir_all(destino)?;
    for entrada in std::fs::read_dir(origen)? {
        let entrada = entrada?;
        let tipo = entrada.file_type()?;
        let dst = destino.join(entrada.file_name());
        if tipo.is_dir() {
            copiados += copiar_dir(&entrada.path(), &dst)?;
        } else {
            std::fs::copy(entrada.path(), &dst)?;
            copiados += 1;
        }
    }
    Ok(copiados)
}

/// Copia el álbum a `ruta_destino` (ex-frmCopAlbum): copia completa (datos +
/// carpeta `imagenes/`) o solo la estructura (campos, categorías, grupos,
/// filtros y reportes, sin registros). Devuelve cuántas imágenes copió.
#[tauri::command]
pub async fn album_copiar(
    state: State<'_, AppState>,
    album_id: u64,
    ruta_destino: String,
    solo_estructura: bool,
) -> Result<u64, String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        use mic_core::error::MicError;

        let destino = PathBuf::from(&ruta_destino);
        if destino == h.db.ruta() {
            return Err(MicError::Invalido(
                "el destino no puede ser el propio álbum".into(),
            ));
        }
        if destino.exists() {
            return Err(MicError::Invalido(format!(
                "ya existe un archivo en '{}'",
                destino.display()
            )));
        }
        if let Some(dir) = destino.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| MicError::Io(format!("no se pudo crear el destino: {e}")))?;
        }

        // Copia binaria consistente de la base (incluye WAL pendiente).
        {
            let conn = h.db.conn()?;
            conn.execute(
                "VACUUM INTO ?1",
                rusqlite::params![destino.to_string_lossy()],
            )
            .map_err(|e| MicError::Io(format!("no se pudo copiar la base: {e}")))?;
        }

        let nombre_nuevo = destino
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Álbum")
            .to_string();

        // Ajustes sobre la copia (conexión directa, fuera del pool).
        {
            let copia = rusqlite::Connection::open(&destino)
                .map_err(|e| MicError::Io(format!("no se pudo abrir la copia: {e}")))?;
            copia
                .execute(
                    "UPDATE mic_album SET valor = ?1 WHERE clave = 'nombre'",
                    rusqlite::params![nombre_nuevo],
                )
                .map_err(|e| MicError::Io(format!("no se pudo renombrar la copia: {e}")))?;
            if solo_estructura {
                copia
                    .execute_batch(
                        "DELETE FROM multidatos;\n\
                         DELETE FROM variantes;\n\
                         DELETE FROM principal;\n\
                         DELETE FROM principal_fts;\n\
                         VACUUM;",
                    )
                    .map_err(|e| {
                        MicError::Io(format!("no se pudo vaciar la copia: {e}"))
                    })?;
            }
        }

        // Carpeta de imágenes (solo en la copia completa).
        if solo_estructura {
            return Ok(0);
        }
        let dir_img = h.dir_imagenes();
        if !dir_img.exists() {
            return Ok(0);
        }
        let destino_img = destino
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("imagenes");
        let copiadas = copiar_dir(&dir_img, &destino_img)
            .map_err(|e| MicError::Io(format!("no se pudieron copiar las imágenes: {e}")))?;
        Ok(copiadas)
    })
    .await
}

/// Añade recursivamente el contenido de `dir` al `writer` del zip, bajo el
/// prefijo `prefijo` (ruta relativa dentro del archivo). Devuelve cuántos
/// archivos añadió.
fn empacar_dir<W: Write + std::io::Seek>(
    writer: &mut zip::ZipWriter<W>,
    dir: &Path,
    prefijo: &str,
    opciones: zip::write::SimpleFileOptions,
) -> std::io::Result<u64> {
    let mut empacados = 0u64;
    for entrada in std::fs::read_dir(dir)? {
        let entrada = entrada?;
        let tipo = entrada.file_type()?;
        let nombre = entrada.file_name();
        let nombre = nombre.to_string_lossy();
        let ruta_zip = format!("{prefijo}/{nombre}");
        if tipo.is_dir() {
            empacados += empacar_dir(writer, &entrada.path(), &ruta_zip, opciones)?;
        } else {
            writer
                .start_file(&ruta_zip, opciones)
                .map_err(std::io::Error::other)?;
            let mut origen = std::fs::File::open(entrada.path())?;
            std::io::copy(&mut origen, writer)?;
            empacados += 1;
        }
    }
    Ok(empacados)
}

/// Empaca el álbum en un `.zip` (ex-EmpacarAlbum/frm3Botones): incluye una copia
/// consistente del `.micdb` (vía `VACUUM INTO` a un temporal) y toda la carpeta
/// `imagenes/` de forma recursiva. Devuelve el número de archivos empacados.
#[tauri::command]
pub async fn album_empacar(
    state: State<'_, AppState>,
    album_id: u64,
    ruta_zip: String,
) -> Result<u64, String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        use mic_core::error::MicError;

        let destino_zip = PathBuf::from(&ruta_zip);
        if let Some(dir) = destino_zip.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| MicError::Io(format!("no se pudo crear el destino: {e}")))?;
        }

        // Copia consistente de la base a un temporal (incluye WAL pendiente).
        let nombre_micdb = h
            .db
            .ruta()
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("album.micdb")
            .to_string();
        let temp_db = std::env::temp_dir().join(format!("mic_empacar_{album_id}.micdb"));
        let _ = std::fs::remove_file(&temp_db);
        {
            let conn = h.db.conn()?;
            conn.execute(
                "VACUUM INTO ?1",
                rusqlite::params![temp_db.to_string_lossy()],
            )
            .map_err(|e| MicError::Io(format!("no se pudo copiar la base: {e}")))?;
        }

        let resultado = (|| -> Result<u64, MicError> {
            let archivo = std::fs::File::create(&destino_zip)
                .map_err(|e| MicError::Io(format!("no se pudo crear el zip: {e}")))?;
            let mut writer = zip::ZipWriter::new(archivo);
            let opciones = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);

            let mut empacados = 0u64;

            // El .micdb en la raíz del zip.
            writer
                .start_file(&nombre_micdb, opciones)
                .map_err(|e| MicError::Io(format!("no se pudo escribir en el zip: {e}")))?;
            let mut origen = std::fs::File::open(&temp_db)
                .map_err(|e| MicError::Io(format!("no se pudo leer la copia de la base: {e}")))?;
            std::io::copy(&mut origen, &mut writer)
                .map_err(|e| MicError::Io(format!("no se pudo escribir la base en el zip: {e}")))?;
            empacados += 1;

            // Carpeta imagenes/ (recursiva, rutas relativas).
            let dir_img = h.dir_imagenes();
            if dir_img.exists() {
                empacados += empacar_dir(&mut writer, &dir_img, "imagenes", opciones)
                    .map_err(|e| MicError::Io(format!("no se pudieron empacar las imágenes: {e}")))?;
            }

            writer
                .finish()
                .map_err(|e| MicError::Io(format!("no se pudo finalizar el zip: {e}")))?;
            Ok(empacados)
        })();

        let _ = std::fs::remove_file(&temp_db);
        resultado
    })
    .await
}

/// Desempaca un `.zip` de álbum a `dir_destino` (lo crea si no existe). Rechaza
/// entradas con `..` (zip-slip). Devuelve la ruta del primer `.micdb` extraído.
/// No abre el álbum: eso lo decide el frontend.
#[tauri::command]
pub async fn album_desempacar(
    ruta_zip: String,
    dir_destino: String,
) -> Result<String, String> {
    tokio::task::spawn_blocking(move || -> Result<String, String> {
        let destino = PathBuf::from(&dir_destino);
        std::fs::create_dir_all(&destino)
            .map_err(|e| format!("no se pudo crear el destino: {e}"))?;

        let archivo = std::fs::File::open(&ruta_zip)
            .map_err(|e| format!("no se pudo abrir el zip: {e}"))?;
        let mut zip = zip::ZipArchive::new(archivo)
            .map_err(|e| format!("el archivo no es un zip válido: {e}"))?;

        let mut ruta_micdb: Option<PathBuf> = None;
        for i in 0..zip.len() {
            let mut entrada = zip
                .by_index(i)
                .map_err(|e| format!("no se pudo leer una entrada del zip: {e}"))?;

            // Ruta segura dentro del destino (rechaza zip-slip).
            let nombre = match entrada.enclosed_name() {
                Some(n) => n,
                None => {
                    return Err(format!(
                        "el zip contiene una ruta no segura: '{}'",
                        entrada.name()
                    ));
                }
            };
            let salida = destino.join(&nombre);

            if entrada.is_dir() {
                std::fs::create_dir_all(&salida)
                    .map_err(|e| format!("no se pudo crear el directorio: {e}"))?;
                continue;
            }
            if let Some(padre) = salida.parent() {
                std::fs::create_dir_all(padre)
                    .map_err(|e| format!("no se pudo crear el directorio: {e}"))?;
            }
            let mut destino_archivo = std::fs::File::create(&salida)
                .map_err(|e| format!("no se pudo crear el archivo: {e}"))?;
            let mut buf = Vec::new();
            entrada
                .read_to_end(&mut buf)
                .map_err(|e| format!("no se pudo leer del zip: {e}"))?;
            destino_archivo
                .write_all(&buf)
                .map_err(|e| format!("no se pudo escribir el archivo: {e}"))?;

            if ruta_micdb.is_none()
                && salida
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("micdb"))
                    .unwrap_or(false)
            {
                ruta_micdb = Some(salida);
            }
        }

        ruta_micdb
            .map(|p| p.to_string_lossy().into_owned())
            .ok_or_else(|| "el zip no contiene ningún archivo .micdb".to_string())
    })
    .await
    .map_err(|e| format!("error interno al desempacar: {e}"))?
}

/// Nombre del archivo JSON que guarda las plantillas de álbum.
const ARCHIVO_PLANTILLAS: &str = "plantillas.json";

/// Una plantilla de álbum: un nombre y la lista de campos iniciales (ex-frmNuevo/.xms).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plantilla {
    pub nombre: String,
    pub campos: Vec<CampoNuevo>,
}

/// Ruta del archivo de plantillas dentro del directorio de configuración.
fn ruta_plantillas(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("no se pudo resolver el directorio de configuración: {e}"))?;
    Ok(dir.join(ARCHIVO_PLANTILLAS))
}

/// Carga las plantillas guardadas (vacía si el archivo no existe o está corrupto).
fn cargar_plantillas(app: &AppHandle) -> Vec<Plantilla> {
    let ruta = match ruta_plantillas(app) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let datos = match std::fs::read_to_string(&ruta) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    serde_json::from_str(&datos).unwrap_or_default()
}

/// Guarda la lista de plantillas en disco.
fn guardar_plantillas(app: &AppHandle, lista: &[Plantilla]) -> Result<(), String> {
    let ruta = ruta_plantillas(app)?;
    if let Some(dir) = ruta.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("no se pudo crear el directorio de configuración: {e}"))?;
    }
    let json = serde_json::to_string_pretty(lista)
        .map_err(|e| format!("no se pudo serializar las plantillas: {e}"))?;
    std::fs::write(&ruta, json)
        .map_err(|e| format!("no se pudo escribir plantillas.json: {e}"))?;
    Ok(())
}

/// Lista las plantillas de álbum guardadas (lista vacía si no hay ninguna).
#[tauri::command]
pub async fn plantillas_listar(app: AppHandle) -> Result<Vec<Plantilla>, String> {
    Ok(cargar_plantillas(&app))
}

/// Guarda (upsert por nombre) una plantilla con los campos indicados.
#[tauri::command]
pub async fn plantilla_guardar(
    app: AppHandle,
    nombre: String,
    campos: Vec<CampoNuevo>,
) -> Result<(), String> {
    let mut lista = cargar_plantillas(&app);
    if let Some(p) = lista.iter_mut().find(|p| p.nombre == nombre) {
        p.campos = campos;
    } else {
        lista.push(Plantilla { nombre, campos });
    }
    guardar_plantillas(&app, &lista)
}

/// Elimina la plantilla con el nombre indicado (no falla si no existe).
#[tauri::command]
pub async fn plantilla_eliminar(app: AppHandle, nombre: String) -> Result<(), String> {
    let mut lista = cargar_plantillas(&app);
    lista.retain(|p| p.nombre != nombre);
    guardar_plantillas(&app, &lista)
}
