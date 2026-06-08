//! mic-core: dominio puro de MIC 3.0 (sin SQL ni Tauri).
//!
//! Contiene el modelo de datos (campos configurables, valores, consultas)
//! y el motor de campos calculados portado del original VB6 (Module5.bas).

pub mod calc;
pub mod error;
pub mod model;

pub use error::MicError;
pub use model::*;
