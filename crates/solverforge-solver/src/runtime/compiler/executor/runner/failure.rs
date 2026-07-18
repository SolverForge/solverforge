use std::any::Any;

use crate::runtime_build_error::RuntimeBuildError;

use super::super::{RuntimeInstantiationError, RuntimeInstantiationErrorKind};

/// Typed transport for a reached compiled-runtime execution failure. The
/// configured entrypoint catches only this payload and resumes foreign panics.
#[derive(Debug)]
struct RuntimeExecutionFailure {
    error: RuntimeBuildError,
}

pub(crate) fn take_runtime_execution_failure(
    payload: Box<dyn Any + Send>,
) -> Result<RuntimeBuildError, Box<dyn Any + Send>> {
    payload
        .downcast::<RuntimeExecutionFailure>()
        .map(|failure| failure.error)
}

pub(super) fn map_preparation_error(error: RuntimeInstantiationError) -> RuntimeBuildError {
    match error.kind {
        RuntimeInstantiationErrorKind::SourceBinding { .. }
        | RuntimeInstantiationErrorKind::SourceRefresh { .. } => RuntimeBuildError::Execution {
            phase_index: error.phase_index,
            message: error.to_string(),
        },
        _ => RuntimeBuildError::Preparation {
            phase_index: error.phase_index,
            message: error.to_string(),
        },
    }
}

pub(super) fn panic_execution_error(error: RuntimeInstantiationError) -> ! {
    panic_runtime_execution_error(RuntimeBuildError::Execution {
        phase_index: error.phase_index,
        message: error.to_string(),
    })
}

pub(super) fn panic_runtime_execution_error(error: RuntimeBuildError) -> ! {
    std::panic::panic_any(RuntimeExecutionFailure { error })
}
