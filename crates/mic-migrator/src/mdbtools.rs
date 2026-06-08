//! Envoltura de los binarios de **mdbtools** (`mdb-tables`, `mdb-schema`,
//! `mdb-export`) usados para leer álbumes Access/Jet `.mdb`.
//!
//! No asumimos que mdbtools esté en el `PATH`: además de `PATH`, probamos las
//! rutas típicas de Homebrew (`/opt/homebrew/bin`, `/usr/local/bin`). La salida
//! se captura **como bytes** (`Vec<u8>`), porque los `.mdb` están en
//! Windows-1252 y la decodificación a UTF-8 la hace [`crate::csv_parser`].

use std::path::Path;
use std::process::Command;

use mic_core::error::MicError;

/// Directorios donde buscar los binarios de mdbtools además del `PATH`.
const DIRS_EXTRA: &[&str] = &["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"];

/// Localiza el ejecutable `nombre` (p. ej. `"mdb-export"`).
///
/// Devuelve la ruta absoluta del binario si existe en alguno de los directorios
/// extra; si no, devuelve el nombre tal cual para que `Command` lo resuelva por
/// `PATH`. La detección real de disponibilidad la hace [`disponible`].
fn localizar(nombre: &str) -> String {
    for dir in DIRS_EXTRA {
        let cand = Path::new(dir).join(nombre);
        if cand.is_file() {
            return cand.to_string_lossy().into_owned();
        }
    }
    nombre.to_string()
}

/// Ejecuta `nombre` con `--help` y comprueba que el binario responde.
///
/// Se usa para `disponible()` y para la verificación previa a inspeccionar o
/// migrar: si mdbtools no está instalado, el comando ni siquiera arranca.
fn responde(nombre: &str) -> bool {
    Command::new(localizar(nombre))
        .arg("--help")
        .output()
        .is_ok()
}

/// Indica si los tres binarios necesarios de mdbtools están disponibles.
///
/// Es la base del comando `migracion_verificar_mdbtools`. Comprueba
/// `mdb-tables`, `mdb-schema` y `mdb-export`.
pub fn disponible() -> bool {
    responde("mdb-tables") && responde("mdb-schema") && responde("mdb-export")
}

/// Ejecuta un binario de mdbtools y devuelve su `stdout` **como bytes**.
///
/// Falla con [`MicError::Migracion`] si el proceso no se puede lanzar (mdbtools
/// ausente) o termina con código distinto de cero (en cuyo caso se adjunta el
/// `stderr` decodificado de forma laxa, solo para diagnóstico).
fn ejecutar(nombre: &str, args: &[&str]) -> Result<Vec<u8>, MicError> {
    let salida = Command::new(localizar(nombre))
        .args(args)
        .output()
        .map_err(|e| {
            MicError::Migracion(format!(
                "no se pudo ejecutar '{nombre}' (¿mdbtools instalado?): {e}"
            ))
        })?;
    if !salida.status.success() {
        let err = String::from_utf8_lossy(&salida.stderr);
        return Err(MicError::Migracion(format!(
            "'{nombre}' falló: {}",
            err.trim()
        )));
    }
    Ok(salida.stdout)
}

/// Lista las tablas de un `.mdb` (una por línea, sin tablas de sistema).
///
/// Usa `mdb-tables -1`. Los nombres se decodifican desde Windows-1252.
pub fn tablas(ruta_mdb: &Path) -> Result<Vec<String>, MicError> {
    let bytes = ejecutar("mdb-tables", &["-1", &ruta_mdb.to_string_lossy()])?;
    let texto = crate::csv_parser::decodificar_cp1252(&bytes);
    Ok(texto
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect())
}

/// Vuelca el esquema SQL de una tabla concreta (`mdb-schema -T <tabla>`).
///
/// Se usa de forma auxiliar/diagnóstica: la migración real lee los datos por
/// CSV y los metadatos de campos por la tabla `propiedades`, no por el DDL.
pub fn esquema(ruta_mdb: &Path, tabla: &str) -> Result<String, MicError> {
    let bytes = ejecutar(
        "mdb-schema",
        &["-T", tabla, &ruta_mdb.to_string_lossy()],
    )?;
    Ok(crate::csv_parser::decodificar_cp1252(&bytes))
}

/// Exporta una tabla a CSV (bytes crudos en Windows-1252).
///
/// Flags fijos (acordados en el plan de migración):
/// - `-D '%Y-%m-%d'`: fechas en ISO-8601, directamente comparables/ordenables.
/// - `-b strip`: descarta el contenido de campos binarios (OLE), que no migramos.
///
/// El delimitador (coma) y la comilla (`"`) son los de por defecto de
/// `mdb-export`, que es lo que espera [`crate::csv_parser`]. La cabecera se
/// conserva (no se pasa `-H`) para conocer los nombres de columna.
pub fn exportar_csv(ruta_mdb: &Path, tabla: &str) -> Result<Vec<u8>, MicError> {
    ejecutar(
        "mdb-export",
        &[
            "-D",
            "%Y-%m-%d",
            "-b",
            "strip",
            &ruta_mdb.to_string_lossy(),
            tabla,
        ],
    )
}
