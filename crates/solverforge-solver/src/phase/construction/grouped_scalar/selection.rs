use std::time::Instant;

use solverforge_config::{ConstructionHeuristicType, ConstructionObligation};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::heuristic::r#move::CompoundScalarMove;
use crate::scope::{PhaseScope, ProgressCallback};

use super::candidate::NormalizedGroupedCandidate;
use crate::phase::construction::decision::keep_current_allowed;
use crate::phase::construction::evaluation::evaluate_trial_move;
use crate::phase::construction::{ConstructionGroupSlotId, ConstructionSlotId};

pub(super) enum GroupedSelection<S>
where
    S: PlanningSolution,
{
    Commit {
        group_slot: ConstructionGroupSlotId,
        scalar_slots: Vec<ConstructionSlotId>,
        mov: CompoundScalarMove<S>,
        score: S::Score,
    },
    CompleteOnly {
        group_slot: ConstructionGroupSlotId,
        scalar_slots: Vec<ConstructionSlotId>,
        kept: bool,
    },
}

pub(super) fn select_candidate_for_next_group_slot<S, D, ProgressCb>(
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
    candidates: Vec<NormalizedGroupedCandidate<S>>,
    construction_type: ConstructionHeuristicType,
    construction_obligation: ConstructionObligation,
) -> Option<GroupedSelection<S>>
where
    S: PlanningSolution + 'static,
    S::Score: Score + Copy,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    if candidates.is_empty() {
        return None;
    }
    validate_heuristic_metadata(construction_type, &candidates);
    let selected_group_slot = next_group_slot(construction_type, &candidates);
    let selected_group_candidates = candidates
        .into_iter()
        .filter(|candidate| candidate.group_slot == selected_group_slot)
        .collect::<Vec<_>>();

    match construction_type {
        ConstructionHeuristicType::FirstFit | ConstructionHeuristicType::FirstFitDecreasing => {
            select_first_fit(
                phase_scope,
                selected_group_candidates,
                construction_obligation,
            )
        }
        ConstructionHeuristicType::CheapestInsertion => select_best_score(
            phase_scope,
            selected_group_candidates,
            construction_obligation,
        ),
        ConstructionHeuristicType::WeakestFit | ConstructionHeuristicType::WeakestFitDecreasing => {
            select_by_strength(
                phase_scope,
                selected_group_candidates,
                construction_obligation,
                StrengthPolicy::Weakest,
            )
        }
        ConstructionHeuristicType::StrongestFit
        | ConstructionHeuristicType::StrongestFitDecreasing => select_by_strength(
            phase_scope,
            selected_group_candidates,
            construction_obligation,
            StrengthPolicy::Strongest,
        ),
        ConstructionHeuristicType::AllocateEntityFromQueue
        | ConstructionHeuristicType::AllocateToValueFromQueue
        | ConstructionHeuristicType::ListRoundRobin
        | ConstructionHeuristicType::ListCheapestInsertion
        | ConstructionHeuristicType::ListRegretInsertion
        | ConstructionHeuristicType::ListClarkeWright
        | ConstructionHeuristicType::ListKOpt => unreachable!(
            "grouped scalar construction should reject unsupported heuristic {:?} before selection",
            construction_type
        ),
    }
}

fn select_first_fit<S, D, ProgressCb>(
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
    mut candidates: Vec<NormalizedGroupedCandidate<S>>,
    construction_obligation: ConstructionObligation,
) -> Option<GroupedSelection<S>>
where
    S: PlanningSolution + 'static,
    S::Score: Score + Copy,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let baseline_score =
        keep_current_allowed(candidates[0].keep_current_legal, construction_obligation)
            .then(|| phase_scope.calculate_score());
    for idx in 0..candidates.len() {
        let score = evaluate_candidate(phase_scope, &candidates[idx].mov);
        if baseline_score.is_none_or(|baseline| score > baseline) {
            let candidate = candidates.remove(idx);
            return Some(commit(candidate, score));
        }
    }
    Some(complete_only(
        candidates.remove(0),
        baseline_score.is_some(),
    ))
}

fn select_best_score<S, D, ProgressCb>(
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
    mut candidates: Vec<NormalizedGroupedCandidate<S>>,
    construction_obligation: ConstructionObligation,
) -> Option<GroupedSelection<S>>
where
    S: PlanningSolution + 'static,
    S::Score: Score + Copy,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let baseline_score =
        keep_current_allowed(candidates[0].keep_current_legal, construction_obligation)
            .then(|| phase_scope.calculate_score());
    let mut best = None;
    for (idx, candidate) in candidates.iter().enumerate() {
        let score = evaluate_candidate(phase_scope, &candidate.mov);
        if best.is_none_or(|(_, best_score)| score > best_score) {
            best = Some((idx, score));
        }
    }

    let Some((best_idx, best_score)) = best else {
        return Some(complete_only(candidates.remove(0), false));
    };
    if baseline_score.is_none_or(|baseline| best_score > baseline) {
        Some(commit(candidates.remove(best_idx), best_score))
    } else {
        Some(complete_only(candidates.remove(0), true))
    }
}

#[derive(Clone, Copy)]
enum StrengthPolicy {
    Weakest,
    Strongest,
}

fn select_by_strength<S, D, ProgressCb>(
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
    mut candidates: Vec<NormalizedGroupedCandidate<S>>,
    construction_obligation: ConstructionObligation,
    policy: StrengthPolicy,
) -> Option<GroupedSelection<S>>
where
    S: PlanningSolution + 'static,
    S::Score: Score + Copy,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let selected_idx = candidates
        .iter()
        .enumerate()
        .min_by(|(left_idx, left), (right_idx, right)| {
            let left_key = left
                .value_order_key
                .expect("validated grouped value order key");
            let right_key = right
                .value_order_key
                .expect("validated grouped value order key");
            let ordering = match policy {
                StrengthPolicy::Weakest => left_key.cmp(&right_key),
                StrengthPolicy::Strongest => right_key.cmp(&left_key),
            };
            ordering
                .then_with(|| left.sequence.cmp(&right.sequence))
                .then(left_idx.cmp(right_idx))
        })
        .map(|(idx, _)| idx)?;

    let score = evaluate_candidate(phase_scope, &candidates[selected_idx].mov);
    let baseline_score = keep_current_allowed(
        candidates[selected_idx].keep_current_legal,
        construction_obligation,
    )
    .then(|| phase_scope.calculate_score());
    if baseline_score.is_none_or(|baseline| score > baseline) {
        Some(commit(candidates.remove(selected_idx), score))
    } else {
        Some(complete_only(candidates.remove(selected_idx), true))
    }
}

fn evaluate_candidate<S, D, ProgressCb>(
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
    mov: &CompoundScalarMove<S>,
) -> S::Score
where
    S: PlanningSolution,
    S::Score: Score,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let generation_started = Instant::now();
    phase_scope.record_generated_move(generation_started.elapsed());
    let evaluation_started = Instant::now();
    let score = evaluate_trial_move(phase_scope.score_director_mut(), mov);
    phase_scope.record_score_calculation();
    phase_scope.record_evaluated_move(evaluation_started.elapsed());
    score
}

fn commit<S>(candidate: NormalizedGroupedCandidate<S>, score: S::Score) -> GroupedSelection<S>
where
    S: PlanningSolution,
{
    GroupedSelection::Commit {
        group_slot: candidate.group_slot,
        scalar_slots: candidate.scalar_slots,
        mov: candidate.mov,
        score,
    }
}

fn complete_only<S>(candidate: NormalizedGroupedCandidate<S>, kept: bool) -> GroupedSelection<S>
where
    S: PlanningSolution,
{
    GroupedSelection::CompleteOnly {
        group_slot: candidate.group_slot,
        scalar_slots: candidate.scalar_slots,
        kept,
    }
}

fn next_group_slot<S>(
    construction_type: ConstructionHeuristicType,
    candidates: &[NormalizedGroupedCandidate<S>],
) -> ConstructionGroupSlotId
where
    S: PlanningSolution,
{
    if !heuristic_requires_entity_order_key(construction_type) {
        return candidates[0].group_slot.clone();
    }
    candidates
        .iter()
        .max_by(|left, right| {
            let left_key = left
                .entity_order_key
                .expect("validated grouped entity order key");
            let right_key = right
                .entity_order_key
                .expect("validated grouped entity order key");
            left_key
                .cmp(&right_key)
                .then_with(|| right.sequence.cmp(&left.sequence))
        })
        .expect("candidate list is non-empty")
        .group_slot
        .clone()
}

fn validate_heuristic_metadata<S>(
    construction_type: ConstructionHeuristicType,
    candidates: &[NormalizedGroupedCandidate<S>],
) where
    S: PlanningSolution,
{
    if heuristic_requires_entity_order_key(construction_type)
        && candidates
            .iter()
            .any(|candidate| candidate.entity_order_key.is_none())
    {
        panic!(
            "grouped scalar construction heuristic {:?} requires ScalarCandidate::with_construction_entity_order_key",
            construction_type
        );
    }
    if heuristic_requires_value_order_key(construction_type)
        && candidates
            .iter()
            .any(|candidate| candidate.value_order_key.is_none())
    {
        panic!(
            "grouped scalar construction heuristic {:?} requires ScalarCandidate::with_construction_value_order_key",
            construction_type
        );
    }
}

fn heuristic_requires_entity_order_key(heuristic: ConstructionHeuristicType) -> bool {
    matches!(
        heuristic,
        ConstructionHeuristicType::FirstFitDecreasing
            | ConstructionHeuristicType::WeakestFitDecreasing
            | ConstructionHeuristicType::StrongestFitDecreasing
    )
}

fn heuristic_requires_value_order_key(heuristic: ConstructionHeuristicType) -> bool {
    matches!(
        heuristic,
        ConstructionHeuristicType::WeakestFit
            | ConstructionHeuristicType::WeakestFitDecreasing
            | ConstructionHeuristicType::StrongestFit
            | ConstructionHeuristicType::StrongestFitDecreasing
    )
}
