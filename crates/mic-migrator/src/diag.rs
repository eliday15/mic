//! Diagnóstico de la migración: paso actual + bitácora en disco.
//!
//! La importación de Access falló en producción varias veces "sin decir nada":
//! el usuario solo veía un spinner. Este módulo garantiza que SIEMPRE haya
//! rastro: cada etapa se registra (a) en un estado global consultable —para que
//! el mensaje de timeout diga el último paso— y (b) en una bitácora
//! `mic-migracion.log` en el directorio temporal, que el usuario puede enviar.
//!
//! Es deliberadamente primitivo (append síncrono, sin `tracing`): tiene que
//! funcionar incluso si todo lo demás está roto.

use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

/// Último paso registrado (para mensajes de error/timeout).
static PASO: Mutex<String> = Mutex::new(String::new());

/// Ruta de la bitácora de migración (en el directorio temporal del sistema;
/// en Windows: `%TEMP%\mic-migracion.log`).
pub fn ruta_log() -> PathBuf {
    std::env::temp_dir().join("mic-migracion.log")
}

/// Registra el paso actual: actualiza el estado global y añade una línea con
/// hora a la bitácora. Nunca falla (los errores de E/S se ignoran: el
/// diagnóstico no debe romper la migración).
pub fn paso(etiqueta: &str) {
    if let Ok(mut p) = PASO.lock() {
        etiqueta.clone_into(&mut p);
    }
    let linea = format!(
        "[{}] {etiqueta}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(ruta_log())
        .and_then(|mut f| f.write_all(linea.as_bytes()));
}

/// Último paso registrado (cadena vacía si aún no hay ninguno).
pub fn paso_actual() -> String {
    PASO.lock().map(|p| p.clone()).unwrap_or_default()
}
