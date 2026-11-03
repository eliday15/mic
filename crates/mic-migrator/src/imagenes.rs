//! Localización y copia de las imágenes del álbum original.
//!
//! El MIC clásico guarda en `_imagen_` la ruta de Windows de cada imagen
//! (`G:\MIC\imagenes\foto.jpg`). Su carpeta de imágenes por defecto es
//! `{MIC}\imagenes`, **hermana** de la de álbumes (`{MIC}\albums\x.mdb`), y el
//! usuario puede cambiarla en Herramientas → Directorios. Buscar solo en
//! `<carpeta del .mdb>\imagenes` deja el álbum migrado sin fotos, así que cada
//! imagen se busca en varios lugares ([`candidatos`]) y se copia una por una.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::migrar::{MigracionReporte, ProgresoMigracion};
use crate::paths;

/// Niveles de carpeta (desde la del `.mdb` hacia arriba) donde se busca la
/// carpeta `imagenes`: la del `.mdb`, su padre (`{MIC}`) y su abuelo.
const NIVELES_BUSQUEDA: usize = 3;

/// Imagen referenciada por un registro del original.
#[derive(Debug, Clone)]
pub struct RefImagen {
    /// Valor de `_imagen_` tal como está en el `.mdb`.
    pub original: String,
    /// Ruta normalizada que se guardó en el `.micdb` (`imagenes/<...>`).
    pub relativa: String,
}

/// Rutas donde puede estar el archivo de una imagen, en orden de preferencia.
///
/// 1. La ruta original, si es absoluta (funciona en la misma red/equipo).
/// 2. `<nivel>/imagenes/<...>` para la carpeta del `.mdb` y sus ancestros.
/// 3. El archivo suelto junto al `.mdb`.
///
/// Args:
///     img: Imagen referenciada.
///     dir_mdb: Carpeta del `.mdb` original (no de su copia local).
///
/// Returns:
///     Lista de rutas candidatas (pueden no existir).
pub fn candidatos(img: &RefImagen, dir_mdb: &Path) -> Vec<PathBuf> {
    let mut salida = Vec::new();
    let original = Path::new(img.original.trim());
    if original.is_absolute() {
        salida.push(original.to_path_buf());
    }
    for base in dir_mdb.ancestors().take(NIVELES_BUSQUEDA) {
        salida.push(base.join(&img.relativa));
    }
    salida.push(dir_mdb.join(paths::nombre_archivo(&img.relativa)));
    salida
}

/// Copia al álbum nuevo cada imagen referenciada y anota en el reporte las
/// encontradas, las faltantes y dónde se buscaron.
///
/// Args:
///     refs: Imágenes referenciadas por Principal y Variantes.
///     dir_mdb: Carpeta del `.mdb` original.
///     dir_album: Carpeta del `.micdb` destino (las rutas relativas cuelgan de ahí).
///     progreso: Callback de progreso de la migración.
///     reporte: Reporte a completar.
pub fn importar(
    refs: &[RefImagen],
    dir_mdb: &Path,
    dir_album: &Path,
    progreso: &ProgresoMigracion,
    reporte: &mut MigracionReporte,
) {
    let mut vistas: HashSet<&str> = HashSet::new();
    let unicas: Vec<&RefImagen> = refs
        .iter()
        .filter(|r| vistas.insert(r.relativa.as_str()))
        .collect();
    let total = unicas.len() as u64;
    let mut encontradas = 0u64;
    let mut faltantes: Vec<&RefImagen> = Vec::new();

    for (n, img) in unicas.iter().enumerate() {
        if n % 50 == 0 {
            progreso("Imágenes", n as u64, total);
        }
        let destino = dir_album.join(&img.relativa);
        if destino.is_file() {
            encontradas += 1;
            continue;
        }
        let Some(origen) = candidatos(img, dir_mdb).into_iter().find(|c| c.is_file()) else {
            faltantes.push(img);
            continue;
        };
        let copia = destino
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|_| std::fs::copy(&origen, &destino));
        match copia {
            Ok(_) => encontradas += 1,
            Err(e) => {
                reporte.advertencias.push(format!(
                    "no se pudo copiar la imagen {}: {e}",
                    origen.display()
                ));
                faltantes.push(img);
            }
        }
    }

    if let Some(primera) = faltantes.first() {
        let mut lugares: Vec<String> = Vec::new();
        for c in candidatos(primera, dir_mdb) {
            let lugar = c.parent().unwrap_or(&c).display().to_string();
            if !lugares.contains(&lugar) {
                lugares.push(lugar);
            }
        }
        reporte.advertencias.push(format!(
            "{} imagen(es) no se encontraron (p. ej. {}); se buscaron en: {}",
            faltantes.len(),
            primera.original.trim(),
            lugares.join(" · ")
        ));
    }
    crate::diag::paso(&format!(
        "imágenes: {encontradas} copiadas/encontradas, {} faltantes de {total}",
        faltantes.len()
    ));

    reporte.imagenes_encontradas = encontradas;
    reporte.imagenes_faltantes = faltantes.into_iter().map(|r| r.relativa.clone()).collect();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sin_progreso() -> ProgresoMigracion {
        Box::new(|_, _, _| {})
    }

    fn ref_img(original: &str) -> RefImagen {
        RefImagen {
            original: original.into(),
            relativa: paths::normalizar_imagen(original, Path::new(".")),
        }
    }

    #[test]
    fn encuentra_imagenes_en_carpeta_hermana_de_albums() {
        // Distribución por defecto del MIC clásico: {MIC}\albums y {MIC}\imagenes.
        let tmp = tempfile::tempdir().unwrap();
        let mic = tmp.path().join("MIC");
        std::fs::create_dir_all(mic.join("albums")).unwrap();
        std::fs::create_dir_all(mic.join("imagenes")).unwrap();
        std::fs::write(mic.join("imagenes").join("a1.jpg"), b"jpg").unwrap();
        let destino = tmp.path().join("nuevo");
        std::fs::create_dir_all(&destino).unwrap();

        let refs = vec![
            ref_img("G:\\MIC\\imagenes\\a1.jpg"),
            ref_img("G:\\MIC\\imagenes\\a1.jpg"),
            ref_img("G:\\MIC\\imagenes\\no-existe.jpg"),
        ];
        let mut reporte = MigracionReporte::default();
        importar(
            &refs,
            &mic.join("albums"),
            &destino,
            &sin_progreso(),
            &mut reporte,
        );

        assert_eq!(reporte.imagenes_encontradas, 1);
        assert_eq!(reporte.imagenes_faltantes, vec!["imagenes/no-existe.jpg"]);
        assert!(destino.join("imagenes/a1.jpg").is_file());
        assert!(reporte.advertencias[0].contains("se buscaron en"));
    }

    #[test]
    fn encuentra_imagen_suelta_junto_al_mdb_y_por_ruta_absoluta() {
        let tmp = tempfile::tempdir().unwrap();
        let dir_mdb = tmp.path().join("album");
        std::fs::create_dir_all(&dir_mdb).unwrap();
        std::fs::write(dir_mdb.join("suelta.jpg"), b"jpg").unwrap();
        let otra = tmp.path().join("fotos");
        std::fs::create_dir_all(&otra).unwrap();
        std::fs::write(otra.join("abs.jpg"), b"jpg").unwrap();
        let destino = tmp.path().join("nuevo");

        let refs = vec![
            ref_img("C:\\cualquier\\suelta.jpg"),
            ref_img(&otra.join("abs.jpg").to_string_lossy()),
        ];
        let mut reporte = MigracionReporte::default();
        importar(&refs, &dir_mdb, &destino, &sin_progreso(), &mut reporte);

        assert_eq!(reporte.imagenes_encontradas, 2);
        assert!(reporte.imagenes_faltantes.is_empty());
        assert!(destino.join("imagenes/suelta.jpg").is_file());
        assert!(destino.join("imagenes/abs.jpg").is_file());
    }
}
