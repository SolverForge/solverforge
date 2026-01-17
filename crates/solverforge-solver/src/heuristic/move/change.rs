//! ChangeMove - assigns a value to a planning variable.
//!
//! This is the most fundamental move type. It takes a value index and assigns
//! it to a planning variable on an entity.
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

/// A move that assigns a value to an entity's variable.
///
/// Uses `VariableOperations` trait for zero-erasure execution.
/// No trait objects, no boxing - all operations are fully typed at compile time.
///
/// # Type Parameters
/// * `S` - The planning solution type (must implement VariableOperations)
#[derive(Clone, Copy)]
pub struct ChangeMove<S> {
    entity_index: usize,
    /// The element/value to assign (as an index into value range for basic variables)
    to_value: usize,
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for ChangeMove<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChangeMove")
            .field("entity_index", &self.entity_index)
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_name", &self.variable_name)
            .field("to_value", &self.to_value)
            .finish()
    }
}

impl<S> ChangeMove<S> {
    /// Creates a new change move.
    ///
    /// # Arguments
    /// * `entity_index` - Index of the entity in its collection
    /// * `to_value` - The value/element index to assign
    /// * `variable_name` - Name of the variable (for debugging)
    /// * `descriptor_index` - Index of the entity descriptor
    pub fn new(
        entity_index: usize,
        to_value: usize,
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_index,
            to_value,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }

    /// Returns the entity index.
    pub fn entity_index(&self) -> usize {
        self.entity_index
    }

    /// Returns the target value index.
    pub fn to_value(&self) -> usize {
        self.to_value
    }
}

impl<S> Move<S> for ChangeMove<S>
where
    S: PlanningSolution + VariableOperations,
{
    fn is_doable<D: ScoreDirector<S>>(&self, score_director: &D) -> bool {
        let solution = score_director.working_solution();

        // For basic variables, list_len is 1 if assigned, 0 if not
        let len = solution.list_len(self.entity_index);

        if len == 0 {
            // Not assigned - always doable to assign
            true
        } else {
            // Get current value and compare
            let current = solution.get(self.entity_index, 0);
            current != self.to_value
        }
    }

    fn do_move<D: ScoreDirector<S>>(&self, score_director: &mut D) {
        let solution = score_director.working_solution();
        let len = solution.list_len(self.entity_index);

        // Capture old value for undo
        let old_value = if len > 0 {
            Some(solution.get(self.entity_index, 0))
        } else {
            None
        };

        // Notify before change
        score_director.before_variable_changed(
            self.descriptor_index,
            self.entity_index,
            self.variable_name,
        );

        // Set value: remove old if exists, then insert new
        {
            let sol = score_director.working_solution_mut();
            if len > 0 {
                sol.remove(self.entity_index, 0);
            }
            sol.insert(self.entity_index, 0, self.to_value);
        }

        // Notify after change
        score_director.after_variable_changed(
            self.descriptor_index,
            self.entity_index,
            self.variable_name,
        );

        // Register undo
        let entity = self.entity_index;
        let new_value = self.to_value;
        score_director.register_undo(Box::new(move |s: &mut S| {
            // Remove the new value
            s.remove(entity, 0);
            // Restore old value if there was one
            if let Some(old) = old_value {
                s.insert(entity, 0, old);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operations::VariableOperations;
    use solverforge_core::domain::SolutionDescriptor;
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::{RecordingScoreDirector, SimpleScoreDirector};
    use std::any::TypeId;

    #[derive(Clone, Debug, PartialEq)]
    struct Task {
        id: usize,
        priority: Option<usize>,
    }

    #[derive(Clone, Debug)]
    struct TaskSolution {
        tasks: Vec<Task>,
        priorities: Vec<usize>, // Available priority values: [1, 2, 3, 4, 5]
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for TaskSolution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    impl VariableOperations for TaskSolution {
        type Element = usize;

        fn element_count(&self) -> usize {
            self.priorities.len()
        }

        fn entity_count(&self) -> usize {
            self.tasks.len()
        }

        fn assigned_elements(&self) -> Vec<Self::Element> {
            self.tasks.iter().filter_map(|t| t.priority).collect()
        }

        fn assign(&mut self, entity_idx: usize, elem: Self::Element) {
            self.tasks[entity_idx].priority = Some(elem);
        }

        fn list_len(&self, entity_idx: usize) -> usize {
            if self.tasks[entity_idx].priority.is_some() {
                1
            } else {
                0
            }
        }

        fn remove(&mut self, entity_idx: usize, _pos: usize) -> Self::Element {
            self.tasks[entity_idx].priority.take().unwrap()
        }

        fn insert(&mut self, entity_idx: usize, _pos: usize, elem: Self::Element) {
            self.tasks[entity_idx].priority = Some(elem);
        }

        fn get(&self, entity_idx: usize, _pos: usize) -> Self::Element {
            self.tasks[entity_idx].priority.unwrap()
        }

        fn value_range(&self) -> Vec<Self::Element> {
            self.priorities.clone()
        }

        fn descriptor_index() -> usize {
            0
        }

        fn variable_name() -> &'static str {
            "priority"
        }

        fn is_list_variable() -> bool {
            false
        }
    }

    fn create_director(
        tasks: Vec<Task>,
    ) -> SimpleScoreDirector<TaskSolution, impl Fn(&TaskSolution) -> SimpleScore> {
        let solution = TaskSolution {
            tasks,
            priorities: vec![1, 2, 3, 4, 5],
            score: None,
        };
        let descriptor = SolutionDescriptor::new("TaskSolution", TypeId::of::<TaskSolution>());
        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn change_to_different_value() {
        let tasks = vec![Task {
            id: 0,
            priority: Some(1),
        }];
        let mut director = create_director(tasks);

        // Change from 1 to 5
        let m = ChangeMove::<TaskSolution>::new(0, 5, "priority", 0);
        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            let val = recording.working_solution().tasks[0].priority;
            assert_eq!(val, Some(5));

            recording.undo_changes();
        }

        let val = director.working_solution().tasks[0].priority;
        assert_eq!(val, Some(1));
    }

    #[test]
    fn change_same_value_not_doable() {
        let tasks = vec![Task {
            id: 0,
            priority: Some(1),
        }];
        let director = create_director(tasks);

        // Same value - not doable
        let m = ChangeMove::<TaskSolution>::new(0, 1, "priority", 0);
        assert!(!m.is_doable(&director));
    }

    #[test]
    fn change_unassigned_entity() {
        let tasks = vec![Task {
            id: 0,
            priority: None,
        }];
        let mut director = create_director(tasks);

        // Assign value to unassigned entity
        let m = ChangeMove::<TaskSolution>::new(0, 3, "priority", 0);
        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            let val = recording.working_solution().tasks[0].priority;
            assert_eq!(val, Some(3));

            recording.undo_changes();
        }

        let val = director.working_solution().tasks[0].priority;
        assert_eq!(val, None);
    }

    #[test]
    fn entity_indices() {
        let m = ChangeMove::<TaskSolution>::new(3, 5, "priority", 0);
        assert_eq!(m.entity_indices(), &[3]);
    }
}
