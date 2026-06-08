//! Mapeo de los metadatos de campos del original (tabla `propiedades`) al modelo
//! nuevo [`CampoNuevo`], y conversión de valores Jet → tipos de MIC 3.0.
//!
//! La tabla `propiedades` del `.mdb` (ver `db.bas`, `SalvaPropCampo`) tiene estas
//! columnas exactas:
//!
//! | Columna       | Tipo Jet  | Significado                                   |
//! |---------------|-----------|-----------------------------------------------|
//! | `Nombre`      | Text      | Nombre visible del campo                       |
//! | `Tipo`        | Byte      | 0..5 = TC_TEXTO..TC_MULTID                     |
//! | `longitud`    | Byte      | longitud máxima (se descarta: TEXT sin límite) |
//! | `decimales`   | Byte      | decimales de presentación                      |
//! | `totalizable` | Boolean   | sumable en reportes                            |
//! | `sInfo`       | Text      | fórmula (solo TC_CALCU)                         |
//! | `TipoSal`     | Byte      | tipo de salida del calculado (no se conserva)  |
//! | `Enprincipal` | Boolean   | True → tabla principal, False → variantes      |
//! | `Modificable` | Boolean   | editable por el usuario                        |
//! | `Visible`     | Boolean   | visible en la grilla                           |
//! | `OrdenVisible`| Byte      | orden de presentación                          |
//!
//! Los campos de sistema (`_imagen_`, `_id_`, `_auxiliar_`, `_variante_`,
//! `_variantes_`, `_idprincipal_`) viven en `propiedades` pero **no** son campos
//! de usuario: [`es_campo_sistema`] los identifica para que la migración los
//! trate aparte.

use mic_core::model::{CampoNuevo, Tabla, TipoCampo};

use crate::csv_parser::TablaCsv;

/// Nombres de columna esperados en la tabla `propiedades` (case-insensitive).
mod col {
    pub const NOMBRE: &str = "Nombre";
    pub const TIPO: &str = "Tipo";
    pub const DECIMALES: &str = "decimales";
    pub const TOTALIZABLE: &str = "totalizable";
    pub const SINFO: &str = "sInfo";
    pub const ENPRINCIPAL: &str = "Enprincipal";
    pub const MODIFICABLE: &str = "Modificable";
    pub const VISIBLE: &str = "Visible";
    pub const ORDEN_VISIBLE: &str = "OrdenVisible";
}

/// Prefijos/nombres de los campos de sistema del original.
pub const CAMPOS_SISTEMA: &[&str] = &[
    "_imagen_",
    "_id_",
    "_auxiliar_",
    "_variante_",
    "_variantes_",
    "_idprincipal_",
    "_idprincipal",
];

/// `true` si `nombre` es un campo de sistema (no de usuario).
///
/// El criterio del original: cualquier nombre que empiece por `_` es interno.
pub fn es_campo_sistema(nombre: &str) -> bool {
    let n = nombre.trim();
    n.starts_with('_') || CAMPOS_SISTEMA.iter().any(|s| s.eq_ignore_ascii_case(n))
}

/// Interpreta un booleano de Jet tal como lo emite `mdb-export`.
///
/// Access exporta los booleanos como `1`/`0` (sin la bandera `-B`). También
/// aceptamos `True`/`False`/`-1` por robustez frente a volcados manuales.
pub fn jet_bool(s: &str) -> bool {
    let v = s.trim();
    matches!(v, "1" | "-1") || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes")
}

/// Convierte un byte de la columna `decimales`/`OrdenVisible` a entero,
/// tolerando vacío.
pub fn jet_entero(s: &str) -> i64 {
    s.trim().parse::<i64>().unwrap_or(0)
}

/// Normaliza un número que pueda venir con coma decimal (`1.234,50`) a un `f64`.
///
/// `mdb-export` emite punto decimal, pero algunos volcados/locale antiguos usan
/// coma. La heurística: si hay coma pero no punto, la coma es decimal; si hay
/// ambos, el punto es separador de miles y se elimina, la coma es decimal.
/// Devuelve `None` si no parsea como número.
pub fn parse_numero(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    let tiene_coma = t.contains(',');
    let tiene_punto = t.contains('.');
    let normalizado = if tiene_coma && tiene_punto {
        // 1.234,50 → 1234.50
        t.replace('.', "").replace(',', ".")
    } else if tiene_coma {
        // 1234,50 → 1234.50
        t.replace(',', ".")
    } else {
        t.to_string()
    };
    normalizado.parse::<f64>().ok()
}

/// Una fila de `propiedades` ya interpretada como [`CampoNuevo`] + metadatos.
#[derive(Debug, Clone)]
pub struct CampoOrigen {
    /// Definición lista para `repo_campos::crear`.
    pub def: CampoNuevo,
    /// Nombre tal cual aparece en `propiedades` (para mapear columnas de datos).
    pub nombre_original: String,
}

/// Lee la tabla `propiedades` y devuelve los campos **de usuario** (ignora los
/// de sistema), preservando el orden de aparición.
///
/// La asignación de tabla (principal/variantes) sale de `Enprincipal`. El tipo
/// sale de `Tipo` vía [`TipoCampo::from_jet`]; un tipo desconocido se trata como
/// texto (mejor migrar el dato como texto que perderlo).
pub fn mapear_campos(props: &TablaCsv) -> Vec<CampoOrigen> {
    let i_nombre = props.indice(col::NOMBRE);
    let i_tipo = props.indice(col::TIPO);
    let i_dec = props.indice(col::DECIMALES);
    let i_tot = props.indice(col::TOTALIZABLE);
    let i_sinfo = props.indice(col::SINFO);
    let i_enppal = props.indice(col::ENPRINCIPAL);
    let i_mod = props.indice(col::MODIFICABLE);
    let i_vis = props.indice(col::VISIBLE);
    let i_orden = props.indice(col::ORDEN_VISIBLE);

    let mut salida = Vec::new();
    let celda = |fila: &[String], idx: Option<usize>| -> String {
        idx.and_then(|i| fila.get(i)).cloned().unwrap_or_default()
    };

    for fila in &props.filas {
        let nombre = celda(fila, i_nombre);
        let nombre = nombre.trim().to_string();
        if nombre.is_empty() || es_campo_sistema(&nombre) {
            continue;
        }

        let tipo_byte = jet_entero(&celda(fila, i_tipo));
        let tipo = TipoCampo::from_jet(tipo_byte).unwrap_or(TipoCampo::Texto);

        let en_principal = match i_enppal {
            // Si no hay columna Enprincipal, asumimos principal.
            Some(_) => jet_bool(&celda(fila, i_enppal)),
            None => true,
        };
        let tabla = if en_principal {
            Tabla::Principal
        } else {
            Tabla::Variantes
        };

        // sInfo → fórmula (solo tiene sentido para calculados).
        let sinfo = celda(fila, i_sinfo);
        let formula = if matches!(tipo, TipoCampo::Calculado) {
            let f = sinfo.trim();
            if f.is_empty() {
                None
            } else {
                Some(f.to_string())
            }
        } else {
            None
        };

        // Visible / Modificable: si faltan columnas, por defecto true (como el
        // `default_true` del modelo nuevo).
        let visible = match i_vis {
            Some(_) => jet_bool(&celda(fila, i_vis)),
            None => true,
        };
        let modificable = match i_mod {
            Some(_) => jet_bool(&celda(fila, i_mod)),
            None => true,
        };

        let def = CampoNuevo {
            nombre: nombre.clone(),
            tabla,
            tipo,
            decimales: jet_entero(&celda(fila, i_dec)).clamp(0, 255) as u8,
            totalizable: match i_tot {
                Some(_) => jet_bool(&celda(fila, i_tot)),
                None => false,
            },
            formula,
            visible,
            modificable,
            orden_visible: jet_entero(&celda(fila, i_orden)) as i32,
            formato: None,
        };

        salida.push(CampoOrigen {
            def,
            nombre_original: nombre,
        });
    }

    salida
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::csv_parser::parsear_texto;

    fn props_csv() -> TablaCsv {
        // Cabecera exacta de la tabla propiedades + filas representativas.
        let texto = "\
Nombre,Tipo,longitud,decimales,totalizable,sInfo,TipoSal,Enprincipal,Modificable,Visible,OrdenVisible
_imagen_,0,255,0,0, ,0,1,1,0,0
Descripcion,0,50,0,0, ,0,1,1,1,1
Precio,2,0,2,1, ,0,1,1,1,2
Total,4,0,2,0,Precio*Cantidad,1,1,0,1,3
Tallas,5,0,0,0, ,0,1,1,1,4
Color,0,30,0,0, ,0,0,1,1,1
";
        parsear_texto(texto).unwrap()
    }

    #[test]
    fn ignora_campos_sistema() {
        let campos = mapear_campos(&props_csv());
        assert!(campos.iter().all(|c| !c.nombre_original.starts_with('_')));
        // 5 campos de usuario (Descripcion, Precio, Total, Tallas en principal +
        // Color en variantes).
        assert_eq!(campos.len(), 5);
    }

    #[test]
    fn mapea_tipos_correctamente() {
        let campos = mapear_campos(&props_csv());
        let buscar = |n: &str| campos.iter().find(|c| c.nombre_original == n).unwrap();
        assert_eq!(buscar("Descripcion").def.tipo, TipoCampo::Texto);
        assert_eq!(buscar("Precio").def.tipo, TipoCampo::Moneda);
        assert_eq!(buscar("Total").def.tipo, TipoCampo::Calculado);
        assert_eq!(buscar("Tallas").def.tipo, TipoCampo::Multidato);
    }

    #[test]
    fn calculado_lleva_formula() {
        let campos = mapear_campos(&props_csv());
        let total = campos.iter().find(|c| c.nombre_original == "Total").unwrap();
        assert_eq!(total.def.formula.as_deref(), Some("Precio*Cantidad"));
        assert!(!total.def.modificable); // Modificable=0 en el fixture
    }

    #[test]
    fn tabla_segun_enprincipal() {
        let campos = mapear_campos(&props_csv());
        let color = campos.iter().find(|c| c.nombre_original == "Color").unwrap();
        assert_eq!(color.def.tabla, Tabla::Variantes);
        let precio = campos.iter().find(|c| c.nombre_original == "Precio").unwrap();
        assert_eq!(precio.def.tabla, Tabla::Principal);
    }

    #[test]
    fn decimales_y_totalizable() {
        let campos = mapear_campos(&props_csv());
        let precio = campos.iter().find(|c| c.nombre_original == "Precio").unwrap();
        assert_eq!(precio.def.decimales, 2);
        assert!(precio.def.totalizable);
    }

    #[test]
    fn jet_bool_variantes() {
        assert!(jet_bool("1"));
        assert!(jet_bool("-1"));
        assert!(jet_bool("True"));
        assert!(!jet_bool("0"));
        assert!(!jet_bool(""));
    }

    #[test]
    fn numero_coma_decimal() {
        assert_eq!(parse_numero("1234,50"), Some(1234.50));
        assert_eq!(parse_numero("1.234,50"), Some(1234.50));
        assert_eq!(parse_numero("10.5"), Some(10.5));
        assert_eq!(parse_numero("42"), Some(42.0));
        assert_eq!(parse_numero(""), None);
        assert_eq!(parse_numero("abc"), None);
    }
}
