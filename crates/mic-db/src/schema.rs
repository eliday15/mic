//! Esquema SQLite base de un álbum MIC 3.0.
//!
//! Decisión de diseño (ver plan, sección "Esquema SQLite"): columnas reales por
//! campo configurable (NO EAV/JSON), porque el orden de 3 niveles y los filtros
//! por campo arbitrario sobre 10k–100k registros se benefician de índices B-tree
//! por columna. `ALTER TABLE ADD COLUMN` es barato y solo ocurre al editar la
//! estructura del álbum.
//!
//! Nombres físicos: el nombre visible elegido por el usuario ("Precio Venta")
//! nunca toca el DDL; en su lugar se usa la columna física `f_<id>` (ver
//! [`col_fisica`]). Esto evita por completo la inyección en sentencias DDL.

/// Versión del esquema que escribe [`crate::pool::AlbumDb::crear`] en `mic_album`.
pub const SCHEMA_VERSION: i64 = 1;

/// Nombre físico de la columna SQLite para el campo con identificador `id`.
///
/// Siempre `f_<id>`. Como `id` es un entero asignado por la base, el resultado
/// es un identificador SQL seguro (nunca contiene input del usuario).
pub fn col_fisica(id: i64) -> String {
    format!("f_{id}")
}

/// Migraciones idempotentes para álbumes creados con esquemas anteriores.
///
/// Se ejecuta al abrir cualquier álbum. Hoy: añade `campos.formato`
/// (presentación 'moneda' | 'porcentaje' | NULL) si la columna no existe.
pub fn migrar(conn: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
    let tiene_formato = conn
        .prepare("SELECT 1 FROM pragma_table_info('campos') WHERE name = 'formato'")?
        .exists([])?;
    if !tiene_formato {
        conn.execute_batch("ALTER TABLE campos ADD COLUMN formato TEXT;")?;
    }
    Ok(())
}

/// DDL base aplicado al crear un álbum nuevo.
///
/// Incluye:
/// - `mic_album`: pares clave/valor (schema_version, nombre, …).
/// - `campos`: metadatos de los campos configurables (ex tabla `propiedades`).
/// - `principal` / `variantes`: registros con columnas fijas `_id_`, `_imagen_`,
///   `_auxiliar_`, `_variantes_` / `_idprincipal_`; las columnas `f_<id>` se
///   añaden dinámicamente con `ALTER TABLE` desde `repo_campos`.
/// - `multidatos`, `categorias`, `grupos`, `filtros_av`, `ligados`, `reportes`.
/// - Índices de apoyo y la tabla FTS5 `principal_fts`.
///
/// Las columnas `_imagen_version_` guardan el `mtime` de la imagen para versionar
/// la URL de miniatura sin recalcular hashes en el frontend.
pub const DDL_BASE: &str = r#"
CREATE TABLE IF NOT EXISTS mic_album (
    clave TEXT PRIMARY KEY,
    valor TEXT
);

CREATE TABLE IF NOT EXISTS campos (
    id            INTEGER PRIMARY KEY,
    nombre        TEXT    NOT NULL,
    col_fisica    TEXT    NOT NULL,
    tabla         TEXT    NOT NULL DEFAULT 'principal',  -- 'principal' | 'variantes'
    tipo          INTEGER NOT NULL,                       -- 0..5 (TipoCampo)
    decimales     INTEGER NOT NULL DEFAULT 0,
    totalizable   INTEGER NOT NULL DEFAULT 0,
    formula       TEXT,
    visible       INTEGER NOT NULL DEFAULT 1,
    modificable   INTEGER NOT NULL DEFAULT 1,
    orden_visible INTEGER NOT NULL DEFAULT 0,
    formato       TEXT                                    -- presentación: 'moneda' | 'porcentaje' | NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS ix_campos_nombre_tabla ON campos (nombre, tabla);

CREATE TABLE IF NOT EXISTS principal (
    _id_              INTEGER PRIMARY KEY,
    _imagen_          TEXT,
    _imagen_version_  INTEGER,
    _auxiliar_        INTEGER NOT NULL DEFAULT 0,
    _variantes_       INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS variantes (
    _id_              INTEGER PRIMARY KEY,
    _idprincipal_     INTEGER NOT NULL REFERENCES principal(_id_) ON DELETE CASCADE,
    _imagen_          TEXT,
    _imagen_version_  INTEGER,
    _auxiliar_        INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS ix_variantes_principal ON variantes (_idprincipal_);

CREATE TABLE IF NOT EXISTS multidatos (
    reg_id    INTEGER NOT NULL,
    principal INTEGER NOT NULL,   -- 1 = tabla principal, 0 = variantes
    campo_id  INTEGER NOT NULL,
    valor     TEXT    NOT NULL
);
CREATE INDEX IF NOT EXISTS ix_multidatos_reg ON multidatos (reg_id, campo_id, principal);
CREATE INDEX IF NOT EXISTS ix_multidatos_valor ON multidatos (campo_id, valor);

CREATE TABLE IF NOT EXISTS categorias (
    campo_id  INTEGER NOT NULL,
    principal INTEGER NOT NULL,
    valor     TEXT    NOT NULL,
    es_default INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS ix_categorias ON categorias (campo_id, principal, valor);

CREATE TABLE IF NOT EXISTS grupos (
    id     INTEGER PRIMARY KEY,
    nombre TEXT NOT NULL UNIQUE,
    por    TEXT NOT NULL,
    luego1 TEXT,
    luego2 TEXT,
    status INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS filtros_av (
    nombre  TEXT    NOT NULL,
    orden   INTEGER NOT NULL,
    op_rel  TEXT,                 -- 'y' | 'o' | NULL (primera condición)
    campo   TEXT    NOT NULL,
    op_comp TEXT    NOT NULL,
    valor   TEXT    NOT NULL
);
CREATE INDEX IF NOT EXISTS ix_filtros_av ON filtros_av (nombre, orden);

CREATE TABLE IF NOT EXISTS ligados (
    id          INTEGER PRIMARY KEY,
    nombre      TEXT,
    config_json TEXT
);

CREATE TABLE IF NOT EXISTS reportes (
    id          INTEGER PRIMARY KEY,
    nombre      TEXT,
    config_json TEXT
);

-- FTS5 con almacenamiento propio (no contentless): permite DELETE por rowid sin
-- conocer el texto original, lo que simplifica el reindexado incremental. El
-- coste es guardar el texto concatenado, despreciable frente a la velocidad de
-- búsqueda sin acentos. El `rowid` coincide con `principal._id_`.
CREATE VIRTUAL TABLE IF NOT EXISTS principal_fts USING fts5(
    texto,
    tokenize='unicode61 remove_diacritics 2'
);
"#;
