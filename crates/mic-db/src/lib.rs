//! mic-db: capa de persistencia SQLite de MIC 3.0.
//!
//! Un álbum = un archivo `.micdb` (SQLite WAL). Esquema con columnas reales
//! por campo configurable (ver schema.rs). Reemplaza DAO/Jet del original.

pub mod fts;
pub mod pool;
pub mod query_builder;
pub mod repo_campos;
pub mod repo_categorias;
pub mod repo_filtros;
pub mod repo_grupos;
pub mod repo_ligados;
pub mod repo_multidatos;
pub mod repo_registros;
pub mod repo_reportes;
pub mod schema;

pub use pool::AlbumDb;
