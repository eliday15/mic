//! Smoke test sin GUI de la capa de persistencia de MIC 3.0.
//!
//! Crea `/tmp/mic_seed.micdb` con cinco campos (uno por tipo configurable:
//! texto, numérico, moneda, fecha y calculado) y siembra 100 000 registros.
//! Luego cronometra una consulta paginada con filtro + orden de 3 niveles
//! (el caso de peor rendimiento de la grilla virtual) y reporta el tiempo.
//!
//! Ejecutar en release (el modo debug es mucho más lento):
//! ```sh
//! cargo run --release -p mic-db --example seed
//! ```
//!
//! Objetivo de latencia de la consulta caliente: < 50 ms sobre 100 000 filas.

use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use mic_core::calc::MotorCalculo;
use mic_core::model::{
    CampoNuevo, CondicionFiltro, Direccion, OpComp, OrdenCampo, QueryReq, Tabla, TipoCampo, Valor,
    Valores,
};
use mic_db::{pool::AlbumDb, repo_campos, repo_registros};
use rusqlite::params;

/// Número de registros a sembrar.
const TOTAL: u32 = 100_000;
/// Cuántas categorías distintas (controla la selectividad del filtro).
const CATEGORIAS: u32 = 50;
/// Cuántas marcas distintas (segundo nivel de orden).
const MARCAS: u32 = 200;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ruta = Path::new("/tmp/mic_seed.micdb");

    // Empezar de cero: borra el álbum anterior y sus artefactos WAL/SHM.
    limpiar_artefactos(ruta);

    println!("Creando álbum en {} ...", ruta.display());
    let db = AlbumDb::crear(ruta, "Semilla")?;

    crear_campos(&db)?;
    let campos = {
        let conn = db.conn()?;
        repo_campos::listar(&conn)?
    };
    let motor = MotorCalculo::new(&campos)?;

    // 1) Inserción de una muestra a través de la API pública real
    //    (`repo_registros::crear`), que recalcula el campo calculado dentro de
    //    su transacción. Verifica que el camino completo funciona end-to-end.
    sembrar_muestra_por_api(&db, &campos, &motor)?;

    // 2) Carga masiva del resto en una sola transacción (rápida) para alcanzar
    //    las 100 000 filas. El valor calculado se computa con el mismo motor y
    //    se persiste igual que haría `crear`, manteniendo paridad de datos.
    sembrar_resto_masivo(&db, &campos, &motor)?;

    {
        let conn = db.conn()?;
        let total = repo_registros::total(&conn)?;
        assert_eq!(total, TOTAL as u64, "se esperaban {TOTAL} registros");
        println!("Sembrados {total} registros.");
    }

    // 3) Consulta cronometrada: filtro por categoría + orden de 3 niveles.
    cronometrar_consulta(&db, &campos)?;

    Ok(())
}

/// Borra el archivo del álbum y los artefactos `-wal`/`-shm` de una corrida previa.
fn limpiar_artefactos(ruta: &Path) {
    for sufijo in ["", "-wal", "-shm"] {
        let p = if sufijo.is_empty() {
            ruta.to_path_buf()
        } else {
            let mut s = ruta.as_os_str().to_owned();
            s.push(sufijo);
            std::path::PathBuf::from(s)
        };
        let _ = std::fs::remove_file(&p);
    }
}

/// Crea los cinco campos del catálogo (uno por tipo de dato).
///
/// El campo calculado `Total = Precio * Cantidad` ejercita el motor de cálculo.
/// El campo `Vencimiento` (fecha) cubre el tipo fecha aunque no participe en la
/// consulta cronometrada.
fn crear_campos(db: &AlbumDb) -> Result<(), Box<dyn std::error::Error>> {
    let conn = db.conn()?;
    repo_campos::crear(&conn, &campo("Categoria", TipoCampo::Texto))?;
    repo_campos::crear(&conn, &campo("Precio", TipoCampo::Moneda))?;
    repo_campos::crear(&conn, &campo("Cantidad", TipoCampo::Numerico))?;
    repo_campos::crear(&conn, &campo("Vencimiento", TipoCampo::Fecha))?;

    let mut total = campo("Total", TipoCampo::Calculado);
    total.formula = Some("Precio * Cantidad".to_string());
    repo_campos::crear(&conn, &total)?;

    // Un segundo campo de texto para el segundo nivel de orden.
    repo_campos::crear(&conn, &campo("Marca", TipoCampo::Texto))?;
    Ok(())
}

/// Construye un `CampoNuevo` principal con presentación a 2 decimales.
fn campo(nombre: &str, tipo: TipoCampo) -> CampoNuevo {
    CampoNuevo {
        nombre: nombre.to_string(),
        tabla: Tabla::Principal,
        tipo,
        decimales: 2,
        totalizable: matches!(tipo, TipoCampo::Numerico | TipoCampo::Moneda),
        formula: None,
        visible: true,
        modificable: true,
        orden_visible: 0,
        formato: None,
    }
}

/// Inserta los primeros 100 registros vía `repo_registros::crear`, ejercitando
/// el camino real (recálculo del calculado, FTS, índices) registro a registro.
fn sembrar_muestra_por_api(
    db: &AlbumDb,
    campos: &[mic_core::model::CampoDef],
    motor: &MotorCalculo,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = db.conn()?;
    for i in 0..100u32 {
        let (cat, marca, precio, cant, fecha) = fila(i);
        let mut v: Valores = HashMap::new();
        v.insert("Categoria".into(), Valor::Texto(cat));
        v.insert("Marca".into(), Valor::Texto(marca));
        v.insert("Precio".into(), Valor::Numero(precio));
        v.insert("Cantidad".into(), Valor::Numero(cant));
        v.insert("Vencimiento".into(), Valor::Texto(fecha));
        repo_registros::crear(
            &mut conn,
            campos,
            Some(motor),
            Tabla::Principal,
            &v,
            &HashMap::new(),
            None,
            None,
            None,
        )?;
    }
    Ok(())
}

/// Carga masiva del resto de registros (desde el índice 100 hasta `TOTAL`) en
/// una única transacción con INSERTs preparados, computando el calculado con el
/// mismo motor para mantener paridad con `repo_registros::crear`.
fn sembrar_resto_masivo(
    db: &AlbumDb,
    campos: &[mic_core::model::CampoDef],
    motor: &MotorCalculo,
) -> Result<(), Box<dyn std::error::Error>> {
    let col = |nombre: &str| -> String {
        campos
            .iter()
            .find(|c| c.nombre == nombre)
            .expect("campo presente")
            .col_fisica
            .clone()
    };
    let f_cat = col("Categoria");
    let f_marca = col("Marca");
    let f_precio = col("Precio");
    let f_cant = col("Cantidad");
    let f_venc = col("Vencimiento");
    let f_total = col("Total");

    let mut conn = db.conn()?;
    let tx = conn.transaction()?;
    {
        let sql = format!(
            "INSERT INTO principal ({f_cat}, {f_marca}, {f_precio}, {f_cant}, {f_venc}, {f_total}) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
        );
        let mut stmt = tx.prepare(&sql)?;
        for i in 100..TOTAL {
            let (cat, marca, precio, cant, fecha) = fila(i);
            // Calcula `Total` con el motor real, igual que haría `crear`.
            let mut v: Valores = HashMap::new();
            v.insert("Precio".into(), Valor::Numero(precio));
            v.insert("Cantidad".into(), Valor::Numero(cant));
            let total = motor
                .evaluar("Total", &v)?
                .como_f64()
                .unwrap_or(0.0);
            stmt.execute(params![cat, marca, precio, cant, fecha, total])?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// Genera los datos sintéticos de la fila `i` (deterministas).
fn fila(i: u32) -> (String, String, f64, f64, String) {
    let cat = format!("Cat{}", i % CATEGORIAS);
    let marca = format!("Marca{}", i % MARCAS);
    let precio = (i % 1000) as f64 + 0.5;
    let cant = (i % 30) as f64;
    // Fecha ISO entre 2020-01-01 y ~2022 (días incrementales).
    let dia = chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()
        + chrono::Duration::days((i % 1000) as i64);
    (cat, marca, precio, cant, dia.format("%Y-%m-%d").to_string())
}

/// Ejecuta y cronometra la consulta de peor caso (filtro + orden de 3 niveles).
fn cronometrar_consulta(
    db: &AlbumDb,
    campos: &[mic_core::model::CampoDef],
) -> Result<(), Box<dyn std::error::Error>> {
    let conn = db.conn()?;
    let req = QueryReq {
        tabla: Tabla::Principal,
        id_principal: None,
        grupo: None,
        filtro_rapido: None,
        condiciones: vec![CondicionFiltro {
            op_rel: None,
            campo: "Categoria".into(),
            op_comp: OpComp::Igual,
            valor: "Cat7".into(),
        }],
        busqueda: None,
        orden: vec![
            OrdenCampo {
                campo: "Marca".into(),
                direccion: Direccion::Asc,
            },
            OrdenCampo {
                campo: "Precio".into(),
                direccion: Direccion::Desc,
            },
            OrdenCampo {
                campo: "Cantidad".into(),
                direccion: Direccion::Asc,
            },
        ],
        incluir_ocultos: false,
        offset: 0,
        limit: 100,
    };

    // Calentamiento: carga páginas/índices en caché de SQLite.
    let _ = repo_registros::query(&conn, campos, &req)?;

    // Mejor de 5 corridas (refleja la latencia caliente que ve la UI).
    let mut mejor = std::time::Duration::MAX;
    let mut total = 0u64;
    let mut filas = 0usize;
    for _ in 0..5 {
        let inicio = Instant::now();
        let page = repo_registros::query(&conn, campos, &req)?;
        let dur = inicio.elapsed();
        if dur < mejor {
            mejor = dur;
        }
        total = page.total;
        filas = page.registros.len();
    }

    let esperado = TOTAL as u64 / CATEGORIAS as u64; // 100000 / 50 = 2000
    assert_eq!(total, esperado, "total filtrado inesperado");
    assert_eq!(filas, 100, "la página debería traer 100 filas");

    println!(
        "registros_query (filtro=Categoria=Cat7, orden 3 niveles) sobre {TOTAL} filas:"
    );
    println!("  total coincidente = {total}, página = {filas} filas");
    println!("  mejor de 5 = {:.3} ms", mejor.as_secs_f64() * 1000.0);

    let ms = mejor.as_secs_f64() * 1000.0;
    if ms < 50.0 {
        println!("  OK: por debajo del objetivo de 50 ms.");
    } else {
        println!("  AVISO: por encima del objetivo de 50 ms.");
    }
    Ok(())
}
