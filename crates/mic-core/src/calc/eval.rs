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
//! - Fechas vacías: si la fórmula usa un campo de tipo Fecha vacío (o con una
//!   fecha ilegible), el resultado es vacío ([`Valor::Nulo`]) en vez de contar
//!   días desde 1900 — `Fin - Inicio` sin `Inicio` no tiene sentido. Los campos
//!   numéricos vacíos siguen valiendo 0 (semántica VB).

use crate::calc::functions::{self, Funcion};
use crate::calc::lexer::{self, TipoToken, Token};
use crate::model::{Valor, Valores};
use chrono::NaiveDate;
use std::collections::HashSet;

/// Contexto de evaluación que no viaja en el AST.
pub struct Contexto<'a> {
    /// Nombres normalizados (espacios → `_`) de los campos de tipo Fecha:
    /// vacíos en ellos vacían el resultado en lugar de valer 0.
    pub fechas: &'a HashSet<String>,
    /// Fecha que devuelve `@HOY` (inyectable para pruebas deterministas).
    pub hoy: NaiveDate,
}

/// Resultado intermedio: `None` = vacío (fecha sin valor), que se propaga.
type Num = Result<Option<f64>, String>;

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
    /// Llamada a función (`@FECHA(arg)`, `@HOY`).
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

        // Las funciones sin argumentos admiten omitir los paréntesis (`@HOY`).
        if funcion.aridad() == 0 && !self.es_actual("(") {
            return Ok(Ast::Llamada {
                funcion,
                args: Vec::new(),
            });
        }
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
/// Si interviene una fecha vacía el resultado es [`Valor::Nulo`].
pub fn evaluar(ast: &Ast, valores: &Valores, ctx: &Contexto) -> Result<Valor, String> {
    let Some(n) = eval_num(ast, valores, ctx)? else {
        return Ok(Valor::Nulo(None));
    };
    // Caso especial: la fórmula es exactamente `@FECHA(arg)` → resultado fecha.
    if matches!(
        ast,
        Ast::Llamada {
            funcion: Funcion::Fecha,
            ..
        }
    ) {
        return Ok(dias_a_valor_fecha(n));
    }
    Ok(Valor::Numero(n))
}

/// `true` si el AST (fórmula ya expandida) usa `@HOY` en algún punto.
pub fn usa_hoy(ast: &Ast) -> bool {
    match ast {
        Ast::Numero(_) | Ast::Campo(_) => false,
        Ast::Unario { arg, .. } => usa_hoy(arg),
        Ast::Binario { izq, der, .. } => usa_hoy(izq) || usa_hoy(der),
        Ast::Llamada { funcion, args } => *funcion == Funcion::Hoy || args.iter().any(usa_hoy),
    }
}

/// Convierte un número de días desde el epoch en una fecha ISO
/// ([`Valor::Texto`] `YYYY-MM-DD`). Las fracciones de día se redondean.
pub fn dias_a_valor_fecha(dias: f64) -> Valor {
    let fecha = functions::dias_a_fecha(dias.round() as i64);
    Valor::Texto(functions::formato_iso(fecha))
}

/// Evalúa un AST a número (`f64`), la moneda interna de la aritmética.
/// `Ok(None)` indica que intervino una fecha vacía.
fn eval_num(ast: &Ast, valores: &Valores, ctx: &Contexto) -> Num {
    match ast {
        Ast::Numero(n) => Ok(Some(*n)),

        Ast::Campo(nombre) => Ok(resolver_campo(nombre, valores, ctx)),

        Ast::Unario { op, arg } => {
            let Some(v) = eval_num(arg, valores, ctx)? else {
                return Ok(None);
            };
            Ok(Some(match op {
                OpUnario::Negacion => -v,
                OpUnario::Identidad => v,
            }))
        }

        Ast::Binario { op, izq, der } => {
            let a = eval_num(izq, valores, ctx)?;
            let b = eval_num(der, valores, ctx)?;
            let (Some(a), Some(b)) = (a, b) else {
                return Ok(None);
            };
            match op {
                OpBinario::Suma => Ok(Some(a + b)),
                OpBinario::Resta => Ok(Some(a - b)),
                OpBinario::Multiplicacion => Ok(Some(a * b)),
                OpBinario::Division => {
                    if b == 0.0 {
                        Err("división por cero".to_string())
                    } else {
                        Ok(Some(a / b))
                    }
                }
            }
        }

        Ast::Llamada { funcion, args } => match funcion {
            Funcion::Fecha => eval_fecha_a_dias(&args[0], valores, ctx),
            Funcion::Hoy => Ok(Some(functions::fecha_a_dias(ctx.hoy) as f64)),
        },
    }
}

/// Resuelve el valor numérico de un campo.
///
/// Reproduce `ScanSustVals`: un campo ausente o `Nulo` vale `0.0` (Empty de
/// VB en aritmética); si su valor es una fecha, se reduce a su número de días
/// desde el epoch; en otro caso se interpreta como número (texto numérico).
/// Excepción: un campo de tipo Fecha vacío o ilegible devuelve `None` (vacío).
///
/// El nombre llega normalizado (con `_`); buscamos primero por la forma con
/// espacios (clave del mapa de `Valores`, que usa el nombre visible) y luego
/// por la forma literal, de modo que `Precio_Venta` resuelva `"Precio Venta"`.
fn resolver_campo(nombre_norm: &str, valores: &Valores, ctx: &Contexto) -> Option<f64> {
    let valor = buscar_valor(nombre_norm, valores);
    if ctx.fechas.contains(nombre_norm) {
        return match valor {
            Some(Valor::Texto(s)) => {
                functions::interpretar_fecha(s).map(|f| functions::fecha_a_dias(f) as f64)
            }
            _ => None,
        };
    }
    let Some(valor) = valor else {
        // Campo no presente → 0.0 (Empty de VB).
        return Some(0.0);
    };
    Some(match valor {
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
    })
}

/// Evalúa el argumento de `@FECHA`: debe ser una fecha válida, que se reduce a
/// su número de días. Reproduce el "Uso Indebido de @FECHA" del original
/// cuando el argumento no es una fecha.
fn eval_fecha_a_dias(arg: &Ast, valores: &Valores, ctx: &Contexto) -> Num {
    // Caso típico: @FECHA(Campo). Si el campo contiene una fecha, días.
    if let Ast::Campo(nombre_norm) = arg {
        return match buscar_valor(nombre_norm, valores) {
            Some(Valor::Texto(s)) if s.trim().is_empty() => Ok(None),
            Some(Valor::Texto(s)) => match functions::interpretar_fecha(s) {
                Some(fecha) => Ok(Some(functions::fecha_a_dias(fecha) as f64)),
                None => Err(format!("uso indebido de @FECHA: '{s}' no es una fecha")),
            },
            // Campo ausente o nulo: no hay fecha → resultado vacío.
            None | Some(Valor::Nulo(_)) => Ok(None),
            // Numérico ya almacenado como días: úsalo directamente.
            Some(otro) => Ok(Some(otro.como_f64().unwrap_or(0.0))),
        };
    }
    // Argumento general (subexpresión): ya está en días/número.
    eval_num(arg, valores, ctx)
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

    /// Evalúa con los campos `fechas` declarados de tipo Fecha y hoy fijo.
    fn eval_con(formula: &str, valores: &Valores, fechas: &[&str]) -> Result<Valor, String> {
        let fechas: HashSet<String> = fechas.iter().map(|s| s.to_string()).collect();
        let ctx = Contexto {
            fechas: &fechas,
            hoy: NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
        };
        evaluar(&compilar(formula).unwrap(), valores, &ctx)
    }

    fn evalua(formula: &str, valores: &Valores) -> Result<Valor, String> {
        eval_con(formula, valores, &[])
    }

    fn num(formula: &str, valores: &Valores) -> f64 {
        match evalua(formula, valores).unwrap() {
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
        let r = evalua("A/0", &v);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("cero"));
    }

    #[test]
    fn fecha_passthrough_devuelve_texto_iso() {
        let v = val(&[("Vencimiento", Valor::Texto("2007-11-15".into()))]);
        match evalua("@FECHA(Vencimiento)", &v).unwrap() {
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
        assert!(evalua("@FECHA(X)", &v).is_err());
    }

    fn fechas_venc() -> Valores {
        val(&[
            ("Fecha Inicio", Valor::Texto("2026-01-01".into())),
            ("Fecha Fin", Valor::Texto("2026-03-01".into())),
            ("Cantidad", Valor::Numero(4.0)),
        ])
    }

    #[test]
    fn resta_directa_de_campos_fecha_da_dias() {
        let v = fechas_venc();
        let r = eval_con("Fecha_Fin - Fecha_Inicio", &v, &["Fecha_Inicio", "Fecha_Fin"]);
        assert_eq!(r.unwrap(), Valor::Numero(59.0));
    }

    #[test]
    fn sumar_y_restar_dias_a_una_fecha() {
        let v = fechas_venc();
        let fechas = ["Fecha_Inicio"];
        let mas = eval_con("Fecha_Inicio + 30", &v, &fechas).unwrap();
        let menos = eval_con("Fecha_Inicio - 7", &v, &fechas).unwrap();
        let mas = mas.como_f64().unwrap();
        let menos = menos.como_f64().unwrap();
        assert_eq!(dias_a_valor_fecha(mas), Valor::Texto("2026-01-31".into()));
        assert_eq!(dias_a_valor_fecha(menos), Valor::Texto("2025-12-25".into()));
    }

    #[test]
    fn hoy_con_y_sin_parentesis() {
        let v = fechas_venc();
        let fechas = ["Fecha_Inicio", "Fecha_Fin"];
        // Hoy fijo en 2026-10-13: desde el 2026-01-01 pasaron 285 días.
        let r = eval_con("@HOY - Fecha_Inicio", &v, &fechas).unwrap();
        assert_eq!(r, Valor::Numero(285.0));
        let r = eval_con("@hoy() - Fecha_Inicio", &v, &fechas).unwrap();
        assert_eq!(r, Valor::Numero(285.0));
        let r = eval_con("Fecha_Fin - @HOY", &v, &fechas).unwrap();
        assert_eq!(r, Valor::Numero(-226.0));
    }

    #[test]
    fn hoy_no_acepta_argumentos() {
        assert!(compilar("@HOY(Fecha)").is_err());
        assert!(compilar("@FECHA").is_err());
    }

    #[test]
    fn fecha_vacia_vacia_el_resultado() {
        let mut v = fechas_venc();
        v.insert("Fecha Fin".into(), Valor::Nulo(None));
        let fechas = ["Fecha_Inicio", "Fecha_Fin"];
        assert_eq!(
            eval_con("Fecha_Fin - Fecha_Inicio", &v, &fechas).unwrap(),
            Valor::Nulo(None)
        );
        // Texto vacío y campo ausente también vacían.
        v.insert("Fecha Fin".into(), Valor::Texto(String::new()));
        assert_eq!(
            eval_con("Fecha_Fin + 1", &v, &fechas).unwrap(),
            Valor::Nulo(None)
        );
        assert_eq!(
            eval_con("@FECHA(Otra) + 1", &v, &fechas).unwrap(),
            Valor::Nulo(None)
        );
    }

    #[test]
    fn numerico_vacio_sigue_siendo_cero_junto_a_fechas() {
        let mut v = fechas_venc();
        v.insert("Cantidad".into(), Valor::Nulo(None));
        let r = eval_con("Fecha_Inicio + Cantidad", &v, &["Fecha_Inicio"]).unwrap();
        assert_eq!(dias_a_valor_fecha(r.como_f64().unwrap()), Valor::Texto("2026-01-01".into()));
    }
}
