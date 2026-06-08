//! Caché de miniaturas en disco y en RAM.
//!
//! Las miniaturas se guardan en `<dir_album>/.thumbs/<size>/<hash>.jpg`, donde
//! `hash` deriva de `(ruta_absoluta, mtime, size)`. Como el mtime forma parte
//! del hash, al modificarse la imagen original cambia la ruta del thumb y la
//! versión antigua queda obsoleta automáticamente (invalidación por contenido).
//!
//! Sobre el disco se monta una caché en RAM (`HashMap` tras `RwLock`) que evita
//! repetir `stat` de la imagen y del thumb una vez resuelta una combinación
//! `(ruta, size)`. La generación concurrente es segura: cada hilo escribe a un
//! archivo temporal único y lo renombra de forma atómica sobre el destino, por
//! lo que dos hilos pueden generar el mismo thumb sin corromperlo.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

use mic_core::error::MicError;

use crate::generator;

/// Nombre del subdirectorio de miniaturas dentro del álbum.
const DIR_THUMBS: &str = ".thumbs";

/// Contador global de procesos para nombres de archivos temporales únicos.
static CONTADOR_TMP: AtomicU64 = AtomicU64::new(0);

/// Caché de miniaturas asociada a un directorio de álbum.
///
/// Es segura para uso concurrente: [`ThumbCache::obtener`] puede invocarse
/// desde varios hilos (p. ej. el protocolo `thumb://`).
pub struct ThumbCache {
    /// Directorio raíz de miniaturas: `<dir_album>/.thumbs`.
    dir_thumbs: PathBuf,
    /// Caché en RAM: `(ruta_absoluta, size)` → ruta del thumb ya resuelta.
    ram: RwLock<HashMap<(String, u32), PathBuf>>,
}

impl ThumbCache {
    /// Crea una caché que almacena las miniaturas bajo
    /// `<dir_album>/.thumbs/<size>/`.
    pub fn new(dir_album: &Path) -> Self {
        Self {
            dir_thumbs: dir_album.join(DIR_THUMBS),
            ram: RwLock::new(HashMap::new()),
        }
    }

    /// Devuelve la ruta de la miniatura de `imagen_abs` al tamaño `size`.
    ///
    /// * `size == 0` devuelve la propia ruta original (tamaño completo: visor).
    /// * Acierto en RAM → ruta cacheada sin tocar el disco.
    /// * Acierto en disco → ruta del `.jpg` existente.
    /// * Fallo → genera la miniatura con [`generator::generar`] y la devuelve.
    ///
    /// El nombre del thumb incluye el mtime de la imagen, de modo que al
    /// cambiar la imagen original el thumb anterior deja de usarse.
    ///
    /// # Errores
    /// [`MicError::NoEncontrado`] si la imagen original no existe;
    /// [`MicError::Io`] ante fallos de E/S y los errores propagados por el
    /// generador.
    pub fn obtener(&self, imagen_abs: &Path, size: u32) -> Result<PathBuf, MicError> {
        // size = 0: original a tamaño completo.
        if size == 0 {
            if imagen_abs.exists() {
                return Ok(imagen_abs.to_path_buf());
            }
            return Err(MicError::NoEncontrado(format!(
                "imagen no encontrada: '{}'",
                imagen_abs.display()
            )));
        }

        let clave_ruta = clave_ruta(imagen_abs);
        let clave = (clave_ruta.clone(), size);

        // 1. Acierto en RAM: validamos que el archivo siga existiendo.
        if let Some(ruta) = self.leer_ram(&clave) {
            if ruta.exists() {
                return Ok(ruta);
            }
            // El thumb fue borrado fuera de la caché: limpiamos y regeneramos.
            self.eliminar_ram(&clave);
        }

        // 2. Calcular el hash a partir de (ruta, mtime, size).
        let mtime = mtime_de(imagen_abs)?;
        let hash = calcular_hash(&clave_ruta, mtime, size);
        let dir_size = self.dir_thumbs.join(size.to_string());
        let destino = dir_size.join(format!("{hash}.jpg"));

        // 3. Acierto en disco.
        if destino.exists() {
            self.escribir_ram(clave, destino.clone());
            return Ok(destino);
        }

        // 4. Fallo: generar. Escribimos a un temporal y renombramos atómicamente
        //    para tolerar generación concurrente del mismo thumb.
        fs::create_dir_all(&dir_size).map_err(|e| {
            MicError::Io(format!(
                "no se pudo crear '{}': {e}",
                dir_size.display()
            ))
        })?;

        let tmp = ruta_temporal(&dir_size, hash);
        match generator::generar(imagen_abs, &tmp, size) {
            Ok(()) => {}
            Err(e) => {
                // Limpieza best-effort del temporal a medio escribir.
                let _ = fs::remove_file(&tmp);
                return Err(e);
            }
        }

        // rename atómico: si otro hilo llegó primero, ambos quedan con un .jpg
        // válido (idéntico salvo metadatos) en `destino`.
        if let Err(e) = fs::rename(&tmp, &destino) {
            let _ = fs::remove_file(&tmp);
            // Si el destino ya existe (otro hilo ganó la carrera) lo damos por bueno.
            if !destino.exists() {
                return Err(MicError::Io(format!(
                    "no se pudo renombrar a '{}': {e}",
                    destino.display()
                )));
            }
        }

        self.escribir_ram(clave, destino.clone());
        Ok(destino)
    }

    /// Invalida todas las miniaturas de `imagen_abs`: borra sus entradas en RAM
    /// y elimina del disco los `.jpg` que le correspondan en todos los tamaños.
    ///
    /// No falla si la imagen ya no existe; en ese caso solo limpia la RAM, ya
    /// que sin mtime no se puede reconstruir el hash de los archivos en disco.
    pub fn invalidar(&self, imagen_abs: &Path) {
        let clave_ruta = clave_ruta(imagen_abs);

        // RAM: eliminar todas las entradas de esa imagen (cualquier size).
        {
            let mut ram = self.ram.write().unwrap_or_else(|e| e.into_inner());
            ram.retain(|(r, _), _| r != &clave_ruta);
        }

        // Disco: el hash depende del mtime actual. Si la imagen existe, borramos
        // el thumb del mtime vigente en cada subdirectorio de tamaño.
        if let Ok(mtime) = mtime_de(imagen_abs) {
            self.borrar_thumbs_de(&clave_ruta, mtime);
        }
    }

    // -- Helpers de disco ---------------------------------------------------

    /// Borra de cada `<dir_thumbs>/<size>/` el `.jpg` cuyo hash corresponde a
    /// `(ruta, mtime, size)`.
    fn borrar_thumbs_de(&self, clave_ruta: &str, mtime: i64) {
        let entradas = match fs::read_dir(&self.dir_thumbs) {
            Ok(e) => e,
            Err(_) => return, // No hay .thumbs todavía: nada que borrar.
        };

        for entrada in entradas.flatten() {
            let dir_size = entrada.path();
            if !dir_size.is_dir() {
                continue;
            }
            // El nombre del subdirectorio es el tamaño.
            let size: u32 = match entrada.file_name().to_str().and_then(|s| s.parse().ok()) {
                Some(s) => s,
                None => continue,
            };
            let hash = calcular_hash(clave_ruta, mtime, size);
            let archivo = dir_size.join(format!("{hash}.jpg"));
            let _ = fs::remove_file(archivo);
        }
    }

    // -- Helpers de RAM -----------------------------------------------------

    fn leer_ram(&self, clave: &(String, u32)) -> Option<PathBuf> {
        let ram = self.ram.read().unwrap_or_else(|e| e.into_inner());
        ram.get(clave).cloned()
    }

    fn escribir_ram(&self, clave: (String, u32), ruta: PathBuf) {
        let mut ram = self.ram.write().unwrap_or_else(|e| e.into_inner());
        ram.insert(clave, ruta);
    }

    fn eliminar_ram(&self, clave: &(String, u32)) {
        let mut ram = self.ram.write().unwrap_or_else(|e| e.into_inner());
        ram.remove(clave);
    }
}

/// Clave de caché estable para una ruta: forma canónica si es posible, o la
/// ruta tal cual en su representación de texto en otro caso.
fn clave_ruta(imagen_abs: &Path) -> String {
    match fs::canonicalize(imagen_abs) {
        Ok(c) => c.to_string_lossy().into_owned(),
        Err(_) => imagen_abs.to_string_lossy().into_owned(),
    }
}

/// Obtiene el mtime de un archivo como segundos desde UNIX_EPOCH.
fn mtime_de(imagen_abs: &Path) -> Result<i64, MicError> {
    let meta = fs::metadata(imagen_abs).map_err(|_| {
        MicError::NoEncontrado(format!(
            "imagen no encontrada: '{}'",
            imagen_abs.display()
        ))
    })?;
    let modificado = meta.modified().map_err(|e| {
        MicError::Io(format!(
            "no se pudo leer mtime de '{}': {e}",
            imagen_abs.display()
        ))
    })?;
    let dur = modificado
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        // mtime anterior a la época: usamos el negativo de la diferencia.
        .unwrap_or_else(|e| -(e.duration().as_secs() as i64));
    Ok(dur)
}

/// Calcula el hash estable de `(ruta, mtime, size)` en hexadecimal.
///
/// Usa [`DefaultHasher`], que es estable dentro de una misma versión del
/// binario: suficiente para nombrar archivos de caché regenerables.
fn calcular_hash(clave_ruta: &str, mtime: i64, size: u32) -> String {
    let mut h = DefaultHasher::new();
    clave_ruta.hash(&mut h);
    mtime.hash(&mut h);
    size.hash(&mut h);
    format!("{:016x}", h.finish())
}

/// Construye una ruta temporal única dentro de `dir` para escritura atómica.
fn ruta_temporal(dir: &Path, hash: String) -> PathBuf {
    let n = CONTADOR_TMP.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    dir.join(format!(".{hash}.{pid}.{n}.tmp"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};
    use std::time::Duration;

    /// Crea una imagen sintética con gradiente y la guarda como PNG.
    fn crear_imagen(ruta: &Path, ancho: u32, alto: u32) {
        let mut img = RgbImage::new(ancho, alto);
        for (x, y, px) in img.enumerate_pixels_mut() {
            let r = ((x * 255) / ancho.max(1)) as u8;
            let g = ((y * 255) / alto.max(1)) as u8;
            let b = (((x + y) * 255) / (ancho + alto).max(1)) as u8;
            *px = Rgb([r, g, b]);
        }
        img.save(ruta).expect("guardar imagen sintética");
    }

    #[test]
    fn genera_thumb_y_acierta_en_segunda_llamada() {
        let dir = tempfile::tempdir().unwrap();
        let img_path = dir.path().join("foto.png");
        crear_imagen(&img_path, 2000, 1500);

        let cache = ThumbCache::new(dir.path());

        // Primera llamada: genera el thumb.
        let thumb = cache.obtener(&img_path, 256).unwrap();
        assert!(thumb.exists(), "el thumb debe existir tras generarlo");
        assert_eq!(thumb.extension().and_then(|e| e.to_str()), Some("jpg"));
        assert!(thumb.starts_with(dir.path().join(".thumbs").join("256")));

        // El thumb decodifica y respeta el aspecto (lado mayor = 256).
        let decoded = image::open(&thumb).unwrap();
        assert_eq!(decoded.width().max(decoded.height()), 256);
        assert_eq!(decoded.width(), 256);
        assert_eq!(decoded.height(), 192);

        // Segunda llamada: acierto. mtime del thumb no cambia (no se regenera).
        let mtime1 = fs::metadata(&thumb).unwrap().modified().unwrap();
        let thumb2 = cache.obtener(&img_path, 256).unwrap();
        assert_eq!(thumb, thumb2);
        let mtime2 = fs::metadata(&thumb2).unwrap().modified().unwrap();
        assert_eq!(mtime1, mtime2, "un acierto no debe regenerar el archivo");
    }

    #[test]
    fn cambiar_mtime_origen_produce_nuevo_hash() {
        let dir = tempfile::tempdir().unwrap();
        let img_path = dir.path().join("foto.png");
        crear_imagen(&img_path, 800, 600);

        let cache = ThumbCache::new(dir.path());
        let thumb1 = cache.obtener(&img_path, 128).unwrap();

        // Vaciamos la caché RAM y tocamos el mtime del origen reescribiéndolo
        // tras un instante: el hash (que incluye mtime) debe cambiar.
        cache.invalidar(&img_path);
        std::thread::sleep(Duration::from_millis(1100));
        crear_imagen(&img_path, 800, 600);

        let thumb2 = cache.obtener(&img_path, 128).unwrap();
        assert_ne!(
            thumb1, thumb2,
            "al cambiar el mtime del origen el thumb debe tener otro hash"
        );
        assert!(thumb2.exists());
    }

    #[test]
    fn size_cero_devuelve_la_ruta_original() {
        let dir = tempfile::tempdir().unwrap();
        let img_path = dir.path().join("foto.png");
        crear_imagen(&img_path, 100, 100);

        let cache = ThumbCache::new(dir.path());
        let r = cache.obtener(&img_path, 0).unwrap();
        assert_eq!(r, img_path);
    }

    #[test]
    fn obtener_imagen_inexistente_es_error() {
        let dir = tempfile::tempdir().unwrap();
        let cache = ThumbCache::new(dir.path());
        let inexistente = dir.path().join("nope.png");
        assert!(matches!(
            cache.obtener(&inexistente, 256),
            Err(MicError::NoEncontrado(_))
        ));
        assert!(matches!(
            cache.obtener(&inexistente, 0),
            Err(MicError::NoEncontrado(_))
        ));
    }

    #[test]
    fn invalidar_borra_thumbs_de_todos_los_tamanos() {
        let dir = tempfile::tempdir().unwrap();
        let img_path = dir.path().join("foto.png");
        crear_imagen(&img_path, 1000, 1000);

        let cache = ThumbCache::new(dir.path());
        let t128 = cache.obtener(&img_path, 128).unwrap();
        let t256 = cache.obtener(&img_path, 256).unwrap();
        let t512 = cache.obtener(&img_path, 512).unwrap();
        assert!(t128.exists() && t256.exists() && t512.exists());

        cache.invalidar(&img_path);

        assert!(!t128.exists(), "invalidar debe borrar el thumb 128");
        assert!(!t256.exists(), "invalidar debe borrar el thumb 256");
        assert!(!t512.exists(), "invalidar debe borrar el thumb 512");

        // Tras invalidar, una nueva petición regenera (misma ruta, mismo mtime).
        let t256b = cache.obtener(&img_path, 256).unwrap();
        assert!(t256b.exists());
    }

    #[test]
    fn no_amplia_imagenes_pequenas() {
        let dir = tempfile::tempdir().unwrap();
        let img_path = dir.path().join("chica.png");
        crear_imagen(&img_path, 120, 90);

        let cache = ThumbCache::new(dir.path());
        let thumb = cache.obtener(&img_path, 256).unwrap();
        let decoded = image::open(&thumb).unwrap();
        // No se amplía: conserva el tamaño original.
        assert_eq!((decoded.width(), decoded.height()), (120, 90));
    }
}
