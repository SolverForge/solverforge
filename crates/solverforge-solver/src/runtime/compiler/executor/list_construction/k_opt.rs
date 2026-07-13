//! Canonical compiled route-local K-opt execution.

use std::fmt;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::builder::context::RuntimeListSlot;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::manager::run_list_k_opt;
use crate::scope::{ProgressCallback, SolverScope, StepControlPolicy};

/// Executes canonical route-local K-opt without an assignment source.
///
/// K-opt is intentionally separate from the list-source-backed construction
/// helpers: it consumes only the route policy and current route contents.
pub(crate) fn execute_runtime_list_k_opt<S, V, DM, IDM, D, ProgressCb>(
    slot: &RuntimeListSlot<S, V, DM, IDM>,
    k: usize,
    control_policy: StepControlPolicy,
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
) where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Into<usize> + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    run_list_k_opt(slot, k, control_policy, solver_scope);
}
