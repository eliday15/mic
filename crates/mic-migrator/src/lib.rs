//! mic-migrator: migración de álbumes Access/Jet (.mdb) a SQLite (.micdb).
//!
//! Lee los `.mdb` **en proceso** con el crate pure-Rust `jetdb` (sin
//! subprocesos, sin binarios ni DLLs), decodifica los textos a UTF-8 y
//! normaliza rutas de imágenes de Windows (G:\MIC\imagenes\...) a relativas.

pub mod csv_parser;
pub mod diag;
pub mod jet;
pub mod migrar;
pub mod paths;
pub mod type_map;

pub use migrar::{
    inspeccionar, migrar, parse_xms, MdbInspeccion, MigracionReporte, ProgresoMigracion,
};
