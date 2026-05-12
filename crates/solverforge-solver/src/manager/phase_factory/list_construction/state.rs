use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

/* Common fields for scored list construction phases.

Holds all function pointers needed to enumerate elements, enumerate entities,
try insertions, and apply the chosen insertion.
*/
pub(super) struct ScoredConstructionState<S, E> {
    pub(super) element_count: fn(&S) -> usize,
    pub(super) get_assigned: fn(&S) -> Vec<E>,
    pub(super) entity_count: fn(&S) -> usize,
    pub(super) list_len: fn(&S, usize) -> usize,
    pub(super) list_insert: fn(&mut S, usize, usize, E),
    pub(super) list_remove: fn(&mut S, usize, usize) -> E,
    pub(super) index_to_element: fn(&S, usize) -> E,
    pub(super) descriptor_index: usize,
}

impl<S, E> ScoredConstructionState<S, E>
where
    S: PlanningSolution,
    E: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + 'static,
{
    /* Evaluate the score delta of inserting `element` at `(entity_idx, pos)`.

    Performs: before_changed -> insert -> score -> remove -> after_changed (undo).
    */
    pub(super) fn eval_insertion<D: Director<S>>(
        &self,
        element: E,
        entity_idx: usize,
        pos: usize,
        score_director: &mut D,
    ) -> Option<S::Score> {
        let descriptor_index = self.descriptor_index;

        let score_state = score_director.snapshot_score_state();
        score_director.before_variable_changed(descriptor_index, entity_idx);
        (self.list_insert)(
            score_director.working_solution_mut(),
            entity_idx,
            pos,
            element,
        );
        score_director.after_variable_changed(descriptor_index, entity_idx);
        let score = score_director.calculate_score();
        score_director.before_variable_changed(descriptor_index, entity_idx);
        (self.list_remove)(score_director.working_solution_mut(), entity_idx, pos);
        score_director.after_variable_changed(descriptor_index, entity_idx);
        score_director.restore_score_state(score_state);

        Some(score)
    }

    /* Find the best (entity_idx, pos, score) for inserting `element`.

    Returns `None` if there are no valid insertion points.
    */
    pub(super) fn best_insertion<D: Director<S>>(
        &self,
        element: E,
        n_entities: usize,
        score_director: &mut D,
    ) -> Option<(usize, usize, S::Score)> {
        let list_len = self.list_len;
        let mut best: Option<(usize, usize, S::Score)> = None;

        for entity_idx in 0..n_entities {
            let len = list_len(score_director.working_solution(), entity_idx);
            for pos in 0..=len {
                if let Some(score) = self.eval_insertion(element, entity_idx, pos, score_director) {
                    let is_better = match &best {
                        None => true,
                        Some((_, _, best_score)) => score > *best_score,
                    };
                    if is_better {
                        best = Some((entity_idx, pos, score));
                    }
                }
            }
        }

        best
    }

    /* Apply the best insertion for `element` permanently. */
    pub(super) fn apply_insertion<D: Director<S>>(
        &self,
        element: E,
        entity_idx: usize,
        pos: usize,
        score_director: &mut D,
    ) {
        score_director.before_variable_changed(self.descriptor_index, entity_idx);
        (self.list_insert)(
            score_director.working_solution_mut(),
            entity_idx,
            pos,
            element,
        );
        score_director.after_variable_changed(self.descriptor_index, entity_idx);
    }
}
