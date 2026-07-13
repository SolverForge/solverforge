use solverforge_config::RecreateHeuristicType;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::builder::RuntimeScalarSlot;

use super::super::spec::RuntimeScalarRecipe;
use super::{recipe_slot, RuntimeScalarMoveUndo};

pub(super) fn is_doable<S>(recipe: &RuntimeScalarRecipe<S>, solution: &S) -> bool
where
    S: PlanningSolution,
{
    match recipe {
        RuntimeScalarRecipe::Change {
            slot,
            entity_index,
            to_value,
        } => {
            entity_exists(slot, solution, *entity_index)
                && slot.current_value(solution, *entity_index) != *to_value
                && change_is_legal(slot, solution, *entity_index, *to_value)
        }
        RuntimeScalarRecipe::Swap {
            slot,
            left_entity_index,
            right_entity_index,
        } => {
            *left_entity_index != *right_entity_index
                && entity_exists(slot, solution, *left_entity_index)
                && entity_exists(slot, solution, *right_entity_index)
                && swap_is_legal(slot, solution, *left_entity_index, *right_entity_index)
        }
        RuntimeScalarRecipe::PillarChange {
            slot,
            entity_indices,
            to_value,
        } => {
            !entity_indices.is_empty()
                && entity_indices
                    .iter()
                    .copied()
                    .all(|entity| entity_exists(slot, solution, entity))
                && entity_indices.iter().any(|&entity| {
                    slot.current_value(solution, entity) != *to_value
                        && change_is_legal(slot, solution, entity, *to_value)
                })
        }
        RuntimeScalarRecipe::PillarSwap {
            slot,
            left_indices,
            right_indices,
        } => {
            let (Some(&left), Some(&right)) = (left_indices.first(), right_indices.first()) else {
                return false;
            };
            left_indices
                .iter()
                .chain(right_indices)
                .copied()
                .all(|entity| entity_exists(slot, solution, entity))
                && slot.current_value(solution, left) != slot.current_value(solution, right)
                && pillar_swap_is_legal(slot, solution, left_indices, right_indices)
        }
        RuntimeScalarRecipe::RuinRecreate {
            slot,
            entity_indices,
            value_candidate_limit,
            ..
        } => {
            !entity_indices.is_empty()
                && entity_indices
                    .iter()
                    .copied()
                    .all(|entity| entity_exists(slot, solution, entity))
                && entity_indices
                    .iter()
                    .any(|&entity| slot.current_value(solution, entity).is_some())
                && (slot.allows_unassigned()
                    || entity_indices.iter().all(|&entity| {
                        slot.current_value(solution, entity).is_none()
                            || has_recreate_candidate(
                                slot,
                                solution,
                                entity,
                                *value_candidate_limit,
                            )
                    }))
        }
    }
}

pub(super) fn do_move<S, D>(
    recipe: &RuntimeScalarRecipe<S>,
    director: &mut D,
) -> RuntimeScalarMoveUndo
where
    S: PlanningSolution,
    S::Score: Score,
    D: Director<S>,
{
    match recipe {
        RuntimeScalarRecipe::Change {
            slot,
            entity_index,
            to_value,
        } => RuntimeScalarMoveUndo::Change(apply_one(slot, director, *entity_index, *to_value)),
        RuntimeScalarRecipe::Swap {
            slot,
            left_entity_index,
            right_entity_index,
        } => {
            let left = slot.current_value(director.working_solution(), *left_entity_index);
            let right = slot.current_value(director.working_solution(), *right_entity_index);
            apply_many(
                slot,
                director,
                [(*left_entity_index, right), (*right_entity_index, left)],
            );
            RuntimeScalarMoveUndo::Swap(left, right)
        }
        RuntimeScalarRecipe::PillarChange {
            slot,
            entity_indices,
            to_value,
        } => RuntimeScalarMoveUndo::Many(apply_many(
            slot,
            director,
            entity_indices
                .iter()
                .copied()
                .map(|entity| (entity, *to_value)),
        )),
        RuntimeScalarRecipe::PillarSwap {
            slot,
            left_indices,
            right_indices,
        } => {
            let left = left_indices
                .first()
                .and_then(|&entity| slot.current_value(director.working_solution(), entity));
            let right = right_indices
                .first()
                .and_then(|&entity| slot.current_value(director.working_solution(), entity));
            RuntimeScalarMoveUndo::Many(apply_many(
                slot,
                director,
                left_indices
                    .iter()
                    .copied()
                    .map(|entity| (entity, right))
                    .chain(right_indices.iter().copied().map(|entity| (entity, left))),
            ))
        }
        RuntimeScalarRecipe::RuinRecreate {
            slot,
            entity_indices,
            value_candidate_limit,
            recreate_heuristic_type,
        } => {
            if !is_doable(recipe, director.working_solution()) {
                return RuntimeScalarMoveUndo::Many(Vec::new());
            }
            RuntimeScalarMoveUndo::Many(run_ruin_recreate(
                slot,
                director,
                entity_indices,
                *value_candidate_limit,
                *recreate_heuristic_type,
            ))
        }
    }
}

pub(super) fn undo_move<S, D>(
    recipe: &RuntimeScalarRecipe<S>,
    director: &mut D,
    undo: RuntimeScalarMoveUndo,
) where
    S: PlanningSolution,
    S::Score: Score,
    D: Director<S>,
{
    match (recipe, undo) {
        (
            RuntimeScalarRecipe::Change {
                slot, entity_index, ..
            },
            RuntimeScalarMoveUndo::Change(old),
        ) => {
            let _ = apply_one(slot, director, *entity_index, old);
        }
        (
            RuntimeScalarRecipe::Swap {
                slot,
                left_entity_index,
                right_entity_index,
            },
            RuntimeScalarMoveUndo::Swap(left, right),
        ) => {
            let _ = apply_many(
                slot,
                director,
                [(*left_entity_index, left), (*right_entity_index, right)],
            );
        }
        (_, RuntimeScalarMoveUndo::Many(old_values)) => {
            let _ = apply_many(recipe_slot(recipe), director, old_values);
        }
        _ => panic!("runtime scalar move undo shape must match recipe"),
    }
}

fn entity_exists<S>(slot: &RuntimeScalarSlot<S>, solution: &S, entity_index: usize) -> bool {
    entity_index < slot.entity_count(solution)
}

pub(super) fn change_is_legal<S>(
    slot: &RuntimeScalarSlot<S>,
    solution: &S,
    entity_index: usize,
    value: Option<usize>,
) -> bool {
    if slot.is_dynamic() {
        slot.value_is_legal(solution, entity_index, value)
    } else {
        value.is_some() || slot.allows_unassigned()
    }
}

fn swap_is_legal<S>(
    slot: &RuntimeScalarSlot<S>,
    solution: &S,
    left_entity_index: usize,
    right_entity_index: usize,
) -> bool {
    let left = slot.current_value(solution, left_entity_index);
    let right = slot.current_value(solution, right_entity_index);
    if !slot.is_dynamic() {
        // Static public `SwapMove` rechecks only value inequality after a
        // candidate has been selected. Source legality was already applied
        // while enumerating the immutable snapshot.
        return left != right;
    }
    left != right
        && slot.swap_destination_is_legal(solution, left_entity_index, right)
        && slot.swap_destination_is_legal(solution, right_entity_index, left)
}

fn pillar_swap_is_legal<S>(
    slot: &RuntimeScalarSlot<S>,
    solution: &S,
    left_indices: &[usize],
    right_indices: &[usize],
) -> bool {
    let (Some(&left), Some(&right)) = (left_indices.first(), right_indices.first()) else {
        return false;
    };
    if !slot.is_dynamic() {
        // As with scalar swap, the native public pillar move retains its
        // selected compatibility and only rechecks the representative values.
        return slot.current_value(solution, left) != slot.current_value(solution, right);
    }
    let left_value = slot.current_value(solution, left);
    let right_value = slot.current_value(solution, right);
    left_indices
        .iter()
        .copied()
        .all(|entity| slot.swap_destination_is_legal(solution, entity, right_value))
        && right_indices
            .iter()
            .copied()
            .all(|entity| slot.swap_destination_is_legal(solution, entity, left_value))
}

fn has_recreate_candidate<S>(
    slot: &RuntimeScalarSlot<S>,
    solution: &S,
    entity_index: usize,
    limit: Option<usize>,
) -> bool {
    let mut found = false;
    slot.visit_candidate_values(solution, entity_index, limit, &mut |_| found = true);
    found
}

pub(super) fn apply_one<S, D>(
    slot: &RuntimeScalarSlot<S>,
    director: &mut D,
    entity_index: usize,
    value: Option<usize>,
) -> Option<usize>
where
    S: PlanningSolution,
    D: Director<S>,
{
    let old = slot.current_value(director.working_solution(), entity_index);
    director.before_variable_changed(slot.descriptor_index(), entity_index);
    slot.set_value(director.working_solution_mut(), entity_index, value);
    director.after_variable_changed(slot.descriptor_index(), entity_index);
    old
}

fn apply_many<S, D, I>(
    slot: &RuntimeScalarSlot<S>,
    director: &mut D,
    edits: I,
) -> Vec<(usize, Option<usize>)>
where
    S: PlanningSolution,
    D: Director<S>,
    I: IntoIterator<Item = (usize, Option<usize>)>,
{
    let edits = edits.into_iter().collect::<Vec<_>>();
    let old = edits
        .iter()
        .map(|(entity, _)| {
            (
                *entity,
                slot.current_value(director.working_solution(), *entity),
            )
        })
        .collect::<Vec<_>>();
    for (entity, _) in &edits {
        director.before_variable_changed(slot.descriptor_index(), *entity);
    }
    for (entity, value) in edits {
        slot.set_value(director.working_solution_mut(), entity, value);
    }
    for (entity, _) in &old {
        director.after_variable_changed(slot.descriptor_index(), *entity);
    }
    old
}

fn run_ruin_recreate<S, D>(
    slot: &RuntimeScalarSlot<S>,
    director: &mut D,
    entity_indices: &[usize],
    value_candidate_limit: Option<usize>,
    recreate_heuristic_type: RecreateHeuristicType,
) -> Vec<(usize, Option<usize>)>
where
    S: PlanningSolution,
    S::Score: Score,
    D: Director<S>,
{
    let mut undo = Vec::with_capacity(entity_indices.len());
    for &entity in entity_indices {
        undo.push((entity, apply_one(slot, director, entity, None)));
    }
    for &entity in entity_indices {
        if slot
            .current_value(director.working_solution(), entity)
            .is_some()
        {
            continue;
        }
        let selected = match recreate_heuristic_type {
            RecreateHeuristicType::FirstFit => {
                choose_first_fit(slot, director, entity, value_candidate_limit)
            }
            RecreateHeuristicType::CheapestInsertion => {
                choose_cheapest_insertion(slot, director, entity, value_candidate_limit)
            }
        };
        if let Some(value) = selected {
            let _ = apply_one(slot, director, entity, Some(value));
        }
    }
    undo
}

fn choose_first_fit<S, D>(
    slot: &RuntimeScalarSlot<S>,
    director: &mut D,
    entity_index: usize,
    value_candidate_limit: Option<usize>,
) -> Option<usize>
where
    S: PlanningSolution,
    S::Score: Score,
    D: Director<S>,
{
    let baseline = slot.allows_unassigned().then(|| director.calculate_score());
    let mut candidates = Vec::new();
    slot.visit_candidate_values(
        director.working_solution(),
        entity_index,
        value_candidate_limit,
        &mut |value| candidates.push(value),
    );
    for value in candidates {
        if !change_is_legal(slot, director.working_solution(), entity_index, Some(value)) {
            continue;
        }
        let score_state = director.snapshot_score_state();
        let undo = apply_one(slot, director, entity_index, Some(value));
        let score = director.calculate_score();
        let _ = apply_one(slot, director, entity_index, undo);
        director.restore_score_state(score_state);
        if baseline.is_none_or(|baseline| score > baseline) {
            return Some(value);
        }
    }
    None
}

fn choose_cheapest_insertion<S, D>(
    slot: &RuntimeScalarSlot<S>,
    director: &mut D,
    entity_index: usize,
    value_candidate_limit: Option<usize>,
) -> Option<usize>
where
    S: PlanningSolution,
    S::Score: Score,
    D: Director<S>,
{
    let baseline = slot.allows_unassigned().then(|| director.calculate_score());
    let mut candidates = Vec::new();
    slot.visit_candidate_values(
        director.working_solution(),
        entity_index,
        value_candidate_limit,
        &mut |value| candidates.push(value),
    );
    let mut best: Option<(usize, usize, S::Score)> = None;
    for (order, value) in candidates.into_iter().enumerate() {
        if !change_is_legal(slot, director.working_solution(), entity_index, Some(value)) {
            continue;
        }
        let score_state = director.snapshot_score_state();
        let undo = apply_one(slot, director, entity_index, Some(value));
        let score = director.calculate_score();
        let _ = apply_one(slot, director, entity_index, undo);
        director.restore_score_state(score_state);
        if best.as_ref().is_none_or(|(best_order, _, best_score)| {
            score > *best_score || (score == *best_score && order < *best_order)
        }) {
            best = Some((order, value, score));
        }
    }
    best.and_then(|(_, value, score)| {
        baseline
            .is_none_or(|baseline| score >= baseline)
            .then_some(value)
    })
}
