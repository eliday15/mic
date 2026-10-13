//! Motor de campos calculados — reemplazo del ScriptControl/VBScript del VB6.
//!
//! Port fiel de Module5.bas: expansión recursiva de fórmulas que referencian
//! otros campos calculados (`SustituyeSiExiste`/`descomponeR`), tokenizado de
//! nombres con espacios (normalizados a `_`), función `@FECHA(...)`, y
//! evaluación aritmética. AST cacheado por campo; detección de ciclos
//! (el original podía colgarse con fórmulas circulares).
//!
//! API pública (estable para mic-db y mic-tauri):
//! - `MotorCalculo::new(&[CampoDef])` compila todas las fórmulas.
//! - `MotorCalculo::evaluar(nombre_campo, &Valores)` evalúa un calculado.
//! - `MotorCalculo::orden_recalculo()` orden topológico de los calculados.
//! - `MotorCalculo::evaluar_formula_libre(campos, formula, &Valores, formato)`
//!   para vista previa en el editor de fórmulas.
//!
//! Tipo de resultado: un calculado con `formato == "fecha"` ([`FORMATO_FECHA`])
//! devuelve su resultado (días desde 1900-01-01) como fecha ISO `YYYY-MM-DD`
//! ([`Valor::Texto`]), igual que un campo Fecha; el resto devuelve número.

mod eval;
mod expand;
mod functions;
mod lexer;

use crate::error::MicError;
use crate::model::{CampoDef, TipoCampo, Valor, Valores};
use chrono::{Local, NaiveDate};
use std::collections::{HashMap, HashSet};

pub use eval::Ast;

/// Valor de `CampoDef::formato` que hace que un calculado devuelva fecha.
pub const FORMATO_FECHA: &str = "fecha";

/// Motor con los AST compilados de todos los campos calculados de un álbum.
pub struct MotorCalculo {
    /// nombre de campo calculado → AST compilado (con fórmulas ya expandidas).
    compilados: HashMap<String, Ast>,
    /// Orden topológico para recálculo incremental.
    orden: Vec<String>,
    /// Nombres normalizados de los campos de tipo Fecha (vacío → resultado vacío).
    fechas: HashSet<String>,
    /// Calculados cuyo resultado se presenta como fecha (`formato == "fecha"`).
    salida_fecha: HashSet<String>,
}

impl MotorCalculo {
    /// Compila las fórmulas de todos los campos `Calculado`.
    /// Expande recursivamente referencias entre calculados y detecta ciclos.
    pub fn new(campos: &[CampoDef]) -> Result<Self, MicError> {
        let (compilados, orden) = expand::compilar_todo(campos)?;
        let salida_fecha = campos
            .iter()
            .filter(|c| c.tipo == TipoCampo::Calculado && es_formato_fecha(c.formato.as_deref()))
            .map(|c| c.nombre.clone())
            .collect();
        Ok(Self {
            compilados,
            orden,
            fechas: campos_fecha(campos),
            salida_fecha,
        })
    }

    /// Evalúa el campo calculado `nombre` con los valores del registro.
    pub fn evaluar(&self, nombre: &str, valores: &Valores) -> Result<Valor, MicError> {
        self.evaluar_en(nombre, valores, Local::now().date_naive())
    }

    /// Como [`Self::evaluar`] pero con la fecha de `@HOY` explícita.
    pub fn evaluar_en(
        &self,
        nombre: &str,
        valores: &Valores,
        hoy: NaiveDate,
    ) -> Result<Valor, MicError> {
        let ast = self.compilados.get(nombre).ok_or_else(|| {
            MicError::NoEncontrado(format!("campo calculado '{nombre}'"))
        })?;
        let ctx = eval::Contexto {
            fechas: &self.fechas,
            hoy,
        };
        let valor = eval::evaluar(ast, valores, &ctx).map_err(|detalle| MicError::Calc {
            campo: nombre.to_string(),
            detalle,
        })?;
        Ok(if self.salida_fecha.contains(nombre) {
            a_fecha(valor)
        } else {
            valor
        })
    }

    /// Calculados cuya fórmula expandida usa `@HOY` (también a través de otro
    /// calculado): son los que caducan al cambiar el día.
    pub fn campos_con_hoy(&self) -> HashSet<&str> {
        self.compilados
            .iter()
            .filter(|(_, ast)| eval::usa_hoy(ast))
            .map(|(nombre, _)| nombre.as_str())
            .collect()
    }

    /// Nombres de los campos calculados en orden de recálculo (topológico).
    pub fn orden_recalculo(&self) -> &[String] {
        &self.orden
    }

    /// Evalúa una fórmula suelta (editor de fórmulas) contra valores de prueba.
    /// `campos` se necesita para expandir referencias a otros calculados;
    /// `formato` es el del campo en edición (`"fecha"` → resultado fecha).
    pub fn evaluar_formula_libre(
        campos: &[CampoDef],
        formula: &str,
        valores: &Valores,
        formato: Option<&str>,
    ) -> Result<Valor, MicError> {
        let ast = expand::compilar_una(campos, formula)?;
        let fechas = campos_fecha(campos);
        let ctx = eval::Contexto {
            fechas: &fechas,
            hoy: Local::now().date_naive(),
        };
        let valor = eval::evaluar(&ast, valores, &ctx).map_err(|detalle| MicError::Calc {
            campo: "<fórmula>".to_string(),
            detalle,
        })?;
        Ok(if es_formato_fecha(formato) {
            a_fecha(valor)
        } else {
            valor
        })
    }
}

/// `true` si el formato de presentación pide resultado fecha.
fn es_formato_fecha(formato: Option<&str>) -> bool {
    formato == Some(FORMATO_FECHA)
}

/// Nombres normalizados (espacios → `_`) de los campos de tipo Fecha.
fn campos_fecha(campos: &[CampoDef]) -> HashSet<String> {
    campos
        .iter()
        .filter(|c| c.tipo == TipoCampo::Fecha)
        .map(|c| lexer::normaliza_nombre(&c.nombre))
        .collect()
}

/// Reinterpreta un resultado numérico (días desde 1900-01-01) como fecha ISO.
/// Vacíos y fechas ya convertidas pasan tal cual.
fn a_fecha(valor: Valor) -> Valor {
    match valor {
        Valor::Numero(n) if n.is_finite() => eval::dias_a_valor_fecha(n),
        otro => otro,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Tabla;

    fn campo(id: i64, nombre: &str, tipo: TipoCampo, formula: Option<&str>, formato: Option<&str>) -> CampoDef {
        CampoDef {
            id,
            nombre: nombre.into(),
            col_fisica: format!("f_{id}"),
            tabla: Tabla::Principal,
            tipo,
            decimales: 0,
            totalizable: false,
            formula: formula.map(Into::into),
            visible: true,
            modificable: false,
            orden_visible: id as i32,
            formato: formato.map(Into::into),
        }
    }

    fn album() -> Vec<CampoDef> {
        vec![
            campo(1, "Fecha Alta", TipoCampo::Fecha, None, None),
            campo(2, "Vencimiento", TipoCampo::Fecha, None, None),
            campo(3, "Vence30", TipoCampo::Calculado, Some("Fecha_Alta + 30"), Some("fecha")),
            campo(4, "Días para vencer", TipoCampo::Calculado, Some("Vencimiento - @HOY"), None),
            campo(5, "Plazo", TipoCampo::Calculado, Some("Vencimiento - Fecha_Alta"), None),
            // Calculado que referencia a otro calculado fecha: opera en días.
            campo(6, "Aviso", TipoCampo::Calculado, Some("Vence30 - 5"), Some("fecha")),
        ]
    }

    fn hoy() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 13).unwrap()
    }

    fn valores(alta: Option<&str>, venc: Option<&str>) -> Valores {
        let mut v = Valores::new();
        let a = |s: Option<&str>| s.map_or(Valor::Nulo(None), |s| Valor::Texto(s.into()));
        v.insert("Fecha Alta".into(), a(alta));
        v.insert("Vencimiento".into(), a(venc));
        v
    }

    #[test]
    fn calculado_formato_fecha_devuelve_iso() {
        let m = MotorCalculo::new(&album()).unwrap();
        let v = valores(Some("2026-01-15"), Some("2026-11-02"));
        assert_eq!(
            m.evaluar_en("Vence30", &v, hoy()).unwrap(),
            Valor::Texto("2026-02-14".into())
        );
    }

    #[test]
    fn calculado_numerico_cuenta_dias() {
        let m = MotorCalculo::new(&album()).unwrap();
        let v = valores(Some("2026-01-15"), Some("2026-11-02"));
        assert_eq!(m.evaluar_en("Días para vencer", &v, hoy()).unwrap(), Valor::Numero(20.0));
        assert_eq!(m.evaluar_en("Plazo", &v, hoy()).unwrap(), Valor::Numero(291.0));
    }

    #[test]
    fn fechas_vacias_dan_resultado_vacio() {
        let m = MotorCalculo::new(&album()).unwrap();
        let v = valores(None, None);
        for nombre in ["Vence30", "Días para vencer", "Plazo", "Aviso"] {
            assert_eq!(m.evaluar_en(nombre, &v, hoy()).unwrap(), Valor::Nulo(None), "{nombre}");
        }
    }

    #[test]
    fn calculado_fecha_encadenado_opera_en_dias() {
        let m = MotorCalculo::new(&album()).unwrap();
        let v = valores(Some("2026-01-15"), None);
        assert_eq!(
            m.evaluar_en("Aviso", &v, hoy()).unwrap(),
            Valor::Texto("2026-02-09".into())
        );
    }

    #[test]
    fn detecta_calculados_que_usan_hoy_incluso_encadenados() {
        let mut campos = album();
        // "Días" no es ASCII (no referenciable): el encadenado usa "Dias".
        campos[3].nombre = "Dias para vencer".into();
        campos.push(campo(7, "Urgencia", TipoCampo::Calculado, Some("Dias_para_vencer * 2"), None));
        campos.push(campo(8, "Semanas", TipoCampo::Calculado, Some("Plazo / 7"), None));
        let m = MotorCalculo::new(&campos).unwrap();
        let mut con_hoy: Vec<&str> = m.campos_con_hoy().into_iter().collect();
        con_hoy.sort();
        assert_eq!(con_hoy, vec!["Dias para vencer", "Urgencia"]);
    }

    #[test]
    fn formula_libre_respeta_formato_fecha() {
        let campos = album();
        let v = valores(Some("2026-01-15"), None);
        let num = MotorCalculo::evaluar_formula_libre(&campos, "Fecha_Alta + 30", &v, None).unwrap();
        assert!(matches!(num, Valor::Numero(_)));
        let fecha =
            MotorCalculo::evaluar_formula_libre(&campos, "Fecha_Alta + 30", &v, Some("fecha")).unwrap();
        assert_eq!(fecha, Valor::Texto("2026-02-14".into()));
    }
}
