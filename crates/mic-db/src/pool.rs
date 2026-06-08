//! Pool de conexiones SQLite para un álbum `.micdb`.
//!
//! Un álbum = un archivo SQLite en modo WAL. Cada conexión recibe los mismos
//! PRAGMAs al abrirse (vía el `customize` del manager), de modo que el pool
//! reparte conexiones equivalentes. Reemplaza al DAO/Jet del original, además de
//! eliminar los lock-files `~$*.dbl` y `micserver.xml` (WAL gestiona la
//! concurrencia lectura/escritura).

use std::path::{Path, PathBuf};

use mic_core::error::MicError;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;

use crate::schema::{DDL_BASE, SCHEMA_VERSION};

/// Conexión tomada del pool. Tipo público estable para el resto de repos.
pub type Conn = r2d2::PooledConnection<SqliteConnectionManager>;

/// Convierte cualquier error de r2d2 en [`MicError::Db`].
fn err_pool<E: std::fmt::Display>(e: E) -> MicError {
    MicError::Db(e.to_string())
}

/// Convierte cualquier error de rusqlite en [`MicError::Db`].
pub(crate) fn err_sql(e: rusqlite::Error) -> MicError {
    MicError::Db(e.to_string())
}

/// PRAGMAs aplicados a cada conexión nueva del pool.
///
/// - `journal_mode=WAL`: concurrencia lectura/escritura sin lock-files.
/// - `synchronous=NORMAL`: durabilidad suficiente en WAL, mucho más rápido.
/// - `foreign_keys=ON`: cascada de variantes al borrar un principal.
/// - `temp_store=MEMORY`, `cache_size=-64000` (64 MiB), `mmap_size=256 MiB`:
///   aprovechar la RAM generosamente (decisión del usuario).
fn aplicar_pragmas(conn: &Connection) -> Result<(), rusqlite::Error> {
    // journal_mode devuelve una fila ("wal"); usamos query para consumirla.
    conn.pragma_update(None, "journal_mode", "WAL")
        .or_else(|_| conn.query_row("PRAGMA journal_mode=WAL", [], |_| Ok(())))?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    conn.pragma_update(None, "cache_size", -64000_i64)?;
    conn.pragma_update(None, "mmap_size", 268435456_i64)?;
    Ok(())
}

/// Un álbum abierto: pool de conexiones + ruta del archivo.
///
/// Clonable a bajo costo (el pool interno es `Arc`), de modo que mic-tauri puede
/// guardarlo por sesión y repartirlo a los comandos.
#[derive(Clone)]
pub struct AlbumDb {
    pool: r2d2::Pool<SqliteConnectionManager>,
    ruta: PathBuf,
}

impl AlbumDb {
    /// Construye el pool sobre `ruta`, aplicando los PRAGMAs a cada conexión.
    fn construir_pool(ruta: &Path) -> Result<r2d2::Pool<SqliteConnectionManager>, MicError> {
        let manager = SqliteConnectionManager::file(ruta).with_init(|c| {
            aplicar_pragmas(c)?;
            Ok(())
        });
        r2d2::Pool::builder()
            .max_size(8)
            .build(manager)
            .map_err(err_pool)
    }

    /// Crea un álbum nuevo en `ruta`: aplica el DDL base, guarda `nombre` y
    /// `schema_version=1` en `mic_album`, y prepara la carpeta de imágenes.
    ///
    /// Falla con [`MicError::Invalido`] si el archivo ya existe (no se sobreescribe).
    pub fn crear(ruta: &Path, nombre: &str) -> Result<AlbumDb, MicError> {
        if ruta.exists() {
            return Err(MicError::Invalido(format!(
                "el archivo ya existe: {}",
                ruta.display()
            )));
        }
        if let Some(dir) = ruta.parent() {
            std::fs::create_dir_all(dir)?;
        }

        let pool = Self::construir_pool(ruta)?;
        {
            let conn = pool.get().map_err(err_pool)?;
            conn.execute_batch(DDL_BASE).map_err(err_sql)?;
            conn.execute(
                "INSERT INTO mic_album (clave, valor) VALUES ('schema_version', ?1)",
                [SCHEMA_VERSION.to_string()],
            )
            .map_err(err_sql)?;
            conn.execute(
                "INSERT INTO mic_album (clave, valor) VALUES ('nombre', ?1)",
                [nombre],
            )
            .map_err(err_sql)?;
        }

        let db = AlbumDb {
            pool,
            ruta: ruta.to_path_buf(),
        };
        db.asegurar_dir_imagenes()?;
        Ok(db)
    }

    /// Abre un álbum existente. Falla con [`MicError::NoEncontrado`] si no existe.
    pub fn abrir(ruta: &Path) -> Result<AlbumDb, MicError> {
        if !ruta.exists() {
            return Err(MicError::NoEncontrado(format!(
                "álbum no encontrado: {}",
                ruta.display()
            )));
        }
        let pool = Self::construir_pool(ruta)?;
        // Validamos que abre al menos una conexión (PRAGMAs aplicados) y
        // aplicamos las migraciones idempotentes de esquema.
        {
            let conn = pool.get().map_err(err_pool)?;
            crate::schema::migrar(&conn).map_err(err_sql)?;
        }
        let db = AlbumDb {
            pool,
            ruta: ruta.to_path_buf(),
        };
        db.asegurar_dir_imagenes()?;
        Ok(db)
    }

    /// Toma una conexión del pool (PRAGMAs ya aplicados).
    pub fn conn(&self) -> Result<Conn, MicError> {
        self.pool.get().map_err(err_pool)
    }

    /// Ruta del archivo `.micdb`.
    pub fn ruta(&self) -> &Path {
        &self.ruta
    }

    /// Carpeta `imagenes` junto al archivo del álbum. La crea si no existe.
    pub fn dir_imagenes(&self) -> PathBuf {
        let dir = self
            .ruta
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
            .join("imagenes");
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    /// Versión interna que propaga el error de E/S al crear/abrir.
    fn asegurar_dir_imagenes(&self) -> Result<(), MicError> {
        let dir = self
            .ruta
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
            .join("imagenes");
        std::fs::create_dir_all(&dir)?;
        Ok(())
    }

    /// Compacta el archivo (reclama espacio tras borrados masivos).
    pub fn vacuum(&self) -> Result<(), MicError> {
        let conn = self.conn()?;
        conn.execute_batch("VACUUM;").map_err(err_sql)?;
        Ok(())
    }
}
