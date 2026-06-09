//! Prueba de integración de los binarios de mdbtools **empaquetados**.
//!
//! En Windows registra el directorio de recursos de la app (donde van los
//! `mdb-*.exe` + sus DLLs) y comprueba que arrancan y leen el `.mdb` de prueba.
//! Si faltara alguna DLL, el `.exe` no arrancaría y el test fallaría: por eso
//! este test corre en un runner Windows real en CI.
//!
//! En Mac/Linux **no** registra el directorio empaquetado: usa el mdbtools del
//! sistema, validando así la lógica de `tablas()` / `exportar_csv()` contra el
//! fixture.

use std::path::PathBuf;

use mic_migrator::csv_parser::decodificar_cp1252;
use mic_migrator::mdbtools;

/// Ruta al `.mdb` de prueba (vive en el crate del migrador).
fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../mic-migrator/tests/fixtures/test.mdb")
}

/// Directorio de recursos con los binarios empaquetados de Windows.
#[cfg(windows)]
fn dir_empaquetado() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/mdbtools/win-x86")
}

#[test]
fn mdbtools_lee_fixture() {
    // En Windows usamos los binarios empaquetados; en Mac/Linux, los del sistema.
    #[cfg(windows)]
    mdbtools::registrar_dir_empaquetado(dir_empaquetado());

    let mdb = fixture();
    assert!(
        mdb.exists(),
        "el fixture de prueba debe existir en {}",
        mdb.display()
    );

    let tablas = mdbtools::tablas(&mdb).expect("mdb-tables debe listar las tablas del fixture");
    assert!(
        tablas.iter().any(|t| t == "Table1"),
        "las tablas del fixture deben incluir 'Table1', pero fueron: {tablas:?}"
    );

    let csv = mdbtools::exportar_csv(&mdb, "Table1").expect("mdb-export debe exportar 'Table1'");
    let texto = decodificar_cp1252(&csv);
    assert!(
        texto.starts_with('A'),
        "el encabezado del CSV exportado debe empezar con 'A', pero empezó por: {:?}",
        texto.chars().take(20).collect::<String>()
    );
}

/// Verifica que los binarios empaquetados realmente arrancan (DLLs presentes).
///
/// Si faltara una DLL, el `.exe` no arrancaría y `disponible()` daría `false`.
#[cfg(windows)]
#[test]
fn mdbtools_disponible_empaquetado() {
    mdbtools::registrar_dir_empaquetado(dir_empaquetado());
    assert!(
        mdbtools::disponible(),
        "los binarios empaquetados deben arrancar (DLLs presentes)"
    );
}
