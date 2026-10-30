//! Builder module for constructing solver components from configuration.
//!
//! Provides wiring between `SolverConfig` and the actual solver types.
//! All builders return concrete monomorphized enums — no `Box<dyn Trait>`.

pub(crate) mod acceptor;
pub(crate) mod context;
pub(crate) mod forager;
pub(crate) mod local_search_policy;
pub(crate) mod search;
pub(crate) mod selector;

pub(crate) use acceptor::AnyAcceptor;
pub(crate) use context::{
    ListVariableSlot, RuntimeCandidateMetricBinding, RuntimeModel, RuntimeProviderHandle,
    RuntimeScalarEdit, RuntimeScalarSlot, RuntimeScalarSlotId, ScalarAccessCapability,
    ScalarAssignmentBinding, ScalarGroupBinding, ScalarGroupBindingKind, ScalarGroupLimits,
    ScalarVariableSlot, ValueSource, VariableSlot,
};
pub(crate) use forager::AnyForager;
pub(crate) use local_search_policy::ConfigValueError;
pub(crate) use local_search_policy::{CompiledAcceptor, CompiledForager};
pub(crate) use search::{
    CustomSearchPhase, RuntimeExtensionPolicy, RuntimeExtensionRegistry, Search, SearchContext,
};
