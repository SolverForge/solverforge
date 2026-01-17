//! ListSwapMove - swaps two elements within or between list variables.
//!
//! This move exchanges two elements at different positions.
//! Useful for TSP-style improvements and route optimization.
//!
//! # Zero-Erasure Design
//!
//! Stores only indices. No value type parameter. Operations use VariableOperations trait.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::operations::VariableOperations;

use super::Move;

/// A move that swaps two elements in list variables.
///
/// Supports both intra-list swaps (within same entity) and inter-list swaps
/// (between different entities). Uses `VariableOperations` trait for zero-erasure.
///
/// # Type Parameters
/// * `S` - The planning solution type (must implement VariableOperations)
#[derive(Clone, Copy)]
pub struct ListSwapMove<S> {
    /// First entity index
    first_entity_index: usize,
    /// Position in first entity's list
    first_position: usize,
    /// Second entity index
    second_entity_index: usize,
    /// Position in second entity's list
    second_position: usize,
    variable_name: &'static str,
    descriptor_index: usize,
    /// Store indices for entity_indices()
    indices: [usize; 2],
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for ListSwapMove<S> {
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

impl<S> ListSwapMove<S> {
    /// Creates a new list swap move.
    ///
    /// # Arguments
    /// * `first_entity_index` - First entity index
    /// * `first_position` - Position in first entity's list
    /// * `second_entity_index` - Second entity index
    /// * `second_position` - Position in second entity's list
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    pub fn new(
        first_entity_index: usize,
        first_position: usize,
        second_entity_index: usize,
        second_position: usize,
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            first_entity_index,
            first_position,
            second_entity_index,
            second_position,
            variable_name,
            descriptor_index,
            indices: [first_entity_index, second_entity_index],
            _phantom: PhantomData,
        }
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

impl<S> Move<S> for ListSwapMove<S>
where
    S: PlanningSolution + VariableOperations,
{
    fn is_doable<D: ScoreDirector<S>>(&self, score_director: &D) -> bool {
        let solution = score_director.working_solution();

        // Check first position is valid
        let first_len = solution.list_len(self.first_entity_index);
        if self.first_position >= first_len {
            return false;
        }

        // Check second position is valid
        let second_len = solution.list_len(self.second_entity_index);
        if self.second_position >= second_len {
            return false;
        }

        // For intra-list, can't swap with self
        if self.is_intra_list() && self.first_position == self.second_position {
            return false;
        }

        // Get values and check they're different
        let first_val = solution.get(self.first_entity_index, self.first_position);
        let second_val = solution.get(self.second_entity_index, self.second_position);

        first_val != second_val
    }

    fn do_move<D: ScoreDirector<S>>(&self, score_director: &mut D) {
        // Get both values
        let first_val = score_director
            .working_solution()
            .get(self.first_entity_index, self.first_position);
        let second_val = score_director
            .working_solution()
            .get(self.second_entity_index, self.second_position);

        // Notify before changes
        score_director.before_variable_changed(
            self.descriptor_index,
            self.first_entity_index,
            self.variable_name,
        );
        if !self.is_intra_list() {
            score_director.before_variable_changed(
                self.descriptor_index,
                self.second_entity_index,
                self.variable_name,
            );
        }

        // Swap: remove and insert at each position
        {
            let sol = score_director.working_solution_mut();
            sol.remove(self.first_entity_index, self.first_position);
            sol.insert(self.first_entity_index, self.first_position, second_val);
            sol.remove(self.second_entity_index, self.second_position);
            sol.insert(self.second_entity_index, self.second_position, first_val);
        }

        // Notify after changes
        score_director.after_variable_changed(
            self.descriptor_index,
            self.first_entity_index,
            self.variable_name,
        );
        if !self.is_intra_list() {
            score_director.after_variable_changed(
                self.descriptor_index,
                self.second_entity_index,
                self.variable_name,
            );
        }

        // Register undo - swap back
        let first_entity = self.first_entity_index;
        let first_pos = self.first_position;
        let second_entity = self.second_entity_index;
        let second_pos = self.second_position;

        score_director.register_undo(Box::new(move |s: &mut S| {
            // Restore original values
            s.remove(first_entity, first_pos);
            s.insert(first_entity, first_pos, first_val);
            s.remove(second_entity, second_pos);
            s.insert(second_entity, second_pos, second_val);
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
    use crate::operations::VariableOperations;
    use solverforge_core::domain::{EntityDescriptor, SolutionDescriptor, TypedEntityExtractor};
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::{RecordingScoreDirector, SimpleScoreDirector};
    use std::any::TypeId;

    #[derive(Clone, Debug)]
    struct Vehicle {
        visits: Vec<usize>,
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

    impl VariableOperations for RoutingSolution {
        type Element = usize;

        fn element_count(&self) -> usize {
            self.vehicles.iter().map(|v| v.visits.len()).sum()
        }

        fn entity_count(&self) -> usize {
            self.vehicles.len()
        }

        fn assigned_elements(&self) -> Vec<Self::Element> {
            self.vehicles
                .iter()
                .flat_map(|v| v.visits.iter().copied())
                .collect()
        }

        fn assign(&mut self, entity_idx: usize, elem: Self::Element) {
            self.vehicles[entity_idx].visits.push(elem);
        }

        fn list_len(&self, entity_idx: usize) -> usize {
            self.vehicles.get(entity_idx).map_or(0, |v| v.visits.len())
        }

        fn remove(&mut self, entity_idx: usize, pos: usize) -> Self::Element {
            self.vehicles[entity_idx].visits.remove(pos)
        }

        fn insert(&mut self, entity_idx: usize, pos: usize, elem: Self::Element) {
            self.vehicles[entity_idx].visits.insert(pos, elem);
        }

        fn get(&self, entity_idx: usize, pos: usize) -> Self::Element {
            self.vehicles[entity_idx].visits[pos]
        }

        fn descriptor_index() -> usize {
            0
        }

        fn variable_name() -> &'static str {
            "visits"
        }

        fn is_list_variable() -> bool {
            true
        }
    }

    fn get_vehicles(s: &RoutingSolution) -> &Vec<Vehicle> {
        &s.vehicles
    }
    fn get_vehicles_mut(s: &mut RoutingSolution) -> &mut Vec<Vehicle> {
        &mut s.vehicles
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
        let m = ListSwapMove::<RoutingSolution>::new(0, 1, 0, 3, "visits", 0);

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
        let m = ListSwapMove::<RoutingSolution>::new(0, 1, 1, 2, "visits", 0);

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

        let m = ListSwapMove::<RoutingSolution>::new(0, 1, 0, 1, "visits", 0);

        assert!(!m.is_doable(&director));
    }

    #[test]
    fn same_values_not_doable() {
        let vehicles = vec![Vehicle {
            visits: vec![5, 5, 5],
        }];
        let director = create_director(vehicles);

        let m = ListSwapMove::<RoutingSolution>::new(0, 0, 0, 2, "visits", 0);

        assert!(!m.is_doable(&director));
    }

    #[test]
    fn invalid_position_not_doable() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3],
        }];
        let director = create_director(vehicles);

        let m = ListSwapMove::<RoutingSolution>::new(0, 1, 0, 10, "visits", 0);

        assert!(!m.is_doable(&director));
    }
}
