//! Shared contiguous-window permutation mechanics.

use smallvec::{smallvec, SmallVec};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::metadata::{
    encode_usize, hash_str, MoveTabuScope, ScopedValueTabuToken, TABU_OP_LIST_PERMUTE,
};
use crate::heuristic::r#move::{MoveTabuSignature, MAX_LIST_PERMUTE_WINDOW_SIZE};
use crate::stats::CandidateTraceIdentity;

use super::{ListRangeAccess, ListWindowAccess};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PermuteCoordinates {
    pub(crate) entity: usize,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

pub(crate) fn permute_is_doable<S, A, D>(
    access: &A,
    coordinates: PermuteCoordinates,
    permutation: &[usize],
    score_director: &D,
) -> bool
where
    S: PlanningSolution,
    A: ListWindowAccess<S>,
    D: Director<S>,
{
    coordinates.start < coordinates.end
        && coordinates.end <= access.list_len(score_director.working_solution(), coordinates.entity)
        && valid_non_identity_permutation(permutation)
}

pub(crate) fn permute_do_move<S, A, D>(
    access: &A,
    coordinates: PermuteCoordinates,
    permutation: &[usize],
    score_director: &mut D,
) -> Vec<A::Element>
where
    S: PlanningSolution,
    A: ListWindowAccess<S>,
    D: Director<S>,
{
    let descriptor_index = access.descriptor_index();
    score_director.before_variable_changed(descriptor_index, coordinates.entity);
    let segment = access
        .sublist_remove(
            score_director.working_solution_mut(),
            coordinates.entity,
            coordinates.start,
            coordinates.end,
        )
        .expect("validated list window access should remove the requested segment");
    let reordered = permutation
        .iter()
        .map(|&index| segment[index].clone())
        .collect();
    access
        .sublist_insert(
            score_director.working_solution_mut(),
            coordinates.entity,
            coordinates.start,
            reordered,
        )
        .expect("validated list window access should insert the reordered segment");
    score_director.after_variable_changed(descriptor_index, coordinates.entity);
    segment
}

pub(crate) fn permute_undo_move<S, A, D>(
    access: &A,
    coordinates: PermuteCoordinates,
    undo: Vec<A::Element>,
    score_director: &mut D,
) where
    S: PlanningSolution,
    A: ListWindowAccess<S>,
    D: Director<S>,
{
    let descriptor_index = access.descriptor_index();
    score_director.before_variable_changed(descriptor_index, coordinates.entity);
    let _ = access
        .sublist_remove(
            score_director.working_solution_mut(),
            coordinates.entity,
            coordinates.start,
            coordinates.end,
        )
        .expect("validated list window access should remove the permuted segment");
    access
        .sublist_insert(
            score_director.working_solution_mut(),
            coordinates.entity,
            coordinates.start,
            undo,
        )
        .expect("validated list window access should restore the original segment");
    score_director.after_variable_changed(descriptor_index, coordinates.entity);
}

pub(crate) fn permute_tabu_signature<S, A, D>(
    access: &A,
    coordinates: PermuteCoordinates,
    permutation: &[usize],
    inverse_permutation: &[usize],
    score_director: &D,
) -> MoveTabuSignature
where
    S: PlanningSolution,
    A: ListRangeAccess<S>,
    D: Director<S>,
{
    let scope = MoveTabuScope::new(access.descriptor_index(), access.variable_name());
    let entity_id = encode_usize(coordinates.entity);
    let variable_id = hash_str(access.variable_name());
    let mut touched_value_ids = SmallVec::<[u64; 8]>::new();
    for position in coordinates.start..coordinates.end {
        let value = access.list_get(
            score_director.working_solution(),
            coordinates.entity,
            position,
        );
        touched_value_ids
            .push(access.tabu_value_id(score_director.working_solution(), value.as_ref()));
    }

    let mut move_id = smallvec![
        TABU_OP_LIST_PERMUTE,
        encode_usize(access.descriptor_index()),
        variable_id,
        entity_id,
        encode_usize(coordinates.start),
        encode_usize(coordinates.end),
    ];
    move_id.extend(permutation.iter().copied().map(encode_usize));
    move_id.extend(touched_value_ids.iter().copied());

    let mut undo_move_id = smallvec![
        TABU_OP_LIST_PERMUTE,
        encode_usize(access.descriptor_index()),
        variable_id,
        entity_id,
        encode_usize(coordinates.start),
        encode_usize(coordinates.end),
    ];
    undo_move_id.extend(inverse_permutation.iter().copied().map(encode_usize));
    undo_move_id.extend(touched_value_ids.iter().copied());

    let destination_value_tokens: SmallVec<[ScopedValueTabuToken; 2]> = touched_value_ids
        .iter()
        .copied()
        .map(|value_id| scope.value_token(value_id))
        .collect();

    MoveTabuSignature::new(scope, move_id, undo_move_id)
        .with_entity_tokens([scope.entity_token(entity_id)])
        .with_destination_value_tokens(destination_value_tokens)
}

pub(crate) fn permute_candidate_trace_identity<S, A>(
    access: &A,
    coordinates: PermuteCoordinates,
    permutation: &[usize],
) -> CandidateTraceIdentity
where
    A: ListRangeAccess<S>,
{
    let mut components = Vec::with_capacity(4 + permutation.len());
    components.extend([
        coordinates.entity,
        coordinates.start,
        coordinates.end,
        permutation.len(),
    ]);
    components.extend(permutation.iter().copied());
    CandidateTraceIdentity::logical_move(
        access.descriptor_index(),
        access.variable_name(),
        "list_permute",
        components,
    )
}

fn valid_non_identity_permutation(permutation: &[usize]) -> bool {
    let len = permutation.len();
    if !(2..=MAX_LIST_PERMUTE_WINDOW_SIZE).contains(&len) {
        return false;
    }
    let mut seen = [false; MAX_LIST_PERMUTE_WINDOW_SIZE];
    let mut is_identity = true;
    for (index, &value) in permutation.iter().enumerate() {
        if value >= len || seen[value] {
            return false;
        }
        seen[value] = true;
        is_identity &= value == index;
    }
    !is_identity
}
