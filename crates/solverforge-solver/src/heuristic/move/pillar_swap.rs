//! PillarSwapMove - exchanges values between two pillars.
//!
//! A pillar is a group of entities that share the same variable value.
//! This move swaps the values between two pillars atomically.
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

/// A move that swaps values between two pillars.
///
/// Stores pillar indices and uses `VariableOperations` for zero-erasure access.
///
/// # Type Parameters
/// * `S` - The planning solution type (must implement VariableOperations)
#[derive(Clone)]
pub struct PillarSwapMove<S> {
    left_indices: Vec<usize>,
    right_indices: Vec<usize>,
    descriptor_index: usize,
    variable_name: &'static str,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for PillarSwapMove<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PillarSwapMove")
            .field("left_indices", &self.left_indices)
            .field("right_indices", &self.right_indices)
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S> PillarSwapMove<S> {
    /// Creates a new pillar swap move.
    ///
    /// # Arguments
    /// * `left_indices` - Indices of entities in the left pillar
    /// * `right_indices` - Indices of entities in the right pillar
    /// * `variable_name` - Name of the variable being swapped
    /// * `descriptor_index` - Index in the entity descriptor
    pub fn new(
        left_indices: Vec<usize>,
        right_indices: Vec<usize>,
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            left_indices,
            right_indices,
            descriptor_index,
            variable_name,
            _phantom: PhantomData,
        }
    }

    /// Returns the left pillar indices.
    pub fn left_indices(&self) -> &[usize] {
        &self.left_indices
    }

    /// Returns the right pillar indices.
    pub fn right_indices(&self) -> &[usize] {
        &self.right_indices
    }
}

impl<S> Move<S> for PillarSwapMove<S>
where
    S: PlanningSolution + VariableOperations,
{
    fn is_doable<D: ScoreDirector<S>>(&self, score_director: &D) -> bool {
        if self.left_indices.is_empty() || self.right_indices.is_empty() {
            return false;
        }

        let solution = score_director.working_solution();

        // Check all entities are assigned
        for &idx in self.left_indices.iter().chain(&self.right_indices) {
            if solution.list_len(idx) == 0 {
                return false;
            }
        }

        // Get representative values
        let left_val = self.left_indices.first().map(|&idx| solution.get(idx, 0));
        let right_val = self.right_indices.first().map(|&idx| solution.get(idx, 0));

        left_val != right_val
    }

    fn do_move<D: ScoreDirector<S>>(&self, score_director: &mut D) {
        // Capture all old values
        let left_old: Vec<(usize, <S as VariableOperations>::Element)> = self
            .left_indices
            .iter()
            .map(|&idx| (idx, score_director.working_solution().get(idx, 0)))
            .collect();
        let right_old: Vec<(usize, <S as VariableOperations>::Element)> = self
            .right_indices
            .iter()
            .map(|&idx| (idx, score_director.working_solution().get(idx, 0)))
            .collect();

        // Get representative values for the swap
        let left_value = left_old.first().map(|(_, v)| *v).unwrap();
        let right_value = right_old.first().map(|(_, v)| *v).unwrap();

        // Notify before changes for all entities
        for &idx in self.left_indices.iter().chain(&self.right_indices) {
            score_director.before_variable_changed(self.descriptor_index, idx, self.variable_name);
        }

        // Swap: left gets right's value
        for &idx in &self.left_indices {
            let sol = score_director.working_solution_mut();
            sol.remove(idx, 0);
            sol.insert(idx, 0, right_value);
        }
        // Right gets left's value
        for &idx in &self.right_indices {
            let sol = score_director.working_solution_mut();
            sol.remove(idx, 0);
            sol.insert(idx, 0, left_value);
        }

        // Notify after changes
        for &idx in self.left_indices.iter().chain(&self.right_indices) {
            score_director.after_variable_changed(self.descriptor_index, idx, self.variable_name);
        }

        // Register undo - restore all original values
        score_director.register_undo(Box::new(move |s: &mut S| {
            for (idx, old_value) in left_old {
                s.remove(idx, 0);
                s.insert(idx, 0, old_value);
            }
            for (idx, old_value) in right_old {
                s.remove(idx, 0);
                s.insert(idx, 0, old_value);
            }
        }));
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        // Return left indices as primary; caller can use left_indices/right_indices for full info
        &self.left_indices
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
    struct Employee {
        id: usize,
        shift: Option<usize>,
    }

    #[derive(Clone, Debug)]
    struct Solution {
        employees: Vec<Employee>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for Solution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    impl VariableOperations for Solution {
        type Element = usize;

        fn element_count(&self) -> usize {
            10 // 10 shift values
        }

        fn entity_count(&self) -> usize {
            self.employees.len()
        }

        fn assigned_elements(&self) -> Vec<Self::Element> {
            self.employees.iter().filter_map(|e| e.shift).collect()
        }

        fn assign(&mut self, entity_idx: usize, elem: Self::Element) {
            self.employees[entity_idx].shift = Some(elem);
        }

        fn list_len(&self, entity_idx: usize) -> usize {
            if self.employees[entity_idx].shift.is_some() {
                1
            } else {
                0
            }
        }

        fn remove(&mut self, entity_idx: usize, _pos: usize) -> Self::Element {
            self.employees[entity_idx].shift.take().unwrap()
        }

        fn insert(&mut self, entity_idx: usize, _pos: usize, elem: Self::Element) {
            self.employees[entity_idx].shift = Some(elem);
        }

        fn get(&self, entity_idx: usize, _pos: usize) -> Self::Element {
            self.employees[entity_idx].shift.unwrap()
        }

        fn descriptor_index() -> usize {
            0
        }

        fn variable_name() -> &'static str {
            "shift"
        }

        fn is_list_variable() -> bool {
            false
        }
    }

    fn create_director(
        employees: Vec<Employee>,
    ) -> SimpleScoreDirector<Solution, impl Fn(&Solution) -> SimpleScore> {
        let solution = Solution {
            employees,
            score: None,
        };
        let extractor = Box::new(TypedEntityExtractor::new(
            "Employee",
            "employees",
            |s: &Solution| &s.employees,
            |s: &mut Solution| &mut s.employees,
        ));
        let entity_desc = EntityDescriptor::new("Employee", TypeId::of::<Employee>(), "employees")
            .with_extractor(extractor);
        let descriptor =
            SolutionDescriptor::new("Solution", TypeId::of::<Solution>()).with_entity(entity_desc);
        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn pillar_swap_all_entities() {
        let mut director = create_director(vec![
            Employee {
                id: 0,
                shift: Some(1),
            },
            Employee {
                id: 1,
                shift: Some(1),
            },
            Employee {
                id: 2,
                shift: Some(2),
            },
            Employee {
                id: 3,
                shift: Some(2),
            },
        ]);

        let m = PillarSwapMove::<Solution>::new(vec![0, 1], vec![2, 3], "shift", 0);
        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            // Verify swap
            assert_eq!(recording.working_solution().employees[0].shift, Some(2));
            assert_eq!(recording.working_solution().employees[1].shift, Some(2));
            assert_eq!(recording.working_solution().employees[2].shift, Some(1));
            assert_eq!(recording.working_solution().employees[3].shift, Some(1));

            recording.undo_changes();
        }

        assert_eq!(director.working_solution().employees[0].shift, Some(1));
        assert_eq!(director.working_solution().employees[1].shift, Some(1));
        assert_eq!(director.working_solution().employees[2].shift, Some(2));
        assert_eq!(director.working_solution().employees[3].shift, Some(2));

        // Verify entity identity preserved
        let solution = director.working_solution();
        assert_eq!(solution.employees[0].id, 0);
        assert_eq!(solution.employees[1].id, 1);
        assert_eq!(solution.employees[2].id, 2);
        assert_eq!(solution.employees[3].id, 3);
    }

    #[test]
    fn pillar_swap_same_value_not_doable() {
        let director = create_director(vec![
            Employee {
                id: 0,
                shift: Some(1),
            },
            Employee {
                id: 1,
                shift: Some(1),
            },
        ]);
        let m = PillarSwapMove::<Solution>::new(vec![0], vec![1], "shift", 0);
        assert!(!m.is_doable(&director));
    }

    #[test]
    fn pillar_swap_empty_pillar_not_doable() {
        let director = create_director(vec![Employee {
            id: 0,
            shift: Some(1),
        }]);
        let m = PillarSwapMove::<Solution>::new(vec![], vec![0], "shift", 0);
        assert!(!m.is_doable(&director));
    }
}
