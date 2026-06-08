//! Tokenizado de fórmulas — port de `ObtenToken`/`ObtenTokensyTipos`
//! (Module2.bas del VB6 original).
//!
//! Reglas del original (`ObtenToken`):
//! - El primer carácter fija el tipo del token:
//!   - letra `A..Z` → `Nombre` (variable o función): tipo 0.
//!   - dígito o `.` → `Numero` (constante numérica): tipo 1.
//!   - `@` → `IniciaFuncion` (inicio de `@FECHA(`): tipo 3.
//!   - cualquier otro → `Operador`: tipo 2.
//! - Un `Nombre` se extiende mientras siga con letras, dígitos o `_`; un `.`
//!   termina el nombre. Esto permite nombres de campo con espacios porque el
//!   original normaliza el nombre visible con `Replace(nombre, " ", "_")` antes
//!   de buscarlo, de modo que `Precio Venta` se escribe `Precio_Venta` dentro
//!   de las fórmulas y el `_` forma parte del token de nombre.
//! - Un `Numero` se extiende mientras siga con dígitos o `.`.
//! - Cada operador (incluidos paréntesis) se emite como token suelto de un
//!   solo carácter: en cuanto el siguiente carácter cambia de clase, el token
//!   operador termina (`bfinToken = True`).
//! - El `@` se emite solo (un carácter); el `FECHA` que le sigue es un token
//!   `Nombre` y el `(` un token `Operador`, tal como en el original
//!   (`@`, `FECHA`, `(`, arg, `)`).
//!
//! La detección de tokens inválidos del original (tipo -1, p.ej. número pegado
//! a letra `12abc`) se reproduce devolviendo `MicError::Invalido`.

use crate::error::MicError;

/// Clasificación de un token, equivalente al `iTipo` de `ObtenToken`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoToken {
    /// Nombre de variable o de función (tipo 0 en el original).
    Nombre,
    /// Constante numérica (tipo 1).
    Numero,
    /// Operador o paréntesis (tipo 2).
    Operador,
    /// Inicio de función: el carácter `@` (tipo 3).
    IniciaFuncion,
}

/// Un token con su texto y su clasificación.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// Texto literal del token tal como aparece en la fórmula.
    pub texto: String,
    pub tipo: TipoToken,
}

/// Normaliza un nombre visible de campo a su forma en fórmula: los espacios
/// se sustituyen por `_`, igual que `Replace(nombre, " ", "_")` del original.
/// Se usa para que el evaluador resuelva tanto `"Precio Venta"` como
/// `"Precio_Venta"` al mismo campo.
pub fn normaliza_nombre(nombre: &str) -> String {
    nombre.replace(' ', "_")
}

/// Indica si un carácter es letra `A..Z`/`a..z`. En el original solo se
/// consideran letras ASCII (`strCaracter >= "A" And strCaracter <= "Z"`
/// tras `UCase`). Reproducimos esa misma puerta para fidelidad.
fn es_letra(c: char) -> bool {
    c.is_ascii_alphabetic()
}

/// Dígito decimal o punto: arranca/continúa una constante numérica.
fn es_digito_o_punto(c: char) -> bool {
    c.is_ascii_digit() || c == '.'
}

/// Tokeniza la fórmula completa. Port de `ObtenTokensyTipos`: descarta tokens
/// vacíos (espacios) y va concatenando lo que devuelve `ObtenToken`.
///
/// Devuelve `MicError::Invalido` si encuentra un token mal formado (por
/// ejemplo, una constante numérica seguida inmediatamente de una letra), que
/// el original marcaba con `iTipo = -1`.
pub fn tokenizar(formula: &str) -> Result<Vec<Token>, MicError> {
    // Trabajamos sobre el vector de chars para indexar como hace el VB con Mid.
    let chars: Vec<char> = formula.chars().collect();
    let n = chars.len();
    let mut tokens = Vec::new();
    let mut i = 0usize;

    while i < n {
        // Los espacios separan tokens pero no producen token propio: el
        // original los descarta porque `ObtenToken` empieza en un carácter no
        // blanco (los espacios dentro de nombres ya vienen como `_`).
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        let (token, fin) = obten_token(&chars, i)?;
        i = fin + 1;
        tokens.push(token);
    }
    Ok(tokens)
}

/// Port directo de `ObtenToken`: extrae un token a partir de la posición
/// `inicio` y devuelve el token junto con el índice del último carácter
/// consumido (`iFinal` del original). Se asume `chars[inicio]` no blanco.
fn obten_token(chars: &[char], inicio: usize) -> Result<(Token, usize), MicError> {
    let n = chars.len();
    let primero = chars[inicio];

    // El primer carácter fija el tipo de token.
    let mut tipo: i8 = if es_letra(primero) {
        0 // Nombre de variable o función
    } else if es_digito_o_punto(primero) {
        1 // Constante numérica
    } else if primero == '@' {
        3 // Inicio de nombre de función
    } else {
        2 // Cualquier otro: operador
    };

    let mut fin = inicio;

    // Si ya no hay más caracteres, el token es de un solo carácter.
    if fin + 1 >= n {
        return construir(chars, inicio, fin, tipo);
    }

    // Localiza dónde acaba el token examinando el siguiente carácter, igual
    // que el bucle `Do ... Loop While Not bfinToken` del original.
    loop {
        let siguiente = chars[fin + 1];
        let mut fin_token = false;

        if es_letra(siguiente) {
            match tipo {
                0 => { /* Letra -> Letra: sigue siendo nombre */ }
                1 => {
                    // Número -> Letra: token inválido (iTipo = -1 en el original).
                    fin_token = true;
                    tipo = -1;
                }
                2 => fin_token = true, // Operador -> Letra
                3 => fin_token = true, // Operador fecha -> Letra
                _ => fin_token = true,
            }
        } else if es_digito_o_punto(siguiente) {
            match tipo {
                0 => {
                    // Nombre -> Num: el dígito sigue salvo que sea un punto,
                    // que termina el nombre (igual que el original).
                    if siguiente == '.' {
                        fin_token = true;
                    }
                }
                1 => { /* Num -> Num: sigue */ }
                2 => fin_token = true, // Operador -> Num
                3 => {
                    // Operador FECHA -> Num: inválido.
                    fin_token = true;
                    tipo = -1;
                }
                _ => fin_token = true,
            }
        } else {
            // Cualquier otro símbolo.
            match tipo {
                0 => {
                    // Letra -> símbolo. El `_` forma parte del nombre (campo
                    // con espacios); cualquier otro símbolo termina el nombre.
                    if siguiente != '_' {
                        fin_token = true;
                    }
                }
                1 => fin_token = true, // Termina valor numérico -> operador
                2 => fin_token = true, // Otro operador -> operador
                3 => {
                    // Op fecha -> operador: inválido.
                    fin_token = true;
                    tipo = -1;
                }
                _ => fin_token = true,
            }
        }

        if !fin_token {
            fin += 1;
        }

        if fin_token || fin + 1 >= n {
            break;
        }
    }

    construir(chars, inicio, fin, tipo)
}

/// Materializa el token con su texto (`Mid(strExpresion, iInicio, ...)`) y su
/// tipo, o devuelve error si el tipo quedó marcado como inválido (-1).
fn construir(
    chars: &[char],
    inicio: usize,
    fin: usize,
    tipo: i8,
) -> Result<(Token, usize), MicError> {
    let texto: String = chars[inicio..=fin].iter().collect();
    let tipo_token = match tipo {
        0 => TipoToken::Nombre,
        1 => TipoToken::Numero,
        2 => TipoToken::Operador,
        3 => TipoToken::IniciaFuncion,
        _ => {
            return Err(MicError::Invalido(format!(
                "token inválido en la fórmula: '{texto}'"
            )));
        }
    };
    Ok((
        Token {
            texto,
            tipo: tipo_token,
        },
        fin,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tipos(formula: &str) -> Vec<(String, TipoToken)> {
        tokenizar(formula)
            .unwrap()
            .into_iter()
            .map(|t| (t.texto, t.tipo))
            .collect()
    }

    #[test]
    fn nombre_con_guion_bajo_es_un_solo_token() {
        let t = tipos("Precio_Venta");
        assert_eq!(t, vec![("Precio_Venta".into(), TipoToken::Nombre)]);
    }

    #[test]
    fn aritmetica_simple() {
        let t = tipos("A+B*2");
        assert_eq!(
            t,
            vec![
                ("A".into(), TipoToken::Nombre),
                ("+".into(), TipoToken::Operador),
                ("B".into(), TipoToken::Nombre),
                ("*".into(), TipoToken::Operador),
                ("2".into(), TipoToken::Numero),
            ]
        );
    }

    #[test]
    fn numero_decimal() {
        let t = tipos("3.14+1");
        assert_eq!(
            t,
            vec![
                ("3.14".into(), TipoToken::Numero),
                ("+".into(), TipoToken::Operador),
                ("1".into(), TipoToken::Numero),
            ]
        );
    }

    #[test]
    fn funcion_fecha_se_descompone() {
        // @, FECHA, (, Vencimiento, ) — tal como el original.
        let t = tipos("@FECHA(Vencimiento)");
        assert_eq!(
            t,
            vec![
                ("@".into(), TipoToken::IniciaFuncion),
                ("FECHA".into(), TipoToken::Nombre),
                ("(".into(), TipoToken::Operador),
                ("Vencimiento".into(), TipoToken::Nombre),
                (")".into(), TipoToken::Operador),
            ]
        );
    }

    #[test]
    fn parentesis_y_espacios() {
        let t = tipos(" ( A + B ) ");
        assert_eq!(
            t,
            vec![
                ("(".into(), TipoToken::Operador),
                ("A".into(), TipoToken::Nombre),
                ("+".into(), TipoToken::Operador),
                ("B".into(), TipoToken::Nombre),
                (")".into(), TipoToken::Operador),
            ]
        );
    }

    #[test]
    fn numero_pegado_a_letra_es_invalido() {
        // En el original esto marca iTipo = -1.
        assert!(tokenizar("12abc").is_err());
    }

    #[test]
    fn normaliza_espacios_a_guion_bajo() {
        assert_eq!(normaliza_nombre("Precio Venta"), "Precio_Venta");
        assert_eq!(normaliza_nombre("Sin_Espacios"), "Sin_Espacios");
    }
}
