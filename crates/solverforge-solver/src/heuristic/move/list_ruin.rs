/* ListRuinMove - ruin-and-recreate move for Large Neighborhood Search on list variables.

Removes selected elements from a list entity, then greedily reinserts each
one into the best available position across all entities. This makes the move
self-contained: it can be accepted by a local search acceptor without leaving
the solution in a degenerate state.

# Zero-Erasure Design

Uses concrete function pointers for list operations. No `dyn Any`, no downcasting.
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use smallvec::{smallvec, SmallVec};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::selector::precedence_route::{
    node_index, PrecedenceRouteGraph, PrecedenceRouteHooks,
};

use super::metadata::{
    encode_option_debug, encode_usize, hash_str, MoveTabuScope, ScopedValueTabuToken,
};
use super::{Move, MoveTabuSignature};

/// A ruin-and-recreate move for Large Neighborhood Search on list variables.
///
/// Removes selected elements from a source entity, then reinserts each one
/// greedily into the best position across all entities (including the source).
/// The move is self-contained: accepting it leaves the solution valid.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The list element value type
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::r#move::ListRuinMove;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SoftScore;
///
/// #[derive(Clone, Debug)]
/// struct Route { stops: Vec<i32>, score: Option<SoftScore> }
///
/// impl PlanningSolution for Route {
///     type Score = SoftScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// fn entity_count(s: &Route) -> usize { 1 }
/// fn list_len(s: &Route, _: usize) -> usize { s.stops.len() }
/// fn list_get(s: &Route, _: usize, pos: usize) -> Option<i32> { s.stops.get(pos).copied() }
/// fn list_remove(s: &mut Route, _: usize, idx: usize) -> i32 { s.stops.remove(idx) }
/// fn list_insert(s: &mut Route, _: usize, idx: usize, v: i32) { s.stops.insert(idx, v); }
///
/// // Ruin elements at indices 1 and 3, then recreate greedily
/// let m = ListRuinMove::<Route, i32>::new(
///     0,
///     &[1, 3],
///     entity_count,
///     list_len, list_get, list_remove, list_insert,
///     "stops", 0,
/// );
/// ```
pub struct ListRuinMove<S, V> {
    entity_index: usize,
    element_indices: SmallVec<[usize; 8]>,
    sources: SmallVec<[(usize, SmallVec<[usize; 8]>); 4]>,
    entity_indices: SmallVec<[usize; 8]>,
    // Number of entities in solution (for recreate phase)
    entity_count: fn(&S) -> usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    // Remove element at index, returning it
    list_remove: fn(&mut S, usize, usize) -> V,
    // Insert element at index
    list_insert: fn(&mut S, usize, usize, V),
    element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    precedence_element_count: Option<fn(&S) -> usize>,
    precedence_index_to_element: Option<fn(&S, usize) -> V>,
    precedence_successors_fn: Option<fn(&S, V, &mut Vec<V>)>,
    skip_empty_destinations: bool,
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<fn() -> V>,
}

impl<S, V> Clone for ListRuinMove<S, V> {
    fn clone(&self) -> Self {
        Self {
            entity_index: self.entity_index,
            element_indices: self.element_indices.clone(),
            sources: self.sources.clone(),
            entity_indices: self.entity_indices.clone(),
            entity_count: self.entity_count,
            list_len: self.list_len,
            list_get: self.list_get,
            list_remove: self.list_remove,
            list_insert: self.list_insert,
            element_owner_fn: self.element_owner_fn,
            precedence_element_count: self.precedence_element_count,
            precedence_index_to_element: self.precedence_index_to_element,
            precedence_successors_fn: self.precedence_successors_fn,
            skip_empty_destinations: self.skip_empty_destinations,
            variable_name: self.variable_name,
            descriptor_index: self.descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V: Debug> Debug for ListRuinMove<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListRuinMove")
            .field("sources", &self.sources.as_slice())
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, V> ListRuinMove<S, V> {
    /* Creates a new list ruin-and-recreate move.

    # Arguments
    * `entity_index` - Entity index to ruin from
    * `element_indices` - Indices of elements to remove
    * `entity_count` - Function returning total entity count
    * `list_len` - Function to get list length for an entity
    * `list_remove` - Function to remove element at index
    * `list_insert` - Function to insert element at index
    * `variable_name` - Name of the list variable
    * `descriptor_index` - Entity descriptor index
    */
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_index: usize,
        element_indices: &[usize],
        entity_count: fn(&S) -> usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_remove: fn(&mut S, usize, usize) -> V,
        list_insert: fn(&mut S, usize, usize, V),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        let mut indices: SmallVec<[usize; 8]> = SmallVec::from_slice(element_indices);
        indices.sort_unstable();
        let sources = smallvec![(entity_index, indices.clone())];
        let entity_indices = smallvec![entity_index];
        Self {
            entity_index,
            element_indices: indices,
            sources,
            entity_indices,
            entity_count,
            list_len,
            list_get,
            list_remove,
            list_insert,
            element_owner_fn: None,
            precedence_element_count: None,
            precedence_index_to_element: None,
            precedence_successors_fn: None,
            skip_empty_destinations: false,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_multi_source(
        sources: &[(usize, SmallVec<[usize; 8]>)],
        entity_count: fn(&S) -> usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_remove: fn(&mut S, usize, usize) -> V,
        list_insert: fn(&mut S, usize, usize, V),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        let mut merged: SmallVec<[(usize, SmallVec<[usize; 8]>); 4]> = SmallVec::new();
        for (entity_index, indices) in sources {
            let mut sorted = indices.clone();
            sorted.sort_unstable();
            sorted.dedup();
            if sorted.is_empty() {
                continue;
            }
            if let Some((_, existing)) = merged
                .iter_mut()
                .find(|(candidate, _)| candidate == entity_index)
            {
                existing.extend(sorted);
                existing.sort_unstable();
                existing.dedup();
            } else {
                merged.push((*entity_index, sorted));
            }
        }
        merged.sort_by_key(|(entity_index, _)| *entity_index);
        let (entity_index, element_indices) = merged
            .first()
            .cloned()
            .unwrap_or_else(|| (0, SmallVec::new()));
        let entity_indices = merged
            .iter()
            .map(|(entity_index, _)| *entity_index)
            .collect();
        Self {
            entity_index,
            element_indices,
            sources: merged,
            entity_indices,
            entity_count,
            list_len,
            list_get,
            list_remove,
            list_insert,
            element_owner_fn: None,
            precedence_element_count: None,
            precedence_index_to_element: None,
            precedence_successors_fn: None,
            skip_empty_destinations: false,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }

    pub fn with_element_owner_fn(
        mut self,
        element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    ) -> Self {
        self.element_owner_fn = element_owner_fn;
        self
    }

    pub(crate) fn with_precedence_hooks(
        mut self,
        element_count: Option<fn(&S) -> usize>,
        index_to_element: Option<fn(&S, usize) -> V>,
        successors_fn: Option<fn(&S, V, &mut Vec<V>)>,
    ) -> Self {
        self.precedence_element_count = element_count;
        self.precedence_index_to_element = index_to_element;
        self.precedence_successors_fn = successors_fn;
        self
    }

    pub fn with_skip_empty_destinations(mut self, skip_empty_destinations: bool) -> Self {
        self.skip_empty_destinations = skip_empty_destinations;
        self
    }

    pub fn entity_index(&self) -> usize {
        self.entity_index
    }

    pub fn element_indices(&self) -> &[usize] {
        &self.element_indices
    }

    pub fn ruin_count(&self) -> usize {
        self.sources.iter().map(|(_, indices)| indices.len()).sum()
    }
}

#[cfg(test)]
pub(crate) fn final_positions_after_insertions(
    placements: &SmallVec<[(usize, usize); 8]>,
) -> SmallVec<[usize; 8]> {
    let mut current_positions: SmallVec<[usize; 8]> = SmallVec::with_capacity(placements.len());

    for i in 0..placements.len() {
        let (entity_i, insert_pos_i) = placements[i];

        for j in 0..i {
            let (entity_j, _) = placements[j];
            if entity_j == entity_i && current_positions[j] >= insert_pos_i {
                current_positions[j] += 1;
            }
        }

        current_positions.push(insert_pos_i);
    }

    current_positions
}

fn final_positions_after_ordered_insertions(
    placements: &SmallVec<[(usize, usize, usize); 8]>,
) -> SmallVec<[usize; 8]> {
    let mut current_positions: SmallVec<[usize; 8]> = SmallVec::with_capacity(placements.len());

    for i in 0..placements.len() {
        let (entity_i, insert_pos_i, _) = placements[i];

        for j in 0..i {
            let (entity_j, _, _) = placements[j];
            if entity_j == entity_i && current_positions[j] >= insert_pos_i {
                current_positions[j] += 1;
            }
        }

        current_positions.push(insert_pos_i);
    }

    current_positions
}

impl<S, V> Move<S> for ListRuinMove<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    type Undo = SmallVec<[(usize, usize, usize); 8]>;

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        if self.sources.is_empty() || self.sources.iter().all(|(_, indices)| indices.is_empty()) {
            return false;
        }
        let solution = score_director.working_solution();

        let Some(owner_fn) = self.element_owner_fn else {
            return self.sources.iter().all(|(entity_index, indices)| {
                let len = (self.list_len)(solution, *entity_index);
                indices.iter().all(|&idx| idx < len)
            });
        };

        let n_entities = (self.entity_count)(solution);
        self.sources.iter().all(|(entity_index, indices)| {
            let len = (self.list_len)(solution, *entity_index);
            indices.iter().all(|&idx| {
                if idx >= len {
                    return false;
                }
                let Some(element) = (self.list_get)(solution, *entity_index, idx) else {
                    return false;
                };
                crate::list_placement::candidate_entity_indices(
                    Some(owner_fn),
                    solution,
                    n_entities,
                    &element,
                )
                .next()
                .is_some()
            })
        })
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        let list_remove = self.list_remove;
        let list_insert = self.list_insert;
        let list_len = self.list_len;
        let list_get = self.list_get;
        let entity_count = self.entity_count;
        let element_owner_fn = self.element_owner_fn;
        let skip_empty_destinations = self.skip_empty_destinations;
        let descriptor = self.descriptor_index;

        // --- Ruin phase: remove selected elements from each source entity ---
        let mut removed: SmallVec<[(usize, usize, V); 8]> = SmallVec::new();
        for (src, indices) in &self.sources {
            score_director.before_variable_changed(descriptor, *src);
            let mut source_removed: SmallVec<[(usize, V); 8]> = SmallVec::new();
            for &idx in indices.iter().rev() {
                let value = list_remove(score_director.working_solution_mut(), *src, idx);
                source_removed.push((idx, value));
            }
            source_removed.reverse();
            removed.extend(
                source_removed
                    .into_iter()
                    .map(|(idx, value)| (*src, idx, value)),
            );
            score_director.after_variable_changed(descriptor, *src);
        }

        // --- Recreate phase: greedily reinsert the best remaining element and position ---
        // Track where each element ends up for the undo closure.
        let mut placements: SmallVec<[(usize, usize, usize); 8]> = SmallVec::new();
        let mut remaining: SmallVec<[(usize, usize, usize, V); 8]> = removed
            .iter()
            .cloned()
            .enumerate()
            .map(|(idx, (src, original_pos, value))| (idx, src, original_pos, value))
            .collect();

        let n_entities = entity_count(score_director.working_solution());
        while !remaining.is_empty() {
            let precedence_graph =
                self.recreate_precedence_graph(score_director.working_solution());
            let mut best_choice: Option<(usize, usize, usize, S::Score)> = None;

            for (remaining_idx, (_, _, _, elem)) in remaining.iter().enumerate() {
                let candidates = crate::list_placement::candidate_entity_indices(
                    element_owner_fn,
                    score_director.working_solution(),
                    n_entities,
                    elem,
                );
                for e in candidates {
                    let len = list_len(score_director.working_solution(), e);
                    if skip_empty_destinations && element_owner_fn.is_none() && len == 0 {
                        continue;
                    }
                    for pos in 0..=len {
                        if precedence_graph.as_ref().is_some_and(|(elements, graph)| {
                            let Some(element_node) = node_index(elements, elem) else {
                                return false;
                            };
                            let prev = (pos > 0)
                                .then(|| list_get(score_director.working_solution(), e, pos - 1))
                                .flatten();
                            let next = (pos < len)
                                .then(|| list_get(score_director.working_solution(), e, pos))
                                .flatten();
                            graph.insertion_introduces_cycle(
                                prev.as_ref().and_then(|value| node_index(elements, value)),
                                element_node,
                                next.as_ref().and_then(|value| node_index(elements, value)),
                            )
                        }) {
                            continue;
                        }

                        score_director.before_variable_changed(descriptor, e);
                        list_insert(score_director.working_solution_mut(), e, pos, elem.clone());
                        score_director.after_variable_changed(descriptor, e);

                        let candidate_score = score_director.calculate_score();
                        if best_choice
                            .is_none_or(|(_, _, _, best_score)| candidate_score > best_score)
                        {
                            best_choice = Some((remaining_idx, e, pos, candidate_score));
                        }

                        score_director.before_variable_changed(descriptor, e);
                        list_remove(score_director.working_solution_mut(), e, pos);
                        score_director.after_variable_changed(descriptor, e);
                    }
                }
            }

            let Some((remaining_idx, best_entity, best_pos, _)) = best_choice else {
                self.restore_removed_elements(score_director, &placements, &removed);
                return SmallVec::new();
            };
            let (original_removed_idx, _, _, elem) = remaining.remove(remaining_idx);

            // Apply the best insertion permanently
            score_director.before_variable_changed(descriptor, best_entity);
            list_insert(
                score_director.working_solution_mut(),
                best_entity,
                best_pos,
                elem.clone(),
            );
            score_director.after_variable_changed(descriptor, best_entity);

            // Store the placement as recorded at insertion time (no adjustment needed;
            // undo will compute actual current positions accounting for later insertions).
            placements.push((best_entity, best_pos, original_removed_idx));
        }

        /* --- Register undo ---
        placements[i] = (entity, pos, original_removed_idx) at the moment an element was inserted.
        Later insertions j > i into the same entity at pos <= placements[i].pos
        shifted element i rightward by 1 for each such j.
        During undo we process in reverse: remove last-placed first.
        At that point, only placements[j] with j > i (already removed) have been
        undone, so the current position of element i is:
        placements[i].pos + #{j > i : same entity AND placements[j].pos <= placements[i].pos}
        which we compute on the fly as we iterate in reverse.

        After collecting values, reinsert them by original source entity and
        original index in ascending order.
        */
        placements
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, placements: Self::Undo) {
        let n = placements.len();
        let mut current_pos = final_positions_after_ordered_insertions(&placements);
        let mut vals: SmallVec<[(usize, usize, V); 8]> = SmallVec::with_capacity(n);

        for i in (0..n).rev() {
            let (entity_index, _, original_removed_idx) = placements[i];
            let actual_pos = current_pos[i];
            score_director.before_variable_changed(self.descriptor_index, entity_index);
            let val = (self.list_remove)(
                score_director.working_solution_mut(),
                entity_index,
                actual_pos,
            );
            let (source_entity, original_pos) =
                removed_source_entry(&self.sources, original_removed_idx)
                    .expect("list ruin undo placement index must map to an original source entry");
            vals.push((source_entity, original_pos, val));
            score_director.after_variable_changed(self.descriptor_index, entity_index);

            for j in 0..i {
                let (other_entity, _, _) = placements[j];
                if other_entity == entity_index && current_pos[j] > actual_pos {
                    current_pos[j] -= 1;
                }
            }
        }
        vals.sort_by_key(|(entity_index, original_pos, _)| (*entity_index, *original_pos));

        let mut current_entity = None;
        for (entity_index, original_pos, val) in vals {
            if current_entity != Some(entity_index) {
                if let Some(previous_entity) = current_entity {
                    score_director.after_variable_changed(self.descriptor_index, previous_entity);
                }
                score_director.before_variable_changed(self.descriptor_index, entity_index);
                current_entity = Some(entity_index);
            }
            (self.list_insert)(
                score_director.working_solution_mut(),
                entity_index,
                original_pos,
                val,
            );
        }
        if let Some(entity_index) = current_entity {
            score_director.after_variable_changed(self.descriptor_index, entity_index);
        }
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        &self.entity_indices
    }

    fn variable_name(&self) -> &str {
        self.variable_name
    }

    fn telemetry_label(&self) -> &'static str {
        "list_ruin"
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        let mut value_ids: SmallVec<[u64; 2]> = SmallVec::new();
        for (entity_index, indices) in &self.sources {
            for &idx in indices {
                let value = (self.list_get)(score_director.working_solution(), *entity_index, idx);
                value_ids.push(encode_option_debug(value.as_ref()));
            }
        }
        let variable_id = hash_str(self.variable_name);
        let scope = MoveTabuScope::new(self.descriptor_index, self.variable_name);
        let destination_value_tokens: SmallVec<[ScopedValueTabuToken; 2]> = value_ids
            .iter()
            .copied()
            .map(|value_id| scope.value_token(value_id))
            .collect();
        let mut move_id = smallvec![
            encode_usize(self.descriptor_index),
            variable_id,
            encode_usize(self.sources.len()),
            encode_usize(self.ruin_count())
        ];
        for (entity_index, indices) in &self.sources {
            move_id.push(encode_usize(*entity_index));
            move_id.push(encode_usize(indices.len()));
            move_id.extend(indices.iter().map(|&idx| encode_usize(idx)));
        }
        move_id.extend(value_ids.iter().copied());

        MoveTabuSignature::new(scope, move_id.clone(), move_id)
            .with_entity_tokens(
                self.entity_indices
                    .iter()
                    .copied()
                    .map(encode_usize)
                    .map(|entity_id| scope.entity_token(entity_id)),
            )
            .with_destination_value_tokens(destination_value_tokens)
    }
}

fn removed_source_entry(
    sources: &SmallVec<[(usize, SmallVec<[usize; 8]>); 4]>,
    target_index: usize,
) -> Option<(usize, usize)> {
    let mut offset = 0usize;
    for (entity_index, indices) in sources {
        if target_index < offset + indices.len() {
            return Some((*entity_index, indices[target_index - offset]));
        }
        offset += indices.len();
    }
    None
}

impl<S, V> ListRuinMove<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq,
{
    fn restore_removed_elements<D: Director<S>>(
        &self,
        score_director: &mut D,
        placements: &SmallVec<[(usize, usize, usize); 8]>,
        removed: &SmallVec<[(usize, usize, V); 8]>,
    ) {
        let mut current_pos = final_positions_after_ordered_insertions(placements);
        for i in (0..placements.len()).rev() {
            let (entity_index, _, _) = placements[i];
            let actual_pos = current_pos[i];
            score_director.before_variable_changed(self.descriptor_index, entity_index);
            (self.list_remove)(
                score_director.working_solution_mut(),
                entity_index,
                actual_pos,
            );
            score_director.after_variable_changed(self.descriptor_index, entity_index);

            for j in 0..i {
                let (other_entity, _, _) = placements[j];
                if other_entity == entity_index && current_pos[j] > actual_pos {
                    current_pos[j] -= 1;
                }
            }
        }

        let mut removed = removed.clone();
        removed.sort_by_key(|(entity_index, original_pos, _)| (*entity_index, *original_pos));
        let mut current_entity = None;
        for (entity_index, original_pos, value) in removed {
            if current_entity != Some(entity_index) {
                if let Some(previous_entity) = current_entity {
                    score_director.after_variable_changed(self.descriptor_index, previous_entity);
                }
                score_director.before_variable_changed(self.descriptor_index, entity_index);
                current_entity = Some(entity_index);
            }
            (self.list_insert)(
                score_director.working_solution_mut(),
                entity_index,
                original_pos,
                value,
            );
        }
        if let Some(entity_index) = current_entity {
            score_director.after_variable_changed(self.descriptor_index, entity_index);
        }
    }

    fn recreate_precedence_graph(&self, solution: &S) -> Option<(Vec<V>, PrecedenceRouteGraph)> {
        let element_count = self.precedence_element_count?;
        let index_to_element = self.precedence_index_to_element?;
        let fixed_successors = self.precedence_successors_fn?;
        let elements = (0..element_count(solution))
            .map(|index| index_to_element(solution, index))
            .collect::<Vec<_>>();
        let hooks = PrecedenceRouteHooks::new(
            element_count,
            index_to_element,
            fixed_successors,
            self.entity_count,
            self.list_len,
            self.list_get,
        );
        let graph = hooks.build_graph_with_elements(solution, &elements);
        Some((elements, graph))
    }
}
