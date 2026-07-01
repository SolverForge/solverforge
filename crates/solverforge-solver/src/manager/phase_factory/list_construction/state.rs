use std::collections::{HashMap, HashSet};

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
    pub(super) element_owner_fn: Option<fn(&S, &E) -> Option<usize>>,
    pub(super) element_order_key: Option<fn(&S, E) -> i64>,
    pub(super) precedence_duration_fn: Option<fn(&S, E) -> usize>,
    pub(super) precedence_successors_fn: Option<fn(&S, E, &mut Vec<E>)>,
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

        let solution = score_director.working_solution();
        let candidates = crate::list_placement::candidate_entity_indices(
            self.element_owner_fn,
            solution,
            n_entities,
            &element,
        );
        for entity_idx in candidates {
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

    pub(super) fn precedence_downstream_by_element(
        &self,
        solution: &S,
        unassigned: &[E],
    ) -> Option<HashMap<E, usize>> {
        let duration_fn = self.precedence_duration_fn?;
        let successors_fn = self.precedence_successors_fn?;
        if unassigned.is_empty() {
            return Some(HashMap::new());
        }

        let mut index_by_element = HashMap::with_capacity(unassigned.len());
        for (idx, &element) in unassigned.iter().enumerate() {
            if index_by_element.insert(element, idx).is_some() {
                return None;
            }
        }

        let durations = unassigned
            .iter()
            .map(|&element| duration_fn(solution, element))
            .collect::<Vec<_>>();
        let mut successors = vec![Vec::new(); unassigned.len()];
        let mut predecessor_counts = vec![0usize; unassigned.len()];
        let mut scratch = Vec::new();
        for (from_idx, &element) in unassigned.iter().enumerate() {
            scratch.clear();
            successors_fn(solution, element, &mut scratch);
            for successor in &scratch {
                if let Some(&to_idx) = index_by_element.get(successor) {
                    successors[from_idx].push(to_idx);
                    predecessor_counts[to_idx] += 1;
                }
            }
        }

        let topo = topological_order(&successors, &predecessor_counts)?;
        let downstream = downstream_durations(&successors, &durations, &topo);
        Some(
            unassigned
                .iter()
                .copied()
                .zip(downstream)
                .collect::<HashMap<_, _>>(),
        )
    }

    pub(super) fn unassigned_elements(
        &self,
        solution: &S,
        n_elements: usize,
        assigned_set: &HashSet<E>,
    ) -> Vec<E> {
        let mut elements: Vec<(usize, E)> = (0..n_elements)
            .map(|idx| (idx, (self.index_to_element)(solution, idx)))
            .filter(|(_, element)| !assigned_set.contains(element))
            .collect();
        if let Some(order_key) = self.element_order_key {
            elements.sort_by_key(|(idx, element)| (order_key(solution, *element), *idx));
        }
        elements.into_iter().map(|(_, element)| element).collect()
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

pub(super) fn topological_order(
    successors: &[Vec<usize>],
    predecessor_counts: &[usize],
) -> Option<Vec<usize>> {
    let mut predecessor_counts = predecessor_counts.to_vec();
    let mut ready = predecessor_counts
        .iter()
        .enumerate()
        .filter_map(|(idx, &count)| (count == 0).then_some(idx))
        .collect::<Vec<_>>();
    let mut order = Vec::with_capacity(successors.len());

    while let Some(idx) = ready.pop() {
        order.push(idx);
        for &successor in &successors[idx] {
            predecessor_counts[successor] -= 1;
            if predecessor_counts[successor] == 0 {
                ready.push(successor);
            }
        }
    }

    (order.len() == successors.len()).then_some(order)
}

pub(super) fn downstream_durations(
    successors: &[Vec<usize>],
    durations: &[usize],
    topological_order: &[usize],
) -> Vec<usize> {
    let mut downstream = durations.to_vec();
    for &idx in topological_order.iter().rev() {
        let successor_tail = successors[idx]
            .iter()
            .map(|&successor| downstream[successor])
            .max()
            .unwrap_or(0);
        downstream[idx] = durations[idx].saturating_add(successor_tail);
    }
    downstream
}
