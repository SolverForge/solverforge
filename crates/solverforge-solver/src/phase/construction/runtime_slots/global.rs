use std::time::Instant;

use crate::list_placement;

use solverforge_config::{
    ConstructionHeuristicConfig, ConstructionHeuristicType, ConstructionObligation,
};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::builder::context::list_access::ListAccess;
use crate::builder::context::{
    unassigned_from_current_assignment, RuntimeListElement, RuntimeListSlot, SourceElement,
};
use crate::builder::RuntimeScalarSlot;
use crate::heuristic::r#move::Move;
use crate::phase::construction::decision::{is_first_fit_improvement, keep_current_allowed};
use crate::phase::construction::evaluation::evaluate_trial_move;
use crate::phase::construction::{record_construction_candidate, run_construction_phase};
use crate::scope::{PhaseScope, ProgressCallback, SolverScope, StepScope};
use crate::stats::{
    CandidateTraceConstructionTarget, CandidateTraceDisposition, CandidateTracePullToken,
    CandidateTraceSource,
};

use super::moves::{RuntimeListInsertionMove, RuntimeScalarConstructionMove};
use super::{FrozenRuntimeListConstructionSlot, ScalarOrMixedSlotOrder};

mod cheapest;
use cheapest::cheapest_iteration;

mod first_fit;
use first_fit::first_fit_iteration;

pub(super) fn solve_global_runtime_slot_scan<S, V, DM, IDM, D, ProgressCb>(
    config: ConstructionHeuristicConfig,
    scalar_slots: Vec<RuntimeScalarSlot<S>>,
    list_slots: Vec<FrozenRuntimeListConstructionSlot<'_, S, V, DM, IDM>>,
    slot_order: Vec<ScalarOrMixedSlotOrder>,
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
) -> bool
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score + Copy,
    V: Clone + PartialEq + Send + Sync + std::fmt::Debug + 'static,
    DM: Clone
        + Send
        + Sync
        + std::fmt::Debug
        + crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter<S>,
    IDM: Clone
        + Send
        + Sync
        + std::fmt::Debug
        + crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter<S>,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    assert!(
        matches!(
            config.construction_heuristic_type,
            ConstructionHeuristicType::FirstFit | ConstructionHeuristicType::CheapestInsertion
        ),
        "global runtime-slot construction supports only its declared first-fit or cheapest-insertion schedules"
    );
    run_construction_phase(solver_scope, 0, "Construction Heuristic", |phase_scope| {
        let previous_best_score = phase_scope.solver_scope().best_score().copied();
        let mut ran_step = false;
        loop {
            if phase_scope
                .solver_scope_mut()
                .should_terminate_construction()
            {
                break;
            }
            let progress = match config.construction_heuristic_type {
                ConstructionHeuristicType::FirstFit => first_fit_iteration(
                    &scalar_slots,
                    &list_slots,
                    &slot_order,
                    config.value_candidate_limit,
                    config.construction_obligation,
                    phase_scope,
                ),
                ConstructionHeuristicType::CheapestInsertion => cheapest_iteration(
                    &scalar_slots,
                    &list_slots,
                    &slot_order,
                    config.value_candidate_limit,
                    config.construction_obligation,
                    phase_scope,
                ),
                _ => unreachable!("global construction heuristic was validated above"),
            };
            match progress {
                Iteration::None => break,
                Iteration::CompletedOnly => ran_step = true,
                Iteration::Candidate(candidate) => {
                    ran_step = true;
                    commit(candidate, phase_scope);
                }
            }
        }
        if ran_step {
            phase_scope.update_best_solution();
            if phase_scope.solver_scope().current_score() == previous_best_score.as_ref() {
                phase_scope.promote_current_solution_on_score_tie();
            }
        }
        ran_step
    })
}

#[expect(
    clippy::large_enum_variant,
    reason = "construction iterations must not heap-allocate each candidate"
)]
enum Iteration<S, V, DM, IDM>
where
    S: PlanningSolution,
{
    None,
    CompletedOnly,
    Candidate(Candidate<S, V, DM, IDM>),
}

#[expect(
    clippy::large_enum_variant,
    reason = "construction candidates must remain value-owned in the hot path"
)]
enum Candidate<S, V, DM, IDM>
where
    S: PlanningSolution,
{
    Scalar {
        move_: RuntimeScalarConstructionMove<S>,
        slot_index: usize,
        score: S::Score,
        order: [usize; 4],
        trace: Option<CandidateTracePullToken>,
    },
    List {
        move_: RuntimeListInsertionMove<S, V, DM, IDM>,
        score: S::Score,
        order: [usize; 4],
        trace: Option<CandidateTracePullToken>,
    },
}

impl<S, V, DM, IDM> Candidate<S, V, DM, IDM>
where
    S: PlanningSolution,
{
    fn score(&self) -> S::Score {
        match self {
            Self::Scalar { score, .. } | Self::List { score, .. } => *score,
        }
    }

    fn order(&self) -> [usize; 4] {
        match self {
            Self::Scalar { order, .. } | Self::List { order, .. } => *order,
        }
    }

    fn ignore<D, ProgressCb>(self, phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>)
    where
        D: Director<S>,
        ProgressCb: ProgressCallback<S>,
    {
        if let Some(token) = self.trace() {
            phase_scope.record_candidate_trace_disposition(
                token,
                CandidateTraceDisposition::ForagerIgnored,
            );
        }
    }

    fn trace(&self) -> Option<CandidateTracePullToken> {
        match self {
            Self::Scalar { trace, .. } | Self::List { trace, .. } => *trace,
        }
    }
}

fn candidate_values<S>(
    slot: &RuntimeScalarSlot<S>,
    solution: &S,
    entity: usize,
    limit: Option<usize>,
) -> Vec<usize> {
    let mut values = Vec::new();
    slot.visit_candidate_values(solution, entity, limit, &mut |value| values.push(value));
    values
}

/// The first admissible owner is read directly from the element-owner
/// restriction; no full owner vector is materialized to take its head.
fn first_owner<S, V, DM, IDM>(
    slot: &RuntimeListSlot<S, V, DM, IDM>,
    element: &RuntimeListElement<V>,
    solution: &S,
) -> Option<usize>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + std::fmt::Debug + 'static,
    DM: Clone
        + Send
        + Sync
        + std::fmt::Debug
        + crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter<S>,
    IDM: Clone
        + Send
        + Sync
        + std::fmt::Debug
        + crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter<S>,
{
    slot.element_owner(solution, element)
        .ok()
        .and_then(|owner| match owner {
            Some(owner) if owner < slot.entity_count(solution) => Some(owner),
            Some(_) => None,
            None => Some(0),
        })
}

fn current_list_sources<S, V, DM, IDM>(
    list: &FrozenRuntimeListConstructionSlot<'_, S, V, DM, IDM>,
    solution: &S,
) -> Vec<SourceElement<RuntimeListElement<V>>>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + std::fmt::Debug + 'static,
    DM: Clone
        + Send
        + Sync
        + std::fmt::Debug
        + crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter<S>,
    IDM: Clone
        + Send
        + Sync
        + std::fmt::Debug
        + crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter<S>,
{
    unassigned_from_current_assignment(&list.slot, list.source_index, solution).unwrap_or_else(
        |error| {
            panic!(
                "frozen scalar-or-mixed list source assignment refresh failed before candidate enumeration: {error:?}"
            )
        },
    )
}

fn evaluate<S, D, ProgressCb, M>(
    move_: &M,
    candidate_index: usize,
    target: CandidateTraceConstructionTarget,
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
) -> Option<(S::Score, Option<CandidateTracePullToken>)>
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
    M: Move<S>,
{
    let trace = phase_scope.record_candidate_pull(
        CandidateTraceSource::Construction,
        None,
        candidate_index,
        Some(target),
        move_,
    );
    let started = Instant::now();
    if crate::pinning::move_changes_pinned(move_, phase_scope.score_director())
        || !move_.is_doable(phase_scope.score_director())
    {
        if let Some(token) = trace {
            phase_scope
                .record_candidate_trace_disposition(token, CandidateTraceDisposition::Evaluated);
            phase_scope
                .record_candidate_trace_disposition(token, CandidateTraceDisposition::NotDoable);
        }
        record_construction_candidate(phase_scope, std::time::Duration::ZERO, started.elapsed());
        return None;
    }
    let score = evaluate_trial_move(phase_scope.score_director_mut(), move_);
    phase_scope.record_score_calculation();
    record_construction_candidate(phase_scope, std::time::Duration::ZERO, started.elapsed());
    if let Some(token) = trace {
        phase_scope.record_candidate_trace_disposition(token, CandidateTraceDisposition::Evaluated);
    }
    Some((score, trace))
}

fn ignore<S, D, ProgressCb>(
    trace: Option<CandidateTracePullToken>,
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
) where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    if let Some(token) = trace {
        phase_scope
            .record_candidate_trace_disposition(token, CandidateTraceDisposition::ForagerIgnored);
    }
}

fn replace_best<S, V, DM, IDM, D, ProgressCb>(
    slot: &mut Option<Candidate<S, V, DM, IDM>>,
    candidate: Candidate<S, V, DM, IDM>,
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
) where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let replaces = slot.as_ref().is_none_or(|current| {
        candidate.score() > current.score()
            || (candidate.score() == current.score() && candidate.order() < current.order())
    });
    if replaces {
        if let Some(previous) = slot.replace(candidate) {
            previous.ignore(phase_scope);
        }
    } else {
        candidate.ignore(phase_scope);
    }
}

fn complete_scalar<S, D, ProgressCb>(
    slot_id: crate::phase::construction::ConstructionSlotId,
    kept: bool,
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
) where
    S: PlanningSolution,
    S::Score: Copy,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let mut step = StepScope::new(phase_scope);
    step.phase_scope_mut()
        .solver_scope_mut()
        .mark_scalar_slot_completed(slot_id);
    if kept {
        step.phase_scope_mut().record_construction_slot_kept();
    } else {
        step.phase_scope_mut().record_construction_slot_no_doable();
    }
    let score = step.calculate_score();
    step.set_step_score(score);
    step.complete();
}

fn commit<S, V, DM, IDM, D, ProgressCb>(
    candidate: Candidate<S, V, DM, IDM>,
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
) where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Copy,
    V: Clone + PartialEq + Send + Sync + std::fmt::Debug + 'static,
    DM: Clone
        + Send
        + Sync
        + std::fmt::Debug
        + crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter<S>,
    IDM: Clone
        + Send
        + Sync
        + std::fmt::Debug
        + crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter<S>,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    match candidate {
        Candidate::Scalar {
            move_,
            slot_index,
            trace,
            ..
        } => {
            if let Some(token) = trace {
                phase_scope
                    .record_candidate_trace_disposition(token, CandidateTraceDisposition::Selected);
            }
            let mut step = StepScope::new(phase_scope);
            step.phase_scope_mut().record_move_accepted();
            step.apply_committed_move(&move_);
            if let Some(token) = trace {
                step.phase_scope_mut()
                    .record_candidate_trace_disposition(token, CandidateTraceDisposition::Applied);
            }
            step.phase_scope_mut().record_move_applied();
            step.phase_scope_mut().record_construction_slot_assigned();
            let score = step.calculate_score();
            step.set_step_score(score);
            step.complete();
            let _ = slot_index;
        }
        Candidate::List { move_, trace, .. } => {
            if let Some(token) = trace {
                phase_scope
                    .record_candidate_trace_disposition(token, CandidateTraceDisposition::Selected);
            }
            let mut step = StepScope::new(phase_scope);
            step.phase_scope_mut().record_move_accepted();
            step.apply_committed_move(&move_);
            if let Some(token) = trace {
                step.phase_scope_mut()
                    .record_candidate_trace_disposition(token, CandidateTraceDisposition::Applied);
            }
            step.phase_scope_mut().record_move_applied();
            let score = step.calculate_score();
            step.set_step_score(score);
            step.complete();
        }
    }
}
