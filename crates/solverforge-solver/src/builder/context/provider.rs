//! Frozen compound-provider bindings for static Rust and host-language models.
//!
//! Static providers retain concrete function pointers and typed edits. Host
//! callbacks alone use object-safe pulls and raw named edits. The registry is
//! immutable after `RuntimeModel` descriptor resolution; cursor execution
//! never reaches back into mutable schema, a global map, or TLS.

mod native;
pub(crate) mod registry;
pub(crate) mod resolver;
pub(crate) mod types;

pub(crate) use registry::RuntimeProviderRegistry;
pub(crate) use resolver::RuntimeProviderSlotResolver;
pub(crate) use types::{
    ProviderNormalizationState, ProviderReasonArena, ProviderReasonId, ProviderResolutionError,
    RawProviderCandidate, RawProviderEdit, ResolvedProviderCandidate, ResolvedProviderEdit,
    RuntimeConflictRepairProviderBinding, RuntimeHostProviderErrorBoundary, RuntimeProviderHandle,
    RuntimeProviderLimits, RuntimeScalarGroupProviderBinding, StaticConflictRepairProviderBinding,
    StaticScalarGroupProviderBinding,
};

#[cfg(test)]
#[path = "provider/tests.rs"]
mod tests;
