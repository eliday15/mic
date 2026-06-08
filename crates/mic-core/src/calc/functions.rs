//! Funciones de fórmula — port de la semántica de fechas de `@FECHA(...)`
//! (Module2.bas: `ScanSustVals`, `ConvierteFechaALong`; Module1.bas:
//! `DiasAFecha`).
//!
//! Semántica del original
//! ----------------------
//! El motor VB6 reducía toda la aritmética de fechas a un número de días:
//!
//! - `ConvierteFechaALong(fecha)` devuelve los días transcurridos desde el
//!   `01/01/1900` hasta `fecha` (`DateIntervals` con intervalo de días, que es
//!   `DateDiff("d", 1900-01-01, fecha)`).
//! - En `ScanSustVals`, todo valor de campo que *sea* una fecha se sustituye
//!   por su número de días antes de evaluar la expresión, de modo que sumar y
//!   restar fechas opera en días.
//! - `@FECHA(campo)` exige que el campo contenga una fecha válida; lo sustituye
//!   por su número de días. Si no es fecha, es "Uso Indebido de @FECHA".
//! - Para mostrar un resultado *como fecha* (cuando el tipo de salida del campo
//!   calculado es Fecha), `DiasAFecha(n)` reconvierte: `1900-01-01 + n días`.
//!
//! Este módulo expone esas conversiones con `chrono` y está pensado para
//! ampliarse con futuras funciones (registro en [`buscar_funcion`]).

use chrono::{Duration, NaiveDate};

/// Epoch del modelo de fechas del original: `01/01/1900`.
/// `@FECHA`/`ConvierteFechaALong` cuentan días desde aquí.
fn epoch() -> NaiveDate {
    // 1900-01-01 siempre es una fecha válida.
    NaiveDate::from_ymd_opt(1900, 1, 1).expect("epoch 1900-01-01 válida")
}

/// Convierte una fecha a su número de días desde el epoch (`01/01/1900`).
/// Port de `ConvierteFechaALong`.
pub fn fecha_a_dias(fecha: NaiveDate) -> i64 {
    (fecha - epoch()).num_days()
}

/// Convierte un número de días desde el epoch de vuelta a fecha.
/// Port de `DiasAFecha`.
pub fn dias_a_fecha(dias: i64) -> NaiveDate {
    epoch() + Duration::days(dias)
}

/// Intenta interpretar una cadena como fecha, replicando con tolerancia el
/// `IsDate`/`CVDate` del original. Acepta los formatos habituales del dominio:
/// ISO (`YYYY-MM-DD`, el formato canónico del contrato), y los formatos con
/// `/` día-primero (`DD/MM/YYYY`) y mes-primero (`MM/DD/YYYY`) de la app VB.
///
/// Devuelve `None` si no parsea como fecha (equivale a `IsDate = False`).
pub fn interpretar_fecha(texto: &str) -> Option<NaiveDate> {
    let s = texto.trim();
    if s.is_empty() {
        return None;
    }
    // Formato canónico del contrato (ISO `YYYY-MM-DD`).
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(d);
    }
    // Variantes con separador `/` que producía la app original (locale).
    for fmt in ["%d/%m/%Y", "%m/%d/%Y", "%Y/%m/%d", "%d-%m-%Y"] {
        if let Ok(d) = NaiveDate::parse_from_str(s, fmt) {
            return Some(d);
        }
    }
    None
}

/// Formatea una fecha como cadena ISO `YYYY-MM-DD` (formato del contrato para
/// valores de tipo fecha).
pub fn formato_iso(fecha: NaiveDate) -> String {
    fecha.format("%Y-%m-%d").to_string()
}

/// Identificador de una función de fórmula reconocida.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Funcion {
    /// `@FECHA(arg)`: interpreta `arg` como fecha y la reduce a su número de
    /// días desde el epoch para permitir aritmética de fechas.
    Fecha,
}

impl Funcion {
    /// Cantidad de argumentos que espera la función.
    pub fn aridad(&self) -> usize {
        match self {
            Funcion::Fecha => 1,
        }
    }
}

/// Resuelve el nombre de una función (tras `@`) a su variante.
/// El original solo conoce `FECHA` (comparado en mayúsculas).
/// Diseñado para ampliarse: añade aquí nuevas funciones futuras.
pub fn buscar_funcion(nombre: &str) -> Option<Funcion> {
    match nombre.to_ascii_uppercase().as_str() {
        "FECHA" => Some(Funcion::Fecha),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_es_cero_dias() {
        assert_eq!(fecha_a_dias(epoch()), 0);
    }

    #[test]
    fn dias_redondea_ida_y_vuelta() {
        let d = NaiveDate::from_ymd_opt(2007, 11, 15).unwrap();
        let n = fecha_a_dias(d);
        assert_eq!(dias_a_fecha(n), d);
    }

    #[test]
    fn diferencia_de_dias_entre_fechas() {
        let a = interpretar_fecha("2007-11-15").unwrap();
        let b = interpretar_fecha("2007-11-10").unwrap();
        assert_eq!(fecha_a_dias(a) - fecha_a_dias(b), 5);
    }

    #[test]
    fn interpreta_varios_formatos() {
        let iso = interpretar_fecha("2020-02-29").unwrap();
        assert_eq!(iso, NaiveDate::from_ymd_opt(2020, 2, 29).unwrap());
        let dmy = interpretar_fecha("29/02/2020").unwrap();
        assert_eq!(dmy, iso);
    }

    #[test]
    fn texto_no_fecha_devuelve_none() {
        assert!(interpretar_fecha("hola").is_none());
        assert!(interpretar_fecha("").is_none());
    }

    #[test]
    fn fecha_es_funcion_conocida_insensible_a_caja() {
        assert_eq!(buscar_funcion("FECHA"), Some(Funcion::Fecha));
        assert_eq!(buscar_funcion("fecha"), Some(Funcion::Fecha));
        assert_eq!(buscar_funcion("SUMA"), None);
    }
}
