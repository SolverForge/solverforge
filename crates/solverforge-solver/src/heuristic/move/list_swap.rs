//! ListSwapMove - swaps two elements within or between list variables.
//!
//! This move exchanges two elements at different positions.
//! Useful for TSP-style improvements and route optimization.
//!
//! # Zero-Erasure Design
//!
//! Uses typed function pointers for list operations. No `dyn Any`, no downcasting.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::Move;

/// A move that swaps two elements in list variables.
///
/// Supports both intra-list swaps (within same entity) and inter-list swaps
/// (between different entities). Uses typed function pointers for zero-erasure.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The list element value type
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::r#move::ListSwapMove;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone, Debug)]
/// struct Vehicle { id: usize, visits: Vec<i32> }
///
/// #[derive(Clone, Debug)]
/// struct Solution { vehicles: Vec<Vehicle>, score: Option<SimpleScore> }
///
/// impl PlanningSolution for Solution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// fn list_len(s: &Solution, entity_idx: usize) -> usize {
///     s.vehicles.get(entity_idx).map_or(0, |v| v.visits.len())
/// }
/// fn list_get(s: &Solution, entity_idx: usize, pos: usize) -> Option<i32> {
///     s.vehicles.get(entity_idx).and_then(|v| v.visits.get(pos).copied())
/// }
/// fn list_set(s: &mut Solution, entity_idx: usize, pos: usize, val: i32) {
///     if let Some(v) = s.vehicles.get_mut(entity_idx) {
///         if let Some(elem) = v.visits.get_mut(pos) { *elem = val; }
///     }
/// }
///
/// // Swap elements at positions 1 and 3 in vehicle 0
/// // Element indices 1 and 3 are the global indices of elements being swapped
/// let m = ListSwapMove::<Solution, i32>::new(
///     0, 1, 1,  // first: entity 0, position 1, element_idx 1
///     0, 3, 3,  // second: entity 0, position 3, element_idx 3
///     list_len, list_get, list_set,
///     "visits", 0,
/// );
/// ```
pub struct ListSwapMove<S, V> {
    /// First entity index
    first_entity_index: usize,
    /// Position in first entity's list
    first_position: usize,
    /// Element index at first position (for O(1) shadow updates)
    first_element_idx: usize,
    /// Second entity index
    second_entity_index: usize,
    /// Position in second entity's list
    second_position: usize,
    /// Element index at second position (for O(1) shadow updates)
    second_element_idx: usize,
    /// Get list length for an entity
    list_len: fn(&S, usize) -> usize,
    /// Get element at position
    list_get: fn(&S, usize, usize) -> Option<V>,
    /// Set element at position
    list_set: fn(&mut S, usize, usize, V),
    variable_name: &'static str,
    descriptor_index: usize,
    /// Store indices for entity_indices()
    indices: [usize; 2],
}

impl<S, V> Clone for ListSwapMove<S, V> {
    fn clone(&self) -> Self {
        Self {
            first_entity_index: self.first_entity_index,
            first_position: self.first_position,
            first_element_idx: self.first_element_idx,
            second_entity_index: self.second_entity_index,
            second_position: self.second_position,
            second_element_idx: self.second_element_idx,
            list_len: self.list_len,
            list_get: self.list_get,
            list_set: self.list_set,
            variable_name: self.variable_name,
            descriptor_index: self.descriptor_index,
            indices: self.indices,
        }
    }
}

impl<S, V> Copy for ListSwapMove<S, V> {}

impl<S, V: Debug> Debug for ListSwapMove<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListSwapMove")
            .field("first_entity", &self.first_entity_index)
            .field("first_position", &self.first_position)
            .field("second_entity", &self.second_entity_index)
            .field("second_position", &self.second_position)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, V> ListSwapMove<S, V> {
    /// Creates a new list swap move with typed function pointers.
    ///
    /// # Arguments
    /// * `first_entity_index` - First entity index
    /// * `first_position` - Position in first entity's list
    /// * `first_element_idx` - Element index at first position (for O(1) shadow updates)
    /// * `second_entity_index` - Second entity index
    /// * `second_position` - Position in second entity's list
    /// * `second_element_idx` - Element index at second position (for O(1) shadow updates)
    /// * `list_len` - Function to get list length
    /// * `list_get` - Function to get element at position
    /// * `list_set` - Function to set element at position
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        first_entity_index: usize,
        first_position: usize,
        first_element_idx: usize,
        second_entity_index: usize,
        second_position: usize,
        second_element_idx: usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_set: fn(&mut S, usize, usize, V),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            first_entity_index,
            first_position,
            first_element_idx,
            second_entity_index,
            second_position,
            second_element_idx,
            list_len,
            list_get,
            list_set,
            variable_name,
            descriptor_index,
            indices: [first_entity_index, second_entity_index],
        }
    }

    /// Returns the first element index.
    pub fn first_element_idx(&self) -> usize {
        self.first_element_idx
    }

    /// Returns the second element index.
    pub fn second_element_idx(&self) -> usize {
        self.second_element_idx
    }

    /// Returns the first entity index.
    pub fn first_entity_index(&self) -> usize {
        self.first_entity_index
    }

    /// Returns the first position.
    pub fn first_position(&self) -> usize {
        self.first_position
    }

    /// Returns the second entity index.
    pub fn second_entity_index(&self) -> usize {
        self.second_entity_index
    }

    /// Returns the second position.
    pub fn second_position(&self) -> usize {
        self.second_position
    }

    /// Returns true if this is an intra-list swap (same entity).
    pub fn is_intra_list(&self) -> bool {
        self.first_entity_index == self.second_entity_index
    }
}

impl<S, V> Move<S> for ListSwapMove<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn is_doable<D: ScoreDirector<S>>(&self, score_director: &D) -> bool {
        let solution = score_director.working_solution();

        // Check first position is valid
        let first_len = (self.list_len)(solution, self.first_entity_index);
        if self.first_position >= first_len {
            return false;
        }

        // Check second position is valid
        let second_len = (self.list_len)(solution, self.second_entity_index);
        if self.second_position >= second_len {
            return false;
        }

        // For intra-list, can't swap with self
        if self.is_intra_list() && self.first_position == self.second_position {
            return false;
        }

        // Get values and check they're different
        let first_val = (self.list_get)(solution, self.first_entity_index, self.first_position);
        let second_val = (self.list_get)(solution, self.second_entity_index, self.second_position);

        first_val != second_val
    }

    fn do_move<D: ScoreDirector<S>>(&self, score_director: &mut D) {
        // Get both values
        let first_val = (self.list_get)(
            score_director.working_solution(),
            self.first_entity_index,
            self.first_position,
        )
        .expect("first position should be valid");

        let second_val = (self.list_get)(
            score_director.working_solution(),
            self.second_entity_index,
            self.second_position,
        )
        .expect("second position should be valid");

        // Notify before changes for first element
        score_director.before_list_variable_changed(
            self.descriptor_index,
            self.first_entity_index,
            self.first_position,
            self.first_element_idx,
            self.variable_name,
        );
        // Notify before changes for second element
        score_director.before_list_variable_changed(
            self.descriptor_index,
            self.second_entity_index,
            self.second_position,
            self.second_element_idx,
            self.variable_name,
        );

        // Swap: first gets second's value, second gets first's value
        (self.list_set)(
            score_director.working_solution_mut(),
            self.first_entity_index,
            self.first_position,
            second_val.clone(),
        );
        (self.list_set)(
            score_director.working_solution_mut(),
            self.second_entity_index,
            self.second_position,
            first_val.clone(),
        );

        // Notify after changes - note: elements swapped positions
        // First position now has second_element_idx
        score_director.after_list_variable_changed(
            self.descriptor_index,
            self.first_entity_index,
            self.first_position,
            self.second_element_idx,
            self.variable_name,
        );
        // Second position now has first_element_idx
        score_director.after_list_variable_changed(
            self.descriptor_index,
            self.second_entity_index,
            self.second_position,
            self.first_element_idx,
            self.variable_name,
        );

        // Register undo - swap back
        let list_set = self.list_set;
        let first_entity = self.first_entity_index;
        let first_pos = self.first_position;
        let second_entity = self.second_entity_index;
        let second_pos = self.second_position;

        score_director.register_undo(Box::new(move |s: &mut S| {
            // Restore original values
            list_set(s, first_entity, first_pos, first_val);
            list_set(s, second_entity, second_pos, second_val);
        }));
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        if self.is_intra_list() {
            &self.indices[0..1]
        } else {
            &self.indices
        }
    }

    fn variable_name(&self) -> &str {
        self.variable_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solverforge_core::domain::{EntityDescriptor, SolutionDescriptor, TypedEntityExtractor};
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::{RecordingScoreDirector, SimpleScoreDirector};
    use std::any::TypeId;

    #[derive(Clone, Debug)]
    struct Vehicle {
        visits: Vec<i32>,
    }

    #[derive(Clone, Debug)]
    struct RoutingSolution {
        vehicles: Vec<Vehicle>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for RoutingSolution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    fn get_vehicles(s: &RoutingSolution) -> &Vec<Vehicle> {
        &s.vehicles
    }
    fn get_vehicles_mut(s: &mut RoutingSolution) -> &mut Vec<Vehicle> {
        &mut s.vehicles
    }

    fn list_len(s: &RoutingSolution, entity_idx: usize) -> usize {
        s.vehicles.get(entity_idx).map_or(0, |v| v.visits.len())
    }
    fn list_get(s: &RoutingSolution, entity_idx: usize, pos: usize) -> Option<i32> {
        s.vehicles
            .get(entity_idx)
            .and_then(|v| v.visits.get(pos).copied())
    }
    fn list_set(s: &mut RoutingSolution, entity_idx: usize, pos: usize, val: i32) {
        if let Some(v) = s.vehicles.get_mut(entity_idx) {
            if let Some(elem) = v.visits.get_mut(pos) {
                *elem = val;
            }
        }
    }

    fn create_director(
        vehicles: Vec<Vehicle>,
    ) -> SimpleScoreDirector<RoutingSolution, impl Fn(&RoutingSolution) -> SimpleScore> {
        let solution = RoutingSolution {
            vehicles,
            score: None,
        };
        let extractor = Box::new(TypedEntityExtractor::new(
            "Vehicle",
            "vehicles",
            get_vehicles,
            get_vehicles_mut,
        ));
        let entity_desc = EntityDescriptor::new("Vehicle", TypeId::of::<Vehicle>(), "vehicles")
            .with_extractor(extractor);
        let descriptor =
            SolutionDescriptor::new("RoutingSolution", TypeId::of::<RoutingSolution>())
                .with_entity(entity_desc);
        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn intra_list_swap() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3, 4, 5],
        }];
        let mut director = create_director(vehicles);

        // Swap positions 1 and 3 (values 2 and 4)
        // first_element_idx=2 (value at pos 1), second_element_idx=4 (value at pos 3)
        let m = ListSwapMove::<RoutingSolution, i32>::new(
            0, 1, 2, 0, 3, 4, list_len, list_get, list_set, "visits", 0,
        );

        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            let visits = &recording.working_solution().vehicles[0].visits;
            assert_eq!(visits, &[1, 4, 3, 2, 5]);

            recording.undo_changes();
        }

        let visits = &director.working_solution().vehicles[0].visits;
        assert_eq!(visits, &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn inter_list_swap() {
        let vehicles = vec![
            Vehicle {
                visits: vec![1, 2, 3],
            },
            Vehicle {
                visits: vec![10, 20, 30],
            },
        ];
        let mut director = create_director(vehicles);

        // Swap vehicle 0 position 1 (value=2) with vehicle 1 position 2 (value=30)
        // first_element_idx=2, second_element_idx=30
        let m = ListSwapMove::<RoutingSolution, i32>::new(
            0, 1, 2, 1, 2, 30, list_len, list_get, list_set, "visits", 0,
        );

        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            let sol = recording.working_solution();
            assert_eq!(sol.vehicles[0].visits, vec![1, 30, 3]);
            assert_eq!(sol.vehicles[1].visits, vec![10, 20, 2]);

            recording.undo_changes();
        }

        let sol = director.working_solution();
        assert_eq!(sol.vehicles[0].visits, vec![1, 2, 3]);
        assert_eq!(sol.vehicles[1].visits, vec![10, 20, 30]);
    }

    #[test]
    fn same_position_not_doable() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3],
        }];
        let director = create_director(vehicles);

        let m = ListSwapMove::<RoutingSolution, i32>::new(
            0, 1, 2, 0, 1, 2, list_len, list_get, list_set, "visits", 0,
        );

        assert!(!m.is_doable(&director));
    }

    #[test]
    fn same_values_not_doable() {
        let vehicles = vec![Vehicle {
            visits: vec![5, 5, 5],
        }];
        let director = create_director(vehicles);

        let m = ListSwapMove::<RoutingSolution, i32>::new(
            0, 0, 5, 0, 2, 5, list_len, list_get, list_set, "visits", 0,
        );

        assert!(!m.is_doable(&director));
    }

    #[test]
    fn invalid_position_not_doable() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3],
        }];
        let director = create_director(vehicles);

        let m = ListSwapMove::<RoutingSolution, i32>::new(
            0, 1, 2, 0, 10, 99, list_len, list_get, list_set, "visits", 0,
        );

        assert!(!m.is_doable(&director));
    }
}
