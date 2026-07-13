//! Fallible boundaries for configured runtime declaration and preparation.
//!
//! The configured runner translates its private compiler and executor errors
//! into this public, non-graph error surface. Host bindings and generated
//! macro code can therefore propagate an actionable failure without naming or
//! constructing a compiled runtime graph.

use std::fmt;

/// Error returned while a configured runtime is being declared, compiled, or
/// prepared for one solve.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeBuildError {
    /// The descriptor-backed runtime declaration could not be formed.
    Declaration {
        /// Actionable explanation of the invalid declaration.
        message: String,
    },
    /// The private runtime graph compiler rejected the declaration.
    Compilation {
        /// Structural configuration path that failed validation.
        path: String,
        /// Actionable explanation of the compiler rejection.
        message: String,
    },
    /// One structurally valid graph could not be prepared for execution.
    Preparation {
        /// Configured phase index being prepared.
        phase_index: usize,
        /// Actionable explanation of the preparation failure.
        message: String,
    },
    /// A reached runtime phase could not bind or execute its declared work.
    Execution {
        /// Configured phase index that reached the failure boundary.
        phase_index: usize,
        /// Actionable explanation of the execution failure.
        message: String,
    },
}

impl RuntimeBuildError {
    /// Creates a declaration error from descriptor/model resolution.
    pub fn declaration(message: impl Into<String>) -> Self {
        Self::Declaration {
            message: message.into(),
        }
    }
}

impl fmt::Display for RuntimeBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Declaration { message } => {
                write!(formatter, "runtime declaration failed: {message}")
            }
            Self::Compilation { path, message } => {
                write!(formatter, "runtime compilation failed at {path}: {message}")
            }
            Self::Preparation {
                phase_index,
                message,
            } => {
                write!(
                    formatter,
                    "runtime preparation failed at phase {phase_index}: {message}"
                )
            }
            Self::Execution {
                phase_index,
                message,
            } => {
                write!(
                    formatter,
                    "runtime execution failed at phase {phase_index}: {message}"
                )
            }
        }
    }
}

impl std::error::Error for RuntimeBuildError {}

/// Result returned by the one configured-runtime declaration/preparation path.
pub type RuntimeBuildResult<T> = Result<T, RuntimeBuildError>;

#[cfg(test)]
mod tests {
    use super::RuntimeBuildError;

    #[test]
    fn display_keeps_the_runtime_build_boundary_and_structural_location() {
        assert_eq!(
            RuntimeBuildError::declaration("unknown variable").to_string(),
            "runtime declaration failed: unknown variable"
        );
        assert_eq!(
            RuntimeBuildError::Compilation {
                path: "phases[1]".to_string(),
                message: "missing route hooks".to_string(),
            }
            .to_string(),
            "runtime compilation failed at phases[1]: missing route hooks"
        );
        assert_eq!(
            RuntimeBuildError::Preparation {
                phase_index: 2,
                message: "source key collision".to_string(),
            }
            .to_string(),
            "runtime preparation failed at phase 2: source key collision"
        );
        assert_eq!(
            RuntimeBuildError::Execution {
                phase_index: 3,
                message: "source callback failed".to_string(),
            }
            .to_string(),
            "runtime execution failed at phase 3: source callback failed"
        );
    }
}
