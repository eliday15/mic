//! Parseo de la salida CSV de `mdb-export`.
//!
//! `mdb-export` emite CSV en la codificación del `.mdb` (Windows-1252), con
//! delimitador coma y comilla doble (`"`), escapando comillas internas
//! duplicándolas (`""`). El flujo es:
//!
//! 1. Decodificar los bytes Windows-1252 → `String` UTF-8 ([`decodificar_cp1252`]).
//! 2. Parsear el texto resultante con el lector `csv` ([`parsear`]).
//!
//! Decodificar **antes** de pasar al lector CSV es importante: así los acentos
//! (`á`, `ñ`, `ü`…) llegan ya como caracteres UTF-8 correctos y nunca se parten
//! en mitad de un campo.

use encoding_rs::WINDOWS_1252;
use mic_core::error::MicError;

/// Una tabla CSV parseada: cabecera + filas, todo como cadenas UTF-8.
#[derive(Debug, Clone, Default)]
pub struct TablaCsv {
    /// Nombres de columna (primera fila del CSV de `mdb-export`).
    pub cabecera: Vec<String>,
    /// Filas de datos; cada fila tiene tantas celdas como la cabecera.
    pub filas: Vec<Vec<String>>,
}

impl TablaCsv {
    /// Índice de la columna `nombre` (case-insensitive), si existe.
    pub fn indice(&self, nombre: &str) -> Option<usize> {
        self.cabecera
            .iter()
            .position(|c| c.eq_ignore_ascii_case(nombre))
    }

    /// Número de filas de datos.
    pub fn len(&self) -> usize {
        self.filas.len()
    }

    /// `true` si no hay filas de datos.
    pub fn is_empty(&self) -> bool {
        self.filas.is_empty()
    }
}

/// Decodifica bytes Windows-1252 (cp1252) a `String` UTF-8.
///
/// Windows-1252 nunca falla: cada byte 0x80–0x9F tiene un carácter asignado
/// (o el reemplazo), de modo que el resultado siempre es válido. Usamos
/// `encoding_rs` para reproducir exactamente la tabla de Microsoft (que difiere
/// de Latin-1 en el rango 0x80–0x9F: comillas tipográficas, €, …).
pub fn decodificar_cp1252(bytes: &[u8]) -> String {
    let (texto, _, _) = WINDOWS_1252.decode(bytes);
    texto.into_owned()
}

/// Decodifica los bytes Windows-1252 y parsea el CSV resultante.
///
/// El lector `csv` se configura con la cabecera **deshabilitada** para tratarla
/// como una fila más y exponerla en [`TablaCsv::cabecera`]; así conservamos los
/// nombres de columna exactos de Access. `flexible(true)` tolera que alguna fila
/// tenga menos celdas (registros antiguos con columnas añadidas después).
pub fn parsear(bytes: &[u8]) -> Result<TablaCsv, MicError> {
    let texto = decodificar_cp1252(bytes);
    parsear_texto(&texto)
}

/// Igual que [`parsear`] pero sobre texto UTF-8 ya decodificado.
///
/// Expuesto aparte para los tests con fixtures sintéticos.
pub fn parsear_texto(texto: &str) -> Result<TablaCsv, MicError> {
    let mut lector = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b',')
        .quote(b'"')
        .double_quote(true)
        .flexible(true)
        .from_reader(texto.as_bytes());

    let mut filas_iter = lector.records();

    let cabecera: Vec<String> = match filas_iter.next() {
        Some(reg) => reg
            .map_err(|e| MicError::Migracion(format!("CSV mal formado (cabecera): {e}")))?
            .iter()
            .map(|c| c.trim().to_string())
            .collect(),
        None => return Ok(TablaCsv::default()),
    };

    let ncols = cabecera.len();
    let mut filas = Vec::new();
    for reg in filas_iter {
        let reg = reg.map_err(|e| MicError::Migracion(format!("CSV mal formado: {e}")))?;
        let mut fila: Vec<String> = reg.iter().map(|c| c.to_string()).collect();
        // Normaliza el ancho a la cabecera (rellena o recorta).
        if fila.len() < ncols {
            fila.resize(ncols, String::new());
        } else if fila.len() > ncols {
            fila.truncate(ncols);
        }
        filas.push(fila);
    }

    Ok(TablaCsv { cabecera, filas })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Construye bytes cp1252 a partir de un literal con escapes byte a byte.
    fn cp1252(s: &[u8]) -> Vec<u8> {
        s.to_vec()
    }

    #[test]
    fn decodifica_acentos_cp1252() {
        // "Café Niño €" en Windows-1252:
        // á=0xE9? -> 'é'=0xE9, 'ñ'=0xF1, '€'=0x80.
        let bytes = cp1252(b"Caf\xe9 Ni\xf1o \x80");
        let texto = decodificar_cp1252(&bytes);
        assert_eq!(texto, "Café Niño €");
    }

    #[test]
    fn parsea_cabecera_y_filas() {
        let bytes = cp1252(b"Nombre,Precio\n\"Mart\xednez\",10.5\n\"L\xf3pez\",20\n");
        let t = parsear(&bytes).unwrap();
        assert_eq!(t.cabecera, vec!["Nombre", "Precio"]);
        assert_eq!(t.len(), 2);
        assert_eq!(t.filas[0], vec!["Martínez", "10.5"]);
        assert_eq!(t.filas[1], vec!["López", "20"]);
    }

    #[test]
    fn indice_case_insensitive() {
        let t = parsear_texto("Nombre,Tipo,Valor\na,b,c\n").unwrap();
        assert_eq!(t.indice("tipo"), Some(1));
        assert_eq!(t.indice("VALOR"), Some(2));
        assert_eq!(t.indice("inexistente"), None);
    }

    #[test]
    fn comillas_internas_dobladas() {
        // mdb-export escapa comillas duplicándolas dentro del campo entrecomillado.
        let t = parsear_texto("Texto,N\n\"Dice \"\"hola\"\"\",1\n").unwrap();
        assert_eq!(t.filas[0][0], "Dice \"hola\"");
    }

    #[test]
    fn comas_dentro_de_comillas() {
        let t = parsear_texto("Lista,N\n\"a, b, c\",3\n").unwrap();
        assert_eq!(t.filas[0][0], "a, b, c");
        assert_eq!(t.filas[0][1], "3");
    }

    #[test]
    fn filas_cortas_se_rellenan() {
        let t = parsear_texto("A,B,C\nx,y\n").unwrap();
        assert_eq!(t.filas[0], vec!["x", "y", ""]);
    }

    #[test]
    fn csv_vacio() {
        let t = parsear_texto("").unwrap();
        assert!(t.cabecera.is_empty());
        assert!(t.is_empty());
    }
}
