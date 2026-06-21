//! Diff de correctitud: `jet::leer_tabla` vs `mdb-export` (mdbtools del sistema).
//!
//! NO se ejecuta por defecto (`#[ignore]`): requiere mdbtools instalado y los
//! `.mdb` de prueba. Compara, tabla por tabla, que el lector pure-Rust (`jetdb`)
//! produce el MISMO número de filas y los mismos valores **parseados**
//! (números, fechas, booleanos) que el `mdb-export` clásico.
//!
//! Correr con:
//! ```sh
//! cargo test -p mic-migrator --test diff_mdbtools -- --ignored --nocapture
//! ```

use std::path::Path;
use std::process::Command;

use mic_migrator::csv_parser::{decodificar_cp1252, parsear_texto, TablaCsv};
use mic_migrator::jet;
use mic_migrator::type_map::parse_numero;

/// Lee una tabla con mdb-export (mismos flags que usaba el migrador: ISO + strip).
fn mdb_export(ruta: &Path, tabla: &str) -> TablaCsv {
    let salida = Command::new("mdb-export")
        .args([
            "-D",
            "%Y-%m-%d",
            "-b",
            "strip",
            &ruta.to_string_lossy(),
            tabla,
        ])
        .output()
        .expect("ejecutar mdb-export");
    assert!(
        salida.status.success(),
        "mdb-export falló en {tabla}: {}",
        String::from_utf8_lossy(&salida.stderr)
    );
    parsear_texto(&decodificar_cp1252(&salida.stdout)).expect("parsear CSV de mdb-export")
}

/// Lista las tablas de usuario con mdb-tables -1.
fn mdb_tablas(ruta: &Path) -> Vec<String> {
    let salida = Command::new("mdb-tables")
        .args(["-1", &ruta.to_string_lossy()])
        .output()
        .expect("ejecutar mdb-tables");
    decodificar_cp1252(&salida.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// Forma canónica de una celda para comparar jetdb vs mdb-export, tolerando las
/// diferencias de FORMATO (no de contenido):
/// - vacío → `""`.
/// - número (entero/decimal, coma o punto) → su `f64` redondeado.
/// - fecha ISO `YYYY-MM-DD...` → los primeros 10 caracteres.
/// - booleano (`-1`/`true`/`1` ↔ `0`/`false`) → `"1"`/`"0"`.
/// - resto → cadena recortada.
fn canonico(s: &str) -> String {
    let t = s.trim();
    if t.is_empty() {
        return String::new();
    }
    // Booleanos del estilo Access ("-1"/"True"/"0"/"False").
    match t.to_ascii_lowercase().as_str() {
        "true" | "-1" => return "1".into(),
        "false" => return "0".into(),
        _ => {}
    }
    // Números: compara por valor f64 (10.5 == 10.50 == 10,5).
    if let Some(n) = parse_numero(t) {
        // Redondeo a 4 decimales: Currency de Jet tiene escala 4.
        return format!("{:.4}", n);
    }
    // Fechas: jetdb emite ISO `YYYY-MM-DD`; mdb-export de Homebrew IGNORA el flag
    // `-D '%Y-%m-%d'` y emite su formato por defecto `M/D/YY HH:MM:SS`. Ambos
    // representan la MISMA fecha, así que normalizamos cualquiera de los dos a
    // ISO para comparar por VALOR, no por formato.
    let solo_fecha = t.split([' ', 'T']).next().unwrap_or(t);
    for fmt in ["%Y-%m-%d", "%m/%d/%y", "%m/%d/%Y", "%d/%m/%Y", "%d-%m-%Y"] {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(solo_fecha, fmt) {
            return d.format("%Y-%m-%d").to_string();
        }
    }
    t.to_string()
}

/// Compara una fila de jetdb contra la de mdb-export, celda a celda, en forma
/// canónica. Devuelve los desajustes encontrados como cadenas legibles.
fn diff_fila(tabla: &str, n: usize, jet: &[String], mdb: &[String]) -> Vec<String> {
    let ancho = jet.len().max(mdb.len());
    let mut difs = Vec::new();
    for c in 0..ancho {
        let vj = jet.get(c).map(|s| s.as_str()).unwrap_or("");
        let vm = mdb.get(c).map(|s| s.as_str()).unwrap_or("");
        let (cj, cm) = (canonico(vj), canonico(vm));
        if cj != cm {
            difs.push(format!(
                "{tabla} fila {n} col {c}: jet={vj:?} (canon {cj:?}) vs mdb={vm:?} (canon {cm:?})"
            ));
        }
    }
    difs
}

/// Ejecuta el diff completo de un `.mdb` y devuelve todas las diferencias.
fn diff_mdb(ruta: &Path) -> Vec<String> {
    let mut difs = Vec::new();

    let tablas_jet = jet::tablas(ruta).expect("jet::tablas");
    let tablas_mdb = mdb_tablas(ruta);
    // Mismo conjunto de tablas de usuario (orden aparte).
    let mut a = tablas_jet.clone();
    let mut b = tablas_mdb.clone();
    a.sort();
    b.sort();
    if a != b {
        difs.push(format!(
            "conjunto de tablas distinto: jet={tablas_jet:?} vs mdb={tablas_mdb:?}"
        ));
    }

    for tabla in &tablas_mdb {
        let leida = jet::leer_tabla(ruta, tabla).expect("jet::leer_tabla");
        let jet = &leida.csv;
        let mdb = mdb_export(ruta, tabla);

        // Mismo número de filas (jetdb puede omitir corruptas; estos fixtures
        // están sanos, así que debe coincidir exactamente).
        if jet.filas.len() != mdb.filas.len() {
            difs.push(format!(
                "{tabla}: jetdb dio {} fila(s), mdb-export dio {} (omitidas por jetdb: {})",
                jet.filas.len(),
                mdb.filas.len(),
                leida.omitidas
            ));
            continue;
        }
        // Misma cabecera (nombres de columna).
        if jet.cabecera != mdb.cabecera {
            difs.push(format!(
                "{tabla}: cabecera distinta: jet={:?} vs mdb={:?}",
                jet.cabecera, mdb.cabecera
            ));
        }
        for (n, (fj, fm)) in jet.filas.iter().zip(mdb.filas.iter()).enumerate() {
            difs.extend(diff_fila(tabla, n, fj, fm));
        }
    }
    difs
}

#[test]
#[ignore = "requiere mdbtools del sistema y los .mdb de prueba"]
fn jet_coincide_con_mdb_export() {
    let candidatos = [
        Path::new("/tmp/jet3.mdb"),
        Path::new("/tmp/jet4.mdb"),
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test.mdb"),
    ];

    let mut total_difs = Vec::new();
    let mut probados = 0;
    for ruta in candidatos {
        if !ruta.exists() {
            eprintln!("(omitido, no existe: {})", ruta.display());
            continue;
        }
        probados += 1;
        eprintln!("=== diff de {} ===", ruta.display());
        let difs = diff_mdb(ruta);
        if difs.is_empty() {
            eprintln!("  OK: idéntico a mdb-export");
        } else {
            for d in &difs {
                eprintln!("  DIFF: {d}");
            }
        }
        total_difs.extend(difs);
    }

    assert!(probados > 0, "no se probó ningún .mdb (¿faltan los fixtures?)");
    assert!(
        total_difs.is_empty(),
        "{} diferencia(s) entre jetdb y mdb-export:\n{}",
        total_difs.len(),
        total_difs.join("\n")
    );
}
