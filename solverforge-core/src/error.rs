//! Error types for SolverForge Core
//!
//! This module defines the error types used throughout the SolverForge library.
//! All errors use the [`SolverForgeError`] enum, which provides specific variants
//! for different error categories.

use thiserror::Error;

pub type SolverForgeResult<T> = Result<T, SolverForgeError>;

/// Main error type for SolverForge operations
#[derive(Error, Debug)]
pub enum SolverForgeError {
    /// Error during serialization/deserialization
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Error during HTTP communication with solver service
    #[error("HTTP error: {0}")]
    Http(String),

    /// Error from the solver service
    #[error("Solver error: {0}")]
    Solver(String),

    /// Error during WASM generation
    #[error("WASM generation error: {0}")]
    WasmGeneration(String),

    /// Error from the language bridge
    #[error("Bridge error: {0}")]
    Bridge(String),

    /// Domain model validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Service lifecycle error (starting/stopping embedded service)
    #[error("Service error: {0}")]
    Service(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic error with message
    #[error("{0}")]
    Other(String),
}

impl From<serde_json::Error> for SolverForgeError {
    fn from(err: serde_json::Error) -> Self {
        SolverForgeError::Serialization(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = SolverForgeError::Serialization("invalid json".to_string());
        assert_eq!(err.to_string(), "Serialization error: invalid json");

        let err = SolverForgeError::Validation("missing solution class".to_string());
        assert_eq!(err.to_string(), "Validation error: missing solution class");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: SolverForgeError = io_err.into();
        assert!(matches!(err, SolverForgeError::Io(_)));
    }

    #[test]
    fn test_json_error_conversion() {
        let json_result: Result<i32, _> = serde_json::from_str("not json");
        let err: SolverForgeError = json_result.unwrap_err().into();
        assert!(matches!(err, SolverForgeError::Serialization(_)));
    }

    #[test]
    fn test_result_type() {
        fn returns_result() -> SolverForgeResult<i32> {
            Ok(42)
        }
        assert_eq!(returns_result().unwrap(), 42);

        fn returns_error() -> SolverForgeResult<i32> {
            Err(SolverForgeError::Other("test error".to_string()))
        }
        assert!(returns_error().is_err());
    }
}
