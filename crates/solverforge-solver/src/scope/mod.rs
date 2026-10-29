/* Scope hierarchy for solver execution.

Scopes maintain state at different levels of the solving process:
- [`SolverScope`]: Top-level, holds working solution and best solution
- [`PhaseScope`]: Per-phase state
- [`StepScope`]: Per-step state within a phase
*/

pub(crate) mod phase;
pub(crate) mod solver;
pub(crate) mod step;

pub(crate) use phase::PhaseScope;
pub(crate) use solver::{PendingControl, SolverScopeChildConfig, WeightModeControl};
pub(crate) use solver::{ProgressCallback, SolverProgressKind, SolverProgressRef, SolverScope};
pub(crate) use step::StepControlPolicy;
pub(crate) use step::StepScope;

#[cfg(test)]
mod tests;
