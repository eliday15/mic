//! Genera un álbum de demostración REAL para probar la app de extremo a extremo:
//! imágenes JPEG generadas, campos de todos los tipos, multidatos con
//! categorías, grupos jerárquicos y variantes.
//!
//! Crea `~/Documents/MIC-demo/demo.micdb` + `imagenes/` con 300 registros.
//!
//! ```sh
//! cargo run --release -p mic-db --example demo
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use image::{ImageBuffer, Rgb};
use mic_core::calc::MotorCalculo;
use mic_core::model::{CampoNuevo, Tabla, TipoCampo, Valor, Valores};
use mic_db::{pool::AlbumDb, repo_campos, repo_grupos, repo_registros};

/// Registros principales a crear.
const TOTAL: u32 = 300;
/// Los primeros N llevan 3 variantes cada uno.
const CON_VARIANTES: u32 = 10;

const CATEGORIAS: [&str; 6] = [
    "Bodega",
    "Vitrina",
    "Almacén",
    "Exhibición",
    "Tránsito",
    "Reserva",
];
const MARCAS: [&str; 8] = [
    "Aurora", "Bruma", "Cedro", "Delta", "Évora", "Faro", "Granito", "Helios",
];
const ETIQUETAS: [&str; 8] = [
    "nuevo", "oferta", "frágil", "premium", "importado", "temporada", "liquidación", "exclusivo",
];
const NOMBRES: [&str; 12] = [
    "Jarrón", "Lámpara", "Reloj", "Cuadro", "Florero", "Tetera", "Caja", "Marco",
    "Escultura", "Bandeja", "Candelabro", "Espejo",
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = dirs_demo();
    let ruta = dir.join("demo.micdb");
    let dir_imagenes = dir.join("imagenes");

    // Empezar de cero.
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir_imagenes)?;

    println!("Creando álbum demo en {} ...", ruta.display());
    let db = AlbumDb::crear(&ruta, "Demo MIC")?;

    crear_campos(&db)?;
    let campos = {
        let conn = db.conn()?;
        repo_campos::listar(&conn)?
    };
    let motor = MotorCalculo::new(&campos)?;

    // Grupo jerárquico: Categoría → Marca.
    {
        let conn = db.conn()?;
        repo_grupos::guardar(
            &conn,
            &mic_core::model::Grupo {
                id: 0,
                nombre: "Por Categoría".into(),
                por: "Categoria".into(),
                luego1: Some("Marca".into()),
                luego2: None,
            },
        )?;
    }

    // Registros con imagen real.
    let mut conn = db.conn()?;
    for i in 0..TOTAL {
        let nombre_img = format!("img_{i:03}.jpg");
        generar_jpeg(&dir_imagenes.join(&nombre_img), i, TOTAL, false)?;

        let mut v: Valores = HashMap::new();
        v.insert(
            "Nombre".into(),
            Valor::Texto(format!(
                "{} {}",
                NOMBRES[(i as usize) % NOMBRES.len()],
                i + 1
            )),
        );
        v.insert(
            "Categoria".into(),
            Valor::Texto(CATEGORIAS[(i as usize) % CATEGORIAS.len()].into()),
        );
        v.insert(
            "Marca".into(),
            Valor::Texto(MARCAS[(i as usize) % MARCAS.len()].into()),
        );
        v.insert("Precio".into(), Valor::Numero(((i % 90) * 25) as f64 + 49.5));
        v.insert("Cantidad".into(), Valor::Numero(((i % 12) + 1) as f64));
        let fecha = chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
            + chrono::Duration::days((i % 700) as i64);
        v.insert(
            "Vencimiento".into(),
            Valor::Texto(fecha.format("%Y-%m-%d").to_string()),
        );

        // 2-3 etiquetas multidato por registro.
        let mut md: HashMap<String, Vec<String>> = HashMap::new();
        let e1 = ETIQUETAS[(i as usize) % ETIQUETAS.len()].to_string();
        let e2 = ETIQUETAS[(i as usize + 3) % ETIQUETAS.len()].to_string();
        let mut lista = vec![e1, e2];
        if i % 3 == 0 {
            lista.push(ETIQUETAS[(i as usize + 5) % ETIQUETAS.len()].to_string());
        }
        md.insert("Etiquetas".into(), lista);

        let id = repo_registros::crear(
            &mut conn,
            &campos,
            Some(&motor),
            Tabla::Principal,
            &v,
            &md,
            Some(&format!("imagenes/{nombre_img}")),
            None,
            Some(&dir_imagenes),
        )?;

        // Variantes (colores) para los primeros CON_VARIANTES registros.
        if i < CON_VARIANTES {
            for nv in 0..3u32 {
                let img_var = format!("img_{i:03}_v{nv}.jpg");
                generar_jpeg(&dir_imagenes.join(&img_var), i * 7 + nv * 31, TOTAL, true)?;
                let mut vv: Valores = HashMap::new();
                vv.insert(
                    "Color".into(),
                    Valor::Texto(["Rojo", "Azul", "Verde"][nv as usize].into()),
                );
                vv.insert("Stock".into(), Valor::Numero((nv * 4 + 2) as f64));
                repo_registros::crear(
                    &mut conn,
                    &campos,
                    Some(&motor),
                    Tabla::Variantes,
                    &vv,
                    &HashMap::new(),
                    Some(&format!("imagenes/{img_var}")),
                    Some(id),
                    Some(&dir_imagenes),
                )?;
            }
        }

        if (i + 1) % 100 == 0 {
            println!("  {} / {TOTAL} registros…", i + 1);
        }
    }

    let total = repo_registros::total(&conn)?;
    println!("Listo: {total} registros principales en {}", ruta.display());
    println!("Ábrelo en MIC con: Abrir… → {}", ruta.display());
    Ok(())
}

/// Carpeta del álbum demo: `~/Documents/MIC-demo`.
fn dirs_demo() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME definido");
    Path::new(&home).join("Documents").join("MIC-demo")
}

/// Crea los campos del catálogo demo (todos los tipos + variantes).
fn crear_campos(db: &AlbumDb) -> Result<(), Box<dyn std::error::Error>> {
    let conn = db.conn()?;
    let mk = |nombre: &str, tabla: Tabla, tipo: TipoCampo, orden: i32| CampoNuevo {
        nombre: nombre.to_string(),
        tabla,
        tipo,
        decimales: 2,
        totalizable: matches!(tipo, TipoCampo::Numerico | TipoCampo::Moneda),
        formula: None,
        visible: true,
        modificable: true,
        orden_visible: orden,
        formato: None,
    };

    repo_campos::crear(&conn, &mk("Nombre", Tabla::Principal, TipoCampo::Texto, 0))?;
    repo_campos::crear(&conn, &mk("Categoria", Tabla::Principal, TipoCampo::Texto, 1))?;
    repo_campos::crear(&conn, &mk("Marca", Tabla::Principal, TipoCampo::Texto, 2))?;
    repo_campos::crear(&conn, &mk("Precio", Tabla::Principal, TipoCampo::Moneda, 3))?;
    repo_campos::crear(&conn, &mk("Cantidad", Tabla::Principal, TipoCampo::Numerico, 4))?;
    repo_campos::crear(&conn, &mk("Vencimiento", Tabla::Principal, TipoCampo::Fecha, 5))?;

    let mut total = mk("Total", Tabla::Principal, TipoCampo::Calculado, 6);
    total.formula = Some("Precio * Cantidad".to_string());
    repo_campos::crear(&conn, &total)?;

    repo_campos::crear(&conn, &mk("Etiquetas", Tabla::Principal, TipoCampo::Multidato, 7))?;

    // Campos de variantes.
    repo_campos::crear(&conn, &mk("Color", Tabla::Variantes, TipoCampo::Texto, 0))?;
    repo_campos::crear(&conn, &mk("Stock", Tabla::Variantes, TipoCampo::Numerico, 1))?;
    Ok(())
}

/// Genera un JPEG 640×480 con un degradado de tono distinto por índice, para
/// que cada miniatura sea visualmente distinguible en la grilla.
fn generar_jpeg(
    destino: &Path,
    indice: u32,
    total: u32,
    variante: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let (w, h) = (640u32, 480u32);
    let matiz = (indice as f64 / total.max(1) as f64) * 360.0;
    let img = ImageBuffer::from_fn(w, h, |x, y| {
        let fx = x as f64 / w as f64;
        let fy = y as f64 / h as f64;
        // Degradado diagonal con saturación según fila; variantes más claras.
        let s = if variante { 0.45 } else { 0.85 };
        let l = 0.30 + 0.40 * (1.0 - fy) + 0.15 * fx;
        let (r, g, b) = hsl_a_rgb(matiz, s, l.min(0.92));
        Rgb([r, g, b])
    });
    img.save(destino)?;
    Ok(())
}

/// Conversión HSL → RGB (h en grados, s/l en [0,1]).
fn hsl_a_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = (h % 360.0) / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    (
        ((r1 + m) * 255.0) as u8,
        ((g1 + m) * 255.0) as u8,
        ((b1 + m) * 255.0) as u8,
    )
}
