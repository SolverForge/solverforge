//! Shared list reversal mechanics.

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::metadata::{
    encode_usize, scoped_move_identity, MoveTabuScope, ScopedValueTabuToken, TABU_OP_LIST_REVERSE,
};
use crate::heuristic::r#move::MoveTabuSignature;
use crate::stats::CandidateTraceIdentity;

use super::{ListRangeAccess, ListReverseAccess};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ReverseCoordinates {
    pub(crate) entity: usize,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

pub(crate) fn reverse_is_doable<S, A, D>(
    access: &A,
    coordinates: ReverseCoordinates,
    score_director: &D,
) -> bool
where
    S: PlanningSolution,
    A: ListReverseAccess<S>,
    D: Director<S>,
{
    coordinates.end > coordinates.start + 1
        && coordinates.end <= access.list_len(score_director.working_solution(), coordinates.entity)
}

pub(crate) fn reverse_do_move<S, A, D>(
    access: &A,
    coordinates: ReverseCoordinates,
    score_director: &mut D,
) where
    S: PlanningSolution,
    A: ListReverseAccess<S>,
    D: Director<S>,
{
    let descriptor_index = access.descriptor_index();
    score_director.before_variable_changed(descriptor_index, coordinates.entity);
    access
        .list_reverse(
            score_director.working_solution_mut(),
            coordinates.entity,
            coordinates.start,
            coordinates.end,
        )
        .expect("validated list reverse access should reverse the requested range");
    score_director.after_variable_changed(descriptor_index, coordinates.entity);
}

pub(crate) fn reverse_tabu_signature<S, A, D>(
    access: &A,
    coordinates: ReverseCoordinates,
    score_director: &D,
) -> MoveTabuSignature
where
    S: PlanningSolution,
    A: ListRangeAccess<S>,
    D: Director<S>,
{
    let mut value_ids: SmallVec<[u64; 2]> = SmallVec::new();
    for position in coordinates.start..coordinates.end {
        let value = access.list_get(
            score_director.working_solution(),
            coordinates.entity,
            position,
        );
        value_ids.push(access.tabu_value_id(score_director.working_solution(), value.as_ref()));
    }
    let entity_id = encode_usize(coordinates.entity);
    let scope = MoveTabuScope::new(access.descriptor_index(), access.variable_name());
    let destination_value_tokens: SmallVec<[ScopedValueTabuToken; 2]> = value_ids
        .iter()
        .copied()
        .map(|value_id| scope.value_token(value_id))
        .collect();
    let move_id = scoped_move_identity(
        scope,
        TABU_OP_LIST_REVERSE,
        [
            entity_id,
            encode_usize(coordinates.start),
            encode_usize(coordinates.end),
        ],
    );

    MoveTabuSignature::new(scope, move_id.clone(), move_id)
        .with_entity_tokens([scope.entity_token(entity_id)])
        .with_destination_value_tokens(destination_value_tokens)
}

pub(crate) fn reverse_candidate_trace_identity<S, A>(
    access: &A,
    coordinates: ReverseCoordinates,
) -> CandidateTraceIdentity
where
    A: ListRangeAccess<S>,
{
    CandidateTraceIdentity::logical_move(
        access.descriptor_index(),
        access.variable_name(),
        "list_reverse",
        [coordinates.entity, coordinates.start, coordinates.end],
    )
}
