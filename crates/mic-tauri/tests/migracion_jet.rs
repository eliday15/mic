//! Prueba de integración del lector de `.mdb` **en proceso** (`mic_migrator::jet`).
//!
//! Lee el fixture Access (`test.mdb`) con el crate pure-Rust `jetdb`, sin
//! binarios externos ni subprocesos: por eso corre en TODAS las plataformas
//! (Mac/Linux/Windows) sin `#[cfg(windows)]`. Valida que se listan las tablas de
//! usuario y que `leer_tabla` convierte bien una tabla con tipos variados
//! (texto, enteros, Double, DateTime, Currency, Boolean).

use std::path::PathBuf;

use mic_migrator::jet;

/// Ruta al `.mdb` de prueba (vive en el crate del migrador).
fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../mic-migrator/tests/fixtures/test.mdb")
}

#[test]
fn jet_lista_tablas_de_usuario() {
    let mdb = fixture();
    assert!(mdb.exists(), "el fixture debe existir en {}", mdb.display());

    let tablas = jet::tablas(&mdb).expect("jet debe listar las tablas del fixture");
    assert!(
        tablas.iter().any(|t| t == "Table1"),
        "las tablas del fixture deben incluir 'Table1', pero fueron: {tablas:?}"
    );
    // No deben colarse tablas de sistema de Access.
    assert!(
        !tablas.iter().any(|t| t.starts_with("MSys")),
        "no debe haber tablas de sistema (MSys*): {tablas:?}"
    );
}

#[test]
fn jet_lee_tabla_con_tipos_variados() {
    let leida = jet::leer_tabla(&fixture(), "Table1").expect("leer Table1");
    let t = &leida.csv;

    // La cabecera del fixture es A, B, C, D, E, F, G, H, I.
    assert_eq!(
        t.cabecera,
        vec!["A", "B", "C", "D", "E", "F", "G", "H", "I"],
        "la cabecera de Table1 debe ser A..I"
    );
    assert!(!t.is_empty(), "Table1 debe tener al menos una fila");
    // Una tabla legítima no debería omitir filas.
    assert_eq!(leida.omitidas, 0, "Table1 no debería tener filas omitidas");

    // Localiza la fila con datos no triviales (la segunda del fixture:
    // A="abcdefg", F=444.555 Double, G fecha, H=3.5 Currency, I=1 Boolean).
    let i_a = t.indice("A").unwrap();
    let i_f = t.indice("F").unwrap();
    let i_g = t.indice("G").unwrap();
    let i_h = t.indice("H").unwrap();
    let i_i = t.indice("I").unwrap();

    let fila = t
        .filas
        .iter()
        .find(|f| f.get(i_a).map(|s| s == "abcdefg").unwrap_or(false))
        .expect("debe existir la fila con A='abcdefg'");

    // Double F: format!("{f}") da una cadena parseable como número.
    let f = &fila[i_f];
    assert_eq!(f, "444.555", "el Double F debe ser 444.555, fue {f:?}");
    assert_eq!(
        mic_migrator::type_map::parse_numero(f),
        Some(444.555),
        "el Double F debe ser parseable como número"
    );

    // DateTime G: debe convertirse a una fecha ISO YYYY-MM-DD válida.
    let g = &fila[i_g];
    assert!(
        chrono::NaiveDate::parse_from_str(g, "%Y-%m-%d").is_ok(),
        "el DateTime G debe ser una fecha ISO YYYY-MM-DD, fue {g:?}"
    );

    // Currency H = 3.5 (jetdb lo da como Money "3.5000"); parseable como número.
    let h = &fila[i_h];
    assert_eq!(
        mic_migrator::type_map::parse_numero(h),
        Some(3.5),
        "la Currency H debe valer 3.5, fue {h:?}"
    );

    // Boolean I = verdadero → "1".
    assert_eq!(fila[i_i], "1", "el Boolean I (verdadero) debe ser '1'");
}
