//! Public dynamic bridge contracts for host-language SolverForge bindings.
//!
//! The bridge crate is intentionally separate from the macro path. Rust models
//! keep using monomorphized descriptors, slots, and constraint sets. Binding
//! crates use this crate to describe logical model identity and Rust-owned
//! dynamic state without pretending that one host-language entity class maps to
//! one concrete Rust entity type.

mod backend;
mod ids;
mod runner;
mod score;
mod slots;

pub use backend::{
    DynamicListAccess, DynamicListAccessCapabilities, DynamicListMetadata,
    DynamicListMetadataCapabilities, DynamicModelBackend, DynamicScalarAccess,
    DynamicScalarAssignmentMetadata, DynamicScalarAssignmentMetadataCapabilities,
};
pub use ids::{EntityClassId, ProblemFactClassId, VariableId};
pub use runner::try_run_dynamic_solver_with_config_parts;
pub use score::{scoped_dynamic_score_family, DynamicScore, DynamicScoreFamily};
pub use slots::{DynamicListVariableSlot, DynamicScalarVariableSlot};

#[cfg(test)]
mod backend_tests;
#[cfg(test)]
mod score_tests;
