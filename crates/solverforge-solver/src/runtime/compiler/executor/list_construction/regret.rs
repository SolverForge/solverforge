//! Canonical compiled source-indexed regret insertion.

use std::fmt;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::builder::context::{
    ListConstructionKernelError, RuntimeListElement, RuntimeListSlot, RuntimeListSourceIndex,
    SourceElement,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::manager::run_regret;
use crate::scope::{ProgressCallback, SolverScope, StepControlPolicy};

/// Executes canonical regret insertion against a borrowed prepared source
/// binding and the executor's validated current source work.
pub(crate) fn execute_runtime_list_regret_insertion<S, V, DM, IDM, D, ProgressCb>(
    slot: &RuntimeListSlot<S, V, DM, IDM>,
    source_index: &RuntimeListSourceIndex<RuntimeListElement<V>>,
    unassigned: &[SourceElement<RuntimeListElement<V>>],
    control_policy: StepControlPolicy,
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
) -> Result<(), ListConstructionKernelError>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    run_regret(slot, source_index, unassigned, control_policy, solver_scope);
    Ok(())
}
