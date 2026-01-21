//! Error types for SolverForge

use thiserror::Error;

/// Main error type for SolverForge operations
#[derive(Debug, Error)]
pub enum SolverForgeError {
    /// Error in solver configuration
    #[error("Configuration error: {0}")]
    Config(String),

    /// Error in domain model definition
    #[error("Domain model error: {0}")]
    DomainModel(String),

    /// Error during score calculation
    #[error("Score calculation error: {0}")]
    ScoreCalculation(String),

    /// Solver was cancelled before completion
    #[error("Solver was cancelled")]
    Cancelled,

    /// Invalid operation for current solver state
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Internal error (should not occur in normal operation)
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type alias for SolverForge operations
pub type Result<T> = std::result::Result<T, SolverForgeError>;
