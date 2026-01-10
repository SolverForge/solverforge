//! SwapMove - exchanges values between two entities.
//!
//! This move swaps the values of a planning variable between two entities.
//! Useful for permutation-based problems.
//!
//! # Zero-Erasure Design
//!
//! SwapMove uses typed function pointers instead of `dyn Any` for complete
//! compile-time type safety. No runtime type checks or downcasting.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::Move;

/// A move that swaps values between two entities.
///
/// Stores entity indices and typed function pointers for zero-erasure access.
/// Undo is handled by `RecordingScoreDirector`, not by this move.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `D` - The score director type
/// * `V` - The variable value type
///
/// # Example
/// ```
/// use solverforge_solver::heuristic::r#move::SwapMove;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone)]
/// struct Sol { values: Vec<Option<i32>>, score: Option<SimpleScore> }
///
/// impl PlanningSolution for Sol {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// // Typed getter/setter with zero erasure
/// fn get_v(s: &Sol, idx: usize) -> Option<i32> { s.values.get(idx).copied().flatten() }
/// fn set_v(s: &mut Sol, idx: usize, v: Option<i32>) { if let Some(x) = s.values.get_mut(idx) { *x = v; } }
///
/// // Swap values between entities 0 and 1
/// let swap = SwapMove::<Sol, _, i32>::new(0, 1, get_v, set_v, "value", 0);
/// ```
pub struct SwapMove<S, D, V> {
    left_entity_index: usize,
    right_entity_index: usize,
    /// Typed getter function pointer - zero erasure.
    getter: fn(&S, usize) -> Option<V>,
    /// Typed setter function pointer - zero erasure.
    setter: fn(&mut S, usize, Option<V>),
    variable_name: &'static str,
    descriptor_index: usize,
    /// Store indices inline for entity_indices() to return a slice.
    indices: [usize; 2],
    _phantom: PhantomData<(fn() -> D, V)>,
}

// Manual Clone impl to avoid D: Clone bound from derive
impl<S, D, V> Clone for SwapMove<S, D, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, D, V> Copy for SwapMove<S, D, V> {}

impl<S, D, V: Debug> Debug for SwapMove<S, D, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SwapMove")
            .field("left_entity_index", &self.left_entity_index)
            .field("right_entity_index", &self.right_entity_index)
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, D, V> SwapMove<S, D, V> {
    /// Creates a new swap move with typed function pointers.
    ///
    /// # Arguments
    /// * `left_entity_index` - Index of the first entity
    /// * `right_entity_index` - Index of the second entity
    /// * `getter` - Typed getter function pointer
    /// * `setter` - Typed setter function pointer
    /// * `variable_name` - Name of the variable being swapped
    /// * `descriptor_index` - Index in the entity descriptor
    pub fn new(
        left_entity_index: usize,
        right_entity_index: usize,
        getter: fn(&S, usize) -> Option<V>,
        setter: fn(&mut S, usize, Option<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            left_entity_index,
            right_entity_index,
            getter,
            setter,
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

impl<S, D, V> Move<S, D> for SwapMove<S, D, V>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn is_doable(&self, score_director: &D) -> bool {
        // Can't swap with self
        if self.left_entity_index == self.right_entity_index {
            return false;
        }

        // Get current values using typed getter - zero erasure
        let left_val = (self.getter)(score_director.working_solution(), self.left_entity_index);
        let right_val = (self.getter)(score_director.working_solution(), self.right_entity_index);

        // Swap only makes sense if values differ
        left_val != right_val
    }

    fn do_move(&self, score_director: &mut D) {
        // Get both values using typed getter - zero erasure
        let left_value = (self.getter)(score_director.working_solution(), self.left_entity_index);
        let right_value = (self.getter)(score_director.working_solution(), self.right_entity_index);

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
        (self.setter)(
            score_director.working_solution_mut(),
            self.left_entity_index,
            right_value.clone(),
        );
        (self.setter)(
            score_director.working_solution_mut(),
            self.right_entity_index,
            left_value.clone(),
        );

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

        // Register typed undo closure - swap back
        let setter = self.setter;
        let left_idx = self.left_entity_index;
        let right_idx = self.right_entity_index;
        score_director.register_undo(Box::new(move |s: &mut S| {
            // Restore original values
            setter(s, left_idx, left_value);
            setter(s, right_idx, right_value);
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
    use solverforge_core::domain::{EntityDescriptor, SolutionDescriptor, TypedEntityExtractor};
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::{RecordingScoreDirector, SimpleScoreDirector};
    use std::any::TypeId;

    #[derive(Clone, Debug)]
    struct Task {
        id: usize,
        priority: Option<i32>,
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

    fn get_tasks(s: &TaskSolution) -> &Vec<Task> {
        &s.tasks
    }

    fn get_tasks_mut(s: &mut TaskSolution) -> &mut Vec<Task> {
        &mut s.tasks
    }

    // Typed getter - zero erasure
    fn get_priority(s: &TaskSolution, idx: usize) -> Option<i32> {
        s.tasks.get(idx).and_then(|t| t.priority)
    }

    // Typed setter - zero erasure
    fn set_priority(s: &mut TaskSolution, idx: usize, v: Option<i32>) {
        if let Some(task) = s.tasks.get_mut(idx) {
            task.priority = v;
        }
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
    fn test_swap_move_do_and_undo() {
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

        let m = SwapMove::<TaskSolution, _, i32>::new(0, 1, get_priority, set_priority, "priority", 0);
        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            // Verify swap using typed getter
            assert_eq!(get_priority(recording.working_solution(), 0), Some(5));
            assert_eq!(get_priority(recording.working_solution(), 1), Some(1));

            // Undo via recording
            recording.undo_changes();
        }

        // Verify restored using typed getter
        assert_eq!(get_priority(director.working_solution(), 0), Some(1));
        assert_eq!(get_priority(director.working_solution(), 1), Some(5));

        // Verify entity identity preserved
        let solution = director.working_solution();
        assert_eq!(solution.tasks[0].id, 0);
        assert_eq!(solution.tasks[1].id, 1);
    }

    #[test]
    fn test_swap_same_value_not_doable() {
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

        let m = SwapMove::<TaskSolution, _, i32>::new(0, 1, get_priority, set_priority, "priority", 0);
        assert!(
            !m.is_doable(&director),
            "swapping same values should not be doable"
        );
    }

    #[test]
    fn test_swap_self_not_doable() {
        let tasks = vec![Task {
            id: 0,
            priority: Some(1),
        }];
        let director = create_director(tasks);

        let m = SwapMove::<TaskSolution, _, i32>::new(0, 0, get_priority, set_priority, "priority", 0);
        assert!(!m.is_doable(&director), "self-swap should not be doable");
    }

    #[test]
    fn test_swap_entity_indices() {
        let m = SwapMove::<TaskSolution, _, i32>::new(2, 5, get_priority, set_priority, "priority", 0);
        assert_eq!(m.entity_indices(), &[2, 5]);
    }
}
