//! mic-inspect: arnés de diagnóstico de la importación de Access.
//!
//! Ejecuta la MISMA ruta de código que usa la app (inspeccionar/migrar) pero
//! como binario de consola, para poder probarla en Windows (o bajo Wine) sin
//! la interfaz. Imprime cada resultado, el tiempo, y la cola de la bitácora.
//!
//! Uso: `mic-inspect <ruta.mdb> [--migrar <destino.micdb>]`

use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("uso: mic-inspect <ruta.mdb> [--migrar <destino.micdb>]");
        std::process::exit(2);
    }
    let ruta = Path::new(&args[1]);
    println!(
        "mic-inspect v{} — inspeccionando {}",
        env!("CARGO_PKG_VERSION"),
        ruta.display()
    );

    let inicio = std::time::Instant::now();
    match mic_migrator::inspeccionar(ruta) {
        Ok(i) => {
            println!("INSPECCIÓN OK en {:.2?}", inicio.elapsed());
            println!("  tablas: {:?}", i.tablas);
            println!("  campos: {:?}", i.campos);
            println!("  total_estimado: {}", i.total_estimado);
            println!("  tiene_variantes: {}", i.tiene_variantes);
        }
        Err(e) => println!("INSPECCIÓN ERROR en {:.2?}: {e}", inicio.elapsed()),
    }

    if let Some(pos) = args.iter().position(|a| a == "--migrar") {
        if let Some(dest) = args.get(pos + 1) {
            println!("— migrando a {dest} —");
            let inicio = std::time::Instant::now();
            let progreso: mic_migrator::ProgresoMigracion =
                Box::new(|f, h, t| println!("  [{f}] {h}/{t}"));
            match mic_migrator::migrar(ruta, Path::new(dest), progreso) {
                Ok(r) => println!(
                    "MIGRACIÓN OK en {:.2?}: principal={} variantes={} multidatos={} \
                     imágenes={} advertencias={:?}",
                    inicio.elapsed(),
                    r.filas_principal,
                    r.filas_variantes,
                    r.filas_multidatos,
                    r.imagenes_encontradas,
                    r.advertencias
                ),
                Err(e) => println!("MIGRACIÓN ERROR en {:.2?}: {e}", inicio.elapsed()),
            }
        }
    }

    println!("bitácora: {}", mic_migrator::diag::ruta_log().display());
    if let Ok(log) = std::fs::read_to_string(mic_migrator::diag::ruta_log()) {
        println!("últimas líneas:");
        let lineas: Vec<&str> = log.lines().collect();
        for l in lineas.iter().rev().take(15).rev() {
            println!("  {l}");
        }
    }
}
