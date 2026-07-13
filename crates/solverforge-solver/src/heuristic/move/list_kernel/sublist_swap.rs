//! Shared contiguous-sublist exchange mechanics.

use smallvec::{smallvec, SmallVec};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::metadata::{
    encode_usize, hash_str, MoveTabuScope, ScopedEntityTabuToken, ScopedValueTabuToken,
};
use crate::heuristic::r#move::segment_layout::{derive_segment_swap_layout, SegmentSwapCoords};
use crate::heuristic::r#move::MoveTabuSignature;
use crate::stats::CandidateTraceIdentity;

use super::{ListRangeAccess, ListWindowAccess};

pub(crate) fn sublist_swap_is_doable<S, A, D>(
    access: &A,
    coordinates: SegmentSwapCoords,
    score_director: &D,
) -> bool
where
    S: PlanningSolution,
    A: ListWindowAccess<S>,
    D: Director<S>,
{
    if coordinates.first_range.start >= coordinates.first_range.end
        || coordinates.second_range.start >= coordinates.second_range.end
    {
        return false;
    }

    let solution = score_director.working_solution();
    if coordinates.first_range.end > access.list_len(solution, coordinates.first_entity_index)
        || coordinates.second_range.end > access.list_len(solution, coordinates.second_entity_index)
    {
        return false;
    }

    !(coordinates.is_intra_list()
        && coordinates.first_range.start < coordinates.second_range.end
        && coordinates.second_range.start < coordinates.first_range.end)
}

pub(crate) fn sublist_swap_do_move<S, A, D>(
    access: &A,
    coordinates: SegmentSwapCoords,
    score_director: &mut D,
) where
    S: PlanningSolution,
    A: ListWindowAccess<S>,
    D: Director<S>,
{
    let layout = derive_segment_swap_layout(
        coordinates.first_entity_index,
        coordinates.first_range.start,
        coordinates.first_range.end,
        coordinates.second_entity_index,
        coordinates.second_range.start,
        coordinates.second_range.end,
    );
    apply_sublist_swap(access, layout.exact, score_director);
}

pub(crate) fn sublist_swap_undo_move<S, A, D>(
    access: &A,
    coordinates: SegmentSwapCoords,
    score_director: &mut D,
) where
    S: PlanningSolution,
    A: ListWindowAccess<S>,
    D: Director<S>,
{
    let layout = derive_segment_swap_layout(
        coordinates.first_entity_index,
        coordinates.first_range.start,
        coordinates.first_range.end,
        coordinates.second_entity_index,
        coordinates.second_range.start,
        coordinates.second_range.end,
    );
    apply_sublist_swap(access, layout.inverse, score_director);
}

fn apply_sublist_swap<S, A, D>(access: &A, coordinates: SegmentSwapCoords, score_director: &mut D)
where
    S: PlanningSolution,
    A: ListWindowAccess<S>,
    D: Director<S>,
{
    let descriptor_index = access.descriptor_index();
    score_director.before_variable_changed(descriptor_index, coordinates.first_entity_index);
    if !coordinates.is_intra_list() {
        score_director.before_variable_changed(descriptor_index, coordinates.second_entity_index);
    }

    if coordinates.is_intra_list() {
        let (early_range, late_range) = coordinates.ordered_ranges();
        let late_elements = access
            .sublist_remove(
                score_director.working_solution_mut(),
                coordinates.first_entity_index,
                late_range.start,
                late_range.end,
            )
            .expect("validated list window access should remove the later segment");
        let early_elements = access
            .sublist_remove(
                score_director.working_solution_mut(),
                coordinates.first_entity_index,
                early_range.start,
                early_range.end,
            )
            .expect("validated list window access should remove the earlier segment");
        let late_len = late_range.len();
        let early_len = early_range.len();
        access
            .sublist_insert(
                score_director.working_solution_mut(),
                coordinates.first_entity_index,
                early_range.start,
                late_elements,
            )
            .expect("validated list window access should insert the later segment");
        let new_late_position = late_range.start - early_len + late_len;
        access
            .sublist_insert(
                score_director.working_solution_mut(),
                coordinates.first_entity_index,
                new_late_position,
                early_elements,
            )
            .expect("validated list window access should insert the earlier segment");
    } else {
        let first_elements = access
            .sublist_remove(
                score_director.working_solution_mut(),
                coordinates.first_entity_index,
                coordinates.first_range.start,
                coordinates.first_range.end,
            )
            .expect("validated list window access should remove the first segment");
        let second_elements = access
            .sublist_remove(
                score_director.working_solution_mut(),
                coordinates.second_entity_index,
                coordinates.second_range.start,
                coordinates.second_range.end,
            )
            .expect("validated list window access should remove the second segment");
        access
            .sublist_insert(
                score_director.working_solution_mut(),
                coordinates.first_entity_index,
                coordinates.first_range.start,
                second_elements,
            )
            .expect("validated list window access should insert the second segment");
        access
            .sublist_insert(
                score_director.working_solution_mut(),
                coordinates.second_entity_index,
                coordinates.second_range.start,
                first_elements,
            )
            .expect("validated list window access should insert the first segment");
    }

    score_director.after_variable_changed(descriptor_index, coordinates.first_entity_index);
    if !coordinates.is_intra_list() {
        score_director.after_variable_changed(descriptor_index, coordinates.second_entity_index);
    }
}

pub(crate) fn sublist_swap_tabu_signature<S, A, D>(
    access: &A,
    coordinates: SegmentSwapCoords,
    score_director: &D,
) -> MoveTabuSignature
where
    S: PlanningSolution,
    A: ListRangeAccess<S>,
    D: Director<S>,
{
    let layout = derive_segment_swap_layout(
        coordinates.first_entity_index,
        coordinates.first_range.start,
        coordinates.first_range.end,
        coordinates.second_entity_index,
        coordinates.second_range.start,
        coordinates.second_range.end,
    );
    let mut first_value_ids: SmallVec<[u64; 2]> = SmallVec::new();
    for position in coordinates.first_range.start..coordinates.first_range.end {
        let value = access.list_get(
            score_director.working_solution(),
            coordinates.first_entity_index,
            position,
        );
        first_value_ids
            .push(access.tabu_value_id(score_director.working_solution(), value.as_ref()));
    }
    let mut second_value_ids: SmallVec<[u64; 2]> = SmallVec::new();
    for position in coordinates.second_range.start..coordinates.second_range.end {
        let value = access.list_get(
            score_director.working_solution(),
            coordinates.second_entity_index,
            position,
        );
        second_value_ids
            .push(access.tabu_value_id(score_director.working_solution(), value.as_ref()));
    }
    let first_entity_id = encode_usize(coordinates.first_entity_index);
    let second_entity_id = encode_usize(coordinates.second_entity_index);
    let scope = MoveTabuScope::new(access.descriptor_index(), access.variable_name());
    let mut entity_tokens: SmallVec<[ScopedEntityTabuToken; 2]> =
        smallvec![scope.entity_token(first_entity_id)];
    if !coordinates.is_intra_list() {
        entity_tokens.push(scope.entity_token(second_entity_id));
    }
    let destination_value_tokens: SmallVec<[ScopedValueTabuToken; 2]> = first_value_ids
        .iter()
        .chain(second_value_ids.iter())
        .copied()
        .map(|value_id| scope.value_token(value_id))
        .collect();
    let mut move_id = smallvec![
        encode_usize(access.descriptor_index()),
        hash_str(access.variable_name()),
        encode_usize(layout.exact.first_entity_index),
        encode_usize(layout.exact.first_range.start),
        encode_usize(layout.exact.first_range.end),
        encode_usize(layout.exact.second_entity_index),
        encode_usize(layout.exact.second_range.start),
        encode_usize(layout.exact.second_range.end)
    ];
    move_id.extend(first_value_ids.iter().copied());
    move_id.extend(second_value_ids.iter().copied());
    let mut undo_move_id = smallvec![
        encode_usize(access.descriptor_index()),
        hash_str(access.variable_name()),
        encode_usize(layout.inverse.first_entity_index),
        encode_usize(layout.inverse.first_range.start),
        encode_usize(layout.inverse.first_range.end),
        encode_usize(layout.inverse.second_entity_index),
        encode_usize(layout.inverse.second_range.start),
        encode_usize(layout.inverse.second_range.end)
    ];
    undo_move_id.extend(second_value_ids.iter().copied());
    undo_move_id.extend(first_value_ids.iter().copied());

    MoveTabuSignature::new(scope, move_id, undo_move_id)
        .with_entity_tokens(entity_tokens)
        .with_destination_value_tokens(destination_value_tokens)
}

pub(crate) fn sublist_swap_candidate_trace_identity<S, A>(
    access: &A,
    coordinates: SegmentSwapCoords,
) -> CandidateTraceIdentity
where
    A: ListRangeAccess<S>,
{
    CandidateTraceIdentity::logical_move(
        access.descriptor_index(),
        access.variable_name(),
        "sublist_swap",
        [
            coordinates.first_entity_index,
            coordinates.first_range.start,
            coordinates.first_range.end,
            coordinates.second_entity_index,
            coordinates.second_range.start,
            coordinates.second_range.end,
        ],
    )
}
