//! Shared contiguous-sublist relocation mechanics.

use smallvec::{smallvec, SmallVec};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::metadata::{
    encode_usize, hash_str, MoveTabuScope, ScopedEntityTabuToken, ScopedValueTabuToken,
};
use crate::heuristic::r#move::segment_layout::{
    derive_segment_relocation_layout, SegmentRelocationCoords,
};
use crate::heuristic::r#move::MoveTabuSignature;
use crate::stats::CandidateTraceIdentity;

use super::{ListRangeAccess, ListWindowAccess};

pub(crate) fn sublist_change_is_doable<S, A, D>(
    access: &A,
    coordinates: SegmentRelocationCoords,
    score_director: &D,
) -> bool
where
    S: PlanningSolution,
    A: ListWindowAccess<S>,
    D: Director<S>,
{
    if coordinates.source_range.start >= coordinates.source_range.end {
        return false;
    }

    let solution = score_director.working_solution();
    let source_len = access.list_len(solution, coordinates.source_entity_index);
    if coordinates.source_range.end > source_len {
        return false;
    }

    let destination_len = access.list_len(solution, coordinates.dest_entity_index);
    let maximum_destination = if coordinates.source_entity_index == coordinates.dest_entity_index {
        source_len - coordinates.source_range.len()
    } else {
        destination_len
    };
    if coordinates.dest_position > maximum_destination {
        return false;
    }

    coordinates.source_entity_index != coordinates.dest_entity_index
        || coordinates.dest_position != coordinates.source_range.start
}

pub(crate) fn sublist_change_do_move<S, A, D>(
    access: &A,
    coordinates: SegmentRelocationCoords,
    score_director: &mut D,
) where
    S: PlanningSolution,
    A: ListWindowAccess<S>,
    D: Director<S>,
{
    let layout = derive_segment_relocation_layout(
        coordinates.source_entity_index,
        coordinates.source_range.start,
        coordinates.source_range.end,
        coordinates.dest_entity_index,
        coordinates.dest_position,
    );
    apply_sublist_change(access, layout.exact, score_director);
}

pub(crate) fn sublist_change_undo_move<S, A, D>(
    access: &A,
    coordinates: SegmentRelocationCoords,
    score_director: &mut D,
) where
    S: PlanningSolution,
    A: ListWindowAccess<S>,
    D: Director<S>,
{
    let layout = derive_segment_relocation_layout(
        coordinates.source_entity_index,
        coordinates.source_range.start,
        coordinates.source_range.end,
        coordinates.dest_entity_index,
        coordinates.dest_position,
    );
    apply_sublist_change(access, layout.inverse, score_director);
}

fn apply_sublist_change<S, A, D>(
    access: &A,
    coordinates: SegmentRelocationCoords,
    score_director: &mut D,
) where
    S: PlanningSolution,
    A: ListWindowAccess<S>,
    D: Director<S>,
{
    let descriptor_index = access.descriptor_index();
    score_director.before_variable_changed(descriptor_index, coordinates.source_entity_index);
    if coordinates.source_entity_index != coordinates.dest_entity_index {
        score_director.before_variable_changed(descriptor_index, coordinates.dest_entity_index);
    }

    let elements = access
        .sublist_remove(
            score_director.working_solution_mut(),
            coordinates.source_entity_index,
            coordinates.source_range.start,
            coordinates.source_range.end,
        )
        .expect("validated list window access should remove the requested segment");
    access
        .sublist_insert(
            score_director.working_solution_mut(),
            coordinates.dest_entity_index,
            coordinates.dest_position,
            elements,
        )
        .expect("validated list window access should insert the requested segment");

    score_director.after_variable_changed(descriptor_index, coordinates.source_entity_index);
    if coordinates.source_entity_index != coordinates.dest_entity_index {
        score_director.after_variable_changed(descriptor_index, coordinates.dest_entity_index);
    }
}

pub(crate) fn sublist_change_tabu_signature<S, A, D>(
    access: &A,
    coordinates: SegmentRelocationCoords,
    score_director: &D,
) -> MoveTabuSignature
where
    S: PlanningSolution,
    A: ListRangeAccess<S>,
    D: Director<S>,
{
    let layout = derive_segment_relocation_layout(
        coordinates.source_entity_index,
        coordinates.source_range.start,
        coordinates.source_range.end,
        coordinates.dest_entity_index,
        coordinates.dest_position,
    );
    let mut moved_ids: SmallVec<[u64; 2]> = SmallVec::new();
    for position in coordinates.source_range.start..coordinates.source_range.end {
        let value = access.list_get(
            score_director.working_solution(),
            coordinates.source_entity_index,
            position,
        );
        moved_ids.push(access.tabu_value_id(score_director.working_solution(), value.as_ref()));
    }
    let source_entity_id = encode_usize(coordinates.source_entity_index);
    let destination_entity_id = encode_usize(coordinates.dest_entity_index);
    let scope = MoveTabuScope::new(access.descriptor_index(), access.variable_name());
    let mut entity_tokens: SmallVec<[ScopedEntityTabuToken; 2]> =
        smallvec![scope.entity_token(source_entity_id)];
    if coordinates.source_entity_index != coordinates.dest_entity_index {
        entity_tokens.push(scope.entity_token(destination_entity_id));
    }
    let destination_value_tokens: SmallVec<[ScopedValueTabuToken; 2]> = moved_ids
        .iter()
        .copied()
        .map(|value_id| scope.value_token(value_id))
        .collect();
    let mut move_id = smallvec![
        encode_usize(access.descriptor_index()),
        hash_str(access.variable_name()),
        source_entity_id,
        encode_usize(layout.exact.source_range.start),
        encode_usize(layout.exact.source_range.end),
        destination_entity_id,
        encode_usize(layout.exact.dest_position)
    ];
    move_id.extend(moved_ids.iter().copied());
    let mut undo_move_id = smallvec![
        encode_usize(access.descriptor_index()),
        hash_str(access.variable_name()),
        encode_usize(layout.inverse.source_entity_index),
        encode_usize(layout.inverse.source_range.start),
        encode_usize(layout.inverse.source_range.end),
        encode_usize(layout.inverse.dest_entity_index),
        encode_usize(layout.inverse.dest_position)
    ];
    undo_move_id.extend(moved_ids.iter().copied());

    MoveTabuSignature::new(scope, move_id, undo_move_id)
        .with_entity_tokens(entity_tokens)
        .with_destination_value_tokens(destination_value_tokens)
}

pub(crate) fn sublist_change_candidate_trace_identity<S, A>(
    access: &A,
    coordinates: SegmentRelocationCoords,
) -> CandidateTraceIdentity
where
    A: ListRangeAccess<S>,
{
    CandidateTraceIdentity::logical_move(
        access.descriptor_index(),
        access.variable_name(),
        "sublist_change",
        [
            coordinates.source_entity_index,
            coordinates.source_range.start,
            coordinates.source_range.end,
            coordinates.dest_entity_index,
            coordinates.dest_position,
        ],
    )
}
