//! Shared mechanics for precedence-critical multi-list swaps.

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::metadata::{
    encode_usize, scoped_move_identity, MoveTabuScope, ScopedEntityTabuToken, ScopedValueTabuToken,
    TABU_OP_LIST_MULTI_SWAP,
};
use crate::heuristic::r#move::MoveTabuSignature;

use super::{ListMoveAccess, ListSwapAccess};

pub(crate) type MultiSwapCoordinates = SmallVec<[(usize, usize, usize); 4]>;

/// Returns affected entities in historic first-coordinate order.
pub(crate) fn multi_swap_entity_indices(
    coordinates: &[(usize, usize, usize)],
) -> SmallVec<[usize; 4]> {
    let mut entities = SmallVec::new();
    for &(entity, _, _) in coordinates {
        if !entities.contains(&entity) {
            entities.push(entity);
        }
    }
    entities
}

pub(crate) fn multi_swap_is_doable<S, A, D>(
    access: &A,
    coordinates: &[(usize, usize, usize)],
    score_director: &D,
) -> bool
where
    S: PlanningSolution,
    A: ListSwapAccess<S>,
    D: Director<S>,
{
    if coordinates.is_empty() {
        return false;
    }

    let solution = score_director.working_solution();
    let mut seen_entities = SmallVec::<[usize; 4]>::new();
    for &(entity, first, second) in coordinates {
        if first == second || seen_entities.contains(&entity) {
            return false;
        }
        seen_entities.push(entity);

        let len = access.list_len(solution, entity);
        if first >= len || second >= len {
            return false;
        }

        if access.list_get(solution, entity, first) == access.list_get(solution, entity, second) {
            return false;
        }
    }
    true
}

pub(crate) fn multi_swap_do_move<S, A, D>(
    access: &A,
    coordinates: &[(usize, usize, usize)],
    entity_indices: &[usize],
    score_director: &mut D,
) where
    S: PlanningSolution,
    A: ListSwapAccess<S>,
    D: Director<S>,
{
    let mut values = SmallVec::<[(A::Element, A::Element); 4]>::new();
    for &(entity, first, second) in coordinates {
        let first_value = access
            .list_get(score_director.working_solution(), entity, first)
            .expect("first multi-swap position should be valid");
        let second_value = access
            .list_get(score_director.working_solution(), entity, second)
            .expect("second multi-swap position should be valid");
        values.push((first_value, second_value));
    }

    let descriptor_index = access.descriptor_index();
    for &entity in entity_indices {
        score_director.before_variable_changed(descriptor_index, entity);
    }

    for (&(entity, first, second), (first_value, second_value)) in coordinates.iter().zip(&values) {
        access
            .list_set(
                score_director.working_solution_mut(),
                entity,
                first,
                second_value.clone(),
            )
            .expect("multi-swap requires a validated direct list-set capability");
        access
            .list_set(
                score_director.working_solution_mut(),
                entity,
                second,
                first_value.clone(),
            )
            .expect("multi-swap requires a validated direct list-set capability");
    }

    for &entity in entity_indices {
        score_director.after_variable_changed(descriptor_index, entity);
    }
}

pub(crate) fn multi_swap_undo_move<S, A, D>(
    access: &A,
    coordinates: &[(usize, usize, usize)],
    entity_indices: &[usize],
    score_director: &mut D,
) where
    S: PlanningSolution,
    A: ListSwapAccess<S>,
    D: Director<S>,
{
    multi_swap_do_move(access, coordinates, entity_indices, score_director);
}

pub(crate) fn multi_swap_tabu_signature<S, A, D>(
    access: &A,
    coordinates: &[(usize, usize, usize)],
    entity_indices: &[usize],
    score_director: &D,
) -> MoveTabuSignature
where
    S: PlanningSolution,
    A: ListMoveAccess<S>,
    D: Director<S>,
{
    let scope = MoveTabuScope::new(access.descriptor_index(), access.variable_name());
    let entity_tokens: SmallVec<[ScopedEntityTabuToken; 2]> = entity_indices
        .iter()
        .copied()
        .map(encode_usize)
        .map(|entity| scope.entity_token(entity))
        .collect();
    let mut destination_value_tokens = SmallVec::<[ScopedValueTabuToken; 2]>::new();
    for &(entity, first, second) in coordinates {
        let first_value = access.list_get(score_director.working_solution(), entity, first);
        let second_value = access.list_get(score_director.working_solution(), entity, second);
        destination_value_tokens.push(scope.value_token(
            access.tabu_value_id(score_director.working_solution(), second_value.as_ref()),
        ));
        destination_value_tokens.push(scope.value_token(
            access.tabu_value_id(score_director.working_solution(), first_value.as_ref()),
        ));
    }

    let mut canonical = coordinates
        .iter()
        .map(|&(entity, first, second)| {
            let (left, right) = if first <= second {
                (first, second)
            } else {
                (second, first)
            };
            (entity, left, right)
        })
        .collect::<SmallVec<[(usize, usize, usize); 4]>>();
    canonical.sort_unstable();

    let mut components = SmallVec::<[u64; 8]>::new();
    components.push(encode_usize(canonical.len()));
    for (entity, first, second) in canonical {
        components.push(encode_usize(entity));
        components.push(encode_usize(first));
        components.push(encode_usize(second));
    }
    let move_id = scoped_move_identity(scope, TABU_OP_LIST_MULTI_SWAP, components);
    MoveTabuSignature::new(scope, move_id.clone(), move_id)
        .with_entity_tokens(entity_tokens)
        .with_destination_value_tokens(destination_value_tokens)
}
