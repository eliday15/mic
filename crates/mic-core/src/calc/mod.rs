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
//! - `MotorCalculo::evaluar_formula_libre(formula, &Valores)` para vista previa
//!   en el editor de fórmulas.

mod eval;
mod expand;
mod functions;
mod lexer;

use crate::error::MicError;
use crate::model::{CampoDef, Valor, Valores};
use std::collections::HashMap;

pub use eval::Ast;

/// Motor con los AST compilados de todos los campos calculados de un álbum.
pub struct MotorCalculo {
    /// nombre de campo calculado → AST compilado (con fórmulas ya expandidas).
    compilados: HashMap<String, Ast>,
    /// Orden topológico para recálculo incremental.
    orden: Vec<String>,
}

impl MotorCalculo {
    /// Compila las fórmulas de todos los campos `Calculado`.
    /// Expande recursivamente referencias entre calculados y detecta ciclos.
    pub fn new(campos: &[CampoDef]) -> Result<Self, MicError> {
        let (compilados, orden) = expand::compilar_todo(campos)?;
        Ok(Self { compilados, orden })
    }

    /// Evalúa el campo calculado `nombre` con los valores del registro.
    pub fn evaluar(&self, nombre: &str, valores: &Valores) -> Result<Valor, MicError> {
        let ast = self.compilados.get(nombre).ok_or_else(|| {
            MicError::NoEncontrado(format!("campo calculado '{nombre}'"))
        })?;
        eval::evaluar(ast, valores).map_err(|detalle| MicError::Calc {
            campo: nombre.to_string(),
            detalle,
        })
    }

    /// Nombres de los campos calculados en orden de recálculo (topológico).
    pub fn orden_recalculo(&self) -> &[String] {
        &self.orden
    }

    /// Evalúa una fórmula suelta (editor de fórmulas) contra valores de prueba.
    /// `campos` se necesita para expandir referencias a otros calculados.
    pub fn evaluar_formula_libre(
        campos: &[CampoDef],
        formula: &str,
        valores: &Valores,
    ) -> Result<Valor, MicError> {
        let ast = expand::compilar_una(campos, formula)?;
        eval::evaluar(&ast, valores).map_err(|detalle| MicError::Calc {
            campo: "<fórmula>".to_string(),
            detalle,
        })
    }
}
