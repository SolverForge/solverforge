use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::MoveArena;
use crate::scope::{PendingControl, ProgressCallback, StepControlPolicy, StepScope};

pub(crate) const GENERATION_POLL_INTERVAL: usize = 256;
pub(crate) const EVALUATION_POLL_INTERVAL: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StepInterrupt {
    Restart,
    TerminatePhase,
}

pub(crate) fn append_interruptibly<'t, 'a, 'b, S, D, ProgressCb, M, I>(
    step_scope: &StepScope<'t, 'a, 'b, S, D, ProgressCb>,
    arena: &mut MoveArena<M>,
    iter: I,
) -> bool
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
    I: IntoIterator<Item = M>,
{
    if has_pending_control(step_scope) {
        return true;
    }

    for (generated, mov) in iter.into_iter().enumerate() {
        if generated != 0
            && generated.is_multiple_of(GENERATION_POLL_INTERVAL)
            && has_pending_control(step_scope)
        {
            return true;
        }
        arena.push(mov);
    }

    has_pending_control(step_scope)
}

pub(crate) fn should_interrupt_before_candidate<'t, 'a, 'b, S, D, ProgressCb>(
    step_scope: &StepScope<'t, 'a, 'b, S, D, ProgressCb>,
) -> bool
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    has_pending_control(step_scope)
}

pub(crate) fn should_interrupt_evaluation<'t, 'a, 'b, S, D, ProgressCb>(
    step_scope: &StepScope<'t, 'a, 'b, S, D, ProgressCb>,
    evaluated: usize,
) -> bool
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    evaluated != 0
        && evaluated.is_multiple_of(EVALUATION_POLL_INTERVAL)
        && has_pending_control(step_scope)
}

pub(crate) fn should_interrupt_before_evaluation<'t, 'a, 'b, S, D, ProgressCb>(
    step_scope: &StepScope<'t, 'a, 'b, S, D, ProgressCb>,
) -> bool
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    has_pending_control(step_scope)
}

pub(crate) fn should_interrupt_after_step<'t, 'a, 'b, S, D, ProgressCb>(
    step_scope: &StepScope<'t, 'a, 'b, S, D, ProgressCb>,
) -> bool
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    has_pending_control(step_scope)
}

pub(crate) fn settle_search_interrupt<'t, 'a, 'b, S, D, ProgressCb>(
    step_scope: &mut StepScope<'t, 'a, 'b, S, D, ProgressCb>,
) -> StepInterrupt
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    if step_scope
        .phase_scope_mut()
        .solver_scope_mut()
        .should_terminate()
    {
        StepInterrupt::TerminatePhase
    } else {
        StepInterrupt::Restart
    }
}

pub(crate) fn settle_construction_interrupt<'t, 'a, 'b, S, D, ProgressCb>(
    step_scope: &mut StepScope<'t, 'a, 'b, S, D, ProgressCb>,
) -> StepInterrupt
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let control_policy = step_scope.control_policy();
    let should_terminate = match control_policy {
        StepControlPolicy::ObserveConfigLimits => step_scope
            .phase_scope_mut()
            .solver_scope_mut()
            .should_terminate_construction(),
        StepControlPolicy::CompleteMandatoryConstruction => step_scope
            .phase_scope_mut()
            .solver_scope_mut()
            .should_interrupt_mandatory_construction(),
    };
    if should_terminate {
        StepInterrupt::TerminatePhase
    } else {
        StepInterrupt::Restart
    }
}

fn has_pending_control<'t, 'a, 'b, S, D, ProgressCb>(
    step_scope: &StepScope<'t, 'a, 'b, S, D, ProgressCb>,
) -> bool
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    !matches!(step_scope.pending_control(), PendingControl::Continue)
}
