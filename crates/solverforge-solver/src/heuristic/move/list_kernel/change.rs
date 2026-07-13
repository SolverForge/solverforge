//! Shared relocation move mechanics.

use smallvec::{smallvec, SmallVec};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::stats::CandidateTraceIdentity;

use super::{ListChangeAccess, ListMoveAccess};
use crate::heuristic::r#move::metadata::{
    encode_usize, hash_str, MoveTabuScope, ScopedEntityTabuToken,
};
use crate::heuristic::r#move::MoveTabuSignature;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ChangeCoordinates {
    pub(crate) source_entity: usize,
    pub(crate) source_position: usize,
    pub(crate) destination_entity: usize,
    pub(crate) destination_position: usize,
}

impl ChangeCoordinates {
    pub(crate) const fn is_intra_list(self) -> bool {
        self.source_entity == self.destination_entity
    }

    pub(crate) const fn adjusted_destination(self) -> usize {
        if self.is_intra_list() && self.destination_position > self.source_position {
            self.destination_position - 1
        } else {
            self.destination_position
        }
    }
}

/// The typed move cloned before insertion; the dynamic move transferred
/// ownership. The explicit policy keeps those public move contracts intact
/// while sharing all coordinates, checks, notifications, and metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChangeValueTransfer {
    CloneBeforeInsert,
    MoveIntoInsert,
}

pub(crate) fn change_is_doable<S, A, D>(
    access: &A,
    coordinates: ChangeCoordinates,
    score_director: &D,
) -> bool
where
    S: PlanningSolution,
    A: ListChangeAccess<S>,
    D: Director<S>,
{
    let solution = score_director.working_solution();
    let source_len = access.list_len(solution, coordinates.source_entity);
    if coordinates.source_position >= source_len {
        return false;
    }

    let destination_len = access.list_len(solution, coordinates.destination_entity);
    let maximum_destination = if coordinates.is_intra_list() {
        source_len
    } else {
        destination_len
    };
    if coordinates.destination_position > maximum_destination {
        return false;
    }

    !coordinates.is_intra_list()
        || (coordinates.source_position != coordinates.destination_position
            && coordinates.destination_position != coordinates.source_position + 1)
}

pub(crate) fn change_do_move<S, A, D>(
    access: &A,
    coordinates: ChangeCoordinates,
    transfer: ChangeValueTransfer,
    score_director: &mut D,
) where
    S: PlanningSolution,
    A: ListChangeAccess<S>,
    D: Director<S>,
{
    let descriptor_index = access.descriptor_index();
    score_director.before_variable_changed(descriptor_index, coordinates.source_entity);
    if !coordinates.is_intra_list() {
        score_director.before_variable_changed(descriptor_index, coordinates.destination_entity);
    }

    let value = access
        .list_remove(
            score_director.working_solution_mut(),
            coordinates.source_entity,
            coordinates.source_position,
        )
        .expect("source position should be valid");
    let destination = coordinates.adjusted_destination();
    match transfer {
        ChangeValueTransfer::CloneBeforeInsert => access.list_insert(
            score_director.working_solution_mut(),
            coordinates.destination_entity,
            destination,
            value.clone(),
        ),
        ChangeValueTransfer::MoveIntoInsert => access.list_insert(
            score_director.working_solution_mut(),
            coordinates.destination_entity,
            destination,
            value,
        ),
    }

    score_director.after_variable_changed(descriptor_index, coordinates.source_entity);
    if !coordinates.is_intra_list() {
        score_director.after_variable_changed(descriptor_index, coordinates.destination_entity);
    }
}

pub(crate) fn change_undo_move<S, A, D>(
    access: &A,
    coordinates: ChangeCoordinates,
    score_director: &mut D,
) where
    S: PlanningSolution,
    A: ListChangeAccess<S>,
    D: Director<S>,
{
    let descriptor_index = access.descriptor_index();
    score_director.before_variable_changed(descriptor_index, coordinates.destination_entity);
    if !coordinates.is_intra_list() {
        score_director.before_variable_changed(descriptor_index, coordinates.source_entity);
    }
    let removed = access
        .list_remove(
            score_director.working_solution_mut(),
            coordinates.destination_entity,
            coordinates.adjusted_destination(),
        )
        .expect("undo destination position should contain moved element");
    access.list_insert(
        score_director.working_solution_mut(),
        coordinates.source_entity,
        coordinates.source_position,
        removed,
    );
    score_director.after_variable_changed(descriptor_index, coordinates.destination_entity);
    if !coordinates.is_intra_list() {
        score_director.after_variable_changed(descriptor_index, coordinates.source_entity);
    }
}

pub(crate) fn change_tabu_signature<S, A, D>(
    access: &A,
    coordinates: ChangeCoordinates,
    score_director: &D,
) -> MoveTabuSignature
where
    S: PlanningSolution,
    A: ListMoveAccess<S>,
    D: Director<S>,
{
    let moved_value = access.list_get(
        score_director.working_solution(),
        coordinates.source_entity,
        coordinates.source_position,
    );
    let moved_id = access.tabu_value_id(score_director.working_solution(), moved_value.as_ref());
    let source_entity_id = encode_usize(coordinates.source_entity);
    let destination_entity_id = encode_usize(coordinates.destination_entity);
    let scope = MoveTabuScope::new(access.descriptor_index(), access.variable_name());
    let adjusted_destination = coordinates.adjusted_destination();
    let mut entity_tokens: SmallVec<[ScopedEntityTabuToken; 2]> =
        smallvec![scope.entity_token(source_entity_id)];
    if !coordinates.is_intra_list() {
        entity_tokens.push(scope.entity_token(destination_entity_id));
    }

    MoveTabuSignature::new(
        scope,
        smallvec![
            encode_usize(access.descriptor_index()),
            hash_str(access.variable_name()),
            source_entity_id,
            encode_usize(coordinates.source_position),
            destination_entity_id,
            encode_usize(adjusted_destination),
            moved_id
        ],
        smallvec![
            encode_usize(access.descriptor_index()),
            hash_str(access.variable_name()),
            destination_entity_id,
            encode_usize(adjusted_destination),
            source_entity_id,
            encode_usize(coordinates.source_position),
            moved_id
        ],
    )
    .with_entity_tokens(entity_tokens)
    .with_destination_value_tokens([scope.value_token(moved_id)])
}

pub(crate) fn change_candidate_trace_identity<S, A>(
    access: &A,
    coordinates: ChangeCoordinates,
) -> CandidateTraceIdentity
where
    A: ListMoveAccess<S>,
{
    CandidateTraceIdentity::logical_move(
        access.descriptor_index(),
        access.variable_name(),
        "list_change",
        [
            coordinates.source_entity,
            coordinates.source_position,
            coordinates.destination_entity,
            coordinates.destination_position,
            coordinates.adjusted_destination(),
        ],
    )
}
