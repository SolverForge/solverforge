//! PillarChangeMove - assigns a value to all entities in a pillar.
//!
//! A pillar is a group of entities that share the same variable value.
//! This move changes all of them to a new value atomically.
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

/// A move that assigns a value to all entities in a pillar.
///
/// Stores entity indices and uses `VariableOperations` for zero-erasure access.
///
/// # Type Parameters
/// * `S` - The planning solution type (must implement VariableOperations)
#[derive(Clone)]
pub struct PillarChangeMove<S> {
    entity_indices: Vec<usize>,
    descriptor_index: usize,
    variable_name: &'static str,
    to_value: usize,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for PillarChangeMove<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PillarChangeMove")
            .field("entity_indices", &self.entity_indices)
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_name", &self.variable_name)
            .field("to_value", &self.to_value)
            .finish()
    }
}

impl<S> PillarChangeMove<S> {
    /// Creates a new pillar change move.
    ///
    /// # Arguments
    /// * `entity_indices` - Indices of entities in the pillar
    /// * `to_value` - The new value index to assign to all entities
    /// * `variable_name` - Name of the variable being changed
    /// * `descriptor_index` - Index in the entity descriptor
    pub fn new(
        entity_indices: Vec<usize>,
        to_value: usize,
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_indices,
            descriptor_index,
            variable_name,
            to_value,
            _phantom: PhantomData,
        }
    }

    /// Returns the pillar size.
    pub fn pillar_size(&self) -> usize {
        self.entity_indices.len()
    }

    /// Returns the target value.
    pub fn to_value(&self) -> usize {
        self.to_value
    }
}

impl<S> Move<S> for PillarChangeMove<S>
where
    S: PlanningSolution + VariableOperations,
{
    fn is_doable<D: ScoreDirector<S>>(&self, score_director: &D) -> bool {
        if self.entity_indices.is_empty() {
            return false;
        }

        let solution = score_director.working_solution();

        // Check first entity exists and is assigned
        if let Some(&first_idx) = self.entity_indices.first() {
            let len = solution.list_len(first_idx);
            if len == 0 {
                // Unassigned - can assign
                return true;
            }

            // Get current value
            let current = solution.get(first_idx, 0);
            current != self.to_value
        } else {
            false
        }
    }

    fn do_move<D: ScoreDirector<S>>(&self, score_director: &mut D) {
        // Capture old values
        let old_values: Vec<(usize, Option<<S as VariableOperations>::Element>)> = self
            .entity_indices
            .iter()
            .map(|&idx| {
                let solution = score_director.working_solution();
                let len = solution.list_len(idx);
                let old_val = if len > 0 {
                    Some(solution.get(idx, 0))
                } else {
                    None
                };
                (idx, old_val)
            })
            .collect();

        // Notify before changes for all entities
        for &idx in &self.entity_indices {
            score_director.before_variable_changed(self.descriptor_index, idx, self.variable_name);
        }

        // Apply new value to all entities
        for &idx in &self.entity_indices {
            let sol = score_director.working_solution_mut();
            let len = sol.list_len(idx);
            if len > 0 {
                sol.remove(idx, 0);
            }
            sol.insert(idx, 0, self.to_value);
        }

        // Notify after changes
        for &idx in &self.entity_indices {
            score_director.after_variable_changed(self.descriptor_index, idx, self.variable_name);
        }

        // Register undo
        score_director.register_undo(Box::new(move |s: &mut S| {
            for (idx, old_value) in old_values {
                // Remove current value
                s.remove(idx, 0);
                // Restore old value if there was one
                if let Some(old) = old_value {
                    s.insert(idx, 0, old);
                }
            }
        }));
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
    struct ScheduleSolution {
        employees: Vec<Employee>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for ScheduleSolution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    impl VariableOperations for ScheduleSolution {
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
    ) -> SimpleScoreDirector<ScheduleSolution, impl Fn(&ScheduleSolution) -> SimpleScore> {
        let solution = ScheduleSolution {
            employees,
            score: None,
        };

        let extractor = Box::new(TypedEntityExtractor::new(
            "Employee",
            "employees",
            |s: &ScheduleSolution| &s.employees,
            |s: &mut ScheduleSolution| &mut s.employees,
        ));
        let entity_desc = EntityDescriptor::new("Employee", TypeId::of::<Employee>(), "employees")
            .with_extractor(extractor);

        let descriptor =
            SolutionDescriptor::new("ScheduleSolution", TypeId::of::<ScheduleSolution>())
                .with_entity(entity_desc);

        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn pillar_change_all_entities() {
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
        ]);

        // Change pillar [0, 1] from shift 1 to shift 5
        let m = PillarChangeMove::<ScheduleSolution>::new(vec![0, 1], 5, "shift", 0);

        assert!(m.is_doable(&director));
        assert_eq!(m.pillar_size(), 2);

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            // Verify ALL entities changed
            assert_eq!(recording.working_solution().employees[0].shift, Some(5));
            assert_eq!(recording.working_solution().employees[1].shift, Some(5));
            assert_eq!(recording.working_solution().employees[2].shift, Some(2)); // Unchanged

            // Undo
            recording.undo_changes();
        }

        assert_eq!(director.working_solution().employees[0].shift, Some(1));
        assert_eq!(director.working_solution().employees[1].shift, Some(1));
        assert_eq!(director.working_solution().employees[2].shift, Some(2));

        // Verify entity identity preserved
        let solution = director.working_solution();
        assert_eq!(solution.employees[0].id, 0);
        assert_eq!(solution.employees[1].id, 1);
        assert_eq!(solution.employees[2].id, 2);
    }

    #[test]
    fn pillar_change_same_value_not_doable() {
        let director = create_director(vec![
            Employee {
                id: 0,
                shift: Some(5),
            },
            Employee {
                id: 1,
                shift: Some(5),
            },
        ]);

        let m = PillarChangeMove::<ScheduleSolution>::new(vec![0, 1], 5, "shift", 0);

        assert!(!m.is_doable(&director));
    }

    #[test]
    fn pillar_change_empty_pillar_not_doable() {
        let director = create_director(vec![Employee {
            id: 0,
            shift: Some(1),
        }]);

        let m = PillarChangeMove::<ScheduleSolution>::new(vec![], 5, "shift", 0);

        assert!(!m.is_doable(&director));
    }

    #[test]
    fn pillar_change_entity_indices() {
        let m = PillarChangeMove::<ScheduleSolution>::new(vec![1, 3, 5], 5, "shift", 0);
        assert_eq!(m.entity_indices(), &[1, 3, 5]);
    }
}
