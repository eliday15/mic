//! Importación de registros desde CSV/XLSX (moderniza el "Importar..." del VB6,
//! Module3.bas `ActualizaConImportado`).
//!
//! Casa las filas del archivo con los registros del álbum por una **llave de
//! importación**: un campo del álbum ↔ una columna del archivo (por defecto la
//! columna homónima del campo). Según la **política** elegida, sustituye,
//! mantiene o rellena los valores en conflicto. Las filas cuya llave no casa con
//! ningún registro se reportan una por una (número de fila + valor) para que el
//! diálogo las muestre como error y pida confirmación; se dan de alta si
//! `crear_faltantes` y se omiten si no. Las **columnas extra** (sin campo en el
//! álbum) se ignoran, o con `importar_extras` se crean como campos nuevos (tipo
//! inferido de sus valores) antes de aplicar. Solo tabla principal.
//!
//! Dos comandos:
//! - `importar_inspeccionar`: lee el archivo y devuelve metadatos (columnas,
//!   encoding, formato, columnas reconocidas/ignoradas, llaves sugeridas, huella)
//!   para poblar la fase de configuración del diálogo.
//! - `importar_registros` con `dry_run`: recorre EXACTAMENTE la misma lógica de
//!   casamiento y políticas. Con `dry_run=true` solo clasifica y cuenta (cálculo
//!   puro de lectura, el resumen previo no miente); con `dry_run=false` aplica.
//!
//! El núcleo ([`aplicar_importacion`]) es una función pura sin Tauri (recibe la
//! conexión, los campos y el motor) para poder probarse con un álbum temporal;
//! el comando solo orquesta lectura de archivo, huella y eventos de progreso.
//!
//! Detalles delicados resueltos (ver análisis de riesgos):
//! - encoding en cascada BOM → UTF-8 estricto → Windows-1252 (todo byte stream
//!   es CP1252-válido, por eso UTF-8 estricto va primero);
//! - autodetección de separador (`,` `;` `\t`) sobre la línea de encabezado;
//! - números es-MX (`$`, miles, NBSP, decimal `,`) y fechas (ISO, dd/MM/yyyy)
//!   normalizados ANTES de construir el `Valor` (si no, `como_f64`/`valor_a_sql`
//!   los guardarían como NULL o como texto basura);
//! - llave numérica con clave canónica sin `.0` en AMBOS lados (`CAST(1.0 AS
//!   TEXT) = '1.0'` nunca casaría `"1"`);
//! - merge SIEMPRE sobre el registro actual completo (jamás `Valores` parcial a
//!   `editar`, que pondría a NULL las columnas ausentes).

use std::collections::{HashMap, HashSet};

use mic_core::calc::MotorCalculo;
use mic_core::error::MicError;
use mic_core::model::{CampoDef, CampoNuevo, Tabla, TipoCampo, Valor, Valores};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::commands::{en_db, handle};
use crate::state::{AlbumHandle, AppState};

/// Nombre del evento de progreso de la importación.
const EVENTO_PROGRESO: &str = "importacion-progreso";

/// Cada cuántas filas procesadas se emite un evento de progreso.
const PASO_PROGRESO: u64 = 50;

/// Máximo de filas detalladas por lista (no coincidentes, llaves vacías o
/// repetidas): el total se informa siempre completo; el detalle se trunca para
/// no inflar el IPC con archivos enormes.
const LIMITE_DETALLE: usize = 500;

/// Una fila del archivo: número (base 1 contando el encabezado, como Excel) y
/// valor de su llave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilaLlave {
    pub fila: u64,
    pub llave: String,
}

/// Lista de filas con el total completo y el detalle truncado a [`LIMITE_DETALLE`].
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListaFilas {
    pub total: u64,
    pub filas: Vec<FilaLlave>,
}

impl ListaFilas {
    /// Cuenta la fila y la guarda en el detalle si aún cabe.
    fn agregar(&mut self, fila: u64, llave: &str) {
        self.total += 1;
        if self.filas.len() < LIMITE_DETALLE {
            self.filas.push(FilaLlave {
                fila,
                llave: llave.to_string(),
            });
        }
    }
}

/// Columna del archivo que no casa con ningún campo del álbum, con el tipo
/// inferido para el campo que se crearía al importarla.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnaExtra {
    pub columna: String,
    pub tipo: TipoCampo,
    pub decimales: u8,
}

/// Metadatos del archivo a importar (fase de configuración del diálogo).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InspeccionImport {
    /// Encabezados del archivo, en orden (sin BOM, ya saneados).
    pub columnas: Vec<String>,
    /// Número de filas de datos (sin contar el encabezado).
    pub total_filas: u64,
    /// `"utf-8"` | `"utf-8-bom"` | `"windows-1252"` (los xlsx reportan `"utf-8"`).
    pub encoding: String,
    /// `"csv"` | `"xlsx"`.
    pub formato: String,
    /// Columnas que casan (case-insensitive, sin orden) con un campo principal.
    pub columnas_reconocidas: Vec<String>,
    /// Columnas que no coinciden con ningún campo del álbum (se ignoran al aplicar).
    pub columnas_no_reconocidas: Vec<String>,
    /// Columnas reconocidas elegibles como llave (no calculado/multidato).
    pub campos_llave_sugeridos: Vec<String>,
    /// Campos principales del álbum elegibles como llave (no calculado/multidato),
    /// estén o no en el archivo: la columna del archivo se elige aparte.
    pub campos_llave_album: Vec<String>,
    /// Columnas extra (sin campo en el álbum) con el tipo inferido de sus valores.
    pub columnas_extra: Vec<ColumnaExtra>,
    /// Huella `len:mtime` del archivo, para detectar cambios entre resumen y aplicar.
    pub huella: String,
}

/// Resultado de una importación (mismo struct para dry-run y aplicación).
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultadoImportacion {
    /// Registros existentes que se actualizaron (o se actualizarían en dry-run).
    pub actualizados: u64,
    /// Registros nuevos dados de alta (o que se darían de alta en dry-run).
    pub creados: u64,
    /// Filas que no produjeron ningún cambio.
    pub sin_cambio: u64,
    /// Filas cuya llave casó con un registro existente.
    pub coincidentes: u64,
    /// Filas cuya llave NO casó con ningún registro (el diálogo lo marca como
    /// error y pide confirmación antes de aplicar).
    pub no_coincidentes: ListaFilas,
    /// Filas sin coincidencia que se omitieron (sin `crear_faltantes`).
    pub omitidos: u64,
    /// Filas con la llave vacía (siempre se omiten).
    pub llaves_vacias: ListaFilas,
    /// Filas con una llave repetida dentro del archivo (se usa la primera).
    pub llaves_duplicadas: ListaFilas,
    /// Columnas extra detectadas, con su tipo inferido.
    pub columnas_extra: Vec<ColumnaExtra>,
    /// Campos creados (o que se crearían, en dry-run) para las columnas extra.
    pub campos_creados: Vec<String>,
    /// Errores por celda/fila (no abortan el lote): mensaje con número de fila.
    pub errores: Vec<String>,
    /// Avisos globales (columnas ignoradas, calculados, llaves duplicadas...).
    pub avisos: Vec<String>,
    /// `true` si fue un análisis en seco (no escribió nada).
    pub dry_run: bool,
}

/// Payload del evento `importacion-progreso`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgresoEvento {
    /// `"analizando"` (dry-run) | `"aplicando"`.
    fase: String,
    hechas: u64,
    total: u64,
}

/// Convierte un error de rusqlite en [`MicError::Db`] (mensaje en español).
fn err_sql(e: rusqlite::Error) -> MicError {
    MicError::Db(e.to_string())
}

// ---------------------------------------------------------------------------
// Comandos Tauri
// ---------------------------------------------------------------------------

/// Inspecciona un archivo CSV/XLSX y devuelve sus metadatos para configurar la
/// importación (sin escribir nada en el álbum).
#[tauri::command]
pub async fn importar_inspeccionar(
    state: State<'_, AppState>,
    album_id: u64,
    ruta_archivo: String,
) -> Result<InspeccionImport, String> {
    let h = handle(&state, album_id)?;
    en_db(h, move |h| inspeccionar(h, &ruta_archivo)).await
}

/// Importa los registros del archivo al álbum.
///
/// Con `dry_run=true` solo analiza y cuenta (resumen previo); con `dry_run=false`
/// aplica los cambios. `columna_llave` es la columna del archivo que se casa con
/// `campo_llave` (default: la homónima). `importar_extras` crea campos para las
/// columnas extra antes de aplicar. Emite `importacion-progreso` durante el proceso.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn importar_registros(
    app: AppHandle,
    state: State<'_, AppState>,
    album_id: u64,
    ruta_archivo: String,
    campo_llave: String,
    columna_llave: Option<String>,
    politica: String,
    crear_faltantes: bool,
    importar_extras: Option<bool>,
    dry_run: bool,
    huella: Option<String>,
) -> Result<ResultadoImportacion, String> {
    let h = handle(&state, album_id)?;
    let opciones = OpcionesImport {
        campo_llave,
        columna_llave,
        politica,
        crear_faltantes,
        importar_extras: importar_extras.unwrap_or(false),
        dry_run,
    };
    en_db(h, move |h| importar(&app, h, &ruta_archivo, &opciones, huella.as_deref())).await
}

// ---------------------------------------------------------------------------
// Lectura de archivo (CSV / XLSX)
// ---------------------------------------------------------------------------

/// Formato detectado por la extensión del archivo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FormatoArchivo {
    Csv,
    Xlsx,
}

/// Detecta el formato por la extensión (`.xlsx` → Xlsx; el resto se trata como CSV).
fn detectar_formato(ruta: &str) -> FormatoArchivo {
    let ext = std::path::Path::new(ruta)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    match ext.as_deref() {
        Some("xlsx") | Some("xlsm") | Some("xlsb") | Some("xls") | Some("ods") => {
            FormatoArchivo::Xlsx
        }
        _ => FormatoArchivo::Csv,
    }
}

/// `(encabezados, filas, encoding, separador)` de un CSV ya decodificado.
type CsvLeido = (Vec<String>, Vec<Vec<String>>, String, u8);

/// Datos crudos leídos de un archivo: encabezados, filas, encoding y separador.
struct ArchivoLeido {
    encabezados: Vec<String>,
    filas: Vec<Vec<String>>,
    encoding: String,
    formato: FormatoArchivo,
    /// Separador detectado del CSV (coma para XLSX: los valores ya vienen tipados).
    separador: u8,
}

/// Lee el archivo (CSV o XLSX) a `(encabezados, filas, encoding, formato)`.
fn leer_archivo(ruta: &str) -> Result<ArchivoLeido, MicError> {
    if ruta.trim().is_empty() {
        return Err(MicError::Invalido("no se indicó ningún archivo".into()));
    }
    if !std::path::Path::new(ruta).exists() {
        return Err(MicError::NoEncontrado(format!(
            "no se encontró el archivo '{ruta}'"
        )));
    }
    match detectar_formato(ruta) {
        FormatoArchivo::Csv => {
            let (encabezados, filas, encoding, separador) = leer_csv(ruta)?;
            Ok(ArchivoLeido {
                encabezados,
                filas,
                encoding,
                formato: FormatoArchivo::Csv,
                separador,
            })
        }
        FormatoArchivo::Xlsx => {
            let (encabezados, filas) = leer_xlsx(ruta)?;
            Ok(ArchivoLeido {
                encabezados,
                filas,
                encoding: "utf-8".into(),
                formato: FormatoArchivo::Xlsx,
                separador: b',',
            })
        }
    }
}

/// Lee un CSV detectando encoding (cascada BOM → UTF-8 estricto → Windows-1252)
/// y autodetectando el separador (`,` `;` `\t`) sobre la línea de encabezado.
///
/// Devuelve `(encabezados, filas, encoding, separador)`. El BOM se elimina antes de parsear
/// para que el primer encabezado (la llave por defecto) no llegue como
/// `"\u{FEFF}Clave"` y deje de casar.
fn leer_csv(ruta: &str) -> Result<CsvLeido, MicError> {
    let bytes = std::fs::read(ruta).map_err(|e| {
        MicError::Io(format!(
            "no se pudo abrir el archivo '{ruta}' (¿está abierto en Excel?): {e}"
        ))
    })?;

    let (texto, encoding) = decodificar(&bytes);
    // Quita el BOM aunque ya se haya detectado, por si quedó pegado al texto.
    let texto = texto.trim_start_matches('\u{FEFF}');

    let separador = detectar_separador(texto);

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(separador)
        .has_headers(false)
        .flexible(true)
        .from_reader(texto.as_bytes());

    let mut filas_brutas: Vec<Vec<String>> = Vec::new();
    for registro in rdr.records() {
        let registro =
            registro.map_err(|e| MicError::Io(format!("error al leer el CSV: {e}")))?;
        filas_brutas.push(registro.iter().map(|s| s.to_string()).collect());
    }

    if filas_brutas.is_empty() {
        return Err(MicError::Invalido(
            "el archivo está vacío (sin encabezados)".into(),
        ));
    }

    let encabezados: Vec<String> = filas_brutas
        .remove(0)
        .into_iter()
        .map(|s| s.trim_start_matches('\u{FEFF}').trim().to_string())
        .collect();

    // Normaliza el ancho de cada fila al del encabezado (rellena con vacíos).
    let ancho = encabezados.len();
    for fila in &mut filas_brutas {
        if fila.len() < ancho {
            fila.resize(ancho, String::new());
        }
    }

    Ok((encabezados, filas_brutas, encoding, separador))
}

/// Decodifica los bytes en cascada y devuelve `(texto, etiqueta_encoding)`.
///
/// Cascada: BOM UTF-8 (`EF BB BF`) → `"utf-8-bom"`; UTF-8 estricto → `"utf-8"`;
/// fallback Windows-1252 → `"windows-1252"`. El orden importa: todo byte stream
/// es CP1252-válido, por eso UTF-8 estricto se prueba antes que el fallback.
fn decodificar(bytes: &[u8]) -> (String, String) {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        let texto = String::from_utf8_lossy(&bytes[3..]).into_owned();
        return (texto, "utf-8-bom".into());
    }
    if let Ok(s) = std::str::from_utf8(bytes) {
        return (s.to_string(), "utf-8".into());
    }
    let texto = encoding_rs::WINDOWS_1252.decode(bytes).0.into_owned();
    (texto, "windows-1252".into())
}

/// Cuenta separadores candidatos fuera de comillas en la línea de encabezado y
/// elige el de mayor frecuencia (Excel-es exporta con `;`). Default: coma.
fn detectar_separador(texto: &str) -> u8 {
    let primera_linea = texto.lines().next().unwrap_or("");
    let mut en_comillas = false;
    let (mut comas, mut puntos_coma, mut tabs) = (0usize, 0usize, 0usize);
    for ch in primera_linea.chars() {
        match ch {
            '"' => en_comillas = !en_comillas,
            ',' if !en_comillas => comas += 1,
            ';' if !en_comillas => puntos_coma += 1,
            '\t' if !en_comillas => tabs += 1,
            _ => {}
        }
    }
    // Elige el de mayor cuenta; empates favorecen la coma (formato propio).
    let max = comas.max(puntos_coma).max(tabs);
    if max == 0 || (comas >= puntos_coma && comas >= tabs) {
        b','
    } else if puntos_coma >= tabs {
        b';'
    } else {
        b'\t'
    }
}

/// Lee la primera hoja de un XLSX con calamine a `(encabezados, filas)`.
///
/// Convierte cada celda a texto siguiendo la regla de `exportar::texto_valor`
/// (floats enteros sin `.0`), fechas a ISO con `as_datetime()` y booleanos a
/// `"1"`/`"0"`. Las filas más cortas que el encabezado se rellenan con vacíos.
fn leer_xlsx(ruta: &str) -> Result<(Vec<String>, Vec<Vec<String>>), MicError> {
    use calamine::{open_workbook_auto, Data, DataType, Reader};

    let mut wb = open_workbook_auto(ruta).map_err(|e| {
        MicError::Io(format!(
            "no se pudo abrir el Excel '{ruta}' (¿está abierto en Excel?): {e}"
        ))
    })?;
    let nombre = wb
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| MicError::Invalido("el archivo Excel no tiene hojas".into()))?;
    let rango = wb
        .worksheet_range(&nombre)
        .map_err(|e| MicError::Io(format!("no se pudo leer la hoja '{nombre}': {e}")))?;

    let celda_a_texto = |c: &Data| -> String {
        match c {
            Data::Empty => String::new(),
            Data::String(s) => s.clone(),
            Data::Int(i) => i.to_string(),
            Data::Float(f) => {
                // Entero exacto sin `.0` (misma regla que exportar::texto_valor).
                if f.fract() == 0.0 && f.is_finite() {
                    format!("{}", *f as i64)
                } else {
                    f.to_string()
                }
            }
            Data::Bool(b) => if *b { "1" } else { "0" }.to_string(),
            Data::DateTime(_) => c
                .as_datetime()
                .map(|dt| dt.format("%Y-%m-%d").to_string())
                .unwrap_or_default(),
            Data::DateTimeIso(s) => {
                // Toma los primeros 10 caracteres (la parte de fecha) si parece ISO.
                s.chars().take(10).collect()
            }
            Data::DurationIso(s) => s.clone(),
            Data::Error(_) => String::new(),
        }
    };

    let mut filas_iter = rango.rows();
    let encabezados: Vec<String> = match filas_iter.next() {
        Some(fila) => fila
            .iter()
            .map(|c| celda_a_texto(c).trim().to_string())
            .collect(),
        None => {
            return Err(MicError::Invalido(
                "la hoja de Excel está vacía (sin encabezados)".into(),
            ))
        }
    };

    let ancho = encabezados.len();
    let mut filas: Vec<Vec<String>> = Vec::new();
    for fila in filas_iter {
        let mut celdas: Vec<String> = fila.iter().map(celda_a_texto).collect();
        if celdas.len() < ancho {
            celdas.resize(ancho, String::new());
        }
        filas.push(celdas);
    }

    Ok((encabezados, filas))
}

// ---------------------------------------------------------------------------
// Inspección
// ---------------------------------------------------------------------------

/// Casa una columna del archivo con un campo principal del álbum
/// (case-insensitive, trim). Devuelve el `CampoDef` si coincide.
fn campo_para_columna<'a>(campos: &'a [CampoDef], columna: &str) -> Option<&'a CampoDef> {
    let col = columna.trim();
    campos
        .iter()
        .find(|c| c.tabla == Tabla::Principal && c.nombre.trim().eq_ignore_ascii_case(col))
}

/// Inspecciona el archivo y produce sus metadatos casados con los campos del álbum.
fn inspeccionar(h: &AlbumHandle, ruta: &str) -> Result<InspeccionImport, MicError> {
    let archivo = leer_archivo(ruta)?;
    let campos = h.campos();

    let mut reconocidas: Vec<String> = Vec::new();
    let mut no_reconocidas: Vec<String> = Vec::new();
    let mut sugeridas: Vec<String> = Vec::new();
    let campos_llave_album: Vec<String> = campos
        .iter()
        .filter(|c| es_llave_elegible(c))
        .map(|c| c.nombre.clone())
        .collect();
    for col in &archivo.encabezados {
        match campo_para_columna(&campos, col) {
            Some(campo) => {
                reconocidas.push(col.clone());
                if es_llave_elegible(campo) {
                    sugeridas.push(col.clone());
                }
            }
            None => no_reconocidas.push(col.clone()),
        }
    }

    let formato = match archivo.formato {
        FormatoArchivo::Csv => "csv",
        FormatoArchivo::Xlsx => "xlsx",
    };
    // Sin llave elegida todavía: el diálogo descarta la columna que elija como llave.
    let columnas_extra =
        detectar_extras(&archivo.encabezados, &archivo.filas, &campos, None, archivo.separador);

    Ok(InspeccionImport {
        columnas: archivo.encabezados,
        total_filas: archivo.filas.len() as u64,
        encoding: archivo.encoding,
        formato: formato.into(),
        columnas_reconocidas: reconocidas,
        columnas_no_reconocidas: no_reconocidas,
        campos_llave_sugeridos: sugeridas,
        campos_llave_album,
        columnas_extra,
        huella: huella_archivo(ruta)?,
    })
}

/// ¿El campo puede servir de llave? Principal y no calculado/multidato.
fn es_llave_elegible(campo: &CampoDef) -> bool {
    campo.tabla == Tabla::Principal
        && !matches!(campo.tipo, TipoCampo::Calculado | TipoCampo::Multidato)
}

/// Columnas extra del archivo: encabezado no vacío y no repetido, distinto de la
/// columna llave y sin campo de ese nombre en NINGUNA tabla del álbum (una
/// columna homónima de un campo de variantes se ignora: no se duplica el nombre).
/// El tipo del campo a crear se infiere de sus valores.
fn detectar_extras(
    encabezados: &[String],
    filas: &[Vec<String>],
    campos: &[CampoDef],
    indice_llave: Option<usize>,
    separador: u8,
) -> Vec<ColumnaExtra> {
    let mut vistos: HashSet<String> = HashSet::new();
    let mut extras: Vec<ColumnaExtra> = Vec::new();
    for (i, col) in encabezados.iter().enumerate() {
        let nombre = col.trim();
        if nombre.is_empty() || !vistos.insert(nombre.to_lowercase()) || indice_llave == Some(i) {
            continue;
        }
        if campos
            .iter()
            .any(|c| c.nombre.trim().eq_ignore_ascii_case(nombre))
        {
            continue;
        }
        let valores = filas.iter().map(|f| f.get(i).map(String::as_str).unwrap_or(""));
        let (tipo, decimales) = inferir_tipo(valores, separador);
        extras.push(ColumnaExtra {
            columna: nombre.to_string(),
            tipo,
            decimales,
        });
    }
    extras
}

/// Infiere el tipo del campo a crear para una columna extra: numérico si TODOS
/// los valores no vacíos parsean como número (0 decimales si todos son enteros,
/// 2 si no), fecha si todos parsean como fecha, texto en cualquier otro caso o si
/// la columna está vacía. Un valor con cero a la izquierda (`"007"`) fuerza
/// texto: es un código y como número perdería los ceros.
fn inferir_tipo<'a>(valores: impl Iterator<Item = &'a str>, separador: u8) -> (TipoCampo, u8) {
    let (mut hay_valores, mut numerico, mut enteros, mut fecha) = (false, true, true, true);
    for valor in valores {
        let t = valor.trim();
        if t.is_empty() {
            continue;
        }
        hay_valores = true;
        if numerico {
            let cero_izquierda =
                t.len() > 1 && t.starts_with('0') && t.as_bytes()[1].is_ascii_digit();
            match parsear_numero(t, separador) {
                Some(n) if n.is_finite() && !cero_izquierda => enteros &= n.fract() == 0.0,
                _ => numerico = false,
            }
        }
        if fecha && parsear_fecha(t).is_none() {
            fecha = false;
        }
        if !numerico && !fecha {
            break;
        }
    }
    match (hay_valores, numerico, fecha) {
        (true, true, _) => (TipoCampo::Numerico, if enteros { 0 } else { 2 }),
        (true, false, true) => (TipoCampo::Fecha, 0),
        _ => (TipoCampo::Texto, 0),
    }
}

/// Huella ligera del archivo: `len:mtime` (segundos epoch). Detecta cambios
/// entre el resumen previo y la aplicación sin leer todo el contenido.
fn huella_archivo(ruta: &str) -> Result<String, MicError> {
    let meta = std::fs::metadata(ruta)
        .map_err(|e| MicError::Io(format!("no se pudo leer el archivo '{ruta}': {e}")))?;
    let len = meta.len();
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Ok(format!("{len}:{mtime}"))
}

// ---------------------------------------------------------------------------
// Normalización de valores por tipo de campo
// ---------------------------------------------------------------------------

/// Resultado de normalizar una celda de texto contra el tipo de su campo.
enum CeldaNorm {
    /// La celda está vacía: el campo se OMITE (no se escribe).
    Omitir,
    /// Valor escalar normalizado (texto/numérico/fecha → `Valor`).
    Escalar(Valor),
    /// Valores de un campo multidato (ya partidos, trim, sin vacíos).
    Multi(Vec<String>),
}

/// Normaliza el texto de una celda según el tipo del campo destino.
///
/// - Vacío / solo espacios → `Omitir` (el export no distingue null de `""`, así
///   que tratar `""` como "borrar" destruiría datos en round-trip).
/// - `Texto` → tal cual (solo se recortan CR/LF de cola).
/// - `Numerico`/`Moneda` → limpia `$ % espacios NBSP` y separador de miles según
///   el `separador` del CSV; error si no parsea a `f64`.
/// - `Fecha` → acepta ISO y dd/MM/yyyy / dd-MM-yyyy → SIEMPRE ISO `YYYY-MM-DD`.
/// - `Multidato` → `split('|')` con trim, descartando vacíos.
/// - `Calculado` → nunca se importa (`Omitir`; el motor recalcula).
fn normalizar_valor(texto: &str, campo: &CampoDef, separador: u8) -> Result<CeldaNorm, String> {
    let limpio = texto.trim_matches(|c| c == '\r' || c == '\n');

    if matches!(campo.tipo, TipoCampo::Calculado) {
        return Ok(CeldaNorm::Omitir);
    }

    if matches!(campo.tipo, TipoCampo::Multidato) {
        let valores: Vec<String> = limpio
            .split('|')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        // Celda vacía → omitir, igual que los escalares.
        if valores.is_empty() {
            return Ok(CeldaNorm::Omitir);
        }
        return Ok(CeldaNorm::Multi(valores));
    }

    if limpio.trim().is_empty() {
        return Ok(CeldaNorm::Omitir);
    }

    match campo.tipo {
        // Trim de orillas: evita que `" A-1 "` deje de casar con `"A-1"` (riesgo 2.4).
        TipoCampo::Texto => Ok(CeldaNorm::Escalar(Valor::Texto(limpio.trim().to_string()))),
        TipoCampo::Numerico | TipoCampo::Moneda => {
            let n = parsear_numero(limpio, separador).ok_or_else(|| {
                format!("'{}' no es un número válido para '{}'", limpio, campo.nombre)
            })?;
            Ok(CeldaNorm::Escalar(Valor::Numero(n)))
        }
        TipoCampo::Fecha => {
            let iso = parsear_fecha(limpio).ok_or_else(|| {
                format!(
                    "fecha inválida '{}' en '{}' (use AAAA-MM-DD o dd/MM/aaaa)",
                    limpio, campo.nombre
                )
            })?;
            Ok(CeldaNorm::Escalar(Valor::Texto(iso)))
        }
        // Calculado/Multidato ya se gestionaron arriba.
        TipoCampo::Calculado | TipoCampo::Multidato => Ok(CeldaNorm::Omitir),
    }
}

/// Parsea un número es-MX/en-US a `f64`, limpiando símbolos y separador de miles.
///
/// - Quita `$ € % ` espacios y NBSP (`\u{00A0}`).
/// - Si el CSV usó `;`, el decimal suele ser `,` y los miles `.`: quita `.`,
///   cambia `,`→`.`. Si usó `,` (formato propio/en-US), quita las `,` de miles.
/// - Casos sueltos: una sola `,` sin `.` se trata como decimal (`12,5`→`12.5`).
fn parsear_numero(texto: &str, separador: u8) -> Option<f64> {
    let mut s: String = texto
        .chars()
        .filter(|c| !matches!(c, '$' | '€' | '%' | ' ' | '\u{00A0}'))
        .collect();
    if s.is_empty() {
        return None;
    }

    let tiene_punto = s.contains('.');
    let tiene_coma = s.contains(',');

    if tiene_punto && tiene_coma {
        if separador == b';' {
            // es-MX: `.`=miles, `,`=decimal.
            s = s.replace('.', "").replace(',', ".");
        } else {
            // en-US / propio: `,`=miles, `.`=decimal.
            s = s.replace(',', "");
        }
    } else if tiene_coma {
        // Solo comas: si hay varias son miles; si hay una, es decimal.
        if s.matches(',').count() > 1 {
            s = s.replace(',', "");
        } else if separador == b';' {
            s = s.replace(',', ".");
        } else {
            // Coma única con separador coma: probablemente decimal es-MX en xlsx/copia.
            s = s.replace(',', ".");
        }
    }

    s.trim().parse::<f64>().ok()
}

/// Parsea una fecha a ISO `YYYY-MM-DD`. Acepta ISO (con o sin hora) y los
/// formatos es-MX `dd/MM/yyyy` y `dd-MM-yyyy`.
fn parsear_fecha(texto: &str) -> Option<String> {
    let t = texto.trim();
    // ISO `YYYY-MM-DD` (posiblemente con hora): valida y normaliza a 10 chars.
    if let Some(iso) = t.get(..10) {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(iso, "%Y-%m-%d") {
            return Some(d.format("%Y-%m-%d").to_string());
        }
    }
    for fmt in ["%d/%m/%Y", "%d-%m-%Y", "%d/%m/%y", "%d-%m-%y"] {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(t, fmt) {
            return Some(d.format("%Y-%m-%d").to_string());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Clave canónica de la llave
// ---------------------------------------------------------------------------

/// Forma canónica de una clave de llave para comparar el archivo con el álbum.
///
/// `trim`; si el campo llave es numérico/moneda, se formatea sin `.0` (regla de
/// `exportar::texto_valor`) para que `"1"` (archivo) case con `1.0` (REAL en DB).
/// Devuelve `None` si la clave queda vacía.
fn clave_canonica(texto: &str, llave_numerica: bool) -> Option<String> {
    let t = texto.trim();
    if t.is_empty() {
        return None;
    }
    if llave_numerica {
        if let Ok(n) = t.parse::<f64>() {
            if n.fract() == 0.0 && n.is_finite() {
                return Some(format!("{}", n as i64));
            }
            return Some(n.to_string());
        }
    }
    Some(t.to_string())
}

/// Clave canónica del valor ACTUAL de un registro (lado del álbum).
fn clave_canonica_valor(v: &Valor, llave_numerica: bool) -> Option<String> {
    if v.es_nulo() {
        return None;
    }
    if llave_numerica {
        if let Some(n) = v.como_f64() {
            if n.fract() == 0.0 && n.is_finite() {
                return Some(format!("{}", n as i64));
            }
            return Some(n.to_string());
        }
        return None;
    }
    let txt = v.como_texto();
    let txt = txt.trim();
    if txt.is_empty() {
        None
    } else {
        Some(txt.to_string())
    }
}

// ---------------------------------------------------------------------------
// Núcleo testeable (sin Tauri)
// ---------------------------------------------------------------------------

/// Una fila del archivo ya casada con los campos del álbum y normalizada.
struct FilaCasada {
    /// Número de fila en el archivo (base 1 contando el encabezado).
    fila: u64,
    /// Clave canónica de la llave (la fila siempre tiene una).
    clave: String,
    /// Campos escalares a escribir (excluye la llave y los calculados).
    escalares: Valores,
    /// Multidatos a escribir, por nombre de campo.
    multidatos: HashMap<String, Vec<String>>,
}

/// Resultado de preparar las filas del archivo antes de aplicar.
struct PreparacionImport {
    /// Filas únicas por llave (primera fila gana en duplicados internos).
    filas: Vec<FilaCasada>,
    /// Campos escalares del álbum que se escribirán (reconocidos, ≠ llave/calc).
    campos_escritura: Vec<CampoDef>,
    /// Campos multidato del álbum presentes en el archivo.
    campos_multidato: Vec<CampoDef>,
    /// Filas con la llave vacía (omitidas).
    llaves_vacias: ListaFilas,
    /// Filas con llave repetida dentro del archivo (omitidas; gana la primera).
    llaves_duplicadas: ListaFilas,
    /// Avisos globales acumulados.
    avisos: Vec<String>,
    /// Errores por celda/fila acumulados.
    errores: Vec<String>,
}

/// Busca el campo llave: debe existir, ser principal y no calculado/multidato.
fn resolver_campo_llave<'a>(
    campos: &'a [CampoDef],
    campo_llave: &str,
) -> Result<&'a CampoDef, MicError> {
    let campo = campos
        .iter()
        .find(|c| c.tabla == Tabla::Principal && c.nombre.eq_ignore_ascii_case(campo_llave.trim()))
        .ok_or_else(|| {
            MicError::Invalido(format!(
                "el campo llave '{campo_llave}' no existe en el álbum"
            ))
        })?;
    if !es_llave_elegible(campo) {
        return Err(MicError::Invalido(format!(
            "el campo llave '{}' no puede ser calculado ni multidato",
            campo.nombre
        )));
    }
    Ok(campo)
}

/// Índice de la columna llave en el archivo (case-insensitive, trim). Error si
/// falta o si aparece repetida (no se adivina cuál usar).
fn indice_columna_llave(encabezados: &[String], columna: &str) -> Result<usize, MicError> {
    let mut coincidencias = encabezados
        .iter()
        .enumerate()
        .filter(|(_, c)| c.trim().eq_ignore_ascii_case(columna.trim()))
        .map(|(i, _)| i);
    let indice = coincidencias.next().ok_or_else(|| {
        MicError::Invalido(format!(
            "la columna llave '{columna}' no está presente en el archivo"
        ))
    })?;
    if coincidencias.next().is_some() {
        return Err(MicError::Invalido(format!(
            "la columna llave '{columna}' aparece duplicada en el archivo"
        )));
    }
    Ok(indice)
}

/// Prepara las filas del archivo: valida la llave, casa columnas, normaliza
/// valores, detecta llaves vacías/repetidas y construye el índice por llave.
fn preparar_importacion(
    encabezados: &[String],
    filas: &[Vec<String>],
    campos: &[CampoDef],
    campo_llave: &str,
    columna_llave: &str,
    separador: u8,
) -> Result<PreparacionImport, MicError> {
    let campo_llave_def = resolver_campo_llave(campos, campo_llave)?;
    let llave_numerica = matches!(
        campo_llave_def.tipo,
        TipoCampo::Numerico | TipoCampo::Moneda
    );
    let nombre_llave = campo_llave_def.nombre.clone();
    let indice_llave = indice_columna_llave(encabezados, columna_llave)?;

    // Índice columna → campo del álbum (case-insensitive). Detecta duplicados.
    let mut avisos: Vec<String> = Vec::new();
    let mut col_a_campo: Vec<Option<CampoDef>> = Vec::with_capacity(encabezados.len());
    let mut vistos: HashSet<String> = HashSet::new();
    let mut calculados_ignorados = false;

    for (i, col) in encabezados.iter().enumerate() {
        if !vistos.insert(col.trim().to_lowercase()) {
            avisos.push(format!(
                "columna '{}' duplicada en el archivo: se ignoran las repetidas",
                col
            ));
            col_a_campo.push(None);
            continue;
        }
        if i == indice_llave {
            // La llave casa, nunca se escribe.
            col_a_campo.push(None);
            continue;
        }
        match campo_para_columna(campos, col) {
            Some(campo) if campo.nombre == nombre_llave => {
                // Columna homónima del campo llave, pero la llave sale de otra.
                avisos.push(format!(
                    "la columna '{col}' corresponde al campo llave y se ignora (la llave se toma de '{}')",
                    encabezados[indice_llave]
                ));
                col_a_campo.push(None);
            }
            Some(campo) if matches!(campo.tipo, TipoCampo::Calculado) => {
                calculados_ignorados = true;
                col_a_campo.push(None);
            }
            Some(campo) => col_a_campo.push(Some(campo.clone())),
            None => col_a_campo.push(None),
        }
    }

    // Columnas no reconocidas (sin contar la llave) → aviso único.
    let no_reconocidas: Vec<&str> = encabezados
        .iter()
        .enumerate()
        .filter(|(i, col)| *i != indice_llave && campo_para_columna(campos, col).is_none())
        .map(|(_, col)| col.as_str())
        .collect();
    if !no_reconocidas.is_empty() {
        avisos.push(format!(
            "columnas ignoradas (no coinciden con ningún campo): {}",
            no_reconocidas.join(", ")
        ));
    }
    if calculados_ignorados {
        avisos.push("columnas calculadas ignoradas: se recalculan automáticamente".into());
    }

    // Campos a escribir (escalares ≠ llave/calc) y multidatos presentes.
    let mut campos_escritura: Vec<CampoDef> = Vec::new();
    let mut campos_multidato: Vec<CampoDef> = Vec::new();
    for campo in col_a_campo.iter().flatten() {
        let destino = if matches!(campo.tipo, TipoCampo::Multidato) {
            &mut campos_multidato
        } else {
            &mut campos_escritura
        };
        if !destino.iter().any(|c| c.id == campo.id) {
            destino.push(campo.clone());
        }
    }

    // Recorre las filas, normaliza y construye el índice (primera fila gana).
    let mut filas_casadas: Vec<FilaCasada> = Vec::new();
    let mut claves_vistas: HashSet<String> = HashSet::new();
    let mut llaves_vacias = ListaFilas::default();
    let mut llaves_duplicadas = ListaFilas::default();
    let mut errores: Vec<String> = Vec::new();

    for (idx_fila, fila) in filas.iter().enumerate() {
        let num_fila = idx_fila as u64 + 2; // +1 por encabezado, +1 base-1 (como Excel).
        let clave_bruta = fila.get(indice_llave).map(|s| s.as_str()).unwrap_or("");
        let clave = match clave_canonica(clave_bruta, llave_numerica) {
            Some(c) => c,
            None => {
                llaves_vacias.agregar(num_fila, "");
                continue;
            }
        };
        if !claves_vistas.insert(clave.clone()) {
            llaves_duplicadas.agregar(num_fila, &clave);
            continue;
        }

        let mut escalares: Valores = HashMap::new();
        let mut multidatos: HashMap<String, Vec<String>> = HashMap::new();
        for (i, campo) in col_a_campo.iter().enumerate() {
            let Some(campo) = campo else { continue };
            let celda = fila.get(i).map(|s| s.as_str()).unwrap_or("");
            match normalizar_valor(celda, campo, separador) {
                Ok(CeldaNorm::Omitir) => {}
                Ok(CeldaNorm::Escalar(v)) => {
                    escalares.insert(campo.nombre.clone(), v);
                }
                Ok(CeldaNorm::Multi(vals)) => {
                    multidatos.insert(campo.nombre.clone(), vals);
                }
                Err(msg) => errores.push(format!("fila {num_fila}: {msg}")),
            }
        }

        filas_casadas.push(FilaCasada {
            fila: num_fila,
            clave,
            escalares,
            multidatos,
        });
    }

    Ok(PreparacionImport {
        filas: filas_casadas,
        campos_escritura,
        campos_multidato,
        llaves_vacias,
        llaves_duplicadas,
        avisos,
        errores,
    })
}

/// Política de conflicto al coincidir la llave.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Politica {
    Sustituir,
    Mantener,
    RellenarVacios,
}

impl Politica {
    fn desde(texto: &str) -> Result<Self, MicError> {
        match texto {
            "sustituir" => Ok(Self::Sustituir),
            "mantener" => Ok(Self::Mantener),
            "rellenar_vacios" => Ok(Self::RellenarVacios),
            otra => Err(MicError::Invalido(format!(
                "política de importación no soportada: '{otra}'"
            ))),
        }
    }
}

/// ¿El valor ACTUAL cuenta como "vacío" para `rellenar_vacios`, según el tipo?
///
/// - texto/fecha: NULL o cadena trim-vacía.
/// - numérico/moneda: SOLO NULL (0 es un valor legítimo).
/// - multidato: sin valores (conteo 0).
fn esta_vacio_actual(v: Option<&Valor>, tipo: TipoCampo) -> bool {
    match tipo {
        TipoCampo::Numerico | TipoCampo::Moneda | TipoCampo::Calculado => {
            matches!(v, None | Some(Valor::Nulo(_)))
        }
        TipoCampo::Multidato => {
            // El valor escalar del multidato es el conteo.
            match v {
                None | Some(Valor::Nulo(_)) => true,
                Some(otro) => otro.como_f64().unwrap_or(0.0) == 0.0,
            }
        }
        _ => match v {
            None | Some(Valor::Nulo(_)) => true,
            Some(otro) => otro.como_texto().trim().is_empty(),
        },
    }
}

/// Opciones de una importación (los parámetros del comando).
#[derive(Debug, Clone)]
struct OpcionesImport {
    /// Campo del álbum que hace de llave.
    campo_llave: String,
    /// Columna del archivo con la llave; `None`/vacía → la homónima del campo.
    columna_llave: Option<String>,
    /// `"sustituir"` | `"mantener"` | `"rellenar_vacios"`.
    politica: String,
    /// Dar de alta las filas cuya llave no casa (si no, se omiten).
    crear_faltantes: bool,
    /// Crear campos para las columnas extra e importar sus valores.
    importar_extras: bool,
    /// Solo analizar y contar, sin escribir.
    dry_run: bool,
}

impl OpcionesImport {
    /// Columna del archivo que contiene la llave.
    fn columna_llave(&self) -> &str {
        self.columna_llave
            .as_deref()
            .filter(|c| !c.trim().is_empty())
            .unwrap_or(&self.campo_llave)
    }
}

/// Campo provisional para una columna extra en el dry-run: permite clasificar
/// sus valores como si el campo ya existiera, sin tocar la estructura. Nunca
/// llega a la DB (en dry-run no se llama a `editar`/`crear`).
fn campo_provisional(extra: &ColumnaExtra, id: i64) -> CampoDef {
    CampoDef {
        id,
        nombre: extra.columna.clone(),
        col_fisica: String::new(),
        tabla: Tabla::Principal,
        tipo: extra.tipo,
        decimales: extra.decimales,
        totalizable: false,
        formula: None,
        visible: true,
        modificable: true,
        orden_visible: 0,
        formato: None,
    }
}

/// Núcleo PURO de la importación (sin Tauri): clasifica/aplica cada fila contra
/// el álbum según la política. Con `dry_run=true` no llama a `editar`/`crear`.
///
/// Las columnas extra deben existir ya como campos al aplicar (las crea
/// [`importar_en_album`]); en dry-run con `importar_extras` se simulan con
/// [`campo_provisional`] para que el resumen cuente sus valores.
///
/// `progreso` es un callback opcional `(hechas, total)` para emitir eventos; en
/// los tests se pasa `None`. La atomicidad es por registro (igual que `ligados`
/// y `editar_lote`): deuda técnica anotada (variantes `*_en_tx` para una sola
/// transacción) en el diseño.
fn aplicar_importacion(
    conn: &mut mic_db::pool::Conn,
    campos: &[CampoDef],
    motor: Option<&MotorCalculo>,
    dir_imagenes: Option<&std::path::Path>,
    opciones: &OpcionesImport,
    archivo: &ArchivoLeido,
    mut progreso: Option<&mut dyn FnMut(u64, u64)>,
) -> Result<ResultadoImportacion, MicError> {
    let politica = Politica::desde(&opciones.politica)?;
    let campo_llave_def = resolver_campo_llave(campos, &opciones.campo_llave)?;
    let nombre_llave = campo_llave_def.nombre.clone();
    let llave_numerica = matches!(
        campo_llave_def.tipo,
        TipoCampo::Numerico | TipoCampo::Moneda
    );
    let indice_llave = indice_columna_llave(&archivo.encabezados, opciones.columna_llave())?;
    let columnas_extra = detectar_extras(
        &archivo.encabezados,
        &archivo.filas,
        campos,
        Some(indice_llave),
        archivo.separador,
    );

    let mut campos_prep = campos.to_vec();
    let mut campos_creados: Vec<String> = Vec::new();
    if opciones.importar_extras && opciones.dry_run {
        for (i, extra) in columnas_extra.iter().enumerate() {
            campos_prep.push(campo_provisional(extra, -(i as i64) - 1));
            campos_creados.push(extra.columna.clone());
        }
    }
    let prep = preparar_importacion(
        &archivo.encabezados,
        &archivo.filas,
        &campos_prep,
        &opciones.campo_llave,
        opciones.columna_llave(),
        archivo.separador,
    )?;

    let mut resultado = ResultadoImportacion {
        errores: prep.errores,
        avisos: prep.avisos,
        llaves_vacias: prep.llaves_vacias,
        llaves_duplicadas: prep.llaves_duplicadas,
        columnas_extra,
        campos_creados,
        dry_run: opciones.dry_run,
        ..Default::default()
    };

    // Índice en memoria del álbum: clave canónica → id (primera/más antigua gana).
    let indice_album =
        indexar_album(conn, campos, &nombre_llave, llave_numerica, &mut resultado.avisos)?;

    let total = prep.filas.len() as u64;
    let mut hechas = 0u64;
    if let Some(cb) = progreso.as_deref_mut() {
        cb(hechas, total);
    }

    for fila in &prep.filas {
        match indice_album.get(&fila.clave) {
            Some(&id) => {
                resultado.coincidentes += 1;
                aplicar_existente(
                    conn,
                    campos,
                    motor,
                    id,
                    fila,
                    &prep.campos_escritura,
                    &prep.campos_multidato,
                    politica,
                    opciones.dry_run,
                    &mut resultado,
                )?;
            }
            None => {
                resultado.no_coincidentes.agregar(fila.fila, &fila.clave);
                if !opciones.crear_faltantes {
                    resultado.omitidos += 1;
                } else {
                    if !opciones.dry_run {
                        crear_registro(
                            conn,
                            campos,
                            motor,
                            dir_imagenes,
                            &nombre_llave,
                            llave_numerica,
                            fila,
                            &prep.campos_escritura,
                            &prep.campos_multidato,
                        )?;
                    }
                    resultado.creados += 1;
                }
            }
        }

        hechas += 1;
        if hechas.is_multiple_of(PASO_PROGRESO) {
            if let Some(cb) = progreso.as_deref_mut() {
                cb(hechas, total);
            }
        }
    }

    if let Some(cb) = progreso {
        cb(total, total);
    }
    Ok(resultado)
}

/// Construye `clave_canonica → id` con UNA query, indexando todos los registros
/// (primera/más antigua gana por `ORDER BY _id_`). Duplicados en el álbum → aviso.
fn indexar_album(
    conn: &mic_db::pool::Conn,
    campos: &[CampoDef],
    nombre_llave: &str,
    llave_numerica: bool,
    avisos: &mut Vec<String>,
) -> Result<HashMap<String, i64>, MicError> {
    let campo = campos
        .iter()
        .find(|c| c.tabla == Tabla::Principal && c.nombre == nombre_llave)
        .ok_or_else(|| {
            MicError::Invalido(format!("campo llave '{nombre_llave}' no encontrado"))
        })?;

    let sql = format!(
        "SELECT _id_, {} FROM principal ORDER BY _id_",
        campo.col_fisica
    );
    let mut stmt = conn.prepare(&sql).map_err(err_sql)?;
    let filas = stmt
        .query_map([], |row| {
            let id: i64 = row.get(0)?;
            let vr = row.get_ref(1)?;
            Ok((id, valor_de_sql(vr)))
        })
        .map_err(err_sql)?;

    let mut indice: HashMap<String, i64> = HashMap::new();
    let mut duplicadas: std::collections::HashSet<String> = std::collections::HashSet::new();
    for fila in filas {
        let (id, valor) = fila.map_err(err_sql)?;
        if let Some(clave) = clave_canonica_valor(&valor, llave_numerica) {
            match indice.entry(clave) {
                std::collections::hash_map::Entry::Occupied(e) => {
                    // Primera/más antigua gana (ORDER BY _id_); avisamos del duplicado.
                    duplicadas.insert(e.key().clone());
                }
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(id);
                }
            }
        }
    }
    for clave in duplicadas {
        avisos.push(format!(
            "la llave '{clave}' aparece varias veces en el álbum; solo se actualizó una"
        ));
    }
    Ok(indice)
}

/// Convierte un `ValueRef` de SQLite a `Valor` (sin conocer el tipo de campo;
/// suficiente para indexar la llave).
fn valor_de_sql(vr: rusqlite::types::ValueRef) -> Valor {
    use rusqlite::types::ValueRef;
    match vr {
        ValueRef::Null => Valor::Nulo(None),
        ValueRef::Integer(i) => Valor::Entero(i),
        ValueRef::Real(r) => Valor::Numero(r),
        ValueRef::Text(t) => Valor::Texto(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(_) => Valor::Nulo(None),
    }
}

/// Aplica una fila a un registro existente según la política (merge sobre el
/// registro actual completo; jamás `Valores` parcial a `editar`).
#[allow(clippy::too_many_arguments)]
fn aplicar_existente(
    conn: &mut mic_db::pool::Conn,
    campos: &[CampoDef],
    motor: Option<&MotorCalculo>,
    id: i64,
    fila: &FilaCasada,
    campos_escritura: &[CampoDef],
    campos_multidato: &[CampoDef],
    politica: Politica,
    dry_run: bool,
    resultado: &mut ResultadoImportacion,
) -> Result<(), MicError> {
    if politica == Politica::Mantener {
        resultado.sin_cambio += 1;
        return Ok(());
    }

    // Registro actual COMPLETO (base del merge; nunca un mapa parcial).
    let actual = mic_db::repo_registros::obtener(conn, campos, Tabla::Principal, id)?;
    let mut nuevos = actual.valores.clone();
    let mut cambio_escalar = false;

    // Campos escalares.
    for campo in campos_escritura {
        let valor_archivo = match fila.escalares.get(&campo.nombre) {
            Some(v) => v,
            None => continue, // la fila no trae este campo (celda omitida/ausente).
        };
        let permitir = match politica {
            Politica::Sustituir => true,
            Politica::RellenarVacios => {
                esta_vacio_actual(actual.valores.get(&campo.nombre), campo.tipo)
            }
            Politica::Mantener => false,
        };
        if !permitir {
            continue;
        }
        let actual_val = actual.valores.get(&campo.nombre);
        if actual_val != Some(valor_archivo) {
            nuevos.insert(campo.nombre.clone(), valor_archivo.clone());
            cambio_escalar = true;
        }
    }

    // Multidatos: solo se escriben si la política los toca.
    let mut multidatos_a_escribir: HashMap<String, Vec<String>> = HashMap::new();
    for campo in campos_multidato {
        let valores_archivo = match fila.multidatos.get(&campo.nombre) {
            Some(v) if !v.is_empty() => v,
            _ => continue,
        };
        let actuales = actual.multidatos.get(&campo.nombre).cloned().unwrap_or_default();
        let permitir = match politica {
            Politica::Sustituir => true,
            Politica::RellenarVacios => actuales.is_empty(),
            Politica::Mantener => false,
        };
        if !permitir {
            continue;
        }
        if &actuales != valores_archivo {
            multidatos_a_escribir.insert(campo.nombre.clone(), valores_archivo.clone());
        }
    }

    let hay_cambio = cambio_escalar || !multidatos_a_escribir.is_empty();
    if !hay_cambio {
        resultado.sin_cambio += 1;
        return Ok(());
    }

    if !dry_run {
        let multi = if multidatos_a_escribir.is_empty() {
            None
        } else {
            Some(&multidatos_a_escribir)
        };
        mic_db::repo_registros::editar(
            conn,
            campos,
            motor,
            Tabla::Principal,
            id,
            &nuevos,
            multi,
        )?;
    }
    resultado.actualizados += 1;
    Ok(())
}

/// Da de alta un registro nuevo con la llave + los campos del archivo (sin imagen).
#[allow(clippy::too_many_arguments)]
fn crear_registro(
    conn: &mut mic_db::pool::Conn,
    campos: &[CampoDef],
    motor: Option<&MotorCalculo>,
    dir_imagenes: Option<&std::path::Path>,
    nombre_llave: &str,
    llave_numerica: bool,
    fila: &FilaCasada,
    campos_escritura: &[CampoDef],
    campos_multidato: &[CampoDef],
) -> Result<(), MicError> {
    let mut nuevos: Valores = HashMap::new();
    // La llave canónica de una columna numérica se guarda como número, no como
    // texto: no depender de la coerción texto→f64 de `valor_a_sql`.
    let valor_llave = match fila.clave.parse::<f64>() {
        Ok(n) if llave_numerica => Valor::Numero(n),
        _ => Valor::Texto(fila.clave.clone()),
    };
    nuevos.insert(nombre_llave.to_string(), valor_llave);
    for campo in campos_escritura {
        if let Some(v) = fila.escalares.get(&campo.nombre) {
            nuevos.insert(campo.nombre.clone(), v.clone());
        }
    }

    let mut multidatos: HashMap<String, Vec<String>> = HashMap::new();
    for campo in campos_multidato {
        if let Some(v) = fila.multidatos.get(&campo.nombre) {
            if !v.is_empty() {
                multidatos.insert(campo.nombre.clone(), v.clone());
            }
        }
    }

    mic_db::repo_registros::crear(
        conn,
        campos,
        motor,
        Tabla::Principal,
        &nuevos,
        &multidatos,
        None, // sin imagen: la grilla muestra placeholder
        None,
        dir_imagenes,
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Orquestación del comando (lectura + huella + progreso)
// ---------------------------------------------------------------------------

/// Orquesta la importación: verifica la huella opcional, lee el archivo e
/// importa emitiendo eventos de progreso.
fn importar(
    app: &AppHandle,
    h: &AlbumHandle,
    ruta: &str,
    opciones: &OpcionesImport,
    huella: Option<&str>,
) -> Result<ResultadoImportacion, MicError> {
    // La huella de la inspección detecta si el archivo cambió entre el resumen
    // previo y la aplicación: en ese caso el resumen ya no es fiable.
    if let Some(esperada) = huella {
        let actual = huella_archivo(ruta)?;
        if actual != esperada {
            return Err(MicError::Invalido(
                "el archivo cambió desde el análisis; vuelve a ver el resumen".into(),
            ));
        }
    }
    let archivo = leer_archivo(ruta)?;

    let fase = if opciones.dry_run { "analizando" } else { "aplicando" };
    let mut cb = |hechas: u64, total: u64| {
        emitir_progreso(app, fase, hechas, total);
    };
    importar_en_album(h, &archivo, opciones, Some(&mut cb))
}

/// Importa en un álbum abierto. Con `importar_extras` (y sin dry-run) primero
/// crea los campos de las columnas extra y REFRESCA campos + motor del handle
/// (como `campo_crear`); solo después aplica, para que esas columnas casen por
/// nombre y los calculados se evalúen con la estructura vigente.
fn importar_en_album(
    h: &AlbumHandle,
    archivo: &ArchivoLeido,
    opciones: &OpcionesImport,
    progreso: Option<&mut dyn FnMut(u64, u64)>,
) -> Result<ResultadoImportacion, MicError> {
    let crear_extras = opciones.importar_extras && !opciones.dry_run;
    let mut columnas_extra: Vec<ColumnaExtra> = Vec::new();
    let mut campos_creados: Vec<String> = Vec::new();

    if crear_extras {
        let campos = h.campos();
        // Valida antes de tocar la estructura: con una llave o política
        // inválidas no debe quedar ningún campo creado.
        Politica::desde(&opciones.politica)?;
        resolver_campo_llave(&campos, &opciones.campo_llave)?;
        let indice_llave = indice_columna_llave(&archivo.encabezados, opciones.columna_llave())?;
        columnas_extra = detectar_extras(
            &archivo.encabezados,
            &archivo.filas,
            &campos,
            Some(indice_llave),
            archivo.separador,
        );
        if !columnas_extra.is_empty() {
            let creacion = (|| -> Result<(), MicError> {
                let conn = h.db.conn()?;
                for (i, extra) in columnas_extra.iter().enumerate() {
                    let def = CampoNuevo {
                        nombre: extra.columna.clone(),
                        tabla: Tabla::Principal,
                        tipo: extra.tipo,
                        decimales: extra.decimales,
                        totalizable: false,
                        formula: None,
                        visible: true,
                        modificable: true,
                        orden_visible: (campos.len() + i) as i32,
                        formato: None,
                    };
                    mic_db::repo_campos::crear(&conn, &def)?;
                    campos_creados.push(extra.columna.clone());
                }
                Ok(())
            })();
            // Refresca aunque la creación falle a medias: la caché debe reflejar
            // los campos que sí quedaron en la DB.
            h.refrescar_campos()?;
            creacion?;
        }
    }

    let campos = h.campos();
    let mut conn = h.db.conn()?;
    let motor = h.motor.read().unwrap_or_else(|e| e.into_inner());
    let dir_imagenes = h.dir_imagenes();
    let mut resultado = aplicar_importacion(
        &mut conn,
        &campos,
        motor.as_ref(),
        Some(dir_imagenes.as_path()),
        opciones,
        archivo,
        progreso,
    )?;
    if crear_extras {
        resultado.columnas_extra = columnas_extra;
        resultado.campos_creados = campos_creados;
    }
    Ok(resultado)
}

/// Emite el evento de progreso, registrando (sin propagar) cualquier fallo.
fn emitir_progreso(app: &AppHandle, fase: &str, hechas: u64, total: u64) {
    let payload = ProgresoEvento {
        fase: fase.to_string(),
        hechas,
        total,
    };
    if let Err(e) = app.emit(EVENTO_PROGRESO, &payload) {
        tracing::warn!(error = %e, "no se pudo emitir progreso de importación");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mic_core::model::CampoNuevo;
    use mic_db::{repo_campos, repo_registros, AlbumDb};
    use std::collections::HashMap;

    // --- helpers de archivos temporales ------------------------------------

    fn ruta_tmp(nombre: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "mic_imp_{}_{}_{nombre}",
            std::process::id(),
            // contador simple para unicidad entre llamadas del mismo test
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn campo_def(nombre: &str, tipo: TipoCampo) -> CampoDef {
        CampoDef {
            id: 0,
            nombre: nombre.to_string(),
            col_fisica: String::new(),
            tabla: Tabla::Principal,
            tipo,
            decimales: 2,
            totalizable: false,
            formula: None,
            visible: true,
            modificable: true,
            orden_visible: 0,
            formato: None,
        }
    }

    // --- encoding / CSV ----------------------------------------------------

    #[test]
    fn csv_utf8_con_bom_se_lee_sin_bom_en_encabezado() {
        let ruta = ruta_tmp("bom.csv");
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice("Clave,Nombre\r\nA-1,Hola\r\n".as_bytes());
        std::fs::write(&ruta, &bytes).unwrap();

        let (enc, filas, encoding, _sep) = leer_csv(ruta.to_str().unwrap()).unwrap();
        std::fs::remove_file(&ruta).ok();

        assert_eq!(encoding, "utf-8-bom");
        assert_eq!(enc, vec!["Clave", "Nombre"]);
        assert_eq!(filas, vec![vec!["A-1".to_string(), "Hola".to_string()]]);
        // El primer encabezado no arrastra el BOM.
        assert_eq!(enc[0], "Clave");
    }

    #[test]
    fn csv_windows_1252_acentos() {
        let ruta = ruta_tmp("cp1252.csv");
        // "Descripción" con 'ó' = 0xF3 en Windows-1252 (no es UTF-8 válido).
        let mut bytes = b"Clave,Descripci\xF3n\r\nA-1,Jarr\xF3n\r\n".to_vec();
        bytes.push(b'\n');
        std::fs::write(&ruta, &bytes).unwrap();

        let (enc, filas, encoding, _sep) = leer_csv(ruta.to_str().unwrap()).unwrap();
        std::fs::remove_file(&ruta).ok();

        assert_eq!(encoding, "windows-1252");
        assert_eq!(enc, vec!["Clave", "Descripción"]);
        assert_eq!(filas[0][1], "Jarrón");
    }

    #[test]
    fn csv_utf8_sin_bom() {
        let ruta = ruta_tmp("utf8.csv");
        std::fs::write(&ruta, "Clave,Café\r\nA-1,café\r\n".as_bytes()).unwrap();
        let (enc, filas, encoding, _sep) = leer_csv(ruta.to_str().unwrap()).unwrap();
        std::fs::remove_file(&ruta).ok();
        assert_eq!(encoding, "utf-8");
        assert_eq!(enc, vec!["Clave", "Café"]);
        assert_eq!(filas[0][1], "café");
    }

    #[test]
    fn csv_separador_punto_y_coma() {
        let ruta = ruta_tmp("sc.csv");
        std::fs::write(&ruta, "Clave;Nombre;Precio\r\nA-1;Hola;1.234,50\r\n".as_bytes())
            .unwrap();
        let (enc, filas, _, _) = leer_csv(ruta.to_str().unwrap()).unwrap();
        std::fs::remove_file(&ruta).ok();
        assert_eq!(enc, vec!["Clave", "Nombre", "Precio"]);
        assert_eq!(filas[0], vec!["A-1", "Hola", "1.234,50"]);
    }

    #[test]
    fn round_trip_csv_mic() {
        // Genera un csv-mic con el exportador y lo relee.
        let ruta = ruta_tmp("mic.csv");
        let filas = vec![vec!["A-1".to_string(), "Jarrón chino".to_string()]];
        crate::commands::exportar::escribir_csv_mic(
            ruta.to_str().unwrap(),
            &["Clave", "Descripción"],
            &filas,
        )
        .unwrap();

        let (enc, leidas, encoding, _sep) = leer_csv(ruta.to_str().unwrap()).unwrap();
        std::fs::remove_file(&ruta).ok();
        assert_eq!(encoding, "windows-1252");
        assert_eq!(enc, vec!["Clave", "Descripción"]);
        assert_eq!(leidas[0], vec!["A-1", "Jarrón chino"]);
    }

    // --- XLSX --------------------------------------------------------------

    #[test]
    fn xlsx_primera_hoja_enteros_sin_punto_y_fecha_iso() {
        use rust_xlsxwriter::{ExcelDateTime, Workbook};
        let ruta = ruta_tmp("hoja.xlsx");
        let mut libro = Workbook::new();
        let hoja = libro.add_worksheet();
        hoja.write_string(0, 0, "Clave").unwrap();
        hoja.write_string(0, 1, "Cantidad").unwrap();
        hoja.write_string(0, 2, "Fecha").unwrap();
        hoja.write_string(1, 0, "A-1").unwrap();
        hoja.write_number(1, 1, 5.0).unwrap(); // entero exacto
        let fecha = ExcelDateTime::from_ymd(2026, 6, 5).unwrap();
        let fmt = rust_xlsxwriter::Format::new().set_num_format("yyyy-mm-dd");
        hoja.write_datetime_with_format(1, 2, &fecha, &fmt).unwrap();
        libro.save(ruta.to_str().unwrap()).unwrap();

        let (enc, filas) = leer_xlsx(ruta.to_str().unwrap()).unwrap();
        std::fs::remove_file(&ruta).ok();

        assert_eq!(enc, vec!["Clave", "Cantidad", "Fecha"]);
        assert_eq!(filas[0][0], "A-1");
        // 5.0 → "5" sin ".0".
        assert_eq!(filas[0][1], "5");
        // La fecha se lee como ISO.
        assert_eq!(filas[0][2], "2026-06-05");
    }

    // --- normalización -----------------------------------------------------

    #[test]
    fn numero_con_miles_y_simbolos() {
        let num = campo_def("Precio", TipoCampo::Moneda);
        // separador coma (formato propio): "1,234.50" → 1234.5
        match normalizar_valor("$1,234.50", &num, b',').unwrap() {
            CeldaNorm::Escalar(Valor::Numero(n)) => assert!((n - 1234.5).abs() < 1e-9),
            otro => panic!("esperaba número, fue otro: {:?}", matches!(otro, CeldaNorm::Omitir)),
        }
        // separador `;` (es-MX): "1.234,50" → 1234.5
        match normalizar_valor("1.234,50", &num, b';').unwrap() {
            CeldaNorm::Escalar(Valor::Numero(n)) => assert!((n - 1234.5).abs() < 1e-9),
            _ => panic!("esperaba número"),
        }
        // coma única decimal: "12,5" → 12.5
        match normalizar_valor("12,5", &num, b';').unwrap() {
            CeldaNorm::Escalar(Valor::Numero(n)) => assert!((n - 12.5).abs() < 1e-9),
            _ => panic!("esperaba número"),
        }
    }

    #[test]
    fn numero_invalido_es_error() {
        let num = campo_def("Precio", TipoCampo::Numerico);
        assert!(normalizar_valor("no-numero", &num, b',').is_err());
    }

    #[test]
    fn fecha_iso_y_ddmmaaaa() {
        let fecha = campo_def("Vence", TipoCampo::Fecha);
        for entrada in ["2026-06-05", "05/06/2026", "05-06-2026"] {
            match normalizar_valor(entrada, &fecha, b',').unwrap() {
                CeldaNorm::Escalar(Valor::Texto(s)) => assert_eq!(s, "2026-06-05"),
                _ => panic!("esperaba fecha ISO para {entrada}"),
            }
        }
        assert!(normalizar_valor("no-fecha", &fecha, b',').is_err());
    }

    #[test]
    fn multidato_split() {
        let multi = campo_def("Etiquetas", TipoCampo::Multidato);
        match normalizar_valor("a | b|c|", &multi, b',').unwrap() {
            CeldaNorm::Multi(v) => assert_eq!(v, vec!["a", "b", "c"]),
            _ => panic!("esperaba multidato"),
        }
    }

    #[test]
    fn texto_tal_cual_y_vacio_omite() {
        let txt = campo_def("Nombre", TipoCampo::Texto);
        match normalizar_valor("  hola mundo  ", &txt, b',').unwrap() {
            CeldaNorm::Escalar(Valor::Texto(s)) => assert_eq!(s, "hola mundo"),
            _ => panic!("esperaba texto"),
        }
        assert!(matches!(
            normalizar_valor("   ", &txt, b',').unwrap(),
            CeldaNorm::Omitir
        ));
    }

    #[test]
    fn calculado_se_omite() {
        let calc = campo_def("Total", TipoCampo::Calculado);
        assert!(matches!(
            normalizar_valor("999", &calc, b',').unwrap(),
            CeldaNorm::Omitir
        ));
    }

    // --- clave canónica ----------------------------------------------------

    #[test]
    fn llave_numerica_canonica_sin_punto() {
        // "1" y 1.0 deben dar la misma clave canónica.
        assert_eq!(clave_canonica("1", true).unwrap(), "1");
        assert_eq!(
            clave_canonica_valor(&Valor::Numero(1.0), true).unwrap(),
            "1"
        );
        assert_eq!(
            clave_canonica("1", true).unwrap(),
            clave_canonica_valor(&Valor::Numero(1.0), true).unwrap()
        );
    }

    // --- casamiento --------------------------------------------------------

    #[test]
    fn casa_columnas_case_insensitive_y_desordenado() {
        let campos = vec![
            campo_def("Clave", TipoCampo::Texto),
            campo_def("Precio", TipoCampo::Moneda),
        ];
        assert!(campo_para_columna(&campos, "  precio ").is_some());
        assert!(campo_para_columna(&campos, "CLAVE").is_some());
        assert!(campo_para_columna(&campos, "desconocida").is_none());
    }

    // --- integración con álbum temporal ------------------------------------

    /// Álbum temporal con campos texto/numerico/fecha/multidato/calculado.
    fn album_tmp() -> (AlbumDb, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("prueba.micdb");
        let db = AlbumDb::crear(&ruta, "Prueba").unwrap();
        (db, dir)
    }

    fn nuevo(nombre: &str, tipo: TipoCampo) -> CampoNuevo {
        CampoNuevo {
            nombre: nombre.to_string(),
            tabla: Tabla::Principal,
            tipo,
            decimales: 2,
            totalizable: false,
            formula: None,
            visible: true,
            modificable: true,
            orden_visible: 0,
            formato: None,
        }
    }

    /// Crea un álbum estándar de pruebas: Clave/Precio/Cantidad/Total(calc)/Notas/Tags.
    fn album_pruebas() -> (AlbumDb, tempfile::TempDir, Vec<CampoDef>, MotorCalculo) {
        let (db, dir) = album_tmp();
        {
            let conn = db.conn().unwrap();
            repo_campos::crear(&conn, &nuevo("Clave", TipoCampo::Texto)).unwrap();
            repo_campos::crear(&conn, &nuevo("Precio", TipoCampo::Moneda)).unwrap();
            repo_campos::crear(&conn, &nuevo("Cantidad", TipoCampo::Numerico)).unwrap();
            let mut total = nuevo("Total", TipoCampo::Calculado);
            total.formula = Some("Precio * Cantidad".into());
            repo_campos::crear(&conn, &total).unwrap();
            repo_campos::crear(&conn, &nuevo("Notas", TipoCampo::Texto)).unwrap();
            repo_campos::crear(&conn, &nuevo("Tags", TipoCampo::Multidato)).unwrap();
        }
        let conn = db.conn().unwrap();
        let campos = repo_campos::listar(&conn).unwrap();
        let motor = MotorCalculo::new(&campos).unwrap();
        (db, dir, campos, motor)
    }

    fn sembrar(
        db: &AlbumDb,
        campos: &[CampoDef],
        motor: &MotorCalculo,
        clave: &str,
        precio: f64,
        cantidad: f64,
        notas: &str,
    ) -> i64 {
        let mut conn = db.conn().unwrap();
        let mut v: Valores = HashMap::new();
        v.insert("Clave".into(), Valor::Texto(clave.into()));
        v.insert("Precio".into(), Valor::Numero(precio));
        v.insert("Cantidad".into(), Valor::Numero(cantidad));
        if !notas.is_empty() {
            v.insert("Notas".into(), Valor::Texto(notas.into()));
        }
        repo_registros::crear(
            &mut conn,
            campos,
            Some(motor),
            Tabla::Principal,
            &v,
            &HashMap::new(),
            None,
            None,
            None,
        )
        .unwrap()
    }

    fn obtener_por_clave(db: &AlbumDb, campos: &[CampoDef], clave: &str) -> Option<RegistroParaTest> {
        let conn = db.conn().unwrap();
        let req = mic_core::model::QueryReq {
            tabla: Tabla::Principal,
            id_principal: None,
            grupo: None,
            filtro_rapido: None,
            condiciones: vec![],
            busqueda: None,
            orden: vec![],
            incluir_ocultos: true,
            excluidos: Vec::new(),
            offset: 0,
            limit: u32::MAX,
        };
        let pagina = repo_registros::query(&conn, campos, &req).unwrap();
        for reg in pagina.registros {
            if reg.valores.get("Clave").map(|v| v.como_texto()) == Some(clave.to_string()) {
                let completo =
                    repo_registros::obtener(&conn, campos, Tabla::Principal, reg.id).unwrap();
                return Some(RegistroParaTest {
                    valores: completo.valores,
                    multidatos: completo.multidatos,
                });
            }
        }
        None
    }

    struct RegistroParaTest {
        valores: Valores,
        multidatos: HashMap<String, Vec<String>>,
    }

    fn opciones(llave: &str, politica: &str, crear: bool, dry_run: bool) -> OpcionesImport {
        OpcionesImport {
            campo_llave: llave.to_string(),
            columna_llave: None,
            politica: politica.to_string(),
            crear_faltantes: crear,
            importar_extras: false,
            dry_run,
        }
    }

    fn archivo_prueba(enc: &[String], filas: &[Vec<String>]) -> ArchivoLeido {
        ArchivoLeido {
            encabezados: enc.to_vec(),
            filas: filas.to_vec(),
            encoding: "utf-8".into(),
            formato: FormatoArchivo::Csv,
            separador: b',',
        }
    }

    fn aplicar_prueba(
        conn: &mut mic_db::pool::Conn,
        campos: &[CampoDef],
        motor: Option<&MotorCalculo>,
        opciones: OpcionesImport,
        enc: &[String],
        filas: &[Vec<String>],
    ) -> ResultadoImportacion {
        aplicar_importacion(conn, campos, motor, None, &opciones, &archivo_prueba(enc, filas), None)
            .unwrap()
    }

    fn encabezados(cols: &[&str]) -> Vec<String> {
        cols.iter().map(|s| s.to_string()).collect()
    }
    fn fila(cels: &[&str]) -> Vec<String> {
        cels.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn sustituir_sobrescribe_y_recalcula() {
        let (db, _d, campos, motor) = album_pruebas();
        sembrar(&db, &campos, &motor, "A-1", 10.0, 3.0, "viejo");

        let enc = encabezados(&["Clave", "Precio", "Cantidad", "Notas"]);
        let filas = vec![fila(&["A-1", "20", "4", "nuevo"])];

        let mut conn = db.conn().unwrap();
        let r = aplicar_prueba(
            &mut conn,
            &campos,
            Some(&motor),
            opciones("Clave", "sustituir", false, false),
            &enc,
            &filas,
        );
        drop(conn);

        assert_eq!(r.actualizados, 1);
        assert_eq!(r.creados, 0);
        let reg = obtener_por_clave(&db, &campos, "A-1").unwrap();
        assert_eq!(reg.valores.get("Precio").unwrap().como_f64().unwrap(), 20.0);
        assert_eq!(reg.valores.get("Notas").unwrap().como_texto(), "nuevo");
        // Total recalculado = 20 * 4 = 80.
        assert_eq!(reg.valores.get("Total").unwrap().como_f64().unwrap(), 80.0);
    }

    #[test]
    fn mantener_no_toca() {
        let (db, _d, campos, motor) = album_pruebas();
        sembrar(&db, &campos, &motor, "A-1", 10.0, 3.0, "intacto");

        let enc = encabezados(&["Clave", "Precio", "Notas"]);
        let filas = vec![fila(&["A-1", "999", "cambiado"])];

        let mut conn = db.conn().unwrap();
        let r = aplicar_prueba(
            &mut conn,
            &campos,
            Some(&motor),
            opciones("Clave", "mantener", false, false),
            &enc,
            &filas,
        );
        drop(conn);

        assert_eq!(r.sin_cambio, 1);
        assert_eq!(r.actualizados, 0);
        let reg = obtener_por_clave(&db, &campos, "A-1").unwrap();
        assert_eq!(reg.valores.get("Precio").unwrap().como_f64().unwrap(), 10.0);
        assert_eq!(reg.valores.get("Notas").unwrap().como_texto(), "intacto");
    }

    #[test]
    fn rellenar_vacios_solo_vacios() {
        let (db, _d, campos, motor) = album_pruebas();
        // Notas vacío, Precio con valor.
        sembrar(&db, &campos, &motor, "A-1", 10.0, 2.0, "");

        let enc = encabezados(&["Clave", "Precio", "Notas"]);
        // Precio=99 (no debe escribirse: ya tiene valor), Notas="relleno" (sí, está vacío).
        let filas = vec![fila(&["A-1", "99", "relleno"])];

        let mut conn = db.conn().unwrap();
        let r = aplicar_prueba(
            &mut conn,
            &campos,
            Some(&motor),
            opciones("Clave", "rellenar_vacios", false, false),
            &enc,
            &filas,
        );
        drop(conn);

        assert_eq!(r.actualizados, 1);
        let reg = obtener_por_clave(&db, &campos, "A-1").unwrap();
        // Precio NO cambió (ya tenía 10).
        assert_eq!(reg.valores.get("Precio").unwrap().como_f64().unwrap(), 10.0);
        // Notas SÍ se rellenó.
        assert_eq!(reg.valores.get("Notas").unwrap().como_texto(), "relleno");
    }

    #[test]
    fn crear_faltantes_da_de_alta() {
        let (db, _d, campos, motor) = album_pruebas();
        sembrar(&db, &campos, &motor, "A-1", 10.0, 2.0, "");

        let enc = encabezados(&["Clave", "Precio", "Cantidad"]);
        let filas = vec![fila(&["B-2", "5", "3"])];

        let mut conn = db.conn().unwrap();
        let r = aplicar_prueba(
            &mut conn,
            &campos,
            Some(&motor),
            opciones("Clave", "sustituir", true, false),
            &enc,
            &filas,
        );
        drop(conn);

        assert_eq!(r.creados, 1);
        let reg = obtener_por_clave(&db, &campos, "B-2").unwrap();
        assert_eq!(reg.valores.get("Clave").unwrap().como_texto(), "B-2");
        // Total calculado = 5 * 3 = 15.
        assert_eq!(reg.valores.get("Total").unwrap().como_f64().unwrap(), 15.0);
    }

    #[test]
    fn sin_crear_faltantes_omite_y_reporta() {
        let (db, _d, campos, motor) = album_pruebas();
        sembrar(&db, &campos, &motor, "A-1", 10.0, 2.0, "");

        let enc = encabezados(&["Clave", "Precio"]);
        let filas = vec![fila(&["NUEVA", "5"])];

        let mut conn = db.conn().unwrap();
        let r = aplicar_prueba(
            &mut conn,
            &campos,
            Some(&motor),
            opciones("Clave", "sustituir", false, false),
            &enc,
            &filas,
        );
        drop(conn);

        assert_eq!(r.creados, 0);
        assert_eq!(r.omitidos, 1);
        assert_eq!(r.sin_cambio, 0);
        assert_eq!(r.no_coincidentes.total, 1);
        assert_eq!(
            r.no_coincidentes.filas,
            vec![FilaLlave { fila: 2, llave: "NUEVA".into() }]
        );
        assert!(obtener_por_clave(&db, &campos, "NUEVA").is_none());
    }

    #[test]
    fn dry_run_no_escribe_y_conteos_coinciden() {
        let (db, _d, campos, motor) = album_pruebas();
        sembrar(&db, &campos, &motor, "A-1", 10.0, 3.0, "viejo");

        let enc = encabezados(&["Clave", "Precio", "Cantidad", "Notas"]);
        let filas = vec![
            fila(&["A-1", "20", "4", "nuevo"]), // actualiza
            fila(&["B-2", "5", "3", "alta"]),   // crea
        ];

        // Dry-run.
        let mut conn = db.conn().unwrap();
        let dry = aplicar_prueba(
            &mut conn,
            &campos,
            Some(&motor),
            opciones("Clave", "sustituir", true, true),
            &enc,
            &filas,
        );
        drop(conn);
        assert!(dry.dry_run);
        assert_eq!(dry.actualizados, 1);
        assert_eq!(dry.creados, 1);
        // Nada cambió en el álbum.
        let reg = obtener_por_clave(&db, &campos, "A-1").unwrap();
        assert_eq!(reg.valores.get("Precio").unwrap().como_f64().unwrap(), 10.0);
        assert!(obtener_por_clave(&db, &campos, "B-2").is_none());

        // Apply: conteos idénticos.
        let mut conn = db.conn().unwrap();
        let apply = aplicar_prueba(
            &mut conn,
            &campos,
            Some(&motor),
            opciones("Clave", "sustituir", true, false),
            &enc,
            &filas,
        );
        drop(conn);
        assert_eq!(apply.actualizados, dry.actualizados);
        assert_eq!(apply.creados, dry.creados);
        assert_eq!(apply.sin_cambio, dry.sin_cambio);
        // Y el apply sí materializó el registro nuevo.
        assert!(obtener_por_clave(&db, &campos, "B-2").is_some());
    }

    #[test]
    fn errores_por_fila_no_abortan() {
        let (db, _d, campos, motor) = album_pruebas();
        sembrar(&db, &campos, &motor, "A-1", 10.0, 2.0, "");
        sembrar(&db, &campos, &motor, "A-2", 10.0, 2.0, "");

        let enc = encabezados(&["Clave", "Precio", "Notas"]);
        let filas = vec![
            fila(&["A-1", "no-numero", "ok1"]), // Precio inválido → error de celda
            fila(&["A-2", "30", "ok2"]),        // válida
        ];

        let mut conn = db.conn().unwrap();
        let r = aplicar_prueba(
            &mut conn,
            &campos,
            Some(&motor),
            opciones("Clave", "sustituir", false, false),
            &enc,
            &filas,
        );
        drop(conn);

        assert_eq!(r.errores.len(), 1, "debe haber 1 error de celda");
        // A-1: el precio inválido se omitió pero Notas sí se aplicó.
        let a1 = obtener_por_clave(&db, &campos, "A-1").unwrap();
        assert_eq!(a1.valores.get("Notas").unwrap().como_texto(), "ok1");
        assert_eq!(a1.valores.get("Precio").unwrap().como_f64().unwrap(), 10.0);
        // A-2 aplicada por completo.
        let a2 = obtener_por_clave(&db, &campos, "A-2").unwrap();
        assert_eq!(a2.valores.get("Precio").unwrap().como_f64().unwrap(), 30.0);
    }

    #[test]
    fn merge_no_anula_campos_fuera_del_archivo() {
        let (db, _d, campos, motor) = album_pruebas();
        // Registro con 3 campos poblados.
        sembrar(&db, &campos, &motor, "A-1", 10.0, 5.0, "conservar");

        // El archivo solo trae Clave + Precio (2 de 5 columnas).
        let enc = encabezados(&["Clave", "Precio"]);
        let filas = vec![fila(&["A-1", "20"])];

        let mut conn = db.conn().unwrap();
        aplicar_prueba(
            &mut conn,
            &campos,
            Some(&motor),
            opciones("Clave", "sustituir", false, false),
            &enc,
            &filas,
        );
        drop(conn);

        let reg = obtener_por_clave(&db, &campos, "A-1").unwrap();
        // Precio cambió.
        assert_eq!(reg.valores.get("Precio").unwrap().como_f64().unwrap(), 20.0);
        // Cantidad y Notas (ausentes del archivo) NO se anularon.
        assert_eq!(reg.valores.get("Cantidad").unwrap().como_f64().unwrap(), 5.0);
        assert_eq!(reg.valores.get("Notas").unwrap().como_texto(), "conservar");
        // Total recalculado = 20 * 5 = 100 (la cantidad se conservó).
        assert_eq!(reg.valores.get("Total").unwrap().como_f64().unwrap(), 100.0);
    }

    #[test]
    fn llave_numerica_1_casa_con_1punto0() {
        let (db, dir) = album_tmp();
        {
            let conn = db.conn().unwrap();
            repo_campos::crear(&conn, &nuevo("Codigo", TipoCampo::Numerico)).unwrap();
            repo_campos::crear(&conn, &nuevo("Nombre", TipoCampo::Texto)).unwrap();
        }
        let conn = db.conn().unwrap();
        let campos = repo_campos::listar(&conn).unwrap();
        drop(conn);

        // Siembra Codigo = 1.0 (REAL) con Nombre original.
        {
            let mut conn = db.conn().unwrap();
            let mut v: Valores = HashMap::new();
            v.insert("Codigo".into(), Valor::Numero(1.0));
            v.insert("Nombre".into(), Valor::Texto("viejo".into()));
            repo_registros::crear(
                &mut conn, &campos, None, Tabla::Principal, &v, &HashMap::new(), None, None, None,
            )
            .unwrap();
        }

        // El archivo trae la llave como "1" (sin .0).
        let enc = encabezados(&["Codigo", "Nombre"]);
        let filas = vec![fila(&["1", "nuevo"])];

        let mut conn = db.conn().unwrap();
        let r = aplicar_prueba(
            &mut conn,
            &campos,
            None,
            opciones("Codigo", "sustituir", true, false),
            &enc,
            &filas,
        );
        drop(conn);

        // Debe ACTUALIZAR (no crear un duplicado).
        assert_eq!(r.actualizados, 1, "1 debe casar con 1.0");
        assert_eq!(r.creados, 0, "no debe crear duplicado");
        assert_eq!(repo_registros::total(&db.conn().unwrap()).unwrap(), 1);
        let _ = dir;
    }

    #[test]
    fn llave_duplicada_en_archivo_usa_primera_fila() {
        let (db, _d, campos, motor) = album_pruebas();
        sembrar(&db, &campos, &motor, "A-1", 10.0, 1.0, "");

        let enc = encabezados(&["Clave", "Notas"]);
        let filas = vec![
            fila(&["A-1", "primera"]),
            fila(&["A-1", "segunda"]),
        ];

        let mut conn = db.conn().unwrap();
        let r = aplicar_prueba(
            &mut conn,
            &campos,
            Some(&motor),
            opciones("Clave", "sustituir", false, false),
            &enc,
            &filas,
        );
        drop(conn);

        assert_eq!(r.llaves_duplicadas.total, 1);
        assert_eq!(
            r.llaves_duplicadas.filas,
            vec![FilaLlave { fila: 3, llave: "A-1".into() }]
        );
        let reg = obtener_por_clave(&db, &campos, "A-1").unwrap();
        assert_eq!(reg.valores.get("Notas").unwrap().como_texto(), "primera");
    }

    #[test]
    fn multidato_se_importa() {
        let (db, _d, campos, motor) = album_pruebas();
        sembrar(&db, &campos, &motor, "A-1", 1.0, 1.0, "");

        let enc = encabezados(&["Clave", "Tags"]);
        let filas = vec![fila(&["A-1", "rojo | verde|azul"])];

        let mut conn = db.conn().unwrap();
        let r = aplicar_prueba(
            &mut conn,
            &campos,
            Some(&motor),
            opciones("Clave", "sustituir", false, false),
            &enc,
            &filas,
        );
        drop(conn);

        assert_eq!(r.actualizados, 1);
        let reg = obtener_por_clave(&db, &campos, "A-1").unwrap();
        assert_eq!(
            reg.multidatos.get("Tags").unwrap(),
            &vec!["rojo".to_string(), "verde".to_string(), "azul".to_string()]
        );
    }

    // --- llave: no coincidentes / vacías / repetidas -----------------------

    #[test]
    fn reporta_llaves_no_coincidentes_vacias_y_repetidas() {
        let (db, _d, campos, motor) = album_pruebas();
        sembrar(&db, &campos, &motor, "A-1", 1.0, 1.0, "");
        sembrar(&db, &campos, &motor, "A-2", 1.0, 1.0, "");

        let enc = encabezados(&["Clave", "Notas"]);
        let filas = vec![
            fila(&["A-1", "x"]),  // fila 2: coincide
            fila(&["X-9", "x"]),  // fila 3: no coincide
            fila(&["  ", "x"]),   // fila 4: llave vacía
            fila(&["A-2", "x"]),  // fila 5: coincide
            fila(&["X-9", "y"]),  // fila 6: repetida en el archivo
            fila(&[" Z-3 ", "x"]), // fila 7: no coincide (trim)
        ];

        let mut conn = db.conn().unwrap();
        let r = aplicar_prueba(
            &mut conn,
            &campos,
            Some(&motor),
            opciones("Clave", "sustituir", false, true),
            &enc,
            &filas,
        );
        drop(conn);

        assert_eq!(r.coincidentes, 2);
        assert_eq!(r.no_coincidentes.total, 2);
        assert_eq!(
            r.no_coincidentes.filas,
            vec![
                FilaLlave { fila: 3, llave: "X-9".into() },
                FilaLlave { fila: 7, llave: "Z-3".into() },
            ]
        );
        assert_eq!(r.llaves_vacias.total, 1);
        assert_eq!(r.llaves_vacias.filas[0].fila, 4);
        assert_eq!(
            r.llaves_duplicadas.filas,
            vec![FilaLlave { fila: 6, llave: "X-9".into() }]
        );
        assert_eq!(r.omitidos, 2);
        assert_eq!(r.creados, 0);
        assert_eq!(r.actualizados, 2);
    }

    #[test]
    fn no_coincidentes_omitir_o_importar_como_nuevos() {
        let (db, _d, campos, motor) = album_pruebas();
        sembrar(&db, &campos, &motor, "A-1", 1.0, 1.0, "");

        let enc = encabezados(&["Clave", "Notas"]);
        let filas = vec![fila(&["A-1", "actual"]), fila(&["N-1", "nueva"])];

        // Omitir: el registro sin coincidencia no se crea.
        let mut conn = db.conn().unwrap();
        let omitir = aplicar_prueba(
            &mut conn,
            &campos,
            Some(&motor),
            opciones("Clave", "sustituir", false, false),
            &enc,
            &filas,
        );
        drop(conn);
        assert_eq!(omitir.no_coincidentes.total, 1);
        assert_eq!(omitir.omitidos, 1);
        assert_eq!(omitir.creados, 0);
        assert!(obtener_por_clave(&db, &campos, "N-1").is_none());

        // Importar como nuevos: se da de alta con sus valores.
        let mut conn = db.conn().unwrap();
        let crear = aplicar_prueba(
            &mut conn,
            &campos,
            Some(&motor),
            opciones("Clave", "sustituir", true, false),
            &enc,
            &filas,
        );
        drop(conn);
        assert_eq!(crear.no_coincidentes.total, 1);
        assert_eq!(crear.omitidos, 0);
        assert_eq!(crear.creados, 1);
        let n1 = obtener_por_clave(&db, &campos, "N-1").unwrap();
        assert_eq!(n1.valores.get("Notas").unwrap().como_texto(), "nueva");
        assert_eq!(repo_registros::total(&db.conn().unwrap()).unwrap(), 2);
    }

    #[test]
    fn llave_desde_columna_con_otro_nombre() {
        let (db, _d, campos, motor) = album_pruebas();
        sembrar(&db, &campos, &motor, "A-1", 1.0, 1.0, "");

        // El archivo trae la llave en "SKU" (no existe como campo).
        let enc = encabezados(&["SKU", "Notas"]);
        let filas = vec![fila(&["A-1", "por sku"])];
        let mut op = opciones("Clave", "sustituir", false, false);
        op.columna_llave = Some("sku".into());

        let mut conn = db.conn().unwrap();
        let r = aplicar_prueba(
            &mut conn,
            &campos,
            Some(&motor),
            op,
            &enc,
            &filas,
        );
        drop(conn);

        assert_eq!(r.coincidentes, 1);
        assert_eq!(r.actualizados, 1);
        // La columna llave no cuenta como extra ni como ignorada.
        assert!(r.columnas_extra.is_empty());
        assert!(!r.avisos.iter().any(|a| a.contains("SKU")));
        let reg = obtener_por_clave(&db, &campos, "A-1").unwrap();
        assert_eq!(reg.valores.get("Notas").unwrap().como_texto(), "por sku");
    }

    #[test]
    fn columna_llave_ausente_es_error() {
        let (db, _d, campos, motor) = album_pruebas();
        let enc = encabezados(&["Notas"]);
        let filas = vec![fila(&["x"])];
        let mut conn = db.conn().unwrap();
        let err = aplicar_importacion(
            &mut conn,
            &campos,
            Some(&motor),
            None,
            &opciones("Clave", "sustituir", false, true),
            &archivo_prueba(&enc, &filas),
            None,
        )
        .unwrap_err();
        assert!(err.to_string().contains("no está presente"));
    }

    // --- columnas extra ----------------------------------------------------

    #[test]
    fn inferir_tipo_de_columnas_extra() {
        let tipo = |vals: &[&str]| inferir_tipo(vals.iter().copied(), b',');
        assert_eq!(tipo(&["1", "20", "", "300"]), (TipoCampo::Numerico, 0));
        assert_eq!(tipo(&["1.5", "2"]), (TipoCampo::Numerico, 2));
        assert_eq!(tipo(&["$1,234.50", "10"]), (TipoCampo::Numerico, 2));
        assert_eq!(tipo(&["2026-06-05", "05/07/2026"]), (TipoCampo::Fecha, 0));
        assert_eq!(tipo(&["12", "rojo"]), (TipoCampo::Texto, 0));
        // Cero a la izquierda: es un código, no un número.
        assert_eq!(tipo(&["007", "12"]), (TipoCampo::Texto, 0));
        assert_eq!(tipo(&["", "  "]), (TipoCampo::Texto, 0));
        assert_eq!(tipo(&["0", "0.25"]), (TipoCampo::Numerico, 2));
    }

    #[test]
    fn detecta_extras_sin_llave_ni_repetidas_ni_homonimas() {
        let campos = vec![
            campo_def("Clave", TipoCampo::Texto),
            CampoDef {
                tabla: Tabla::Variantes,
                ..campo_def("Talla", TipoCampo::Texto)
            },
        ];
        let enc = encabezados(&["SKU", "Clave", "Color", "color", "", "Talla", "Peso"]);
        let filas = vec![fila(&["A", "A", "rojo", "x", "x", "M", "1.5"])];
        let extras = detectar_extras(&enc, &filas, &campos, Some(0), b',');
        let nombres: Vec<&str> = extras.iter().map(|e| e.columna.as_str()).collect();
        // SKU es la llave; "color" repetida; "" sin nombre; Talla existe en variantes.
        assert_eq!(nombres, vec!["Color", "Peso"]);
        assert_eq!(extras[1].tipo, TipoCampo::Numerico);
    }

    /// Handle real sobre el álbum de pruebas (para probar el refresco de campos).
    fn handle_pruebas() -> (AlbumHandle, tempfile::TempDir) {
        let (db, dir, campos, motor) = album_pruebas();
        sembrar(&db, &campos, &motor, "A-1", 10.0, 2.0, "");
        (AlbumHandle::nuevo(db).unwrap(), dir)
    }

    fn archivo_con_extras() -> ArchivoLeido {
        archivo_prueba(
            &encabezados(&["Clave", "Precio", "Cantidad", "Color", "Peso", "Llegada"]),
            &[
                fila(&["A-1", "10", "2", "rojo", "1.5", "2026-06-05"]),
                fila(&["B-2", "5", "3", "azul", "2", "07/06/2026"]),
            ],
        )
    }

    #[test]
    fn extras_sin_marcar_se_reportan_y_se_ignoran() {
        let (h, _d) = handle_pruebas();
        let antes = h.campos().len();
        let r = importar_en_album(
            &h,
            &archivo_con_extras(),
            &opciones("Clave", "sustituir", true, false),
            None,
        )
        .unwrap();

        assert_eq!(h.campos().len(), antes, "no se crean campos");
        assert!(r.campos_creados.is_empty());
        let extras: Vec<&str> = r.columnas_extra.iter().map(|e| e.columna.as_str()).collect();
        assert_eq!(extras, vec!["Color", "Peso", "Llegada"]);
        assert!(r.avisos.iter().any(|a| a.contains("columnas ignoradas")));
    }

    #[test]
    fn extras_en_dry_run_no_crean_campos_pero_cuentan() {
        let (h, _d) = handle_pruebas();
        let antes = h.campos().len();
        let mut op = opciones("Clave", "sustituir", false, true);
        op.importar_extras = true;
        let r = importar_en_album(&h, &archivo_con_extras(), &op, None).unwrap();

        assert_eq!(h.campos().len(), antes, "el dry-run no toca la estructura");
        assert_eq!(r.campos_creados, vec!["Color", "Peso", "Llegada"]);
        // A-1 solo cambia por las columnas extra (Precio/Cantidad iguales).
        assert_eq!(r.actualizados, 1);
        assert!(!r.avisos.iter().any(|a| a.contains("columnas ignoradas")));
    }

    #[test]
    fn extras_crean_campos_refrescan_handle_e_importan() {
        let (h, _d) = handle_pruebas();
        let mut op = opciones("Clave", "sustituir", true, false);
        op.importar_extras = true;
        let r = importar_en_album(&h, &archivo_con_extras(), &op, None).unwrap();

        assert_eq!(r.campos_creados, vec!["Color", "Peso", "Llegada"]);
        assert_eq!(r.actualizados, 1);
        assert_eq!(r.creados, 1);

        // La caché del handle ya ve los campos nuevos con el tipo inferido.
        let campos = h.campos();
        let tipo = |n: &str| campos.iter().find(|c| c.nombre == n).map(|c| c.tipo);
        assert_eq!(tipo("Color"), Some(TipoCampo::Texto));
        assert_eq!(tipo("Peso"), Some(TipoCampo::Numerico));
        assert_eq!(tipo("Llegada"), Some(TipoCampo::Fecha));
        assert!(h.motor.read().unwrap().is_some(), "motor recompilado");

        // Los valores se importaron: el refresco ocurrió ANTES de aplicar.
        let a1 = obtener_por_clave(&h.db, &campos, "A-1").unwrap();
        assert_eq!(a1.valores.get("Color").unwrap().como_texto(), "rojo");
        assert_eq!(a1.valores.get("Peso").unwrap().como_f64().unwrap(), 1.5);
        assert_eq!(a1.valores.get("Llegada").unwrap().como_texto(), "2026-06-05");
        let b2 = obtener_por_clave(&h.db, &campos, "B-2").unwrap();
        assert_eq!(b2.valores.get("Llegada").unwrap().como_texto(), "2026-06-07");
        // El motor vigente calcula el registro nuevo: Total = 5 * 3.
        assert_eq!(b2.valores.get("Total").unwrap().como_f64().unwrap(), 15.0);

        // Reimportar ya no detecta extras ni crea nada.
        let r2 = importar_en_album(&h, &archivo_con_extras(), &op, None).unwrap();
        assert!(r2.campos_creados.is_empty());
        assert_eq!(h.campos().len(), campos.len());
    }

    #[test]
    fn extras_con_llave_invalida_no_crean_campos() {
        let (h, _d) = handle_pruebas();
        let antes = h.campos().len();
        let mut op = opciones("Clave", "sustituir", false, false);
        op.importar_extras = true;
        op.columna_llave = Some("NoExiste".into());
        assert!(importar_en_album(&h, &archivo_con_extras(), &op, None).is_err());
        assert_eq!(h.campos().len(), antes);
    }
}
