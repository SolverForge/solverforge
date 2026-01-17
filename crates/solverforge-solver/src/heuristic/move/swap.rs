//! SwapMove - exchanges values between two entities.
//!
//! This move swaps the values of a planning variable between two entities.
//! Useful for permutation-based problems.
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

/// A move that swaps values between two entities.
///
/// Stores entity indices and uses `VariableOperations` for zero-erasure access.
///
/// # Type Parameters
/// * `S` - The planning solution type (must implement VariableOperations)
#[derive(Clone, Copy)]
pub struct SwapMove<S> {
    left_entity_index: usize,
    right_entity_index: usize,
    variable_name: &'static str,
    descriptor_index: usize,
    /// Store indices inline for entity_indices() to return a slice.
    indices: [usize; 2],
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for SwapMove<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SwapMove")
            .field("left_entity_index", &self.left_entity_index)
            .field("right_entity_index", &self.right_entity_index)
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S> SwapMove<S> {
    /// Creates a new swap move.
    ///
    /// # Arguments
    /// * `left_entity_index` - Index of the first entity
    /// * `right_entity_index` - Index of the second entity
    /// * `variable_name` - Name of the variable being swapped
    /// * `descriptor_index` - Index in the entity descriptor
    pub fn new(
        left_entity_index: usize,
        right_entity_index: usize,
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            left_entity_index,
            right_entity_index,
            variable_name,
            descriptor_index,
            indices: [left_entity_index, right_entity_index],
            _phantom: PhantomData,
        }
    }

    /// Returns the left entity index.
    pub fn left_entity_index(&self) -> usize {
        self.left_entity_index
    }

    /// Returns the right entity index.
    pub fn right_entity_index(&self) -> usize {
        self.right_entity_index
    }
}

impl<S> Move<S> for SwapMove<S>
where
    S: PlanningSolution + VariableOperations,
{
    fn is_doable<D: ScoreDirector<S>>(&self, score_director: &D) -> bool {
        // Can't swap with self
        if self.left_entity_index == self.right_entity_index {
            return false;
        }

        let solution = score_director.working_solution();

        // Both must be assigned
        let left_len = solution.list_len(self.left_entity_index);
        let right_len = solution.list_len(self.right_entity_index);

        if left_len == 0 || right_len == 0 {
            return false;
        }

        // Swap only makes sense if values differ
        let left_val = solution.get(self.left_entity_index, 0);
        let right_val = solution.get(self.right_entity_index, 0);

        left_val != right_val
    }

    fn do_move<D: ScoreDirector<S>>(&self, score_director: &mut D) {
        let solution = score_director.working_solution();

        // Get both values
        let left_value = solution.get(self.left_entity_index, 0);
        let right_value = solution.get(self.right_entity_index, 0);

        // Notify before changes
        score_director.before_variable_changed(
            self.descriptor_index,
            self.left_entity_index,
            self.variable_name,
        );
        score_director.before_variable_changed(
            self.descriptor_index,
            self.right_entity_index,
            self.variable_name,
        );

        // Swap: left gets right's value, right gets left's value
        {
            let sol = score_director.working_solution_mut();
            sol.remove(self.left_entity_index, 0);
            sol.insert(self.left_entity_index, 0, right_value);
            sol.remove(self.right_entity_index, 0);
            sol.insert(self.right_entity_index, 0, left_value);
        }

        // Notify after changes
        score_director.after_variable_changed(
            self.descriptor_index,
            self.left_entity_index,
            self.variable_name,
        );
        score_director.after_variable_changed(
            self.descriptor_index,
            self.right_entity_index,
            self.variable_name,
        );

        // Register undo - swap back
        let left_idx = self.left_entity_index;
        let right_idx = self.right_entity_index;
        score_director.register_undo(Box::new(move |s: &mut S| {
            // Restore original values
            s.remove(left_idx, 0);
            s.insert(left_idx, 0, left_value);
            s.remove(right_idx, 0);
            s.insert(right_idx, 0, right_value);
        }));
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        &self.indices
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
    struct Task {
        id: usize,
        priority: Option<usize>,
    }

    #[derive(Clone, Debug)]
    struct TaskSolution {
        tasks: Vec<Task>,
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
            5 // Assume 5 priority values
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

    fn get_tasks(s: &TaskSolution) -> &Vec<Task> {
        &s.tasks
    }

    fn get_tasks_mut(s: &mut TaskSolution) -> &mut Vec<Task> {
        &mut s.tasks
    }

    fn create_director(
        tasks: Vec<Task>,
    ) -> SimpleScoreDirector<TaskSolution, impl Fn(&TaskSolution) -> SimpleScore> {
        let solution = TaskSolution { tasks, score: None };

        let extractor = Box::new(TypedEntityExtractor::new(
            "Task",
            "tasks",
            get_tasks,
            get_tasks_mut,
        ));
        let entity_desc =
            EntityDescriptor::new("Task", TypeId::of::<Task>(), "tasks").with_extractor(extractor);

        let descriptor = SolutionDescriptor::new("TaskSolution", TypeId::of::<TaskSolution>())
            .with_entity(entity_desc);

        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn swap_move_do_and_undo() {
        let tasks = vec![
            Task {
                id: 0,
                priority: Some(1),
            },
            Task {
                id: 1,
                priority: Some(5),
            },
        ];
        let mut director = create_director(tasks);

        let m = SwapMove::<TaskSolution>::new(0, 1, "priority", 0);
        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            // Verify swap
            assert_eq!(recording.working_solution().tasks[0].priority, Some(5));
            assert_eq!(recording.working_solution().tasks[1].priority, Some(1));

            // Undo via recording
            recording.undo_changes();
        }

        // Verify restored
        assert_eq!(director.working_solution().tasks[0].priority, Some(1));
        assert_eq!(director.working_solution().tasks[1].priority, Some(5));

        // Verify entity identity preserved
        let solution = director.working_solution();
        assert_eq!(solution.tasks[0].id, 0);
        assert_eq!(solution.tasks[1].id, 1);
    }

    #[test]
    fn swap_same_value_not_doable() {
        let tasks = vec![
            Task {
                id: 0,
                priority: Some(5),
            },
            Task {
                id: 1,
                priority: Some(5),
            },
        ];
        let director = create_director(tasks);

        let m = SwapMove::<TaskSolution>::new(0, 1, "priority", 0);
        assert!(
            !m.is_doable(&director),
            "swapping same values should not be doable"
        );
    }

    #[test]
    fn swap_self_not_doable() {
        let tasks = vec![Task {
            id: 0,
            priority: Some(1),
        }];
        let director = create_director(tasks);

        let m = SwapMove::<TaskSolution>::new(0, 0, "priority", 0);
        assert!(!m.is_doable(&director), "self-swap should not be doable");
    }

    #[test]
    fn swap_entity_indices() {
        let m = SwapMove::<TaskSolution>::new(2, 5, "priority", 0);
        assert_eq!(m.entity_indices(), &[2, 5]);
    }
}
