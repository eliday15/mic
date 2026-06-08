//! mic-migrator: migración de álbumes Access/Jet (.mdb) a SQLite (.micdb).
//!
//! Usa mdbtools (mdb-tables / mdb-schema / mdb-export) vía shell-out,
//! decodifica Windows-1252 → UTF-8 y normaliza rutas de imágenes de
//! Windows (G:\MIC\imagenes\...) a relativas.

pub mod csv_parser;
pub mod mdbtools;
pub mod migrar;
pub mod paths;
pub mod type_map;

pub use migrar::{
    inspeccionar, migrar, parse_xms, MdbInspeccion, MigracionReporte, ProgresoMigracion,
};
