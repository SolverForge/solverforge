//! Scope hierarchy for solver execution.
//!
//! Scopes maintain state at different levels of the solving process:
//! - [`SolverScope`]: Top-level, holds working solution and best solution
//! - [`PhaseScope`]: Per-phase state
//! - [`StepScope`]: Per-step state within a phase

mod phase;
mod solver;
mod step;

pub use phase::PhaseScope;
pub use solver::SolverScope;
pub use step::StepScope;

#[cfg(test)]
mod tests;
