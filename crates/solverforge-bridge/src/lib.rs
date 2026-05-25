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

pub use backend::{DynamicListAccess, DynamicModelBackend, DynamicScalarAccess};
pub use ids::{EntityClassId, ProblemFactClassId, VariableId};
pub use runner::run_dynamic_solver_with_config;
pub use score::{DynamicScore, DynamicScoreFamily};

#[cfg(test)]
mod backend_tests;
#[cfg(test)]
mod score_tests;
