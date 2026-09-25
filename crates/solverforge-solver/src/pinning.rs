use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;

#[inline]
pub(crate) fn entity_is_pinned<S: PlanningSolution, D: Director<S>>(
    director: &D,
    descriptor_index: usize,
    entity_index: usize,
) -> bool {
    director
        .solution_descriptor()
        .entity_descriptors
        .get(descriptor_index)
        .is_some_and(|descriptor| descriptor.is_pinned(director.working_solution(), entity_index))
}

#[inline]
pub(crate) fn move_changes_pinned<S: PlanningSolution, D: Director<S>, M: Move<S>>(
    mov: &M,
    director: &D,
) -> bool {
    let mut pinned = false;
    mov.for_each_affected_entity(&mut |entity| {
        if !pinned {
            pinned = entity_is_pinned(director, entity.descriptor_index, entity.entity_index);
        }
    });
    pinned
}
