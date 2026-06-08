//! Normalización de rutas de imágenes del original.
//!
//! En los `.mdb` antiguos la columna `_imagen_` guarda rutas de Windows, a veces
//! absolutas con unidad de red (`G:\MIC\imagenes\foto.jpg`), a veces relativas
//! (`.\imagenes\foto.jpg`), con barras invertidas. MIC 3.0 guarda siempre rutas
//! **relativas** al álbum, con barras normales, de la forma `imagenes/<archivo>`.
//!
//! La estrategia ([`normalizar_imagen`]):
//! 1. Pasar `\` → `/` y partir en segmentos.
//! 2. Buscar (case-insensitive) el segmento `imagenes`; si aparece, conservar
//!    desde ahí (`imagenes/sub/foto.jpg`).
//! 3. Si no aparece, quedarnos solo con el nombre de archivo y anteponer
//!    `imagenes/` (las imágenes del original cuelgan de una sola carpeta plana).

use std::path::Path;

/// Nombre de la carpeta de imágenes (en minúsculas, salida canónica).
pub const DIR_IMAGENES: &str = "imagenes";

/// Convierte una ruta de imagen del original en una ruta relativa
/// `imagenes/<...>` con barras normales.
///
/// - `dir_mdb` es la carpeta donde está el `.mdb` (se usa solo como contexto;
///   la salida es siempre relativa, nunca absoluta).
/// - Cadena vacía o solo espacios ⇒ cadena vacía (registro sin imagen).
///
/// # Ejemplos
/// - `G:\MIC\imagenes\foto.jpg`   → `imagenes/foto.jpg`
/// - `.\imagenes\sub\f.jpg`       → `imagenes/sub/f.jpg`
/// - `C:\Datos\IMAGENES\F.JPG`    → `imagenes/F.JPG`
/// - `foto.jpg`                   → `imagenes/foto.jpg`
pub fn normalizar_imagen(ruta_original: &str, _dir_mdb: &Path) -> String {
    let limpio = ruta_original.trim();
    if limpio.is_empty() {
        return String::new();
    }

    // Unifica separadores y descarta segmentos vacíos / "." (de ".\imagenes").
    let normal = limpio.replace('\\', "/");
    let segmentos: Vec<&str> = normal
        .split('/')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && *s != ".")
        .collect();

    if segmentos.is_empty() {
        return String::new();
    }

    // Busca el último segmento "imagenes" (case-insensitive). Tomamos el último
    // por si la ruta absoluta tuviera carpetas intermedias con ese nombre.
    let pos_imagenes = segmentos
        .iter()
        .rposition(|s| s.eq_ignore_ascii_case(DIR_IMAGENES));

    match pos_imagenes {
        Some(i) => {
            // Conserva desde "imagenes" en adelante, canonizando el nombre de la
            // carpeta a minúsculas pero respetando los subsegmentos.
            let mut salida = String::from(DIR_IMAGENES);
            for seg in &segmentos[i + 1..] {
                salida.push('/');
                salida.push_str(seg);
            }
            salida
        }
        None => {
            // Sin carpeta "imagenes": solo el nombre de archivo bajo imagenes/.
            let archivo = segmentos.last().copied().unwrap_or_default();
            format!("{DIR_IMAGENES}/{archivo}")
        }
    }
}

/// Extrae solo el nombre de archivo (último segmento) de una ruta normalizada o
/// del original, para comprobar la existencia física en disco.
pub fn nombre_archivo(ruta_rel: &str) -> &str {
    ruta_rel
        .rsplit(['/', '\\'])
        .next()
        .map(|s| s.trim())
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn dir() -> PathBuf {
        PathBuf::from("/algun/lugar/album")
    }

    #[test]
    fn ruta_absoluta_windows_unidad_red() {
        assert_eq!(
            normalizar_imagen("G:\\MIC\\imagenes\\foto.jpg", &dir()),
            "imagenes/foto.jpg"
        );
    }

    #[test]
    fn ruta_relativa_punto() {
        assert_eq!(
            normalizar_imagen(".\\imagenes\\foto.jpg", &dir()),
            "imagenes/foto.jpg"
        );
    }

    #[test]
    fn ruta_con_subcarpeta() {
        assert_eq!(
            normalizar_imagen(".\\imagenes\\sub\\f.jpg", &dir()),
            "imagenes/sub/f.jpg"
        );
    }

    #[test]
    fn carpeta_imagenes_mayusculas() {
        assert_eq!(
            normalizar_imagen("C:\\Datos\\IMAGENES\\F.JPG", &dir()),
            "imagenes/F.JPG"
        );
    }

    #[test]
    fn solo_nombre_archivo() {
        assert_eq!(normalizar_imagen("foto.jpg", &dir()), "imagenes/foto.jpg");
    }

    #[test]
    fn barras_normales_unix() {
        assert_eq!(
            normalizar_imagen("/home/u/imagenes/foto.png", &dir()),
            "imagenes/foto.png"
        );
    }

    #[test]
    fn vacio_o_espacios() {
        assert_eq!(normalizar_imagen("", &dir()), "");
        assert_eq!(normalizar_imagen("   ", &dir()), "");
    }

    #[test]
    fn ruta_sin_carpeta_pero_con_directorios() {
        // No hay segmento "imagenes": cae a solo el nombre de archivo.
        assert_eq!(
            normalizar_imagen("C:\\fotos\\productos\\art1.jpg", &dir()),
            "imagenes/art1.jpg"
        );
    }

    #[test]
    fn nombre_archivo_extrae_ultimo() {
        assert_eq!(nombre_archivo("imagenes/sub/f.jpg"), "f.jpg");
        assert_eq!(nombre_archivo("G:\\x\\y.png"), "y.png");
    }
}
