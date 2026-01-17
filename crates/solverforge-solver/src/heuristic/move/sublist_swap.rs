//! SubListSwapMove - swaps two contiguous sublists within or between list variables.
//!
//! This move exchanges two ranges of elements. Essential for vehicle routing
//! where segments need to be swapped between vehicles.
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

/// A move that swaps two contiguous sublists.
///
/// Supports both intra-list swaps (within same entity) and inter-list swaps
/// (between different entities). Uses `VariableOperations` trait for zero-erasure.
///
/// # Type Parameters
/// * `S` - The planning solution type (must implement VariableOperations)
#[derive(Clone, Copy)]
pub struct SubListSwapMove<S> {
    /// First entity index
    first_entity_index: usize,
    /// Start of first range (inclusive)
    first_start: usize,
    /// End of first range (exclusive)
    first_end: usize,
    /// Second entity index
    second_entity_index: usize,
    /// Start of second range (inclusive)
    second_start: usize,
    /// End of second range (exclusive)
    second_end: usize,
    variable_name: &'static str,
    descriptor_index: usize,
    /// Store indices for entity_indices()
    indices: [usize; 2],
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for SubListSwapMove<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubListSwapMove")
            .field("first_entity", &self.first_entity_index)
            .field("first_range", &(self.first_start..self.first_end))
            .field("second_entity", &self.second_entity_index)
            .field("second_range", &(self.second_start..self.second_end))
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S> SubListSwapMove<S> {
    /// Creates a new sublist swap move.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        first_entity_index: usize,
        first_start: usize,
        first_end: usize,
        second_entity_index: usize,
        second_start: usize,
        second_end: usize,
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            first_entity_index,
            first_start,
            first_end,
            second_entity_index,
            second_start,
            second_end,
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

    /// Returns the first range start.
    pub fn first_start(&self) -> usize {
        self.first_start
    }

    /// Returns the first range end.
    pub fn first_end(&self) -> usize {
        self.first_end
    }

    /// Returns the first sublist length.
    pub fn first_len(&self) -> usize {
        self.first_end.saturating_sub(self.first_start)
    }

    /// Returns the second entity index.
    pub fn second_entity_index(&self) -> usize {
        self.second_entity_index
    }

    /// Returns the second range start.
    pub fn second_start(&self) -> usize {
        self.second_start
    }

    /// Returns the second range end.
    pub fn second_end(&self) -> usize {
        self.second_end
    }

    /// Returns the second sublist length.
    pub fn second_len(&self) -> usize {
        self.second_end.saturating_sub(self.second_start)
    }

    /// Returns true if this is an intra-list swap (same entity).
    pub fn is_intra_list(&self) -> bool {
        self.first_entity_index == self.second_entity_index
    }
}

impl<S> Move<S> for SubListSwapMove<S>
where
    S: PlanningSolution + VariableOperations,
{
    fn is_doable<D: ScoreDirector<S>>(&self, score_director: &D) -> bool {
        let solution = score_director.working_solution();

        // Both ranges must be valid (start < end)
        if self.first_start >= self.first_end || self.second_start >= self.second_end {
            return false;
        }

        // Check first range is within bounds
        let first_list_len = solution.list_len(self.first_entity_index);
        if self.first_end > first_list_len {
            return false;
        }

        // Check second range is within bounds
        let second_list_len = solution.list_len(self.second_entity_index);
        if self.second_end > second_list_len {
            return false;
        }

        // For intra-list swaps, ranges must not overlap
        if self.is_intra_list() {
            let overlaps = self.first_start < self.second_end && self.second_start < self.first_end;
            if overlaps {
                return false;
            }
        }

        true
    }

    fn do_move<D: ScoreDirector<S>>(&self, score_director: &mut D) {
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

        if self.is_intra_list() {
            // Intra-list swap: handle index shifts carefully
            let (early_start, early_end, late_start, late_end) =
                if self.first_start < self.second_start {
                    (
                        self.first_start,
                        self.first_end,
                        self.second_start,
                        self.second_end,
                    )
                } else {
                    (
                        self.second_start,
                        self.second_end,
                        self.first_start,
                        self.first_end,
                    )
                };

            let sol = score_director.working_solution_mut();

            // Remove later range first
            let late_elements = sol.remove_sublist(self.first_entity_index, late_start, late_end);

            // Remove earlier range
            let early_elements = sol.remove_sublist(self.first_entity_index, early_start, early_end);

            // Insert late elements at early position
            sol.insert_sublist(self.first_entity_index, early_start, late_elements.clone());

            // Insert early elements at adjusted late position
            let late_len = late_end - late_start;
            let early_len = early_end - early_start;
            let new_late_pos = late_start - early_len + late_len;
            sol.insert_sublist(self.first_entity_index, new_late_pos, early_elements.clone());

            // Register undo
            let entity = self.first_entity_index;

            score_director.register_undo(Box::new(move |s: &mut S| {
                // Remove late elements (now at early position with late_len)
                let late_at_early = s.remove_sublist(entity, early_start, early_start + late_len);
                // Remove early elements (now at new_late_pos with early_len)
                let early_at_late =
                    s.remove_sublist(entity, new_late_pos - late_len, new_late_pos - late_len + early_len);
                // Insert early back at early
                s.insert_sublist(entity, early_start, early_at_late);
                // Insert late back at late
                s.insert_sublist(entity, late_start, late_at_early);
            }));
        } else {
            // Inter-list swap: simpler
            let sol = score_director.working_solution_mut();

            let first_elements =
                sol.remove_sublist(self.first_entity_index, self.first_start, self.first_end);
            let second_elements =
                sol.remove_sublist(self.second_entity_index, self.second_start, self.second_end);

            // Insert swapped
            sol.insert_sublist(self.first_entity_index, self.first_start, second_elements.clone());
            sol.insert_sublist(self.second_entity_index, self.second_start, first_elements.clone());

            // Register undo
            let first_entity = self.first_entity_index;
            let first_start = self.first_start;
            let second_entity = self.second_entity_index;
            let second_start = self.second_start;
            let first_len = self.first_len();
            let second_len = self.second_len();

            score_director.register_undo(Box::new(move |s: &mut S| {
                let second_at_first = s.remove_sublist(first_entity, first_start, first_start + second_len);
                let first_at_second =
                    s.remove_sublist(second_entity, second_start, second_start + first_len);
                s.insert_sublist(first_entity, first_start, first_at_second);
                s.insert_sublist(second_entity, second_start, second_at_first);
            }));
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
    fn inter_list_swap() {
        let vehicles = vec![
            Vehicle {
                visits: vec![1, 2, 3, 4],
            },
            Vehicle {
                visits: vec![10, 20, 30],
            },
        ];
        let mut director = create_director(vehicles);

        // Swap [1..3) from vehicle 0 with [0..2) from vehicle 1
        let m = SubListSwapMove::<RoutingSolution>::new(0, 1, 3, 1, 0, 2, "visits", 0);

        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            let sol = recording.working_solution();
            assert_eq!(sol.vehicles[0].visits, vec![1, 10, 20, 4]);
            assert_eq!(sol.vehicles[1].visits, vec![2, 3, 30]);

            recording.undo_changes();
        }

        let sol = director.working_solution();
        assert_eq!(sol.vehicles[0].visits, vec![1, 2, 3, 4]);
        assert_eq!(sol.vehicles[1].visits, vec![10, 20, 30]);
    }

    #[test]
    fn intra_list_swap() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3, 4, 5, 6, 7, 8],
        }];
        let mut director = create_director(vehicles);

        // Swap [1..3) with [5..7) in same list
        let m = SubListSwapMove::<RoutingSolution>::new(0, 1, 3, 0, 5, 7, "visits", 0);

        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            // After: [1, 6, 7, 4, 5, 2, 3, 8]
            let visits = &recording.working_solution().vehicles[0].visits;
            assert_eq!(visits, &[1, 6, 7, 4, 5, 2, 3, 8]);

            recording.undo_changes();
        }

        let visits = &director.working_solution().vehicles[0].visits;
        assert_eq!(visits, &[1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn overlapping_ranges_not_doable() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3, 4, 5],
        }];
        let director = create_director(vehicles);

        // Ranges [1..4) and [2..5) overlap
        let m = SubListSwapMove::<RoutingSolution>::new(0, 1, 4, 0, 2, 5, "visits", 0);

        assert!(!m.is_doable(&director));
    }

    #[test]
    fn empty_range_not_doable() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3],
        }];
        let director = create_director(vehicles);

        let m = SubListSwapMove::<RoutingSolution>::new(0, 1, 1, 0, 2, 3, "visits", 0);

        assert!(!m.is_doable(&director));
    }

    #[test]
    fn out_of_bounds_not_doable() {
        let vehicles = vec![Vehicle {
            visits: vec![1, 2, 3],
        }];
        let director = create_director(vehicles);

        let m = SubListSwapMove::<RoutingSolution>::new(0, 0, 2, 0, 2, 10, "visits", 0);

        assert!(!m.is_doable(&director));
    }
}
