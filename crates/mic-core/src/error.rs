//! Errores del dominio MIC.

use thiserror::Error;

/// Error general del dominio. Los crates superiores lo convierten a String
/// para serializarlo hacia el frontend.
#[derive(Debug, Error)]
pub enum MicError {
    #[error("error de base de datos: {0}")]
    Db(String),

    #[error("error de fórmula en '{campo}': {detalle}")]
    Calc { campo: String, detalle: String },

    #[error("ciclo detectado en campos calculados: {0}")]
    CicloCalculo(String),

    #[error("no encontrado: {0}")]
    NoEncontrado(String),

    #[error("dato inválido: {0}")]
    Invalido(String),

    #[error("error de E/S: {0}")]
    Io(String),

    #[error("error de migración: {0}")]
    Migracion(String),
}

impl From<std::io::Error> for MicError {
    fn from(e: std::io::Error) -> Self {
        MicError::Io(e.to_string())
    }
}
