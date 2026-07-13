//! Canonical compiled Clarke-Wright execution.

use std::fmt;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::builder::context::{
    ListConstructionKernelError, RuntimeListElement, RuntimeListSlot, RuntimeListSourceIndex,
    SourceElement,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::manager::run_clarke_wright;
use crate::scope::{ProgressCallback, SolverScope, StepControlPolicy};

/// Executes the canonical Clarke-Wright savings/merge/completion kernel using
/// a borrowed prepared source binding.
///
/// This does not implement an alternate construction strategy.  It is the
/// one direct call site for `run_clarke_wright` used by the compiled runtime
/// runner, so CVRP retains its established candidate
/// ordering and trace sources without cloning the frozen source index.
pub(crate) fn execute_runtime_list_clarke_wright<S, V, DM, IDM, D, ProgressCb>(
    slot: &RuntimeListSlot<S, V, DM, IDM>,
    source_index: &RuntimeListSourceIndex<RuntimeListElement<V>>,
    unassigned: &[SourceElement<RuntimeListElement<V>>],
    control_policy: StepControlPolicy,
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
) -> Result<(), ListConstructionKernelError>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Into<usize> + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    run_clarke_wright(slot, source_index, unassigned, control_policy, solver_scope);
    Ok(())
}
