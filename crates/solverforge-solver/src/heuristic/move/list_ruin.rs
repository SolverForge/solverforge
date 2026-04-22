/* ListRuinMove - ruin-and-recreate move for Large Neighborhood Search on list variables.

Removes selected elements from a list entity, then greedily reinserts each
one into the best available position across all entities. This makes the move
self-contained: it can be accepted by a local search acceptor without leaving
the solution in a degenerate state.

# Zero-Erasure Design

Uses typed function pointers for list operations. No `dyn Any`, no downcasting.
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use smallvec::{smallvec, SmallVec};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

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
    // Entity index to ruin from
    entity_index: usize,
    // Indices of elements to remove (sorted ascending)
    element_indices: SmallVec<[usize; 8]>,
    // Number of entities in solution (for recreate phase)
    entity_count: fn(&S) -> usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    // Remove element at index, returning it
    list_remove: fn(&mut S, usize, usize) -> V,
    // Insert element at index
    list_insert: fn(&mut S, usize, usize, V),
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<fn() -> V>,
}

impl<S, V> Clone for ListRuinMove<S, V> {
    fn clone(&self) -> Self {
        Self {
            entity_index: self.entity_index,
            element_indices: self.element_indices.clone(),
            entity_count: self.entity_count,
            list_len: self.list_len,
            list_get: self.list_get,
            list_remove: self.list_remove,
            list_insert: self.list_insert,
            variable_name: self.variable_name,
            descriptor_index: self.descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V: Debug> Debug for ListRuinMove<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListRuinMove")
            .field("entity", &self.entity_index)
            .field("elements", &self.element_indices.as_slice())
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
        Self {
            entity_index,
            element_indices: indices,
            entity_count,
            list_len,
            list_get,
            list_remove,
            list_insert,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }

    pub fn entity_index(&self) -> usize {
        self.entity_index
    }

    pub fn element_indices(&self) -> &[usize] {
        &self.element_indices
    }

    pub fn ruin_count(&self) -> usize {
        self.element_indices.len()
    }
}

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

impl<S, V> Move<S> for ListRuinMove<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        if self.element_indices.is_empty() {
            return false;
        }
        let solution = score_director.working_solution();
        let len = (self.list_len)(solution, self.entity_index);
        self.element_indices.iter().all(|&idx| idx < len)
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) {
        let list_remove = self.list_remove;
        let list_insert = self.list_insert;
        let list_len = self.list_len;
        let entity_count = self.entity_count;
        let src = self.entity_index;
        let descriptor = self.descriptor_index;

        // --- Ruin phase: remove elements from source entity ---
        score_director.before_variable_changed(descriptor, src);
        let mut removed: SmallVec<[V; 8]> = SmallVec::new();
        for &idx in self.element_indices.iter().rev() {
            let value = list_remove(score_director.working_solution_mut(), src, idx);
            removed.push(value);
        }
        // removed is in reverse removal order; reverse to get original order
        removed.reverse();
        score_director.after_variable_changed(descriptor, src);

        // --- Recreate phase: greedily reinsert each element at best position ---
        // Track where each element ends up for the undo closure.
        let mut placements: SmallVec<[(usize, usize); 8]> = SmallVec::new();

        let n_entities = entity_count(score_director.working_solution());

        for elem in removed.iter() {
            let mut best_score: Option<S::Score> = None;
            let mut best_entity = src;
            let mut best_pos = list_len(score_director.working_solution(), src);

            for e in 0..n_entities {
                let len = list_len(score_director.working_solution(), e);
                for pos in 0..=len {
                    score_director.before_variable_changed(descriptor, e);
                    list_insert(score_director.working_solution_mut(), e, pos, elem.clone());
                    score_director.after_variable_changed(descriptor, e);

                    let candidate_score = score_director.calculate_score();
                    if best_score.is_none_or(|b| candidate_score > b) {
                        best_score = Some(candidate_score);
                        best_entity = e;
                        best_pos = pos;
                    }

                    score_director.before_variable_changed(descriptor, e);
                    list_remove(score_director.working_solution_mut(), e, pos);
                    score_director.after_variable_changed(descriptor, e);
                }
            }

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
            placements.push((best_entity, best_pos));
        }

        /* --- Register undo ---
        placements[i] = (entity, pos) at the moment element i was inserted.
        Later insertions j > i into the same entity at pos <= placements[i].pos
        shifted element i rightward by 1 for each such j.
        During undo we process in reverse: remove last-placed first.
        At that point, only placements[j] with j > i (already removed) have been
        undone, so the current position of element i is:
        placements[i].pos + #{j > i : same entity AND placements[j].pos <= placements[i].pos}
        which we compute on the fly as we iterate in reverse.

        After collecting values, reinsert at original indices (ascending) in source entity.
        Reinserting at orig_indices[k] in order k=0,1,... shifts later indices by 1,
        but orig_indices is sorted ascending so each insertion at idx shifts positions > idx,
        which are exactly the later orig_indices — so we insert at orig_indices[k] + k
        to account for the k prior insertions that each shifted by 1.
        */
        let orig_entity = src;
        let orig_indices: SmallVec<[usize; 8]> = self.element_indices.clone();

        score_director.register_undo(Box::new(move |s: &mut S| {
            let n = placements.len();
            let mut current_pos = final_positions_after_insertions(&placements);

            /* Remove in reverse insertion order (i = n-1 downto 0).
            When removing element i, elements j > i have already been removed.
            Any earlier element in the same entity that currently sits after the
            removed position shifts left by one.
            */
            let mut vals: SmallVec<[V; 8]> = SmallVec::with_capacity(n);
            for i in (0..n).rev() {
                let (e_i, _) = placements[i];
                let actual_pos = current_pos[i];
                vals.push(list_remove(s, e_i, actual_pos));

                for j in 0..i {
                    let (e_j, _) = placements[j];
                    if e_j == e_i && current_pos[j] > actual_pos {
                        current_pos[j] -= 1;
                    }
                }
            }
            // vals is in reverse original order; reverse to get forward original order.
            vals.reverse();

            /* Reinsert at original positions (ascending, sorted).
            orig_indices[k] is the position in the pre-ruin source entity.
            Inserting at orig_indices[k] shifts all positions > orig_indices[k] right.
            Since orig_indices is sorted ascending, each insertion k shifts positions
            that are >= orig_indices[k], which includes orig_indices[k+1..] only if
            they are >= orig_indices[k]. They are (sorted), so each later index needs
            +k adjustment (k prior insertions each shifted it once).
            But orig_indices[k] itself does not shift — we insert at the exact original
            index before any of the k prior insertions were accounted for.
            Actually: after k insertions at positions orig_indices[0..k] (all <= orig_indices[k]
            since sorted), orig_indices[k]'s effective position has shifted by k.
            */
            for (&idx, val) in orig_indices.iter().zip(vals) {
                list_insert(s, orig_entity, idx, val);
            }
        }));
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        std::slice::from_ref(&self.entity_index)
    }

    fn variable_name(&self) -> &str {
        self.variable_name
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        let mut value_ids: SmallVec<[u64; 2]> = SmallVec::new();
        for &idx in &self.element_indices {
            let value = (self.list_get)(score_director.working_solution(), self.entity_index, idx);
            value_ids.push(encode_option_debug(value.as_ref()));
        }
        let entity_id = encode_usize(self.entity_index);
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
            entity_id,
            encode_usize(self.element_indices.len())
        ];
        move_id.extend(self.element_indices.iter().map(|&idx| encode_usize(idx)));
        move_id.extend(value_ids.iter().copied());

        MoveTabuSignature::new(scope, move_id.clone(), move_id)
            .with_entity_tokens([scope.entity_token(entity_id)])
            .with_destination_value_tokens(destination_value_tokens)
    }
}
