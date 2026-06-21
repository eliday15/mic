//! mic-tauri: aplicación de escritorio MIC 3.0 (Tauri 2).
//!
//! Expone los comandos del `CONTRACT.md`, mantiene el estado de los álbumes
//! abiertos ([`state::AppState`]) y sirve miniaturas vía el protocolo custom
//! `thumb://`.
//!
//! ## Protocolo `thumb://`
//! URL: `thumb://localhost/{albumId}/{tabla}/{id}?size={0|128|256|512}&v={version}`
//! (en Windows el webview la traduce a `http://thumb.localhost/...`; la ruta es
//! idéntica en ambos casos, por lo que el parseo es uniforme).
//!
//! - `size > 0`: devuelve la miniatura JPEG generada por [`mic_thumbs::ThumbCache`],
//!   con `Content-Type: image/jpeg` y `Cache-Control` inmutable a largo plazo.
//! - `size == 0`: devuelve la imagen original a tamaño completo (visor), con el
//!   `Content-Type` deducido de la extensión.
//! - Si el registro no tiene imagen o el archivo no existe: `404`.

pub mod commands;
pub mod state;

use std::path::Path;

use tauri::http::{Request, Response, StatusCode};
use tauri::{Manager, UriSchemeContext, UriSchemeResponder};

use mic_core::model::Tabla;
use state::AppState;

/// `Cache-Control` de las miniaturas: el versionado (`?v=`) hace seguro un TTL
/// largo e inmutable; al cambiar la imagen, cambia `v` y por tanto la URL.
const CACHE_CONTROL_THUMB: &str = "public, max-age=31536000, immutable";

/// Punto de entrada de la app Tauri.
pub fn run() {
    inicializar_tracing();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::nuevo())
        .register_asynchronous_uri_scheme_protocol("thumb", manejar_thumb)
        .invoke_handler(tauri::generate_handler![
            // Álbum
            commands::album::album_crear,
            commands::album::album_abrir,
            commands::album::album_cerrar,
            commands::album::album_compactar,
            commands::album::albumes_recientes,
            commands::album::album_recalcular,
            commands::album::album_copiar,
            commands::album::album_empacar,
            commands::album::album_desempacar,
            commands::album::plantillas_listar,
            commands::album::plantilla_guardar,
            commands::album::plantilla_eliminar,
            // Campos
            commands::campos::campos_listar,
            commands::campos::campo_crear,
            commands::campos::campo_editar,
            commands::campos::campo_eliminar,
            commands::campos::campos_reordenar,
            commands::campos::formula_probar,
            commands::exportar::exportar_registros,
            // Registros
            commands::registros::registros_query,
            commands::registros::registro_obtener,
            commands::registros::registro_crear,
            commands::registros::registro_editar,
            commands::registros::registros_eliminar,
            commands::registros::registro_imagen_set,
            commands::registros::registros_editar_lote,
            commands::registros::registros_set_auxiliar,
            commands::registros::registros_totalizar,
            commands::registros::registros_estadisticas,
            commands::registros::registros_actualizar_masivo,
            commands::registros::registros_crear_desde_carpeta,
            commands::registros::variantes_listar,
            // Multidatos / categorías
            commands::multidatos::categorias_sugerir,
            commands::multidatos::categorias_listar,
            commands::multidatos::categorias_actualizar,
            // Grupos
            commands::grupos::grupos_listar,
            commands::grupos::grupo_guardar,
            commands::grupos::grupo_eliminar,
            commands::grupos::grupo_arbol,
            // Álbumes ligados
            commands::ligados::ligados_listar,
            commands::ligados::liga_guardar,
            commands::ligados::liga_eliminar,
            commands::ligados::liga_actualizar,
            commands::ligados::ligas_actualizar_todas,
            // Importación de registros
            commands::importar::importar_inspeccionar,
            commands::importar::importar_registros,
            // Filtros avanzados
            commands::filtros::filtros_listar,
            commands::filtros::filtro_obtener,
            commands::filtros::filtro_guardar,
            commands::filtros::filtro_eliminar,
            // Reportes (impresión)
            commands::reportes::reportes_listar,
            commands::reportes::reporte_guardar,
            commands::reportes::reporte_eliminar,
            // Miniaturas
            commands::thumbs::thumb_invalidar,
            // Migración
            commands::migracion::migracion_verificar_mdbtools,
            commands::migracion::migracion_inspeccionar,
            commands::migracion::migracion_ejecutar,
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar MIC");
}

/// Inicializa `tracing_subscriber` para el logging del backend.
///
/// Respeta `RUST_LOG` si está definido; por defecto registra a nivel `info`.
fn inicializar_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};
    let filtro = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    // Ignora el error si ya hay un subscriber global (p. ej. en tests).
    let _ = fmt().with_env_filter(filtro).try_init();
}

/// Handler asíncrono del protocolo `thumb://`.
///
/// Parsea la URL, resuelve la imagen del registro y responde con bytes. El
/// trabajo de E/S (DB + generación de miniatura) se ejecuta en el pool de
/// bloqueo para no bloquear el hilo del runtime.
fn manejar_thumb(
    ctx: UriSchemeContext<'_, tauri::Wry>,
    request: Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let app = ctx.app_handle().clone();
    let path = request.uri().path().to_string();
    let query = request.uri().query().unwrap_or("").to_string();

    tauri::async_runtime::spawn_blocking(move || {
        let respuesta = match resolver_thumb(&app, &path, &query) {
            Ok(resp) => resp,
            Err(estado) => respuesta_error(estado),
        };
        responder.respond(respuesta);
    });
}

/// Resuelve la petición de miniatura a una respuesta HTTP con bytes.
///
/// Devuelve `Err(StatusCode)` cuando hay que responder un error (404/400/500).
fn resolver_thumb(
    app: &tauri::AppHandle,
    path: &str,
    query: &str,
) -> Result<Response<Vec<u8>>, StatusCode> {
    let (album_id, tabla, id) = parsear_ruta(path).ok_or(StatusCode::BAD_REQUEST)?;
    let (size, _version) = parsear_query(query);

    let state = app.state::<AppState>();
    let handle = state
        .obtener(album_id)
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Ruta absoluta de la imagen del registro (404 si el registro no tiene).
    let imagen_abs = match commands::thumbs::ruta_imagen_abs(&handle, tabla, id) {
        Ok(Some(p)) => p,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::warn!(error = %e, "thumb: no se pudo resolver la imagen del registro");
            return Err(StatusCode::NOT_FOUND);
        }
    };

    // `obtener` con size=0 devuelve la ruta original; con size>0, genera/cachea.
    let ruta_archivo = handle.thumbs.obtener(&imagen_abs, size).map_err(|e| {
        tracing::warn!(error = %e, "thumb: no se pudo obtener la miniatura");
        StatusCode::NOT_FOUND
    })?;

    let bytes = std::fs::read(&ruta_archivo).map_err(|e| {
        tracing::error!(error = %e, "thumb: no se pudo leer el archivo de imagen");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Content-Type: las miniaturas siempre son JPEG; el original, según extensión.
    let content_type = if size == 0 {
        content_type_por_extension(&ruta_archivo)
    } else {
        "image/jpeg"
    };

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", content_type)
        .header("Cache-Control", CACHE_CONTROL_THUMB)
        .header("Access-Control-Allow-Origin", "*")
        .body(bytes)
        .map_err(|e| {
            tracing::error!(error = %e, "thumb: no se pudo construir la respuesta");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// Parsea `/{albumId}/{tabla}/{id}` en sus tres componentes.
///
/// `tabla` debe ser `principal` o `variantes`; cualquier otra cosa es inválida.
fn parsear_ruta(path: &str) -> Option<(u64, Tabla, i64)> {
    let partes: Vec<&str> = path.trim_matches('/').split('/').collect();
    if partes.len() != 3 {
        return None;
    }
    let album_id: u64 = partes[0].parse().ok()?;
    let tabla = match partes[1] {
        "principal" => Tabla::Principal,
        "variantes" => Tabla::Variantes,
        _ => return None,
    };
    let id: i64 = partes[2].parse().ok()?;
    Some((album_id, tabla, id))
}

/// Extrae `size` (por defecto 256) y `v` (versión, ignorada en el cómputo: solo
/// sirve para que el webview cachee por URL) de la query string.
fn parsear_query(query: &str) -> (u32, Option<String>) {
    let mut size: u32 = 256;
    let mut version: Option<String> = None;
    for par in query.split('&') {
        let mut it = par.splitn(2, '=');
        match (it.next(), it.next()) {
            (Some("size"), Some(v)) => {
                if let Ok(n) = v.parse::<u32>() {
                    size = n;
                }
            }
            (Some("v"), Some(v)) => version = Some(v.to_string()),
            _ => {}
        }
    }
    (size, version)
}

/// Construye una respuesta de error con cuerpo vacío y el estado indicado.
fn respuesta_error(estado: StatusCode) -> Response<Vec<u8>> {
    Response::builder()
        .status(estado)
        .header("Access-Control-Allow-Origin", "*")
        .body(Vec::new())
        .unwrap_or_else(|_| {
            let mut r = Response::new(Vec::new());
            *r.status_mut() = estado;
            r
        })
}

/// Deduce el `Content-Type` de una imagen original por su extensión.
fn content_type_por_extension(ruta: &Path) -> &'static str {
    match ruta
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        Some("tif") | Some("tiff") => "image/tiff",
        _ => "application/octet-stream",
    }
}
