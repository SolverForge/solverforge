//! Shared K-opt mutation and exact tabu mechanics.

use smallvec::{smallvec, SmallVec};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::k_opt_reconnection::KOptReconnection;
use crate::heuristic::r#move::metadata::{encode_usize, MoveTabuScope, ScopedValueTabuToken};
use crate::heuristic::r#move::{CutPoint, MoveTabuSignature};

use super::{ListRangeAccess, ListWindowAccess};

pub(crate) fn k_opt_is_doable<S, A, D>(
    access: &A,
    cuts: &[CutPoint],
    reconnection: &KOptReconnection,
    entity: usize,
    score_director: &D,
) -> bool
where
    S: PlanningSolution,
    A: ListWindowAccess<S>,
    D: Director<S>,
{
    let k = cuts.len();
    if k < 2 || reconnection.k() != k {
        return false;
    }

    let len = access.list_len(score_director.working_solution(), entity);
    if cuts.iter().any(|cut| cut.position() > len) {
        return false;
    }

    !cuts.iter().all(|cut| cut.entity_index() == entity)
        || cuts
            .windows(2)
            .all(|pair| pair[1].position() > pair[0].position())
}

pub(crate) fn k_opt_do_move<S, A, D>(
    access: &A,
    cuts: &[CutPoint],
    reconnection: &KOptReconnection,
    entity: usize,
    score_director: &mut D,
) -> Vec<A::Element>
where
    S: PlanningSolution,
    A: ListWindowAccess<S>,
    D: Director<S>,
{
    let descriptor_index = access.descriptor_index();
    score_director.before_variable_changed(descriptor_index, entity);

    let len = access.list_len(score_director.working_solution(), entity);
    let all_elements = access
        .sublist_remove(score_director.working_solution_mut(), entity, 0, len)
        .expect("validated list window access should remove the full K-opt route");

    let mut boundaries = Vec::with_capacity(cuts.len() + 2);
    boundaries.push(0);
    boundaries.extend(cuts.iter().map(CutPoint::position));
    boundaries.push(len);

    let mut segments: Vec<Vec<A::Element>> = Vec::with_capacity(cuts.len() + 1);
    for boundary in boundaries.windows(2) {
        segments.push(all_elements[boundary[0]..boundary[1]].to_vec());
    }

    let mut new_elements = Vec::with_capacity(len);
    for position in 0..reconnection.segment_count() {
        let segment_index = reconnection.segment_at(position);
        let mut segment = std::mem::take(&mut segments[segment_index]);
        if reconnection.should_reverse(segment_index) {
            segment.reverse();
        }
        new_elements.extend(segment);
    }

    access
        .sublist_insert(
            score_director.working_solution_mut(),
            entity,
            0,
            new_elements,
        )
        .expect("validated list window access should insert the reconnected K-opt route");
    score_director.after_variable_changed(descriptor_index, entity);

    all_elements
}

pub(crate) fn k_opt_undo_move<S, A, D>(
    access: &A,
    entity: usize,
    undo: Vec<A::Element>,
    score_director: &mut D,
) where
    S: PlanningSolution,
    A: ListWindowAccess<S>,
    D: Director<S>,
{
    let descriptor_index = access.descriptor_index();
    score_director.before_variable_changed(descriptor_index, entity);
    let len = access.list_len(score_director.working_solution(), entity);
    let _ = access
        .sublist_remove(score_director.working_solution_mut(), entity, 0, len)
        .expect("validated list window access should remove the reconnected K-opt route");
    access
        .sublist_insert(score_director.working_solution_mut(), entity, 0, undo)
        .expect("validated list window access should restore the original K-opt route");
    score_director.after_variable_changed(descriptor_index, entity);
}

pub(crate) fn k_opt_tabu_signature<S, A, D>(
    access: &A,
    cuts: &[CutPoint],
    reconnection: &KOptReconnection,
    variable_id: u64,
    entity: usize,
    score_director: &D,
) -> MoveTabuSignature
where
    S: PlanningSolution,
    A: ListRangeAccess<S>,
    D: Director<S>,
{
    let mut touched_value_ids: SmallVec<[u64; 2]> = SmallVec::new();
    let len = access.list_len(score_director.working_solution(), entity);
    let first_position = cuts[0].position();
    let last_position = cuts[cuts.len() - 1].position();
    for position in first_position..len.min(last_position.max(first_position)) {
        let value = access.list_get(score_director.working_solution(), entity, position);
        touched_value_ids
            .push(access.tabu_value_id(score_director.working_solution(), value.as_ref()));
    }

    let entity_id = encode_usize(entity);
    let scope = MoveTabuScope::new(access.descriptor_index(), access.variable_name());
    let destination_value_tokens: SmallVec<[ScopedValueTabuToken; 2]> = touched_value_ids
        .iter()
        .copied()
        .map(|value_id| scope.value_token(value_id))
        .collect();
    let mut move_id = smallvec![
        encode_usize(access.descriptor_index()),
        variable_id,
        entity_id,
        encode_usize(cuts.len())
    ];
    move_id.extend(cuts.iter().map(|cut| encode_usize(cut.position())));
    move_id.extend(reconnection.segment_order().iter().copied().map(u64::from));
    move_id.extend((0..reconnection.segment_count()).map(|index| {
        if reconnection.should_reverse(index) {
            1
        } else {
            0
        }
    }));
    move_id.extend(touched_value_ids.iter().copied());

    let inverse_order = reconnection.inverse_segment_order();
    let mut undo_move_id = smallvec![
        encode_usize(access.descriptor_index()),
        variable_id,
        entity_id,
        encode_usize(cuts.len())
    ];
    undo_move_id.extend(cuts.iter().map(|cut| encode_usize(cut.position())));
    undo_move_id.extend(
        inverse_order[..reconnection.segment_count()]
            .iter()
            .copied()
            .map(u64::from),
    );
    undo_move_id.extend((0..reconnection.segment_count()).map(|index| {
        if reconnection.should_reverse(index) {
            1
        } else {
            0
        }
    }));
    undo_move_id.extend(touched_value_ids.iter().copied());

    MoveTabuSignature::new(scope, move_id, undo_move_id)
        .with_entity_tokens([scope.entity_token(entity_id)])
        .with_destination_value_tokens(destination_value_tokens)
}
