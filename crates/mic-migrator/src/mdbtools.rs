//! Envoltura de los binarios de **mdbtools** (`mdb-tables`, `mdb-schema`,
//! `mdb-export`) usados para leer álbumes Access/Jet `.mdb`.
//!
//! No asumimos que mdbtools esté en el `PATH`: además de `PATH`, probamos las
//! rutas típicas de Homebrew (`/opt/homebrew/bin`, `/usr/local/bin`). La salida
//! se captura **como bytes** (`Vec<u8>`), porque los `.mdb` están en
//! Windows-1252 y la decodificación a UTF-8 la hace [`crate::csv_parser`].

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use mic_core::error::MicError;

/// Plazo máximo para una operación de mdbtools sobre el `.mdb`.
///
/// Los álbumes suelen estar en carpetas de red o unidades en la nube; una
/// lectura atascada no debe colgar la app para siempre. 120 s da margen de
/// sobra para archivos legítimamente lentos.
const PLAZO_OPERACION: Duration = Duration::from_secs(120);

/// Plazo corto para la simple comprobación de que un binario arranca
/// (`--help`). Si ni eso responde (p. ej. un diálogo de DLL faltante bloqueado
/// en Windows), se reporta como no disponible en vez de colgarse.
const PLAZO_ARRANQUE: Duration = Duration::from_secs(10);

/// Directorios donde buscar los binarios de mdbtools además del `PATH`.
const DIRS_EXTRA: &[&str] = &["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"];

/// Directorio de recursos donde van los binarios de mdbtools **embebidos** en
/// la app (caso Windows: el instalador trae `mdb-*.exe` + sus DLLs junto al
/// ejecutable). Lo registra la capa Tauri al arrancar con
/// [`registrar_dir_empaquetado`]; si no se registra (Mac/Linux con mdbtools del
/// sistema, o en tests), se cae a `DIRS_EXTRA`/`PATH`.
static DIR_EMPAQUETADO: OnceLock<PathBuf> = OnceLock::new();

/// Registra el directorio de recursos donde residen los binarios embebidos de
/// mdbtools (Windows).
///
/// Lo invoca la app Tauri en su `.setup(...)`, resolviendo la ruta del bundle de
/// recursos. En Windows los ejecutables se distribuyen dentro del instalador
/// (`mdb-tables.exe`, `mdb-export.exe`, ... + DLLs), de modo que el usuario no
/// necesita instalar mdbtools por su cuenta. Es idempotente: solo el primer
/// registro tiene efecto (los siguientes son no-op).
pub fn registrar_dir_empaquetado(dir: PathBuf) {
    let _ = DIR_EMPAQUETADO.set(dir);
}

/// Directorio de binarios embebidos a usar: el registrado, o (solo Windows) el
/// auto-descubierto junto al ejecutable.
///
/// El auto-descubrimiento (`<dir del exe>/resources/mdbtools/win-x86`, la misma
/// ruta relativa que declara `tauri.conf.json`) es una red de seguridad: cubre
/// el caso en que nadie llamó a [`registrar_dir_empaquetado`] (tests, otro punto
/// de entrada) o en que la resolución de recursos de Tauri cambiara. Se calcula
/// una sola vez y solo se acepta si contiene `mdb-export.exe`.
#[cfg(target_os = "windows")]
fn dir_embebido() -> Option<&'static PathBuf> {
    if let Some(dir) = DIR_EMPAQUETADO.get() {
        return Some(dir);
    }
    static AUTO: OnceLock<Option<PathBuf>> = OnceLock::new();
    AUTO.get_or_init(|| {
        let exe = std::env::current_exe().ok()?;
        let cand = exe.parent()?.join("resources").join("mdbtools").join("win-x86");
        cand.join("mdb-export.exe").is_file().then_some(cand)
    })
    .as_ref()
}

/// Localiza el ejecutable `nombre` (p. ej. `"mdb-export"`).
///
/// Orden de prioridad:
/// 1. El directorio empaquetado (si se registró): se prueba `dir/nombre` y
///    `dir/nombre.exe`. En Windows el archivo lleva extensión `.exe`, y para una
///    ruta absoluta hay que nombrarlo completo (`Command` no añade `.exe` cuando
///    se le pasa una ruta con directorio, solo cuando resuelve por `PATH`).
/// 2. Los directorios extra del sistema (Homebrew, `/usr/bin`...), probando
///    también `nombre` y `nombre.exe`.
/// 3. El nombre tal cual, para que `Command` lo resuelva por `PATH`.
///
/// La detección real de disponibilidad la hace [`disponible`].
fn localizar(nombre: &str) -> String {
    let nombre_exe = format!("{nombre}.exe");

    // (1) Binarios embebidos en la app. SOLO en Windows: son ejecutables PE
    // (.exe/.dll) que no corren en Mac/Linux. Aunque el dir se registre en otras
    // plataformas (el bundle los incluye), aquí se ignora para no devolver un
    // .exe inservible y caer así al mdbtools del sistema.
    #[cfg(target_os = "windows")]
    if let Some(dir) = dir_embebido() {
        for cand_nombre in [nombre, nombre_exe.as_str()] {
            let cand = dir.join(cand_nombre);
            if cand.is_file() {
                return cand.to_string_lossy().into_owned();
            }
        }
    }

    // (2) Rutas típicas del sistema.
    for dir in DIRS_EXTRA {
        for cand_nombre in [nombre, nombre_exe.as_str()] {
            let cand = Path::new(dir).join(cand_nombre);
            if cand.is_file() {
                return cand.to_string_lossy().into_owned();
            }
        }
    }

    // (3) Que lo resuelva el PATH.
    nombre.to_string()
}

/// Ejecuta `nombre` con `--help` y comprueba que el binario responde.
///
/// Se usa para `disponible()` y para la verificación previa a inspeccionar o
/// migrar: si mdbtools no está instalado, el comando ni siquiera arranca.
/// Solo importa que arranque y termine dentro del plazo; el código de salida
/// da igual (muchas herramientas devuelven ≠0 ante `--help`).
fn responde(nombre: &str) -> bool {
    lanzar_con_plazo(nombre, &["--help"], PLAZO_ARRANQUE).is_ok()
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
    let (estado, stdout, stderr) = lanzar_con_plazo(nombre, args, PLAZO_OPERACION)?;
    if !estado.success() {
        let err = String::from_utf8_lossy(&stderr);
        return Err(MicError::Migracion(format!(
            "'{nombre}' falló: {}",
            err.trim()
        )));
    }
    Ok(stdout)
}

/// Lanza un binario de mdbtools con un plazo máximo y devuelve
/// `(estado, stdout, stderr)`.
///
/// A diferencia de `Command::output()`, NUNCA espera para siempre: si el
/// proceso no termina dentro del plazo (típico de un `.mdb` en una carpeta de
/// red que no responde), se mata y se devuelve un error claro. Los pipes se
/// leen en hilos aparte para que el hijo no se bloquee por buffers llenos
/// mientras aquí se vigila el reloj.
fn lanzar_con_plazo(
    nombre: &str,
    args: &[&str],
    plazo: Duration,
) -> Result<(ExitStatus, Vec<u8>, Vec<u8>), MicError> {
    let mut cmd = Command::new(localizar(nombre));
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Sin ventana de consola por cada proceso hijo (en Windows, cada spawn de
    // una herramienta de consola desde una app gráfica parpadea una ventana
    // negra; la inspección lanza varios).
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let mut hijo = cmd.spawn().map_err(|e| {
        MicError::Migracion(format!(
            "no se pudo ejecutar '{nombre}' (¿mdbtools instalado?): {e}"
        ))
    })?;

    let mut out = hijo.stdout.take().expect("stdout en piped");
    let mut err = hijo.stderr.take().expect("stderr en piped");
    let lector_out = std::thread::spawn(move || {
        let mut v = Vec::new();
        let _ = out.read_to_end(&mut v);
        v
    });
    let lector_err = std::thread::spawn(move || {
        let mut v = Vec::new();
        let _ = err.read_to_end(&mut v);
        v
    });

    let inicio = Instant::now();
    let estado = loop {
        match hijo.try_wait() {
            Ok(Some(estado)) => break estado,
            Ok(None) if inicio.elapsed() >= plazo => {
                let _ = hijo.kill();
                let _ = hijo.wait();
                return Err(MicError::Migracion(format!(
                    "'{nombre}' tardó más de {} s y se canceló. Si el archivo \
                     está en una carpeta de red o en la nube, cópielo a esta \
                     computadora e inténtelo de nuevo desde ahí.",
                    plazo.as_secs()
                )));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => {
                let _ = hijo.kill();
                let _ = hijo.wait();
                return Err(MicError::Migracion(format!(
                    "error esperando a '{nombre}': {e}"
                )));
            }
        }
    };

    let stdout = lector_out.join().unwrap_or_default();
    let stderr = lector_err.join().unwrap_or_default();
    Ok((estado, stdout, stderr))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Un proceso que excede el plazo se mata y devuelve un error claro, en
    /// vez de colgar la app (el caso real: un `.mdb` en una carpeta de red
    /// que no responde).
    #[cfg(unix)]
    #[test]
    fn proceso_colgado_se_cancela_por_plazo() {
        let r = lanzar_con_plazo("sleep", &["5"], Duration::from_millis(300));
        let err = r.expect_err("debe agotar el plazo").to_string();
        assert!(
            err.contains("tardó más de"),
            "el error debe explicar el plazo agotado: {err}"
        );
    }

    /// Un proceso que termina dentro del plazo devuelve su salida normal.
    #[cfg(unix)]
    #[test]
    fn proceso_rapido_no_se_ve_afectado_por_el_plazo() {
        let (estado, stdout, _) =
            lanzar_con_plazo("echo", &["hola"], Duration::from_secs(10)).expect("echo debe correr");
        assert!(estado.success());
        assert_eq!(String::from_utf8_lossy(&stdout).trim(), "hola");
    }
}
