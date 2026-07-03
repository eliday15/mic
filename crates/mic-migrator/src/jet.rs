//! Lectura de álbumes Access/Jet `.mdb` **en proceso** con el crate pure-Rust
//! [`jetdb`].
//!
//! Sustituye al antiguo `mdbtools` (shell-out a binarios de 32 bits): no lanza
//! subprocesos, no necesita `.exe` ni DLLs, y por tanto NO se cuelga en carpetas
//! de red. jetdb lee el archivo directamente (Jet3/Jet4, que es lo que usa MIC).
//!
//! La capa de migración sigue trabajando con [`crate::csv_parser::TablaCsv`]
//! (todo cadenas): [`leer_tabla`] convierte cada [`jetdb::Value`] a la misma
//! representación textual que antes producía `mdb-export`, de modo que el resto
//! del migrador (`convertir_valor_sql`, `normalizar_fecha_iso`, `parse_numero`,
//! `jet_bool`, `type_map::mapear_campos`) no necesita cambios.

use std::path::Path;
use std::time::Duration;

use jetdb::format::ObjectType;
use jetdb::{read_catalog, read_table_def, read_table_rows, PageReader, Value};

use mic_core::error::MicError;

use crate::csv_parser::TablaCsv;

/// Plazo máximo para una lectura completa de jetdb (catálogo + def + filas).
///
/// jetdb es rápido (un `.mdb` de 10 MB / 66k filas se lee en menos de 1 s), pero
/// envolver la lectura en un plazo es defensa en profundidad: un `.mdb` corrupto
/// que hiciera entrar a jetdb en un bucle NUNCA debe colgar la app. Igual que en
/// `copia_local`, el trabajo corre en un hilo vigilado por el reloj.
const PLAZO_LECTURA: Duration = Duration::from_secs(120);

/// Una tabla leída del `.mdb`: sus datos como [`TablaCsv`] más el número de filas
/// que jetdb tuvo que omitir por errores de parseo.
///
/// `omitidas > 0` no es un error: el migrador lo reporta como advertencia.
pub struct TablaLeida {
    /// Datos de la tabla (cabecera + filas, todo cadenas), igual que el CSV
    /// que antes producía `mdb-export`.
    pub csv: TablaCsv,
    /// Filas que jetdb no pudo leer y omitió (`ReadResult::skipped_rows`).
    pub omitidas: usize,
}

/// Lista las tablas de **usuario** de un `.mdb` (excluye tablas de sistema).
///
/// Equivale a `mdb-tables -1`: devuelve solo objetos de tipo tabla cuyo flag de
/// sistema/oculto está apagado (las MSys* y demás internas de Access quedan
/// fuera). Las tablas de un álbum MIC (Principal, Variantes, Multidatos,
/// Categorias, Propiedades, Grupos, FiltrosAv, Reportes…) son todas de usuario.
pub fn tablas(ruta: &Path) -> Result<Vec<String>, MicError> {
    crate::diag::paso("abriendo el .mdb y leyendo el catálogo de tablas");
    con_plazo(ruta, |reader| {
        let catalogo = read_catalog(reader).map_err(err_jet)?;
        Ok(catalogo
            .into_iter()
            .filter(es_tabla_usuario)
            .map(|e| e.name)
            .collect())
    })
}

/// Lee una tabla completa y la devuelve como [`TablaLeida`].
///
/// Busca la tabla por nombre (case-insensitive) en el catálogo, lee su
/// definición y sus filas, y construye la [`TablaCsv`]: la cabecera son los
/// nombres de columna y cada celda es el valor convertido a cadena por
/// [`valor_a_string`]. Si la tabla no existe, falla con
/// [`MicError::NoEncontrado`].
pub fn leer_tabla(ruta: &Path, tabla: &str) -> Result<TablaLeida, MicError> {
    crate::diag::paso(&format!("leyendo la tabla '{tabla}'"));
    let tabla = tabla.to_string();
    con_plazo(ruta, move |reader| {
        let catalogo = read_catalog(reader).map_err(err_jet)?;
        let entrada = catalogo
            .into_iter()
            .filter(es_tabla_usuario)
            .find(|e| e.name.eq_ignore_ascii_case(&tabla))
            .ok_or_else(|| {
                MicError::NoEncontrado(format!("la tabla '{tabla}' no existe en el .mdb"))
            })?;

        let def = read_table_def(reader, &entrada.name, entrada.table_page).map_err(err_jet)?;
        let cabecera: Vec<String> = def.columns.iter().map(|c| c.name.clone()).collect();

        let res = read_table_rows(reader, &def).map_err(err_jet)?;
        let filas: Vec<Vec<String>> = res
            .rows
            .iter()
            .map(|fila| fila.iter().map(valor_a_string).collect())
            .collect();

        Ok(TablaLeida {
            csv: TablaCsv { cabecera, filas },
            omitidas: res.skipped_rows,
        })
    })
}

/// Indica si una entrada del catálogo es una tabla de **usuario** (no de sistema).
///
/// Aplica el mismo criterio que `jetdb::table_names`: objeto de tipo tabla con
/// los flags de sistema y oculto apagados. Como red de seguridad adicional,
/// descarta también cualquier nombre que empiece por `MSys` (las tablas internas
/// de Access), por si algún `.mdb` antiguo no marcara el flag.
fn es_tabla_usuario(e: &jetdb::CatalogEntry) -> bool {
    use jetdb::format::catalog_flags;
    e.object_type == ObjectType::Table
        && (e.flags & (catalog_flags::SYSTEM | catalog_flags::HIDDEN)) == 0
        && !e.name.starts_with("MSys")
}

/// Convierte un [`jetdb::Value`] a la cadena que el migrador espera (la misma
/// forma textual que producía `mdb-export`).
///
/// - `Null`/`Binary` → cadena vacía (los binarios son OLE; `mdb-export -b strip`
///   también los dejaba vacíos; el migrador trata "" como NULL).
/// - Booleanos → `"1"`/`"0"` (lo que entiende `type_map::jet_bool`).
/// - Enteros → su valor decimal.
/// - `Float`/`Double` → `format!("{f}")` (la representación más corta que
///   round-trippea; sin notación científica para magnitudes normales). La parsea
///   `parse_numero`.
/// - `Money`/`Numeric` → la cadena decimal tal cual (ya viene con punto decimal).
/// - `Text`/`Guid` → la cadena tal cual.
/// - `Timestamp(días)` → fecha ISO `YYYY-MM-DD` (MIC solo guarda la fecha). Fuera
///   de rango → vacía.
/// - `DateTimeExtended(s)` → los primeros 10 caracteres (la parte de la fecha).
fn valor_a_string(v: &Value) -> String {
    match v {
        Value::Null | Value::Binary(_) => String::new(),
        Value::Bool(b) => if *b { "1" } else { "0" }.to_string(),
        Value::Byte(n) => n.to_string(),
        Value::Int(n) => n.to_string(),
        Value::Long(n) => n.to_string(),
        Value::BigInt(n) => n.to_string(),
        Value::Float(f) => format!("{f}"),
        Value::Double(f) => format!("{f}"),
        Value::Money(s) | Value::Numeric(s) => s.clone(),
        Value::Text(s) | Value::Guid(s) => s.clone(),
        Value::Timestamp(dias) => timestamp_a_iso(*dias),
        Value::DateTimeExtended(s) => s.chars().take(10).collect(),
    }
}

/// Convierte un timestamp de Jet (días desde 1899-12-30) a fecha ISO
/// `YYYY-MM-DD`, o cadena vacía si la fecha queda fuera de rango representable.
fn timestamp_a_iso(dias: f64) -> String {
    let base = match chrono::NaiveDate::from_ymd_opt(1899, 12, 30) {
        Some(d) => d,
        None => return String::new(),
    };
    match base.checked_add_signed(chrono::Duration::days(dias.trunc() as i64)) {
        Some(fecha) => fecha.format("%Y-%m-%d").to_string(),
        None => String::new(),
    }
}

/// Ejecuta una lectura de jetdb sobre `ruta` vigilada por [`PLAZO_LECTURA`].
///
/// Abre el `PageReader` y corre `f` en un hilo trabajador; si no termina dentro
/// del plazo (un `.mdb` corrupto que atascara a jetdb), se devuelve un error
/// claro en vez de colgar la app. El hilo huérfano se abandona: no puede dañar
/// nada porque solo lee la copia local del archivo.
fn con_plazo<T, F>(ruta: &Path, f: F) -> Result<T, MicError>
where
    T: Send + 'static,
    F: FnOnce(&mut PageReader) -> Result<T, MicError> + Send + 'static,
{
    let ruta = ruta.to_path_buf();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let resultado = PageReader::open(&ruta)
            .map_err(err_jet)
            .and_then(|mut reader| f(&mut reader));
        let _ = tx.send(resultado);
    });

    match rx.recv_timeout(PLAZO_LECTURA) {
        Ok(resultado) => resultado,
        Err(_) => Err(MicError::Migracion(format!(
            "la lectura del .mdb tardó más de {} s y se canceló (¿archivo dañado?).",
            PLAZO_LECTURA.as_secs()
        ))),
    }
}

/// Traduce un error de jetdb a [`MicError::Migracion`] con un mensaje claro.
fn err_jet(e: jetdb::FileError) -> MicError {
    MicError::Migracion(format!("no se pudo leer el .mdb: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valor_a_string_basicos() {
        assert_eq!(valor_a_string(&Value::Null), "");
        assert_eq!(valor_a_string(&Value::Binary(vec![1, 2, 3])), "");
        assert_eq!(valor_a_string(&Value::Bool(true)), "1");
        assert_eq!(valor_a_string(&Value::Bool(false)), "0");
        assert_eq!(valor_a_string(&Value::Byte(7)), "7");
        assert_eq!(valor_a_string(&Value::Int(-3)), "-3");
        assert_eq!(valor_a_string(&Value::Long(12345)), "12345");
        assert_eq!(valor_a_string(&Value::BigInt(9_000_000_000)), "9000000000");
        assert_eq!(valor_a_string(&Value::Text("hola".into())), "hola");
        assert_eq!(
            valor_a_string(&Value::Money("12345.6789".into())),
            "12345.6789"
        );
        assert_eq!(valor_a_string(&Value::Numeric("3.14".into())), "3.14");
        assert_eq!(
            valor_a_string(&Value::Guid("{ABCD}".into())),
            "{ABCD}"
        );
    }

    #[test]
    fn valor_a_string_doubles_parseables() {
        // format!("{f}") da una cadena que parse_numero acepta y round-trippea.
        let s = valor_a_string(&Value::Double(10.5));
        assert_eq!(s, "10.5");
        assert_eq!(crate::type_map::parse_numero(&s), Some(10.5));

        let s = valor_a_string(&Value::Float(3.25));
        assert_eq!(crate::type_map::parse_numero(&s), Some(3.25));
    }

    #[test]
    fn timestamp_a_fecha_iso() {
        // 1899-12-30 es el día 0 de Jet.
        assert_eq!(timestamp_a_iso(0.0), "1899-12-30");
        // 2007-10-29 = 39384 días desde 1899-12-30.
        assert_eq!(timestamp_a_iso(39384.0), "2007-10-29");
        // La parte de hora se ignora (solo fecha).
        assert_eq!(timestamp_a_iso(39384.75), "2007-10-29");
    }

    #[test]
    fn datetime_extended_recorta_fecha() {
        assert_eq!(
            valor_a_string(&Value::DateTimeExtended(
                "2021-06-14 22:45:12.3456789".into()
            )),
            "2021-06-14"
        );
    }
}
