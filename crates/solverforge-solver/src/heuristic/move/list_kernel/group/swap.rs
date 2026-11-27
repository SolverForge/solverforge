//! Group swap: two equally sized groups on two owners exchange their member
//! slots.

use smallvec::{smallvec, SmallVec};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::metadata::{encode_usize, hash_str, MoveTabuScope};
use crate::heuristic::r#move::MoveTabuSignature;
use crate::stats::CandidateTraceIdentity;

use super::super::{ListChangeAccess, ListMoveAccess};
use super::{
    entities_are_free, entity_tokens, member_value_ids, notify_after, notify_before, place_members,
    positions_are_distinct_and_in_bounds, push_coordinates, sorted, take_members, value_tokens,
    GroupSwapCoordinates, MemberCoordinates,
};

fn swap_sides(coordinates: &GroupSwapCoordinates) -> (MemberCoordinates, MemberCoordinates) {
    let first = coordinates
        .first_positions
        .iter()
        .map(|&position| (coordinates.first_entity, position))
        .collect();
    let second = coordinates
        .second_positions
        .iter()
        .map(|&position| (coordinates.second_entity, position))
        .collect();
    (first, second)
}

/// Member slots of one side, taken over in declared order by the other
/// group: its `i`-th member gets the `i`-th lowest slot.
fn exchanged_slots(entity: usize, positions: &[usize]) -> MemberCoordinates {
    sorted(positions)
        .into_iter()
        .map(|position| (entity, position))
        .collect()
}

pub(crate) fn group_swap_is_doable<S, A, D>(
    access: &A,
    coordinates: &GroupSwapCoordinates,
    score_director: &D,
) -> bool
where
    S: PlanningSolution,
    A: ListChangeAccess<S>,
    D: Director<S>,
{
    let solution = score_director.working_solution();
    !coordinates.first_positions.is_empty()
        && coordinates.first_positions.len() == coordinates.second_positions.len()
        && coordinates.first_entity != coordinates.second_entity
        && positions_are_distinct_and_in_bounds(
            &coordinates.first_positions,
            access.list_len(solution, coordinates.first_entity),
        )
        && positions_are_distinct_and_in_bounds(
            &coordinates.second_positions,
            access.list_len(solution, coordinates.second_entity),
        )
        && entities_are_free(
            score_director,
            access.descriptor_index(),
            &coordinates.entities(),
        )
}

pub(crate) fn group_swap_do_move<S, A, D>(
    access: &A,
    coordinates: &GroupSwapCoordinates,
    score_director: &mut D,
) where
    S: PlanningSolution,
    A: ListChangeAccess<S>,
    D: Director<S>,
{
    let descriptor_index = access.descriptor_index();
    let entities = coordinates.entities();
    let (first, second) = swap_sides(coordinates);
    notify_before(score_director, descriptor_index, &entities);
    let first_values = take_members(access, &first, score_director);
    let second_values = take_members(access, &second, score_director);
    let first_slots = exchanged_slots(coordinates.first_entity, &coordinates.first_positions);
    let second_slots = exchanged_slots(coordinates.second_entity, &coordinates.second_positions);
    place_members(access, second_values, &first_slots, score_director);
    place_members(access, first_values, &second_slots, score_director);
    notify_after(score_director, descriptor_index, &entities);
}

pub(crate) fn group_swap_undo_move<S, A, D>(
    access: &A,
    coordinates: &GroupSwapCoordinates,
    score_director: &mut D,
) where
    S: PlanningSolution,
    A: ListChangeAccess<S>,
    D: Director<S>,
{
    let descriptor_index = access.descriptor_index();
    let entities = coordinates.entities();
    let (first, second) = swap_sides(coordinates);
    let first_slots = exchanged_slots(coordinates.first_entity, &coordinates.first_positions);
    let second_slots = exchanged_slots(coordinates.second_entity, &coordinates.second_positions);
    notify_before(score_director, descriptor_index, &entities);
    let second_values = take_members(access, &first_slots, score_director);
    let first_values = take_members(access, &second_slots, score_director);
    place_members(access, first_values, &first, score_director);
    place_members(access, second_values, &second, score_director);
    notify_after(score_director, descriptor_index, &entities);
}

pub(crate) fn group_swap_tabu_signature<S, A, D>(
    access: &A,
    coordinates: &GroupSwapCoordinates,
    score_director: &D,
) -> MoveTabuSignature
where
    S: PlanningSolution,
    A: ListMoveAccess<S>,
    D: Director<S>,
{
    let (first, second) = swap_sides(coordinates);
    let first_ids = member_value_ids(access, &first, score_director);
    let second_ids = member_value_ids(access, &second, score_director);
    let scope = MoveTabuScope::new(access.descriptor_index(), access.variable_name());
    let header = || -> SmallVec<[u64; 8]> {
        let mut header: SmallVec<[u64; 8]> = smallvec![
            encode_usize(access.descriptor_index()),
            hash_str(access.variable_name()),
            encode_usize(first.len())
        ];
        push_coordinates(&mut header, &first);
        push_coordinates(&mut header, &second);
        header
    };
    let mut forward = header();
    forward.extend(first_ids.iter().copied());
    forward.extend(second_ids.iter().copied());
    let mut backward = header();
    backward.extend(second_ids.iter().copied());
    backward.extend(first_ids.iter().copied());
    let moved: SmallVec<[u64; 8]> = first_ids.iter().chain(&second_ids).copied().collect();
    MoveTabuSignature::from_identity(scope, forward, backward)
        .with_entity_tokens(entity_tokens(scope, &coordinates.entities()))
        .with_destination_value_tokens(value_tokens(scope, &moved))
}

pub(crate) fn group_swap_candidate_trace_identity<S, A>(
    access: &A,
    coordinates: &GroupSwapCoordinates,
) -> CandidateTraceIdentity
where
    A: ListMoveAccess<S>,
{
    CandidateTraceIdentity::logical_move(
        access.descriptor_index(),
        access.variable_name(),
        "list_group_swap",
        std::iter::once(coordinates.first_entity)
            .chain(coordinates.first_positions.iter().copied())
            .chain(std::iter::once(coordinates.second_entity))
            .chain(coordinates.second_positions.iter().copied()),
    )
}
