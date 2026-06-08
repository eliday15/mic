//! Modelo de dominio: campos configurables, valores, consultas y DTOs
//! compartidos entre mic-db, mic-migrator y mic-tauri.
//!
//! Serialización: camelCase hacia el frontend (serde).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tipo de un campo configurable (equivale al enum TipoCar del VB6:
/// TC_TEXTO=0, TC_NUMERICO=1, TC_MONEDA=2, TC_FECHA=3, TC_CALCU=4, TC_MULTID=5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TipoCampo {
    Texto,
    Numerico,
    Moneda,
    Fecha,
    Calculado,
    Multidato,
}

impl TipoCampo {
    /// Conversión desde el byte `Tipo` de la tabla `propiedades` de Jet.
    pub fn from_jet(n: i64) -> Option<Self> {
        match n {
            0 => Some(Self::Texto),
            1 => Some(Self::Numerico),
            2 => Some(Self::Moneda),
            3 => Some(Self::Fecha),
            4 => Some(Self::Calculado),
            5 => Some(Self::Multidato),
            _ => None,
        }
    }

    /// Tipo de columna SQLite para este tipo de campo.
    /// Sin límites de longitud: TEXT siempre es ilimitado.
    pub fn sqlite_type(&self) -> &'static str {
        match self {
            Self::Texto => "TEXT",
            Self::Numerico | Self::Moneda => "REAL",
            Self::Fecha => "TEXT",      // ISO-8601, ordenable
            Self::Calculado => "REAL",  // resultado persistido (numérico en el original)
            Self::Multidato => "INTEGER", // conteo; valores reales en tabla multidatos
        }
    }
}

/// Tabla destino de un campo o registro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tabla {
    Principal,
    Variantes,
}

impl Tabla {
    pub fn nombre(&self) -> &'static str {
        match self {
            Self::Principal => "principal",
            Self::Variantes => "variantes",
        }
    }
}

/// Definición de un campo configurable (equivale a clsImagenMic del VB6,
/// sin `longitud` como restricción — hoy la memoria sobra).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CampoDef {
    pub id: i64,
    /// Nombre visible elegido por el usuario (puede tener espacios/acentos).
    pub nombre: String,
    /// Nombre físico de la columna SQLite: `f_<id>` (evita inyección en DDL).
    pub col_fisica: String,
    pub tabla: Tabla,
    pub tipo: TipoCampo,
    /// Decimales de presentación (moneda/numérico). Solo formato, no restricción.
    pub decimales: u8,
    pub totalizable: bool,
    /// Fórmula para campos calculados (ex-sInfo del VB6).
    pub formula: Option<String>,
    pub visible: bool,
    pub modificable: bool,
    pub orden_visible: i32,
    /// Formato de presentación del valor: `"moneda"` | `"porcentaje"` | None.
    /// Solo presentación (aplica a número y calculado); el dato sigue REAL.
    #[serde(default)]
    pub formato: Option<String>,
}

/// Datos para crear/editar un campo (sin id ni col_fisica, que asigna mic-db).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CampoNuevo {
    pub nombre: String,
    pub tabla: Tabla,
    pub tipo: TipoCampo,
    #[serde(default)]
    pub decimales: u8,
    #[serde(default)]
    pub totalizable: bool,
    #[serde(default)]
    pub formula: Option<String>,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default = "default_true")]
    pub modificable: bool,
    #[serde(default)]
    pub orden_visible: i32,
    /// Formato de presentación: `"moneda"` | `"porcentaje"` | None.
    #[serde(default)]
    pub formato: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Valor dinámico de un campo. Serializa como JSON plano (untagged):
/// null / string / number / bool — el frontend recibe objetos simples.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Valor {
    Nulo(Option<()>), // serializa/deserializa null
    Bool(bool),
    Numero(f64),
    Entero(i64),
    Texto(String),
}

impl Valor {
    pub fn es_nulo(&self) -> bool {
        matches!(self, Valor::Nulo(_))
    }

    pub fn como_f64(&self) -> Option<f64> {
        match self {
            Valor::Numero(n) => Some(*n),
            Valor::Entero(n) => Some(*n as f64),
            Valor::Texto(s) => s.trim().parse().ok(),
            Valor::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            Valor::Nulo(_) => None,
        }
    }

    pub fn como_texto(&self) -> String {
        match self {
            Valor::Texto(s) => s.clone(),
            Valor::Numero(n) => n.to_string(),
            Valor::Entero(n) => n.to_string(),
            Valor::Bool(b) => if *b { "1" } else { "0" }.to_string(),
            Valor::Nulo(_) => String::new(),
        }
    }
}

impl Default for Valor {
    fn default() -> Self {
        Valor::Nulo(None)
    }
}

/// Mapa nombre-de-campo → valor (claves = `CampoDef.nombre`).
pub type Valores = HashMap<String, Valor>;

// ---------------------------------------------------------------------------
// Consultas (reemplazo de la paginación manual clsPaginas)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direccion {
    Asc,
    Desc,
}

/// Un nivel de ordenamiento (hasta 3, como frmOrdenar).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrdenCampo {
    pub campo: String,
    pub direccion: Direccion,
}

/// Operador de comparación de filtros (frmFA).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpComp {
    Igual,
    Distinto,
    Mayor,
    Menor,
    MayorIgual,
    MenorIgual,
    Contiene,
    Empieza,
}

/// Conector lógico entre condiciones (frmFA: Y/O).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpRel {
    Y,
    O,
}

/// Una condición de filtro avanzado.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CondicionFiltro {
    /// Conector con la condición anterior (None en la primera).
    pub op_rel: Option<OpRel>,
    pub campo: String,
    pub op_comp: OpComp,
    pub valor: String,
}

/// Filtro rápido del panel lateral: campo = valor (o multidato contiene valor).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiltroRapido {
    pub campo: String,
    pub valor: String,
}

/// Petición de ventana de registros para el scroll virtual.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryReq {
    pub tabla: Tabla,
    /// Solo variantes de este principal (cuando tabla = Variantes).
    #[serde(default)]
    pub id_principal: Option<i64>,
    /// Grupo jerárquico activo: valores de hasta 3 niveles (por, luego1, luego2).
    #[serde(default)]
    pub grupo: Option<SeleccionGrupo>,
    #[serde(default)]
    pub filtro_rapido: Option<FiltroRapido>,
    #[serde(default)]
    pub condiciones: Vec<CondicionFiltro>,
    /// Búsqueda libre FTS5 (sin acentos).
    #[serde(default)]
    pub busqueda: Option<String>,
    /// Hasta 3 niveles de orden.
    #[serde(default)]
    pub orden: Vec<OrdenCampo>,
    /// Incluir registros ocultos (`_auxiliar_ = 1`). Por defecto se excluyen,
    /// como las imágenes "ocultas" sin eliminar del original.
    #[serde(default)]
    pub incluir_ocultos: bool,
    pub offset: u64,
    pub limit: u32,
}

/// Selección en el árbol de grupos: el grupo y los valores elegidos por nivel.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeleccionGrupo {
    pub grupo_id: i64,
    /// Valores seleccionados por nivel (1 a 3); None = todo el nivel.
    pub valores: Vec<Option<String>>,
}

/// Registro ligero para la grilla (sin multidatos completos).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistroLigero {
    pub id: i64,
    pub imagen: Option<String>,
    /// mtime de la imagen para versionar la URL de miniatura.
    pub imagen_version: Option<i64>,
    pub tiene_variantes: bool,
    /// Registro oculto (`_auxiliar_ = 1`); solo aparece con `incluir_ocultos`.
    #[serde(default)]
    pub oculto: bool,
    pub valores: Valores,
}

/// Página de resultados para el scroll virtual.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryPage {
    pub total: u64,
    pub offset: u64,
    pub registros: Vec<RegistroLigero>,
}

/// Suma de un campo totalizable (resultado de "Totalizar", ex-frmTotalizar).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TotalCampo {
    pub campo: String,
    pub suma: f64,
}

/// Resultado de totalizar los campos marcados `totalizable` del conjunto
/// filtrado actual (respeta grupo, filtros, búsqueda).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Totales {
    /// Número de registros que cumplen el filtro.
    pub registros: u64,
    pub totales: Vec<TotalCampo>,
}

/// Estadísticas de un campo numérico sobre el conjunto filtrado.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstadisticaCampo {
    pub campo: String,
    /// Valores no nulos considerados.
    pub cuenta: u64,
    pub suma: f64,
    pub media: Option<f64>,
    pub mediana: Option<f64>,
    /// Valor más frecuente (None si no hay valores).
    pub moda: Option<f64>,
    /// Cuántas veces aparece la moda.
    pub moda_conteo: u64,
    pub minimo: Option<f64>,
    pub maximo: Option<f64>,
}

/// Resultado del panel de estadísticas (ex-Totalizar ampliado).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Estadisticas {
    /// Registros que cumplen el filtro activo.
    pub registros: u64,
    pub campos: Vec<EstadisticaCampo>,
}

/// Registro completo (editor): valores + multidatos.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistroCompleto {
    pub id: i64,
    pub tabla: Tabla,
    pub imagen: Option<String>,
    pub imagen_version: Option<i64>,
    pub valores: Valores,
    /// nombre de campo multidato → lista de valores.
    pub multidatos: HashMap<String, Vec<String>>,
}

// ---------------------------------------------------------------------------
// Grupos, categorías, álbum
// ---------------------------------------------------------------------------

/// Definición de un grupo jerárquico (tabla Grupos del original).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Grupo {
    pub id: i64,
    pub nombre: String,
    /// Campo del nivel 1 (obligatorio).
    pub por: String,
    /// Campo del nivel 2 (opcional).
    pub luego1: Option<String>,
    /// Campo del nivel 3 (opcional).
    pub luego2: Option<String>,
}

/// Nodo del árbol de grupos resuelto (valores distintos por nivel).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodoGrupo {
    pub valor: String,
    pub conteo: u64,
    #[serde(default)]
    pub hijos: Vec<NodoGrupo>,
}

/// Valor de categoría para autocomplete de multidatos.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoriaVal {
    pub valor: String,
    pub es_default: bool,
}

/// Información del álbum abierto.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumInfo {
    /// Id de sesión asignado al abrir (clave para todos los comandos).
    pub album_id: u64,
    pub ruta: String,
    pub nombre: String,
    pub total_registros: u64,
    pub tiene_variantes: bool,
    pub campos: Vec<CampoDef>,
}

/// Filtro avanzado guardado (con nombre).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiltroGuardado {
    pub nombre: String,
    pub condiciones: Vec<CondicionFiltro>,
}
