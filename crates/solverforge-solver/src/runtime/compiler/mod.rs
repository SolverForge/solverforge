//! Immutable runtime-graph compilation for typed and dynamic models.
//!
//! This declaration-only module exposes one compiler shape. Typed and
//! dynamic slots are physical payload variants only; selector semantics,
//! recursive decorators, capability validation, and provider policy are
//! represented once in the immutable graph.

mod compile;
mod construction;
mod default_local_search;
mod defaults;
pub(crate) mod executor;
mod graph;
mod input;
mod local_search;
mod providers;
mod selector_tree;
mod slots;
mod types;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod defaults_tests;

#[cfg(test)]
mod default_local_search_scalar_tests;

#[cfg(test)]
mod extension_tests;

pub(crate) use compile::compile_runtime_graph;
pub(crate) use default_local_search::{
    DefaultLocalSearchAcceptorPolicy, DefaultLocalSearchComponents, DefaultLocalSearchForagerPolicy,
};
pub(crate) use defaults::{
    DefaultConstructionStage, DefaultConstructionStepKind, DefaultListPolicyProvenance,
    DefaultLocalSearchEligibility, DefaultPreconstructionStage, DefaultRuntimeBindings,
    ResolvedDefaultConstructionPlan,
};
pub(crate) use executor::CompiledRuntimeExecutor;
pub(crate) use graph::{
    CompiledAcceptorForagerSelector, CompiledLocalSearch, CompiledProviderPlan,
    CompiledRuntimeExtension, CompiledRuntimePhase, CompiledSelectorNode, ListLeafKind,
    ProviderBindingPlan, ProviderBindingPolicy, ProviderMoveKind, ProviderSchedule, ScalarLeafKind,
};
pub(crate) use input::RuntimeGraphInput;

#[cfg(test)]
pub(crate) use default_local_search::{
    DefaultLocalSearchPlan, DefaultLocalSearchSelectorFamily, DefaultSelectorCapabilityPolicy,
};
#[cfg(test)]
pub(crate) use executor::PreparedRuntimePhase;
#[cfg(test)]
pub(crate) use graph::{
    CompiledConstruction, CompiledRuntimeGraph, ListConstructionKind, ProviderCandidateContract,
    ProviderCandidateDeduplication, ProviderPullTiming, ProviderReasonStorage,
    ProviderTabuIdentity,
};
#[cfg(test)]
pub(crate) use types::{
    RuntimeCapability, RuntimeCompileError, RuntimeCompileErrorKind, RuntimeExtensionKind,
};
