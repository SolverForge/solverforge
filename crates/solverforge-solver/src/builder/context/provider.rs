//! Frozen compound-provider bindings for static Rust and host-language models.
//!
//! Static providers retain concrete function pointers and typed edits. Host
//! callbacks alone use object-safe pulls and raw named edits. The registry is
//! immutable after `RuntimeModel` descriptor resolution; cursor execution
//! never reaches back into mutable schema, a global map, or TLS.

mod native;
mod registry;
mod resolver;
mod types;

pub use registry::RuntimeProviderRegistry;
pub use resolver::RuntimeProviderSlotResolver;
pub use types::{
    ProviderNormalizationState, ProviderReasonArena, ProviderReasonId, ProviderResolutionError,
    RawProviderCandidate, RawProviderEdit, ResolvedProviderCandidate, ResolvedProviderEdit,
    RuntimeConflictRepairProviderBinding, RuntimeHostCompoundProvider,
    RuntimeHostProviderErrorBoundary, RuntimeProviderHandle, RuntimeProviderLimits,
    RuntimeScalarGroupProviderBinding, StaticConflictRepairProviderBinding,
    StaticScalarGroupProviderBinding,
};

#[cfg(test)]
#[path = "provider/tests.rs"]
mod tests;
