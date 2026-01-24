//! ListRuinMove - removes elements from a list variable for LNS.
//!
//! This move removes selected elements from a list, allowing a construction
//! heuristic to reinsert them in potentially better positions.
//!
//! # Zero-Erasure Design
//!
//! Uses typed function pointers for list operations. No `dyn Any`, no downcasting.

use std::fmt::Debug;
use std::marker::PhantomData;

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::traits::Move;

/// A move that removes elements from a list for Large Neighborhood Search.
///
/// Elements are removed by index and stored for reinsertion by a construction
/// heuristic. This is the list-variable equivalent of `RuinMove`.
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
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone, Debug)]
/// struct Route { stops: Vec<i32>, score: Option<SimpleScore> }
///
/// impl PlanningSolution for Route {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// fn list_len(s: &Route, _: usize) -> usize { s.stops.len() }
/// fn list_remove(s: &mut Route, _: usize, idx: usize) -> i32 { s.stops.remove(idx) }
/// fn list_insert(s: &mut Route, _: usize, idx: usize, v: i32) { s.stops.insert(idx, v); }
/// fn list_get_element_idx(s: &Route, _: usize, pos: usize) -> usize {
///     s.stops.get(pos).copied().unwrap_or(0) as usize
/// }
///
/// // Remove elements at positions 1 and 3 from the route
/// let m = ListRuinMove::<Route, i32>::new(
///     0,
///     &[1, 3],    // positions
///     list_len, list_remove, list_insert, list_get_element_idx,
///     "stops", 0,
/// );
/// ```
pub struct ListRuinMove<S, V> {
    /// Entity index
    entity_index: usize,
    /// Positions of elements to remove (in ascending order for correct removal)
    positions: SmallVec<[usize; 8]>,
    /// Get list length
    list_len: fn(&S, usize) -> usize,
    /// Remove element at index, returning it
    list_remove: fn(&mut S, usize, usize) -> V,
    /// Insert element at index
    list_insert: fn(&mut S, usize, usize, V),
    /// Get element index at position (for O(1) shadow updates)
    list_get_element_idx: fn(&S, usize, usize) -> usize,
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<V>,
}

impl<S, V> Clone for ListRuinMove<S, V> {
    fn clone(&self) -> Self {
        Self {
            entity_index: self.entity_index,
            positions: self.positions.clone(),
            list_len: self.list_len,
            list_remove: self.list_remove,
            list_insert: self.list_insert,
            list_get_element_idx: self.list_get_element_idx,
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
            .field("positions", &self.positions.as_slice())
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, V> ListRuinMove<S, V> {
    /// Creates a new list ruin move with typed function pointers.
    ///
    /// # Arguments
    /// * `entity_index` - Entity index
    /// * `positions` - Positions of elements to remove
    /// * `list_len` - Function to get list length
    /// * `list_remove` - Function to remove element at index
    /// * `list_insert` - Function to insert element at index
    /// * `list_get_element_idx` - Function to get element index at position
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    ///
    /// # Note
    /// Positions are sorted internally for correct removal order.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_index: usize,
        positions: &[usize],
        list_len: fn(&S, usize) -> usize,
        list_remove: fn(&mut S, usize, usize) -> V,
        list_insert: fn(&mut S, usize, usize, V),
        list_get_element_idx: fn(&S, usize, usize) -> usize,
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        // Sort positions for correct removal order
        let mut sorted_positions: SmallVec<[usize; 8]> = positions.iter().copied().collect();
        sorted_positions.sort_unstable();

        Self {
            entity_index,
            positions: sorted_positions,
            list_len,
            list_remove,
            list_insert,
            list_get_element_idx,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }

    /// Returns the entity index.
    pub fn entity_index(&self) -> usize {
        self.entity_index
    }

    /// Returns the positions of elements being removed.
    pub fn positions(&self) -> &[usize] {
        &self.positions
    }

    /// Returns the number of elements being removed.
    pub fn ruin_count(&self) -> usize {
        self.positions.len()
    }
}

impl<S, V> Move<S> for ListRuinMove<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    fn is_doable<C>(&self, score_director: &ScoreDirector<S, C>) -> bool
    where
        C: solverforge_scoring::ConstraintSet<S, S::Score>,
    {
        if self.positions.is_empty() {
            return false;
        }

        let solution = score_director.working_solution();
        let len = (self.list_len)(solution, self.entity_index);

        // All positions must be within bounds
        self.positions.iter().all(|&pos| pos < len)
    }

    fn do_move<C>(&self, score_director: &mut ScoreDirector<S, C>)
    where
        C: solverforge_scoring::ConstraintSet<S, S::Score>,
    {
        let list_remove = self.list_remove;
        let list_insert = self.list_insert;
        let entity = self.entity_index;

        // Look up element indices at runtime for correct shadow updates
        let element_ids: SmallVec<[usize; 8]> = self
            .positions
            .iter()
            .map(|&pos| (self.list_get_element_idx)(score_director.working_solution(), entity, pos))
            .collect();

        // Notify before removal for each element
        for (&pos, &element_id) in self.positions.iter().zip(element_ids.iter()) {
            score_director.before_list_element_changed(entity, pos, element_id);
        }

        // Remove elements in reverse order (highest position first) to preserve positions
        let mut removed: SmallVec<[(usize, usize, V); 8]> = SmallVec::new();
        for (&pos, &element_id) in self.positions.iter().zip(element_ids.iter()).rev() {
            let value = list_remove(score_director.working_solution_mut(), entity, pos);
            removed.push((pos, element_id, value));
        }

        // Notify after removal for each element
        for (&pos, &element_id) in self.positions.iter().zip(element_ids.iter()) {
            score_director.after_list_element_changed(entity, pos, element_id);
        }

        // Register undo - reinsert in original order (lowest position first)
        score_director.register_undo(Box::new(move |s: &mut S| {
            for (pos, _element_id, value) in removed.into_iter().rev() {
                list_insert(s, entity, pos, value);
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
}
