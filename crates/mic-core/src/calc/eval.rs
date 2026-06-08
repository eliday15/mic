//! Parser (precedencia de operadores / Pratt) y evaluador del AST.
//!
//! Reemplaza al `ScriptControl`/VBScript del original (`CalcCalc`/`CalcCalcrs`
//! de Module2.bas), que armaba una `Function fmic() ... End Function` y la
//! ejecutaba. Aquí compilamos la fórmula —ya expandida (ver `expand.rs`)— a un
//! árbol `Ast` cacheable y lo evaluamos sobre los valores de un registro.
//!
//! Semántica reproducida del original:
//! - Aritmética en `f64` (el VBScript operaba con números).
//! - Un campo ausente o `Nulo` vale `0.0` (semántica VB de `Empty` en
//!   aritmética: `Empty + n = n`).
//! - Las fechas se reducen a número de días desde `1900-01-01` (ver
//!   `functions.rs`), de modo que sumar/restar fechas opera en días.
//! - `@FECHA(arg)` exige que `arg` sea una fecha válida y la convierte a días.
//! - División por cero → error con mensaje claro (en el original era el
//!   "Overflow"/"!Error" del ScriptControl).
//! - Resultado numérico → [`Valor::Numero`]. Si la fórmula es exactamente un
//!   `@FECHA(arg)` (passthrough de fecha, equivalente al campo calculado con
//!   tipo de salida Fecha del original), el resultado se reconvierte a fecha y
//!   se devuelve como [`Valor::Texto`] en ISO `YYYY-MM-DD`.

use crate::calc::functions::{self, Funcion};
use crate::calc::lexer::{self, TipoToken, Token};
use crate::model::{Valor, Valores};

/// Árbol de sintaxis abstracta de una fórmula compilada.
#[derive(Debug, Clone, PartialEq)]
pub enum Ast {
    /// Constante numérica literal.
    Numero(f64),
    /// Referencia a un campo por su nombre normalizado (con `_`).
    /// Se guarda también el nombre visible original para poder resolver tanto
    /// `"Precio Venta"` como `"Precio_Venta"` contra el mapa de valores.
    Campo(String),
    /// Operación unaria (signo).
    Unario { op: OpUnario, arg: Box<Ast> },
    /// Operación binaria aritmética.
    Binario {
        op: OpBinario,
        izq: Box<Ast>,
        der: Box<Ast>,
    },
    /// Llamada a función (`@FECHA(arg)`).
    Llamada { funcion: Funcion, args: Vec<Ast> },
}

/// Operadores binarios soportados (aritmética del original).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpBinario {
    Suma,
    Resta,
    Multiplicacion,
    Division,
}

/// Operadores unarios soportados.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpUnario {
    Negacion,
    Identidad,
}

// ---------------------------------------------------------------------------
// Parser de precedencia de operadores (Pratt)
// ---------------------------------------------------------------------------

/// Compila una fórmula (ya expandida) a su `Ast`.
/// Devuelve `Err(String)` con un mensaje en español si la sintaxis es inválida.
pub fn compilar(formula: &str) -> Result<Ast, String> {
    let tokens = lexer::tokenizar(formula).map_err(|e| e.to_string())?;
    if tokens.is_empty() {
        return Err("la fórmula está vacía".to_string());
    }
    let mut parser = Parser { tokens, pos: 0 };
    let ast = parser.expresion(0)?;
    if parser.pos != parser.tokens.len() {
        let sobra = &parser.tokens[parser.pos].texto;
        return Err(format!("token inesperado tras la expresión: '{sobra}'"));
    }
    Ok(ast)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn actual(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn avanzar(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    /// Potencia de enlace por la izquierda de un operador binario; mayor =
    /// liga más fuerte. Reproduce la precedencia aritmética estándar (la misma
    /// que aplicaba VBScript): `* /` antes que `+ -`.
    fn potencia_binaria(texto: &str) -> Option<(OpBinario, u8)> {
        match texto {
            "+" => Some((OpBinario::Suma, 1)),
            "-" => Some((OpBinario::Resta, 1)),
            "*" => Some((OpBinario::Multiplicacion, 2)),
            "/" => Some((OpBinario::Division, 2)),
            _ => None,
        }
    }

    /// Analiza una expresión con precedencia mínima `min_bp` (Pratt).
    fn expresion(&mut self, min_bp: u8) -> Result<Ast, String> {
        let mut izq = self.prefijo()?;

        while let Some(tok) = self.actual() {
            if tok.tipo != TipoToken::Operador {
                break;
            }
            let Some((op, bp)) = Self::potencia_binaria(&tok.texto) else {
                // Paréntesis de cierre u otro símbolo: lo maneja el nivel
                // superior (no es un operador binario que consumamos aquí).
                break;
            };
            if bp < min_bp {
                break;
            }
            self.avanzar(); // consume el operador
            // Asociatividad por la izquierda: el lado derecho exige bp+1.
            let der = self.expresion(bp + 1)?;
            izq = Ast::Binario {
                op,
                izq: Box::new(izq),
                der: Box::new(der),
            };
        }
        Ok(izq)
    }

    /// Analiza un término prefijo: número, campo, función, paréntesis o signo.
    fn prefijo(&mut self) -> Result<Ast, String> {
        let tok = self
            .avanzar()
            .ok_or_else(|| "se esperaba una expresión".to_string())?;

        match tok.tipo {
            TipoToken::Numero => {
                let n: f64 = tok
                    .texto
                    .parse()
                    .map_err(|_| format!("número inválido: '{}'", tok.texto))?;
                Ok(Ast::Numero(n))
            }
            TipoToken::Nombre => Ok(Ast::Campo(tok.texto)),
            TipoToken::IniciaFuncion => self.llamada_funcion(),
            TipoToken::Operador => match tok.texto.as_str() {
                "(" => {
                    let dentro = self.expresion(0)?;
                    self.esperar(")")?;
                    Ok(dentro)
                }
                "+" => {
                    let arg = self.prefijo()?;
                    Ok(Ast::Unario {
                        op: OpUnario::Identidad,
                        arg: Box::new(arg),
                    })
                }
                "-" => {
                    let arg = self.prefijo()?;
                    Ok(Ast::Unario {
                        op: OpUnario::Negacion,
                        arg: Box::new(arg),
                    })
                }
                otro => Err(format!("operador inesperado: '{otro}'")),
            },
        }
    }

    /// Analiza `@NOMBRE( args )`. El `@` ya fue consumido por `prefijo`.
    fn llamada_funcion(&mut self) -> Result<Ast, String> {
        let nombre = self
            .avanzar()
            .filter(|t| t.tipo == TipoToken::Nombre)
            .ok_or_else(|| "se esperaba el nombre de la función tras '@'".to_string())?;
        let funcion = functions::buscar_funcion(&nombre.texto)
            .ok_or_else(|| format!("función desconocida: '@{}'", nombre.texto))?;

        self.esperar("(")?;
        let mut args = Vec::new();
        // El original solo admite un argumento; aceptamos lista separada por
        // comas para extensibilidad futura.
        if !self.es_actual(")") {
            loop {
                args.push(self.expresion(0)?);
                if self.es_actual(",") {
                    self.avanzar();
                } else {
                    break;
                }
            }
        }
        self.esperar(")")?;

        if args.len() != funcion.aridad() {
            return Err(format!(
                "la función '@{}' espera {} argumento(s), recibió {}",
                nombre.texto,
                funcion.aridad(),
                args.len()
            ));
        }
        Ok(Ast::Llamada { funcion, args })
    }

    fn es_actual(&self, texto: &str) -> bool {
        self.actual().map(|t| t.texto == texto).unwrap_or(false)
    }

    fn esperar(&mut self, texto: &str) -> Result<(), String> {
        match self.avanzar() {
            Some(t) if t.texto == texto => Ok(()),
            Some(t) => Err(format!("se esperaba '{texto}', se encontró '{}'", t.texto)),
            None => Err(format!("se esperaba '{texto}' al final de la fórmula")),
        }
    }
}

// ---------------------------------------------------------------------------
// Evaluación
// ---------------------------------------------------------------------------

/// Evalúa un AST contra los valores de un registro.
///
/// El resultado es [`Valor::Numero`] salvo que la fórmula completa sea un
/// `@FECHA(arg)` (passthrough de fecha), en cuyo caso se devuelve la fecha en
/// ISO como [`Valor::Texto`], replicando el tipo de salida Fecha del original.
pub fn evaluar(ast: &Ast, valores: &Valores) -> Result<Valor, String> {
    // Caso especial: la fórmula es exactamente `@FECHA(arg)` → resultado fecha.
    if let Ast::Llamada {
        funcion: Funcion::Fecha,
        args,
    } = ast
    {
        let dias = eval_num(ast, valores)?;
        let _ = args; // ya validado en parseo
        let fecha = functions::dias_a_fecha(dias.round() as i64);
        return Ok(Valor::Texto(functions::formato_iso(fecha)));
    }

    let n = eval_num(ast, valores)?;
    Ok(Valor::Numero(n))
}

/// Evalúa un AST a número (`f64`), la moneda interna de la aritmética.
fn eval_num(ast: &Ast, valores: &Valores) -> Result<f64, String> {
    match ast {
        Ast::Numero(n) => Ok(*n),

        Ast::Campo(nombre) => Ok(resolver_campo(nombre, valores)),

        Ast::Unario { op, arg } => {
            let v = eval_num(arg, valores)?;
            Ok(match op {
                OpUnario::Negacion => -v,
                OpUnario::Identidad => v,
            })
        }

        Ast::Binario { op, izq, der } => {
            let a = eval_num(izq, valores)?;
            let b = eval_num(der, valores)?;
            match op {
                OpBinario::Suma => Ok(a + b),
                OpBinario::Resta => Ok(a - b),
                OpBinario::Multiplicacion => Ok(a * b),
                OpBinario::Division => {
                    if b == 0.0 {
                        Err("división por cero".to_string())
                    } else {
                        Ok(a / b)
                    }
                }
            }
        }

        Ast::Llamada { funcion, args } => match funcion {
            Funcion::Fecha => {
                let arg = &args[0];
                eval_fecha_a_dias(arg, valores)
            }
        },
    }
}

/// Resuelve el valor numérico de un campo.
///
/// Reproduce `ScanSustVals`: un campo ausente o `Nulo` vale `0.0` (Empty de
/// VB en aritmética); si su valor es una fecha, se reduce a su número de días
/// desde el epoch; en otro caso se interpreta como número (texto numérico).
///
/// El nombre llega normalizado (con `_`); buscamos primero por la forma con
/// espacios (clave del mapa de `Valores`, que usa el nombre visible) y luego
/// por la forma literal, de modo que `Precio_Venta` resuelva `"Precio Venta"`.
fn resolver_campo(nombre_norm: &str, valores: &Valores) -> f64 {
    let Some(valor) = buscar_valor(nombre_norm, valores) else {
        // Campo no presente → 0.0 (Empty de VB).
        return 0.0;
    };
    match valor {
        Valor::Nulo(_) => 0.0,
        Valor::Numero(n) => *n,
        Valor::Entero(n) => *n as f64,
        Valor::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Valor::Texto(s) => {
            // Si el texto es una fecha, conviértelo a días (como el original).
            if let Some(fecha) = functions::interpretar_fecha(s) {
                functions::fecha_a_dias(fecha) as f64
            } else {
                // Texto numérico → número; texto no numérico → 0.0.
                s.trim().parse().unwrap_or(0.0)
            }
        }
    }
}

/// Evalúa el argumento de `@FECHA`: debe ser una fecha válida, que se reduce a
/// su número de días. Reproduce el "Uso Indebido de @FECHA" del original
/// cuando el argumento no es una fecha.
fn eval_fecha_a_dias(arg: &Ast, valores: &Valores) -> Result<f64, String> {
    // Caso típico: @FECHA(Campo). Si el campo contiene una fecha, días.
    if let Ast::Campo(nombre_norm) = arg {
        match buscar_valor(nombre_norm, valores) {
            Some(Valor::Texto(s)) => {
                if let Some(fecha) = functions::interpretar_fecha(s) {
                    return Ok(functions::fecha_a_dias(fecha) as f64);
                }
                return Err(format!("uso indebido de @FECHA: '{s}' no es una fecha"));
            }
            // Campo ausente o nulo: el original también lo trataría como 0
            // (Empty) tras fallar IsDate; aquí lo consideramos 0 días (epoch).
            None | Some(Valor::Nulo(_)) => return Ok(0.0),
            Some(otro) => {
                // Numérico ya almacenado como días: úsalo directamente.
                return Ok(otro.como_f64().unwrap_or(0.0));
            }
        }
    }
    // Argumento general (subexpresión): ya está en días/número.
    eval_num(arg, valores)
}

/// Busca un valor por nombre normalizado, probando la forma con espacios
/// (clave real del mapa, que usa el nombre visible) y la forma literal.
fn buscar_valor<'a>(nombre_norm: &str, valores: &'a Valores) -> Option<&'a Valor> {
    // Forma visible: `Precio_Venta` → `Precio Venta`.
    let con_espacios = nombre_norm.replace('_', " ");
    valores
        .get(&con_espacios)
        .or_else(|| valores.get(nombre_norm))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn val(pares: &[(&str, Valor)]) -> Valores {
        let mut m = HashMap::new();
        for (k, v) in pares {
            m.insert((*k).to_string(), v.clone());
        }
        m
    }

    fn num(formula: &str, valores: &Valores) -> f64 {
        let ast = compilar(formula).unwrap();
        match evaluar(&ast, valores).unwrap() {
            Valor::Numero(n) => n,
            otro => panic!("se esperaba número, hubo {otro:?}"),
        }
    }

    #[test]
    fn precedencia_multiplicacion_antes_que_suma() {
        let v = val(&[]);
        assert_eq!(num("2+3*4", &v), 14.0);
        assert_eq!(num("(2+3)*4", &v), 20.0);
    }

    #[test]
    fn negacion_unaria() {
        let v = val(&[]);
        assert_eq!(num("-3+5", &v), 2.0);
        assert_eq!(num("10*-2", &v), -20.0);
    }

    #[test]
    fn campo_con_espacios_se_resuelve() {
        let v = val(&[("Precio Venta", Valor::Numero(100.0))]);
        // En la fórmula el nombre va con `_`.
        assert_eq!(num("Precio_Venta*2", &v), 200.0);
    }

    #[test]
    fn campo_faltante_es_cero() {
        let v = val(&[("A", Valor::Numero(5.0))]);
        assert_eq!(num("A+NoExiste", &v), 5.0);
    }

    #[test]
    fn campo_nulo_es_cero() {
        let v = val(&[("A", Valor::Numero(5.0)), ("B", Valor::Nulo(None))]);
        assert_eq!(num("A+B", &v), 5.0);
    }

    #[test]
    fn division_por_cero_da_error() {
        let v = val(&[("A", Valor::Numero(5.0))]);
        let ast = compilar("A/0").unwrap();
        let r = evaluar(&ast, &v);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("cero"));
    }

    #[test]
    fn fecha_passthrough_devuelve_texto_iso() {
        let v = val(&[("Vencimiento", Valor::Texto("2007-11-15".into()))]);
        let ast = compilar("@FECHA(Vencimiento)").unwrap();
        match evaluar(&ast, &v).unwrap() {
            Valor::Texto(s) => assert_eq!(s, "2007-11-15"),
            otro => panic!("se esperaba texto fecha, hubo {otro:?}"),
        }
    }

    #[test]
    fn diferencia_de_fechas_en_dias() {
        let v = val(&[
            ("Fin", Valor::Texto("2007-11-15".into())),
            ("Inicio", Valor::Texto("2007-11-10".into())),
        ]);
        // Días entre dos fechas vía @FECHA.
        assert_eq!(num("@FECHA(Fin)-@FECHA(Inicio)", &v), 5.0);
    }

    #[test]
    fn fecha_de_campo_se_reduce_a_dias_en_aritmetica() {
        // Un campo fecha usado directamente en aritmética se reduce a días.
        let v = val(&[("F", Valor::Texto("1900-01-01".into()))]);
        assert_eq!(num("F+10", &v), 10.0);
    }

    #[test]
    fn fecha_invalida_en_funcion_da_error() {
        let v = val(&[("X", Valor::Texto("no es fecha".into()))]);
        let ast = compilar("@FECHA(X)").unwrap();
        assert!(evaluar(&ast, &v).is_err());
    }
}
