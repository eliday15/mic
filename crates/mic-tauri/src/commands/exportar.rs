//! Exportación de registros (ex-frmExp del VB6).
//!
//! Vuelca el conjunto filtrado actual (respetando filtro y orden de la
//! `QueryReq`) a un archivo CSV, XLSX o CSV-MIC. Las columnas y su orden vienen
//! dados por `campos` (nombres visibles). Los valores se escriben sin formato
//! regional: números con punto decimal, fechas en ISO y multidatos uniendo sus
//! valores reales con " | ". Consulta TODOS los registros (sin paginar) usando
//! el mismo `query_builder` que la grilla.
//!
//! El formato `csv-mic` produce un CSV que el "Importar..." del MIC clásico
//! (VB6, micNOV2007) puede leer para actualizar su álbum por campo llave.

use mic_core::error::MicError;
use mic_core::model::{CampoDef, QueryReq, RegistroLigero, Tabla, TipoCampo, Valor};
use rust_xlsxwriter::{Format, Workbook};
use tauri::State;

use crate::commands::{en_db, handle};
use crate::state::{AlbumHandle, AppState};

/// Exporta los registros filtrados a `ruta_destino` en `formato` ("csv" | "xlsx").
///
/// Las columnas son los `campos` (nombres visibles) en el orden indicado.
/// Devuelve cuántos registros se exportaron. Errores en español.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn exportar_registros(
    state: State<'_, AppState>,
    album_id: u64,
    req: QueryReq,
    campos: Vec<String>,
    formato: String,
    ruta_destino: String,
) -> Result<u64, String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| {
        exportar(h, &req, &campos, &formato, &ruta_destino)
    })
    .await
}

/// Resuelve los campos a exportar (preservando el orden de `nombres`), consulta
/// todos los registros y escribe el archivo según el formato.
fn exportar(
    h: &AlbumHandle,
    req: &QueryReq,
    nombres: &[String],
    formato: &str,
    ruta_destino: &str,
) -> Result<u64, MicError> {
    if nombres.is_empty() {
        return Err(MicError::Invalido(
            "no hay campos seleccionados para exportar".into(),
        ));
    }

    let todos = h.campos();
    // Campos seleccionados en el orden pedido (omite nombres desconocidos).
    let seleccion: Vec<CampoDef> = nombres
        .iter()
        .filter_map(|n| todos.iter().find(|c| &c.nombre == n).cloned())
        .collect();
    if seleccion.is_empty() {
        return Err(MicError::Invalido(
            "ninguno de los campos indicados existe en el álbum".into(),
        ));
    }

    let conn = h.db.conn()?;
    // Consulta TODOS los registros que cumplen el filtro/orden, sin paginar.
    let req_total = QueryReq {
        offset: 0,
        limit: u32::MAX,
        ..req.clone()
    };
    let pagina = mic_db::repo_registros::query(&conn, &todos, &req_total)?;
    let principal = matches!(req.tabla, Tabla::Principal);

    // Pre-resuelve los valores de cada celda (texto plano) por filas.
    let mut filas: Vec<Vec<String>> = Vec::with_capacity(pagina.registros.len());
    for reg in &pagina.registros {
        let mut fila: Vec<String> = Vec::with_capacity(seleccion.len());
        for campo in &seleccion {
            fila.push(celda(&conn, campo, reg, principal)?);
        }
        filas.push(fila);
    }

    let encabezados: Vec<&str> = seleccion.iter().map(|c| c.nombre.as_str()).collect();
    match formato {
        "csv" => escribir_csv(ruta_destino, &encabezados, &filas)?,
        "xlsx" => escribir_xlsx(ruta_destino, &encabezados, &filas)?,
        "csv-mic" => escribir_csv_mic(ruta_destino, &encabezados, &filas)?,
        otro => {
            return Err(MicError::Invalido(format!(
                "formato de exportación no soportado: '{otro}'"
            )))
        }
    }

    Ok(filas.len() as u64)
}

/// Texto de una celda según el tipo del campo. Multidatos → valores reales
/// unidos con " | "; números con punto decimal; fechas ISO; texto tal cual.
fn celda(
    conn: &mic_db::pool::Conn,
    campo: &CampoDef,
    reg: &RegistroLigero,
    principal: bool,
) -> Result<String, MicError> {
    if matches!(campo.tipo, TipoCampo::Multidato) {
        let vals = mic_db::repo_multidatos::listar(conn, reg.id, campo.id, principal)?;
        return Ok(vals.join(" | "));
    }
    let valor = reg.valores.get(&campo.nombre);
    Ok(match valor {
        None => String::new(),
        Some(v) => texto_valor(v),
    })
}

/// Representación textual de un valor sin formato regional (punto decimal,
/// enteros sin `.0`). Las fechas ya llegan como texto ISO desde la consulta.
fn texto_valor(v: &Valor) -> String {
    match v {
        Valor::Nulo(_) => String::new(),
        Valor::Texto(s) => s.clone(),
        Valor::Bool(b) => if *b { "1" } else { "0" }.to_string(),
        Valor::Entero(n) => n.to_string(),
        Valor::Numero(n) => {
            // Enteros exactos sin parte decimal; resto con punto decimal.
            if n.fract() == 0.0 && n.is_finite() {
                format!("{}", *n as i64)
            } else {
                n.to_string()
            }
        }
    }
}

/// Escribe un CSV UTF-8 con BOM (para que Excel lo abra bien), separador coma.
fn escribir_csv(
    ruta: &str,
    encabezados: &[&str],
    filas: &[Vec<String>],
) -> Result<(), MicError> {
    let mut datos: Vec<u8> = Vec::new();
    // BOM UTF-8 para compatibilidad con Excel.
    datos.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    {
        let mut wtr = csv::Writer::from_writer(&mut datos);
        wtr.write_record(encabezados)
            .map_err(|e| MicError::Io(format!("no se pudo escribir el CSV: {e}")))?;
        for fila in filas {
            wtr.write_record(fila)
                .map_err(|e| MicError::Io(format!("no se pudo escribir el CSV: {e}")))?;
        }
        wtr.flush()
            .map_err(|e| MicError::Io(format!("no se pudo escribir el CSV: {e}")))?;
    }
    std::fs::write(ruta, &datos).map_err(|e| {
        MicError::Io(format!("no se pudo guardar el archivo '{ruta}': {e}"))
    })?;
    Ok(())
}

/// Escribe un CSV compatible con el "Importar..." del MIC clásico (VB6).
///
/// El importador viejo (`MuestraCVS`/`ProcesaActImp` de Module3.bas) lee el
/// archivo con `Split(linea, ",")` — sin comillas ni escape — en ANSI
/// (Windows-1252) y toma la PRIMERA columna como campo llave. Por eso:
/// - sin BOM (corrompería el nombre del primer campo),
/// - separador coma sin comillas: toda coma/tab/salto de línea dentro de un
///   valor se sustituye por espacio,
/// - codificación Windows-1252 (caracteres sin mapeo → '?'),
/// - líneas terminadas en CRLF.
pub(crate) fn escribir_csv_mic(
    ruta: &str,
    encabezados: &[&str],
    filas: &[Vec<String>],
) -> Result<(), MicError> {
    let mut datos: Vec<u8> = Vec::new();
    let linea = |celdas: Vec<String>| celdas.join(",") + "\r\n";

    let enc: Vec<String> = encabezados.iter().map(|c| sanear_mic(c)).collect();
    datos.extend_from_slice(&a_windows_1252(&linea(enc)));
    for fila in filas {
        let celdas: Vec<String> = fila.iter().map(|c| sanear_mic(c)).collect();
        datos.extend_from_slice(&a_windows_1252(&linea(celdas)));
    }

    std::fs::write(ruta, &datos).map_err(|e| {
        MicError::Io(format!("no se pudo guardar el archivo '{ruta}': {e}"))
    })?;
    Ok(())
}

/// Sustituye por espacio los caracteres que romperían al lector del MIC
/// clásico: coma (separador), tab (separador interno de su grilla) y saltos de
/// línea.
fn sanear_mic(s: &str) -> String {
    s.replace([',', '\t', '\r', '\n'], " ")
}

/// Codifica a Windows-1252. Los caracteres sin representación se sustituyen
/// por '?' (encoding_rs insertaría referencias HTML `&#…;` que el MIC clásico
/// mostraría literalmente).
fn a_windows_1252(s: &str) -> Vec<u8> {
    let (bytes, _, hubo_error) = encoding_rs::WINDOWS_1252.encode(s);
    if !hubo_error {
        return bytes.into_owned();
    }
    // Hay caracteres sin mapeo: re-codifica carácter por carácter.
    let mut out = Vec::with_capacity(s.len());
    let mut buf = [0u8; 4];
    for ch in s.chars() {
        let (b, _, err) = encoding_rs::WINDOWS_1252.encode(ch.encode_utf8(&mut buf));
        if err {
            out.push(b'?');
        } else {
            out.extend_from_slice(&b);
        }
    }
    out
}

/// Escribe un XLSX con encabezados en negrita y ancho de columna razonable.
fn escribir_xlsx(
    ruta: &str,
    encabezados: &[&str],
    filas: &[Vec<String>],
) -> Result<(), MicError> {
    let mut libro = Workbook::new();
    let hoja = libro.add_worksheet();
    let negrita = Format::new().set_bold();

    // Encabezados.
    for (col, enc) in encabezados.iter().enumerate() {
        hoja.write_string_with_format(0, col as u16, *enc, &negrita)
            .map_err(err_xlsx)?;
    }

    // Datos.
    for (i, fila) in filas.iter().enumerate() {
        let row = (i + 1) as u32;
        for (col, celda) in fila.iter().enumerate() {
            hoja.write_string(row, col as u16, celda).map_err(err_xlsx)?;
        }
    }

    // Auto-ancho razonable: el mayor entre el encabezado y el contenido de la
    // columna, acotado a un máximo para no generar columnas gigantes.
    for (col, enc) in encabezados.iter().enumerate() {
        let mut ancho = enc.chars().count();
        for fila in filas {
            if let Some(c) = fila.get(col) {
                ancho = ancho.max(c.chars().count());
            }
        }
        let ancho = (ancho as f64 + 2.0).clamp(8.0, 60.0);
        hoja.set_column_width(col as u16, ancho).map_err(err_xlsx)?;
    }

    libro.save(ruta).map_err(|e| {
        MicError::Io(format!("no se pudo guardar el archivo '{ruta}': {e}"))
    })?;
    Ok(())
}

/// Convierte un error de `rust_xlsxwriter` en [`MicError`] (mensaje en español).
fn err_xlsx(e: rust_xlsxwriter::XlsxError) -> MicError {
    MicError::Io(format!("error al generar el Excel: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ruta temporal única para no chocar entre tests.
    fn ruta_tmp(nombre: &str) -> String {
        std::env::temp_dir()
            .join(format!("mic_test_{}_{nombre}", std::process::id()))
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn csv_mic_sin_bom_cp1252_y_crlf() {
        let ruta = ruta_tmp("basico.csv");
        let filas = vec![vec!["A-1".into(), "Jarrón chino".into()]];
        escribir_csv_mic(&ruta, &["Clave", "Descripción"], &filas).unwrap();
        let bytes = std::fs::read(&ruta).unwrap();
        std::fs::remove_file(&ruta).ok();

        // Sin BOM: arranca directo con la 'C' de "Clave".
        assert_eq!(&bytes[..5], b"Clave");
        // "ó" de "Descripción" en Windows-1252 = 0xF3 (un solo byte, no UTF-8).
        assert!(bytes.contains(&0xF3));
        assert!(!bytes.windows(2).any(|w| w == [0xC3, 0xB3])); // no UTF-8
        // Líneas CRLF.
        let texto: Vec<u8> = bytes.clone();
        assert_eq!(texto.windows(2).filter(|w| *w == b"\r\n").count(), 2);
        // Encabezado y datos separados por coma, sin comillas.
        assert!(bytes.windows(17).any(|w| w == b"Clave,Descripci\xF3n".as_slice()));
    }

    #[test]
    fn csv_mic_sanea_separadores_dentro_de_valores() {
        let ruta = ruta_tmp("saneo.csv");
        let filas = vec![vec!["A-1".into(), "rojo, grande\tcon\nsalto".into()]];
        escribir_csv_mic(&ruta, &["Clave", "Notas"], &filas).unwrap();
        let bytes = std::fs::read(&ruta).unwrap();
        std::fs::remove_file(&ruta).ok();

        let texto = encoding_rs::WINDOWS_1252.decode(&bytes).0;
        let lineas: Vec<&str> = texto.trim_end().split("\r\n").collect();
        // Sigue siendo 1 encabezado + 1 registro (el \n del valor no partió la línea).
        assert_eq!(lineas.len(), 2);
        // El valor mantiene una sola coma por fila: la del separador.
        assert_eq!(lineas[1].matches(',').count(), 1);
        assert_eq!(lineas[1], "A-1,rojo  grande con salto");
    }

    #[test]
    fn csv_mic_caracter_sin_mapeo_va_como_interrogacion() {
        let ruta = ruta_tmp("mapeo.csv");
        let filas = vec![vec!["A-1".into(), "nieve ☃ y €uro".into()]];
        escribir_csv_mic(&ruta, &["Clave", "Notas"], &filas).unwrap();
        let bytes = std::fs::read(&ruta).unwrap();
        std::fs::remove_file(&ruta).ok();

        let texto = encoding_rs::WINDOWS_1252.decode(&bytes).0;
        // '☃' no existe en cp1252 → '?'; '€' sí existe (0x80) y se conserva.
        assert!(texto.contains("nieve ? y €uro"));
    }
}
