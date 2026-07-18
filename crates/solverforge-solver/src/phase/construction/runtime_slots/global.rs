use std::time::Instant;

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

fn first_fit_iteration<S, V, DM, IDM, D, ProgressCb>(
    scalar_slots: &[RuntimeScalarSlot<S>],
    list_slots: &[FrozenRuntimeListConstructionSlot<'_, S, V, DM, IDM>],
    slot_order: &[ScalarOrMixedSlotOrder],
    value_limit: Option<usize>,
    obligation: ConstructionObligation,
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
) -> Iteration<S, V, DM, IDM>
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
    let mut completed = false;
    for entry in slot_order {
        match *entry {
            ScalarOrMixedSlotOrder::Scalar {
                scalar_index,
                construction_slot_index,
            } => {
                let slot = scalar_slots
                    .get(scalar_index)
                    .expect("frozen global order must reference a scalar slot");
                for entity_index in
                    0..slot.entity_count(phase_scope.score_director().working_solution())
                {
                    let slot_id = crate::phase::construction::ConstructionSlotId::new(
                        construction_slot_index,
                        entity_index,
                    );
                    if phase_scope.solver_scope().is_scalar_slot_completed(slot_id)
                        || slot
                            .current_value(
                                phase_scope.score_director().working_solution(),
                                entity_index,
                            )
                            .is_some()
                    {
                        continue;
                    }
                    let values = candidate_values(
                        slot,
                        phase_scope.score_director().working_solution(),
                        entity_index,
                        value_limit,
                    );
                    if values.is_empty() {
                        if slot.allows_unassigned() {
                            complete_scalar(slot_id, false, phase_scope);
                            return Iteration::CompletedOnly;
                        }
                        continue;
                    }
                    let baseline = keep_current_allowed(slot.allows_unassigned(), obligation)
                        .then(|| phase_scope.calculate_score());
                    for (value_index, value) in values.into_iter().enumerate() {
                        let move_ =
                            RuntimeScalarConstructionMove::new(slot.clone(), entity_index, value);
                        let Some((score, trace)) = evaluate(
                            &move_,
                            value_index,
                            CandidateTraceConstructionTarget {
                                descriptor_index: move_.descriptor_index(),
                                entity_index,
                            },
                            phase_scope,
                        ) else {
                            continue;
                        };
                        if baseline
                            .is_some_and(|baseline| !is_first_fit_improvement(baseline, score))
                        {
                            ignore(trace, phase_scope);
                            continue;
                        }
                        return Iteration::Candidate(Candidate::Scalar {
                            move_,
                            slot_index: construction_slot_index,
                            score,
                            order: [construction_slot_index, entity_index, value_index, 0],
                            trace,
                        });
                    }
                    if slot.allows_unassigned() {
                        complete_scalar(slot_id, baseline.is_some(), phase_scope);
                        return Iteration::CompletedOnly;
                    }
                }
            }
            ScalarOrMixedSlotOrder::List {
                list_index,
                construction_slot_index,
            } => {
                let list = list_slots
                    .get(list_index)
                    .expect("frozen global order must reference a prepared list slot");
                let sources =
                    current_list_sources(list, phase_scope.score_director().working_solution());
                for source in &sources {
                    let element_id = crate::phase::construction::ConstructionListElementId::new(
                        construction_slot_index,
                        source.source_index,
                    );
                    if phase_scope
                        .solver_scope()
                        .is_list_element_completed(element_id)
                    {
                        continue;
                    }
                    let Some(entity_index) = first_owner(
                        &list.slot,
                        &source.element,
                        phase_scope.score_director().working_solution(),
                    ) else {
                        phase_scope
                            .solver_scope_mut()
                            .mark_list_element_completed(element_id);
                        completed = true;
                        continue;
                    };
                    let move_ = RuntimeListInsertionMove::new(
                        list.slot.clone(),
                        source.element.clone(),
                        source.source_index,
                        entity_index,
                        0,
                    );
                    let Some((score, trace)) = evaluate(
                        &move_,
                        0,
                        CandidateTraceConstructionTarget {
                            descriptor_index: move_.descriptor_index(),
                            entity_index,
                        },
                        phase_scope,
                    ) else {
                        phase_scope
                            .solver_scope_mut()
                            .mark_list_element_completed(element_id);
                        completed = true;
                        continue;
                    };
                    return Iteration::Candidate(Candidate::List {
                        move_,
                        score,
                        order: [
                            construction_slot_index,
                            source.source_index,
                            entity_index,
                            0,
                        ],
                        trace,
                    });
                }
            }
        }
    }
    if completed {
        Iteration::CompletedOnly
    } else {
        Iteration::None
    }
}

fn cheapest_iteration<S, V, DM, IDM, D, ProgressCb>(
    scalar_slots: &[RuntimeScalarSlot<S>],
    list_slots: &[FrozenRuntimeListConstructionSlot<'_, S, V, DM, IDM>],
    slot_order: &[ScalarOrMixedSlotOrder],
    value_limit: Option<usize>,
    obligation: ConstructionObligation,
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
) -> Iteration<S, V, DM, IDM>
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
    let mut global = None;
    let mut completed = false;
    for entry in slot_order {
        match *entry {
            ScalarOrMixedSlotOrder::Scalar {
                scalar_index,
                construction_slot_index,
            } => {
                let slot = scalar_slots
                    .get(scalar_index)
                    .expect("frozen global scalar slot");
                for entity_index in
                    0..slot.entity_count(phase_scope.score_director().working_solution())
                {
                    let slot_id = crate::phase::construction::ConstructionSlotId::new(
                        construction_slot_index,
                        entity_index,
                    );
                    if phase_scope.solver_scope().is_scalar_slot_completed(slot_id)
                        || slot
                            .current_value(
                                phase_scope.score_director().working_solution(),
                                entity_index,
                            )
                            .is_some()
                    {
                        continue;
                    }
                    let values = candidate_values(
                        slot,
                        phase_scope.score_director().working_solution(),
                        entity_index,
                        value_limit,
                    );
                    if values.is_empty() {
                        if slot.allows_unassigned() {
                            complete_scalar(slot_id, false, phase_scope);
                            completed = true;
                        }
                        continue;
                    }
                    let baseline = keep_current_allowed(slot.allows_unassigned(), obligation)
                        .then(|| phase_scope.calculate_score());
                    let mut best = None;
                    for (value_index, value) in values.into_iter().enumerate() {
                        let move_ =
                            RuntimeScalarConstructionMove::new(slot.clone(), entity_index, value);
                        let Some((score, trace)) = evaluate(
                            &move_,
                            value_index,
                            CandidateTraceConstructionTarget {
                                descriptor_index: move_.descriptor_index(),
                                entity_index,
                            },
                            phase_scope,
                        ) else {
                            continue;
                        };
                        let candidate = Candidate::Scalar {
                            move_,
                            slot_index: construction_slot_index,
                            score,
                            order: [construction_slot_index, entity_index, value_index, 0],
                            trace,
                        };
                        replace_best(&mut best, candidate, phase_scope);
                    }
                    match best {
                        Some(candidate)
                            if baseline.is_none_or(|baseline| candidate.score() >= baseline) =>
                        {
                            replace_best(&mut global, candidate, phase_scope)
                        }
                        Some(candidate) => {
                            candidate.ignore(phase_scope);
                            complete_scalar(slot_id, true, phase_scope);
                            completed = true;
                        }
                        None if slot.allows_unassigned() => {
                            complete_scalar(slot_id, false, phase_scope);
                            completed = true;
                        }
                        None => {}
                    }
                }
            }
            ScalarOrMixedSlotOrder::List {
                list_index,
                construction_slot_index,
            } => {
                let list = list_slots.get(list_index).expect("frozen global list slot");
                let sources =
                    current_list_sources(list, phase_scope.score_director().working_solution());
                for source in &sources {
                    let element_id = crate::phase::construction::ConstructionListElementId::new(
                        construction_slot_index,
                        source.source_index,
                    );
                    if phase_scope
                        .solver_scope()
                        .is_list_element_completed(element_id)
                    {
                        continue;
                    }
                    let mut best = None;
                    let mut candidate_index = 0;
                    for entity_index in owners(
                        &list.slot,
                        &source.element,
                        phase_scope.score_director().working_solution(),
                    ) {
                        let len = list.slot.list_len(
                            phase_scope.score_director().working_solution(),
                            entity_index,
                        );
                        for position in 0..=len {
                            let move_ = RuntimeListInsertionMove::new(
                                list.slot.clone(),
                                source.element.clone(),
                                source.source_index,
                                entity_index,
                                position,
                            );
                            let Some((score, trace)) = evaluate(
                                &move_,
                                candidate_index,
                                CandidateTraceConstructionTarget {
                                    descriptor_index: move_.descriptor_index(),
                                    entity_index,
                                },
                                phase_scope,
                            ) else {
                                candidate_index += 1;
                                continue;
                            };
                            candidate_index += 1;
                            replace_best(
                                &mut best,
                                Candidate::List {
                                    move_,
                                    score,
                                    order: [
                                        construction_slot_index,
                                        source.source_index,
                                        entity_index,
                                        position,
                                    ],
                                    trace,
                                },
                                phase_scope,
                            );
                        }
                    }
                    if let Some(candidate) = best {
                        replace_best(&mut global, candidate, phase_scope);
                    } else {
                        phase_scope
                            .solver_scope_mut()
                            .mark_list_element_completed(element_id);
                        completed = true;
                    }
                }
            }
        }
    }
    if let Some(candidate) = global {
        Iteration::Candidate(candidate)
    } else if completed {
        Iteration::CompletedOnly
    } else {
        Iteration::None
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

fn owners<S, V, DM, IDM>(
    slot: &RuntimeListSlot<S, V, DM, IDM>,
    element: &RuntimeListElement<V>,
    solution: &S,
) -> Vec<usize>
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
    match slot.element_owner(solution, element) {
        Ok(None) => (0..slot.entity_count(solution)).collect(),
        Ok(Some(owner)) if owner < slot.entity_count(solution) => vec![owner],
        Ok(Some(_)) | Err(_) => Vec::new(),
    }
}

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
    owners(slot, element, solution).into_iter().next()
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
    if !move_.is_doable(phase_scope.score_director()) {
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
