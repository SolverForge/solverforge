/* High-level solver management with zero-erasure API.

# Zero-Erasure Design

All types flow through generics - no Box, Arc, or dyn anywhere.
Runtime configuration from TOML is handled by the macro layer
which generates concrete types at compile time.
*/

pub(crate) mod phase_factory;
pub(crate) mod solution_manager;
pub(crate) mod solver_manager;

#[cfg(test)]
mod mod_tests_integration;

pub(crate) use phase_factory::{
    run_cheapest, run_clarke_wright, run_list_k_opt, run_regret, run_round_robin,
    PhaseCheapestInsertionObserver, ScoredListConstructionAccess,
};
#[cfg(test)]
pub(crate) use phase_factory::{CheapestInsertionObserver, CheapestInsertionTrial};
pub(crate) use solver_manager::{
    SolverEvent, SolverLifecycleState, SolverRuntime, SolverSnapshot, SolverTerminalReason,
};
