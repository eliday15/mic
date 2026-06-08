//! mic-thumbs: pipeline de miniaturas de MIC 3.0.
//!
//! Genera miniaturas JPEG en `.thumbs/<size>/<hash>.jpg` junto al álbum,
//! con resize SIMD (fast_image_resize) y caché LRU en RAM. Servidas al
//! webview vía el protocolo `thumb://` (ver mic-tauri).

pub mod cache;
pub mod generator;

pub use cache::ThumbCache;
