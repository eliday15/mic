//! Estado global de la aplicación: álbumes abiertos en memoria.
//!
//! Cada álbum abierto se identifica con un `album_id` numérico (asignado al
//! abrir/crear) y se guarda como un [`AlbumHandle`] compartido (`Arc`) en el
//! mapa [`AppState::albums`]. El handle agrupa el pool de la base
//! ([`AlbumDb`]), la lista de campos cacheada, el motor de campos calculados
//! ([`MotorCalculo`]) y la caché de miniaturas ([`ThumbCache`]).
//!
//! La lista de campos y el motor viven tras `RwLock` porque cambian al
//! crear/editar/eliminar campos; el resto de comandos solo lee. El `db` y los
//! `thumbs` son internamente `Sync`/clonables, así que no necesitan lock.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use mic_core::error::MicError;
use mic_core::model::CampoDef;
use mic_core::calc::MotorCalculo;
use mic_db::AlbumDb;
use mic_thumbs::ThumbCache;

/// Un álbum abierto: base de datos, campos, motor de cálculo y caché de thumbs.
///
/// Se comparte como `Arc<AlbumHandle>` entre los comandos. El motor es
/// `Option` porque un álbum puede no tener ningún campo calculado (en cuyo caso
/// no hace falta compilar nada y se evita el coste).
pub struct AlbumHandle {
    /// Pool de conexiones SQLite del álbum (clonable, internamente `Arc`).
    pub db: AlbumDb,
    /// Definición de campos del álbum, cacheada y refrescada al editar estructura.
    pub campos: RwLock<Vec<CampoDef>>,
    /// Motor de campos calculados (recompilado al crear/editar/eliminar campos).
    pub motor: RwLock<Option<MotorCalculo>>,
    /// Caché de miniaturas asociada al directorio del álbum.
    pub thumbs: ThumbCache,
}

impl AlbumHandle {
    /// Construye un handle a partir de una base abierta, leyendo sus campos y
    /// compilando el motor de cálculo si hay fórmulas.
    pub fn nuevo(db: AlbumDb) -> Result<Self, MicError> {
        let conn = db.conn()?;
        let campos = mic_db::repo_campos::listar(&conn)?;
        drop(conn);
        let motor = construir_motor(&campos)?;
        let thumbs = ThumbCache::new(dir_album(db.ruta()));
        Ok(Self {
            db,
            campos: RwLock::new(campos),
            motor: RwLock::new(motor),
            thumbs,
        })
    }

    /// Carpeta donde residen las imágenes del álbum (`<dir>/imagenes`).
    pub fn dir_imagenes(&self) -> PathBuf {
        self.db.dir_imagenes()
    }

    /// Refresca la lista de campos cacheada y recompila el motor de cálculo.
    ///
    /// Se llama tras cualquier cambio de estructura (crear/editar/eliminar/
    /// reordenar campos) para que las consultas posteriores vean la nueva forma.
    pub fn refrescar_campos(&self) -> Result<(), MicError> {
        let conn = self.db.conn()?;
        let nuevos = mic_db::repo_campos::listar(&conn)?;
        drop(conn);
        let motor = construir_motor(&nuevos)?;
        *self.campos.write().unwrap_or_else(|e| e.into_inner()) = nuevos;
        *self.motor.write().unwrap_or_else(|e| e.into_inner()) = motor;
        Ok(())
    }

    /// Clona la lista de campos vigente (para pasarla a los repos sin retener el lock).
    pub fn campos(&self) -> Vec<CampoDef> {
        self.campos
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

/// Compila el motor de cálculo si el álbum tiene al menos un campo calculado.
fn construir_motor(campos: &[CampoDef]) -> Result<Option<MotorCalculo>, MicError> {
    let hay_calculados = campos
        .iter()
        .any(|c| matches!(c.tipo, mic_core::model::TipoCampo::Calculado));
    if hay_calculados {
        Ok(Some(MotorCalculo::new(campos)?))
    } else {
        Ok(None)
    }
}

/// Directorio que contiene el archivo del álbum (raíz de `imagenes/` y `.thumbs/`).
fn dir_album(ruta: &Path) -> &Path {
    ruta.parent().unwrap_or_else(|| Path::new("."))
}

/// Estado global compartido de la app (gestionado por Tauri con `.manage`).
///
/// Mantiene el mapa de álbumes abiertos y el contador para asignar nuevos
/// `album_id`. El acceso es concurrente: el mapa está tras `RwLock` y el
/// contador es atómico.
pub struct AppState {
    /// Álbumes abiertos por id de sesión.
    pub albums: RwLock<HashMap<u64, Arc<AlbumHandle>>>,
    /// Próximo id de álbum a asignar (empieza en 1).
    pub next_id: AtomicU64,
}

impl AppState {
    /// Crea el estado vacío inicial.
    pub fn nuevo() -> Self {
        Self {
            albums: RwLock::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        }
    }

    /// Registra un álbum recién abierto y devuelve su id asignado.
    pub fn registrar(&self, handle: AlbumHandle) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        self.albums
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id, Arc::new(handle));
        id
    }

    /// Obtiene el handle de un álbum abierto por id (clon del `Arc`).
    pub fn obtener(&self, album_id: u64) -> Result<Arc<AlbumHandle>, String> {
        self.albums
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(&album_id)
            .cloned()
            .ok_or_else(|| format!("álbum {album_id} no está abierto"))
    }

    /// Cierra un álbum (lo elimina del mapa). No falla si ya no estaba.
    pub fn cerrar(&self, album_id: u64) {
        self.albums
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&album_id);
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::nuevo()
    }
}
