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

use super::Move;

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
///
/// // Remove elements at indices 1 and 3 from the route
/// let m = ListRuinMove::<Route, i32>::new(
///     0,
///     &[1, 3],
///     list_len, list_remove, list_insert,
///     "stops", 0,
/// );
/// ```
pub struct ListRuinMove<S, V> {
    /// Entity index
    entity_index: usize,
    /// Indices of elements to remove (in ascending order for correct removal)
    element_indices: SmallVec<[usize; 8]>,
    /// Get list length
    list_len: fn(&S, usize) -> usize,
    /// Remove element at index, returning it
    list_remove: fn(&mut S, usize, usize) -> V,
    /// Insert element at index
    list_insert: fn(&mut S, usize, usize, V),
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<V>,
}

impl<S, V> Clone for ListRuinMove<S, V> {
    fn clone(&self) -> Self {
        Self {
            entity_index: self.entity_index,
            element_indices: self.element_indices.clone(),
            list_len: self.list_len,
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
    /// Creates a new list ruin move with typed function pointers.
    ///
    /// # Arguments
    /// * `entity_index` - Entity index
    /// * `element_indices` - Indices of elements to remove
    /// * `list_len` - Function to get list length
    /// * `list_remove` - Function to remove element at index
    /// * `list_insert` - Function to insert element at index
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    ///
    /// # Note
    /// Indices are sorted internally for correct removal order.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_index: usize,
        element_indices: &[usize],
        list_len: fn(&S, usize) -> usize,
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
            list_len,
            list_remove,
            list_insert,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }

    /// Returns the entity index.
    pub fn entity_index(&self) -> usize {
        self.entity_index
    }

    /// Returns the element indices being removed.
    pub fn element_indices(&self) -> &[usize] {
        &self.element_indices
    }

    /// Returns the number of elements being removed.
    pub fn ruin_count(&self) -> usize {
        self.element_indices.len()
    }
}

impl<S, V> Move<S> for ListRuinMove<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    fn is_doable<D: ScoreDirector<S>>(&self, score_director: &D) -> bool {
        if self.element_indices.is_empty() {
            return false;
        }

        let solution = score_director.working_solution();
        let len = (self.list_len)(solution, self.entity_index);

        // All indices must be within bounds
        self.element_indices.iter().all(|&idx| idx < len)
    }

    fn do_move<D: ScoreDirector<S>>(&self, score_director: &mut D) {
        let list_remove = self.list_remove;
        let list_insert = self.list_insert;
        let entity = self.entity_index;
        let descriptor = self.descriptor_index;
        let variable_name = self.variable_name;

        // Notify before change
        score_director.before_variable_changed(descriptor, entity, variable_name);

        // Remove elements in reverse order (highest index first) to preserve indices
        let mut removed: SmallVec<[(usize, V); 8]> = SmallVec::new();
        for &idx in self.element_indices.iter().rev() {
            let value = list_remove(score_director.working_solution_mut(), entity, idx);
            removed.push((idx, value));
        }

        // Notify after change
        score_director.after_variable_changed(descriptor, entity, variable_name);

        // Register undo - reinsert in original order (lowest index first)
        score_director.register_undo(Box::new(move |s: &mut S| {
            for (idx, value) in removed.into_iter().rev() {
                list_insert(s, entity, idx, value);
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
