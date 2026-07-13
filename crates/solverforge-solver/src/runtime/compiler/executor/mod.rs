//! Execution of one immutable compiled runtime graph.
//!
//! This module owns no schema discovery. It lowers frozen declarations into
//! the shared neighborhood kernels and executes the resulting phase sequence.

mod execute;
mod execution_record;
pub(crate) mod list_construction;
mod list_leaf;
pub(crate) mod local_search;
mod prepared;
mod runner;
mod trace;

#[cfg(test)]
mod trace_tests;

pub(crate) use execute::{execute_prepared_construction, execute_prepared_default_construction};
pub(crate) use execution_record::{
    ConstructionExecution, DefaultConstructionStageExecutionRecord,
    DefaultRuntimeConstructionExecution, ResolvedConstructionExecutionOutcome,
    ResolvedConstructionExecutionStep,
};
pub(crate) use list_leaf::{
    CompiledListNeighborhoodLeafAdapter, RuntimeListMove, RuntimeListNeighborhoodCursor,
    RuntimeListNeighborhoodLeaf, RuntimeListNeighborhoodStreamState,
};
pub(crate) use prepared::{
    CompiledRuntimeExecutor, PreparedConstruction, PreparedDefaultRuntime, PreparedListSlot,
    PreparedRuntimeExecution, PreparedRuntimePhase, RuntimeInstantiationError,
    RuntimeInstantiationErrorKind,
};
pub(crate) use runner::{take_runtime_execution_failure, CompiledRuntimePhaseRunner};
