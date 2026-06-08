//! Generación de miniaturas JPEG.
//!
//! Decodifica la imagen origen con el crate `image`, corrige la orientación
//! EXIF, reduce el tamaño manteniendo la relación de aspecto (lado mayor =
//! `size`) con resize SIMD (`fast_image_resize`, Lanczos3) y codifica el
//! resultado como JPEG de calidad 82.
//!
//! Decisión de formato: JPEG en lugar de WebP por simplicidad de compilación;
//! las miniaturas son fotografías y el JPEG es más que suficiente.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use fast_image_resize::images::Image as FirImage;
use fast_image_resize::{PixelType, Resizer};
use image::codecs::jpeg::JpegEncoder;
use image::{ColorType, DynamicImage, ImageDecoder, ImageEncoder, ImageReader};

use mic_core::error::MicError;

/// Calidad JPEG de las miniaturas (0-100). Buen equilibrio peso/calidad
/// para fotografías reducidas.
pub const CALIDAD_JPEG: u8 = 82;

/// Genera una miniatura JPEG de `origen` en `destino`.
///
/// La imagen se reduce manteniendo la relación de aspecto de modo que su lado
/// mayor mida `size` píxeles. Si la imagen ya es más pequeña que `size` no se
/// amplía: se reescribe a JPEG conservando su tamaño original. Se corrige la
/// orientación EXIF cuando el decodificador la expone.
///
/// # Argumentos
/// * `origen` - Ruta de la imagen original (cualquier formato soportado por `image`).
/// * `destino` - Ruta del archivo JPEG a escribir. Su directorio padre debe existir.
/// * `size` - Lado mayor objetivo en píxeles (debe ser > 0).
///
/// # Errores
/// Devuelve [`MicError::Invalido`] si `size` es 0 o la imagen está vacía,
/// [`MicError::Io`] ante fallos de lectura/escritura o decodificación, y
/// propaga cualquier error de redimensionado o codificación como
/// [`MicError::Io`].
pub fn generar(origen: &Path, destino: &Path, size: u32) -> Result<(), MicError> {
    if size == 0 {
        return Err(MicError::Invalido(
            "el tamaño de miniatura debe ser mayor que cero".into(),
        ));
    }

    // 1. Abrir y decodificar, leyendo antes la orientación EXIF.
    let lector = ImageReader::open(origen)
        .map_err(|e| MicError::Io(format!("no se pudo abrir '{}': {e}", origen.display())))?
        .with_guessed_format()
        .map_err(|e| {
            MicError::Io(format!(
                "no se pudo determinar el formato de '{}': {e}",
                origen.display()
            ))
        })?;

    let mut decodificador = lector.into_decoder().map_err(|e| {
        MicError::Io(format!(
            "no se pudo decodificar '{}': {e}",
            origen.display()
        ))
    })?;

    // Orientación EXIF: por defecto NoTransforms si el formato no la expone.
    let orientacion = decodificador
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);

    let mut imagen = DynamicImage::from_decoder(decodificador).map_err(|e| {
        MicError::Io(format!(
            "no se pudo decodificar '{}': {e}",
            origen.display()
        ))
    })?;
    imagen.apply_orientation(orientacion);

    // 2. Normalizar a RGB8: el JPEG no tiene alfa y así el resize trabaja con
    //    un único tipo de píxel (U8x3).
    let origen_rgb = imagen.into_rgb8();
    let (ancho_o, alto_o) = origen_rgb.dimensions();
    if ancho_o == 0 || alto_o == 0 {
        return Err(MicError::Invalido(format!(
            "imagen vacía: '{}'",
            origen.display()
        )));
    }

    // 3. Calcular el destino manteniendo aspecto (sin ampliar).
    let (ancho_d, alto_d) = dimensiones_destino(ancho_o, alto_o, size);

    let buffer_jpeg = if ancho_d == ancho_o && alto_d == alto_o {
        // La imagen ya cabe: no se redimensiona, solo se recodifica.
        codificar_jpeg(origen_rgb.as_raw(), ancho_o, alto_o)?
    } else {
        let src = DynamicImage::ImageRgb8(origen_rgb);
        let mut dst = FirImage::new(ancho_d, alto_d, PixelType::U8x3);

        // Resizer por defecto: Convolution(Lanczos3) y mejores extensiones SIMD.
        let mut resizer = Resizer::new();
        resizer
            .resize(&src, &mut dst, None)
            .map_err(|e| MicError::Io(format!("error al redimensionar: {e}")))?;

        codificar_jpeg(dst.buffer(), ancho_d, alto_d)?
    };

    // 4. Escribir el archivo JPEG.
    let archivo = File::create(destino).map_err(|e| {
        MicError::Io(format!(
            "no se pudo crear '{}': {e}",
            destino.display()
        ))
    })?;
    let mut escritor = BufWriter::new(archivo);
    use std::io::Write;
    escritor.write_all(&buffer_jpeg).map_err(|e| {
        MicError::Io(format!(
            "no se pudo escribir '{}': {e}",
            destino.display()
        ))
    })?;
    escritor.flush().map_err(|e| {
        MicError::Io(format!(
            "no se pudo finalizar '{}': {e}",
            destino.display()
        ))
    })?;

    Ok(())
}

/// Calcula las dimensiones de destino para que el lado mayor mida `size`,
/// conservando la relación de aspecto y sin ampliar imágenes ya pequeñas.
///
/// Cada lado se redondea al entero más cercano con un mínimo de 1 píxel.
fn dimensiones_destino(ancho: u32, alto: u32, size: u32) -> (u32, u32) {
    let mayor = ancho.max(alto);
    if mayor <= size {
        return (ancho, alto);
    }
    let factor = size as f64 / mayor as f64;
    let nuevo_ancho = ((ancho as f64) * factor).round().max(1.0) as u32;
    let nuevo_alto = ((alto as f64) * factor).round().max(1.0) as u32;
    (nuevo_ancho, nuevo_alto)
}

/// Codifica un buffer RGB8 contiguo (`ancho * alto * 3` bytes) como JPEG en
/// memoria con la calidad [`CALIDAD_JPEG`].
fn codificar_jpeg(rgb: &[u8], ancho: u32, alto: u32) -> Result<Vec<u8>, MicError> {
    let mut salida = Vec::new();
    let encoder = JpegEncoder::new_with_quality(&mut salida, CALIDAD_JPEG);
    encoder
        .write_image(rgb, ancho, alto, ColorType::Rgb8.into())
        .map_err(|e| MicError::Io(format!("error al codificar JPEG: {e}")))?;
    Ok(salida)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimensiones_mantienen_aspecto() {
        assert_eq!(dimensiones_destino(2000, 1500, 256), (256, 192));
        assert_eq!(dimensiones_destino(1500, 2000, 256), (192, 256));
        // Cuadrada.
        assert_eq!(dimensiones_destino(1000, 1000, 256), (256, 256));
        // Más pequeña que el objetivo: no se amplía.
        assert_eq!(dimensiones_destino(100, 80, 256), (100, 80));
    }
}
