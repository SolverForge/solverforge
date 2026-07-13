//! Shared exchange move mechanics.

use smallvec::{smallvec, SmallVec};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::stats::CandidateTraceIdentity;

use super::{ListMoveAccess, ListSwapAccess};
use crate::heuristic::r#move::metadata::{
    encode_usize, ordered_coordinate_pair, scoped_move_identity, MoveTabuScope,
    ScopedEntityTabuToken, TABU_OP_LIST_SWAP,
};
use crate::heuristic::r#move::MoveTabuSignature;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SwapCoordinates {
    pub(crate) first_entity: usize,
    pub(crate) first_position: usize,
    pub(crate) second_entity: usize,
    pub(crate) second_position: usize,
}

impl SwapCoordinates {
    pub(crate) const fn is_intra_list(self) -> bool {
        self.first_entity == self.second_entity
    }
}

pub(crate) fn swap_is_doable<S, A, D>(
    access: &A,
    coordinates: SwapCoordinates,
    score_director: &D,
) -> bool
where
    S: PlanningSolution,
    A: ListSwapAccess<S>,
    D: Director<S>,
{
    let solution = score_director.working_solution();
    if coordinates.first_position >= access.list_len(solution, coordinates.first_entity)
        || coordinates.second_position >= access.list_len(solution, coordinates.second_entity)
        || (coordinates.is_intra_list()
            && coordinates.first_position == coordinates.second_position)
    {
        return false;
    }
    access.list_get(
        solution,
        coordinates.first_entity,
        coordinates.first_position,
    ) != access.list_get(
        solution,
        coordinates.second_entity,
        coordinates.second_position,
    )
}

pub(crate) fn swap_do_move<S, A, D>(
    access: &A,
    coordinates: SwapCoordinates,
    score_director: &mut D,
) where
    S: PlanningSolution,
    A: ListSwapAccess<S>,
    D: Director<S>,
{
    let first_value = access
        .list_get(
            score_director.working_solution(),
            coordinates.first_entity,
            coordinates.first_position,
        )
        .expect("first position should be valid");
    let second_value = access
        .list_get(
            score_director.working_solution(),
            coordinates.second_entity,
            coordinates.second_position,
        )
        .expect("second position should be valid");

    let descriptor_index = access.descriptor_index();
    score_director.before_variable_changed(descriptor_index, coordinates.first_entity);
    if !coordinates.is_intra_list() {
        score_director.before_variable_changed(descriptor_index, coordinates.second_entity);
    }
    access
        .list_set(
            score_director.working_solution_mut(),
            coordinates.first_entity,
            coordinates.first_position,
            second_value.clone(),
        )
        .expect("validated list swap access should set the first value");
    access
        .list_set(
            score_director.working_solution_mut(),
            coordinates.second_entity,
            coordinates.second_position,
            first_value.clone(),
        )
        .expect("validated list swap access should set the second value");
    score_director.after_variable_changed(descriptor_index, coordinates.first_entity);
    if !coordinates.is_intra_list() {
        score_director.after_variable_changed(descriptor_index, coordinates.second_entity);
    }
}

pub(crate) fn swap_tabu_signature<S, A, D>(
    access: &A,
    coordinates: SwapCoordinates,
    score_director: &D,
) -> MoveTabuSignature
where
    S: PlanningSolution,
    A: ListMoveAccess<S>,
    D: Director<S>,
{
    let first_value = access.list_get(
        score_director.working_solution(),
        coordinates.first_entity,
        coordinates.first_position,
    );
    let first_id = access.tabu_value_id(score_director.working_solution(), first_value.as_ref());
    let second_value = access.list_get(
        score_director.working_solution(),
        coordinates.second_entity,
        coordinates.second_position,
    );
    let second_id = access.tabu_value_id(score_director.working_solution(), second_value.as_ref());
    let first_entity_id = encode_usize(coordinates.first_entity);
    let second_entity_id = encode_usize(coordinates.second_entity);
    let scope = MoveTabuScope::new(access.descriptor_index(), access.variable_name());
    let mut entity_tokens: SmallVec<[ScopedEntityTabuToken; 2]> =
        smallvec![scope.entity_token(first_entity_id)];
    if !coordinates.is_intra_list() {
        entity_tokens.push(scope.entity_token(second_entity_id));
    }
    let coordinate_pair = ordered_coordinate_pair(
        (first_entity_id, encode_usize(coordinates.first_position)),
        (second_entity_id, encode_usize(coordinates.second_position)),
    );
    let move_id = scoped_move_identity(
        scope,
        TABU_OP_LIST_SWAP,
        coordinate_pair
            .into_iter()
            .flat_map(|(entity_id, position)| [entity_id, position]),
    );

    MoveTabuSignature::new(scope, move_id.clone(), move_id)
        .with_entity_tokens(entity_tokens)
        .with_destination_value_tokens([scope.value_token(second_id), scope.value_token(first_id)])
}

pub(crate) fn swap_candidate_trace_identity<S, A>(
    access: &A,
    coordinates: SwapCoordinates,
) -> CandidateTraceIdentity
where
    A: ListMoveAccess<S>,
{
    CandidateTraceIdentity::logical_move(
        access.descriptor_index(),
        access.variable_name(),
        "list_swap",
        [
            coordinates.first_entity,
            coordinates.first_position,
            coordinates.second_entity,
            coordinates.second_position,
        ],
    )
}
