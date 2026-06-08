//! Expansión recursiva de fórmulas y orden topológico — port de
//! `descomponeR`/`DescomponeC`/`SustituyeSiExiste` (Module5.bas) y de la
//! lógica de `ValsCalcu` que llamaba a `DescomponeC` antes de calcular.
//!
//! Cuando una fórmula referencia OTRO campo calculado, su nombre se sustituye
//! por `"(su_fórmula)"` recursivamente antes de compilar, de modo que el
//! evaluador final solo ve campos de datos y constantes. El original repetía la
//! sustitución hasta que no había cambios (`DescomponeC` se llamaba a sí misma
//! mientras `modifico = True`) y se colgaba con fórmulas circulares; aquí
//! detectamos ciclos con un conjunto de visitados y un límite de profundidad,
//! devolviendo [`MicError::CicloCalculo`].

use crate::calc::eval::{self, Ast};
use crate::calc::lexer::{self, TipoToken};
use crate::error::MicError;
use crate::model::{CampoDef, TipoCampo};
use std::collections::{HashMap, HashSet};

/// Profundidad máxima de expansión (cinturón de seguridad ante ciclos que el
/// conjunto de visitados no atrape por nombres equivalentes).
const PROFUNDIDAD_MAX: usize = 64;

/// Compila todas las fórmulas de los campos `Calculado` de un álbum.
///
/// Devuelve:
/// - un mapa `nombre visible del campo → Ast` (con referencias a otros
///   calculados ya expandidas), y
/// - el orden topológico de recálculo (dependencias antes que dependientes).
///
/// Errores: [`MicError::CicloCalculo`] si hay dependencias circulares;
/// [`MicError::Calc`] si una fórmula no compila.
pub fn compilar_todo(
    campos: &[CampoDef],
) -> Result<(HashMap<String, Ast>, Vec<String>), MicError> {
    // Índice de campos calculados por nombre normalizado → (nombre visible, fórmula).
    let calculados = indexar_calculados(campos);

    let mut asts: HashMap<String, Ast> = HashMap::new();
    for campo in campos {
        if campo.tipo != TipoCampo::Calculado {
            continue;
        }
        let formula = campo.formula.as_deref().unwrap_or("");
        // Calculado sin fórmula todavía: se omite (queda NULL hasta definirla
        // en el editor de estructura). Antes hacía fallar el motor completo y
        // con ello la creación/apertura del álbum.
        if formula.trim().is_empty() {
            continue;
        }
        let expandida = expandir(
            formula,
            &lexer::normaliza_nombre(&campo.nombre),
            &calculados,
            &mut HashSet::new(),
            0,
        )?;
        let ast = eval::compilar(&expandida).map_err(|detalle| MicError::Calc {
            campo: campo.nombre.clone(),
            detalle,
        })?;
        asts.insert(campo.nombre.clone(), ast);
    }

    // El orden de recálculo solo incluye los campos con fórmula compilada.
    let orden: Vec<String> = orden_topologico(campos, &calculados)?
        .into_iter()
        .filter(|n| asts.contains_key(n))
        .collect();
    Ok((asts, orden))
}

/// Compila una sola fórmula suelta (editor de fórmulas), expandiendo
/// referencias a los campos calculados existentes.
pub fn compilar_una(campos: &[CampoDef], formula: &str) -> Result<Ast, MicError> {
    let calculados = indexar_calculados(campos);
    // Sin nombre propio que omitir: usamos un centinela que no colisiona.
    let expandida = expandir(formula, "\u{0}<libre>", &calculados, &mut HashSet::new(), 0)?;
    eval::compilar(&expandida).map_err(|detalle| MicError::Calc {
        campo: "<fórmula>".to_string(),
        detalle,
    })
}

/// Información de un campo calculado indexada por su nombre normalizado.
struct CampoCalc {
    /// Nombre normalizado (con `_`), clave de búsqueda en tokens.
    nombre_norm: String,
    /// Fórmula tal como la escribió el usuario.
    formula: String,
}

/// Construye el índice `nombre normalizado → CampoCalc` de los calculados.
fn indexar_calculados(campos: &[CampoDef]) -> HashMap<String, CampoCalc> {
    let mut mapa = HashMap::new();
    for campo in campos {
        if campo.tipo == TipoCampo::Calculado {
            let norm = lexer::normaliza_nombre(&campo.nombre);
            mapa.insert(
                norm.to_ascii_uppercase(),
                CampoCalc {
                    nombre_norm: norm,
                    formula: campo.formula.clone().unwrap_or_default(),
                },
            );
        }
    }
    mapa
}

/// Expande recursivamente, en `sexpr`, las referencias a campos calculados por
/// `"(su_fórmula)"`. Port de `SustituyeSiExiste` + `DescomponeC`:
/// - omite el propio campo (`nombre_propio`),
/// - solo sustituye tokens `Nombre` que NO sean el nombre de una función
///   (es decir, el token anterior no es `@`),
/// - repite hasta que no haya cambios.
///
/// `visitados` lleva los nombres ya en expansión en la rama actual para
/// detectar ciclos; `profundidad` es el cinturón de seguridad.
fn expandir(
    sexpr: &str,
    nombre_propio: &str,
    calculados: &HashMap<String, CampoCalc>,
    visitados: &mut HashSet<String>,
    profundidad: usize,
) -> Result<String, MicError> {
    if profundidad > PROFUNDIDAD_MAX {
        return Err(MicError::CicloCalculo(format!(
            "profundidad de expansión excedida (posible ciclo) en '{nombre_propio}'"
        )));
    }

    let tokens = lexer::tokenizar(sexpr).map_err(|e| MicError::Calc {
        campo: nombre_propio.to_string(),
        detalle: e.to_string(),
    })?;

    let mut salida = String::new();
    let mut modifico = false;
    let mut tipo_anterior: Option<TipoToken> = None;

    for token in &tokens {
        let mut sustituyo = false;

        if token.tipo == TipoToken::Nombre {
            // No sustituir el nombre de una función: en el original `ilast = 3`
            // (token anterior `@`) marca que este `Nombre` es `FECHA`.
            let es_nombre_funcion = tipo_anterior == Some(TipoToken::IniciaFuncion);
            if !es_nombre_funcion {
                let clave = lexer::normaliza_nombre(&token.texto).to_ascii_uppercase();
                if clave != nombre_propio.to_ascii_uppercase() {
                    if let Some(calc) = calculados.get(&clave) {
                        // Referencia a otro calculado → detectar ciclo.
                        if visitados.contains(&clave) {
                            return Err(MicError::CicloCalculo(format!(
                                "el campo calculado '{}' participa en una dependencia circular",
                                calc.nombre_norm
                            )));
                        }
                        visitados.insert(clave.clone());
                        let interna = expandir(
                            &calc.formula,
                            &calc.nombre_norm,
                            calculados,
                            visitados,
                            profundidad + 1,
                        )?;
                        visitados.remove(&clave);
                        // Encerrar entre paréntesis para preservar precedencia,
                        // tal como `"(" & campon.sInfo & ")"` del original.
                        salida.push('(');
                        salida.push_str(&interna);
                        salida.push(')');
                        sustituyo = true;
                        modifico = true;
                    }
                }
            }
        }

        if !sustituyo {
            salida.push_str(&token.texto);
        }
        tipo_anterior = Some(token.tipo);
    }

    if modifico {
        // Hubo sustitución: vuelve a revisar (el original recursa sobre la
        // expresión modificada). Las referencias ya expandidas quedan dentro
        // de paréntesis y `visitados` protege contra ciclos.
        expandir(&salida, nombre_propio, calculados, visitados, profundidad + 1)
    } else {
        Ok(salida)
    }
}

/// Calcula el orden topológico de recálculo de los campos calculados: un campo
/// se recalcula después de todos los calculados que referencia. Es la base del
/// recálculo incremental (un campo calculado puede depender de otro).
fn orden_topologico(
    campos: &[CampoDef],
    calculados: &HashMap<String, CampoCalc>,
) -> Result<Vec<String>, MicError> {
    // Dependencias directas: nombre visible → claves normalizadas de los
    // calculados que aparecen en su fórmula.
    let mut deps: HashMap<String, Vec<String>> = HashMap::new();
    // clave normalizada → nombre visible (para la salida).
    let mut visible: HashMap<String, String> = HashMap::new();

    for campo in campos {
        if campo.tipo != TipoCampo::Calculado {
            continue;
        }
        let clave = lexer::normaliza_nombre(&campo.nombre).to_ascii_uppercase();
        visible.insert(clave.clone(), campo.nombre.clone());

        let formula = campo.formula.as_deref().unwrap_or("");
        let tokens = lexer::tokenizar(formula).map_err(|e| MicError::Calc {
            campo: campo.nombre.clone(),
            detalle: e.to_string(),
        })?;

        let mut dependencias = Vec::new();
        let mut tipo_anterior: Option<TipoToken> = None;
        for token in &tokens {
            if token.tipo == TipoToken::Nombre
                && tipo_anterior != Some(TipoToken::IniciaFuncion)
            {
                let dep = lexer::normaliza_nombre(&token.texto).to_ascii_uppercase();
                if dep != clave && calculados.contains_key(&dep) && !dependencias.contains(&dep)
                {
                    dependencias.push(dep);
                }
            }
            tipo_anterior = Some(token.tipo);
        }
        deps.insert(clave, dependencias);
    }

    // Orden topológico por DFS con detección de ciclo (colores).
    #[derive(Clone, Copy, PartialEq)]
    enum Color {
        Blanco,
        Gris,
        Negro,
    }
    let mut color: HashMap<String, Color> = deps.keys().map(|k| (k.clone(), Color::Blanco)).collect();
    let mut orden = Vec::new();

    fn visitar(
        nodo: &str,
        deps: &HashMap<String, Vec<String>>,
        color: &mut HashMap<String, Color>,
        visible: &HashMap<String, String>,
        orden: &mut Vec<String>,
    ) -> Result<(), MicError> {
        match color.get(nodo) {
            Some(Color::Negro) => return Ok(()),
            Some(Color::Gris) => {
                let n = visible.get(nodo).cloned().unwrap_or_else(|| nodo.to_string());
                return Err(MicError::CicloCalculo(format!(
                    "dependencia circular en el campo calculado '{n}'"
                )));
            }
            _ => {}
        }
        color.insert(nodo.to_string(), Color::Gris);
        if let Some(hijos) = deps.get(nodo) {
            for hijo in hijos {
                visitar(hijo, deps, color, visible, orden)?;
            }
        }
        color.insert(nodo.to_string(), Color::Negro);
        if let Some(n) = visible.get(nodo) {
            orden.push(n.clone());
        }
        Ok(())
    }

    // Recorremos en el orden de declaración para una salida determinista.
    for campo in campos {
        if campo.tipo != TipoCampo::Calculado {
            continue;
        }
        let clave = lexer::normaliza_nombre(&campo.nombre).to_ascii_uppercase();
        visitar(&clave, &deps, &mut color, &visible, &mut orden)?;
    }

    Ok(orden)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Tabla, Valor};
    use std::collections::HashMap;

    fn calc(id: i64, nombre: &str, formula: &str) -> CampoDef {
        CampoDef {
            id,
            nombre: nombre.to_string(),
            col_fisica: format!("f_{id}"),
            tabla: Tabla::Principal,
            tipo: TipoCampo::Calculado,
            decimales: 2,
            totalizable: false,
            formula: Some(formula.to_string()),
            visible: true,
            modificable: false,
            orden_visible: id as i32,
            formato: None,
        }
    }

    fn dato(id: i64, nombre: &str) -> CampoDef {
        CampoDef {
            id,
            nombre: nombre.to_string(),
            col_fisica: format!("f_{id}"),
            tabla: Tabla::Principal,
            tipo: TipoCampo::Numerico,
            decimales: 2,
            totalizable: true,
            formula: None,
            visible: true,
            modificable: true,
            orden_visible: id as i32,
            formato: None,
        }
    }

    fn valores(pares: &[(&str, f64)]) -> crate::model::Valores {
        let mut m = HashMap::new();
        for (k, v) in pares {
            m.insert((*k).to_string(), Valor::Numero(*v));
        }
        m
    }

    #[test]
    fn formula_simple_compila_y_evalua() {
        let campos = vec![
            dato(1, "Cantidad"),
            dato(2, "Precio"),
            calc(3, "Total", "Cantidad*Precio"),
        ];
        let (asts, _) = compilar_todo(&campos).unwrap();
        let v = valores(&[("Cantidad", 3.0), ("Precio", 10.0)]);
        let r = eval::evaluar(asts.get("Total").unwrap(), &v).unwrap();
        assert_eq!(r, Valor::Numero(30.0));
    }

    #[test]
    fn anidacion_a_usa_b_usa_c() {
        // C = base ; B = C*2 ; A = B+1  → con base=5 → C=10, B=20? no: C=base.
        let campos = vec![
            dato(1, "Base"),
            calc(2, "C", "Base*2"),
            calc(3, "B", "C+1"),
            calc(4, "A", "B*10"),
        ];
        let (asts, orden) = compilar_todo(&campos).unwrap();
        let v = valores(&[("Base", 5.0)]);
        // C = 10, B = 11, A = 110.
        assert_eq!(
            eval::evaluar(asts.get("C").unwrap(), &v).unwrap(),
            Valor::Numero(10.0)
        );
        assert_eq!(
            eval::evaluar(asts.get("B").unwrap(), &v).unwrap(),
            Valor::Numero(11.0)
        );
        assert_eq!(
            eval::evaluar(asts.get("A").unwrap(), &v).unwrap(),
            Valor::Numero(110.0)
        );
        // Orden topológico: C antes que B antes que A.
        let pos = |n: &str| orden.iter().position(|x| x == n).unwrap();
        assert!(pos("C") < pos("B"));
        assert!(pos("B") < pos("A"));
    }

    #[test]
    fn precedencia_se_preserva_al_expandir() {
        // B = X+1 ; A = B*2  → con X=3, B=4, A=8 (no 3+1*2=5).
        let campos = vec![dato(1, "X"), calc(2, "B", "X+1"), calc(3, "A", "B*2")];
        let (asts, _) = compilar_todo(&campos).unwrap();
        let v = valores(&[("X", 3.0)]);
        assert_eq!(
            eval::evaluar(asts.get("A").unwrap(), &v).unwrap(),
            Valor::Numero(8.0)
        );
    }

    #[test]
    fn ciclo_directo_da_error() {
        let campos = vec![calc(1, "A", "B+1"), calc(2, "B", "A+1")];
        let r = compilar_todo(&campos);
        assert!(matches!(r, Err(MicError::CicloCalculo(_))));
    }

    #[test]
    fn ciclo_indirecto_da_error() {
        let campos = vec![calc(1, "A", "B+1"), calc(2, "B", "C+1"), calc(3, "C", "A+1")];
        let r = compilar_todo(&campos);
        assert!(matches!(r, Err(MicError::CicloCalculo(_))));
    }

    #[test]
    fn campo_con_espacios_en_formula() {
        let campos = vec![
            dato(1, "Precio Venta"),
            dato(2, "Unidades"),
            calc(3, "Importe", "Precio_Venta*Unidades"),
        ];
        let (asts, _) = compilar_todo(&campos).unwrap();
        let v = valores(&[("Precio Venta", 12.5), ("Unidades", 4.0)]);
        assert_eq!(
            eval::evaluar(asts.get("Importe").unwrap(), &v).unwrap(),
            Valor::Numero(50.0)
        );
    }

    #[test]
    fn compilar_una_expande_referencias() {
        let campos = vec![dato(1, "X"), calc(2, "B", "X+1")];
        let ast = compilar_una(&campos, "B*3").unwrap();
        let v = valores(&[("X", 2.0)]);
        assert_eq!(eval::evaluar(&ast, &v).unwrap(), Valor::Numero(9.0));
    }
}
