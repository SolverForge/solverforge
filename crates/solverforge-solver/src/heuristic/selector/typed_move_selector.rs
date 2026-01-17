//! Typed move selectors for zero-allocation move generation.
//!
//! Typed move selectors yield concrete move types directly, enabling
//! monomorphization and arena allocation.
//!
//! # Zero-Erasure Design
//!
//! No value type parameter. Uses VariableOperations trait for value access.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::change::ChangeMove;
use crate::heuristic::r#move::swap::SwapMove;
use crate::heuristic::r#move::traits::Move;
use crate::operations::VariableOperations;

use super::entity::{EntitySelector, FromSolutionEntitySelector};

/// A typed move selector that yields moves of type `M` directly.
///
/// Unlike erased selectors, this returns concrete moves inline,
/// eliminating heap allocation per move.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M` - The move type
pub trait MoveSelector<S: PlanningSolution, M: Move<S>>: Send + Debug {
    /// Returns an iterator over typed moves.
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = M> + 'a>;

    /// Returns the approximate number of moves.
    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize;

    /// Returns true if this selector may return the same move multiple times.
    fn is_never_ending(&self) -> bool {
        false
    }
}

/// A change move selector that generates `ChangeMove` instances.
///
/// Uses `VariableOperations` trait for zero-erasure value access.
/// No function pointers required.
///
/// # Type Parameters
/// * `S` - The solution type (must implement VariableOperations)
/// * `ES` - The entity selector type
pub struct ChangeMoveSelector<S, ES> {
    entity_selector: ES,
    descriptor_index: usize,
    variable_name: &'static str,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, ES: Debug> Debug for ChangeMoveSelector<S, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChangeMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, ES> ChangeMoveSelector<S, ES> {
    /// Creates a new change move selector.
    ///
    /// # Arguments
    /// * `entity_selector` - Selects entities to modify
    /// * `variable_name` - Name of the variable
    /// * `descriptor_index` - Index of the entity descriptor
    pub fn new(entity_selector: ES, variable_name: &'static str, descriptor_index: usize) -> Self {
        Self {
            entity_selector,
            descriptor_index,
            variable_name,
            _phantom: PhantomData,
        }
    }
}

impl<S: PlanningSolution> ChangeMoveSelector<S, FromSolutionEntitySelector> {
    /// Creates a simple selector using the default entity selector.
    pub fn simple(variable_name: &'static str, descriptor_index: usize) -> Self {
        Self {
            entity_selector: FromSolutionEntitySelector::new(descriptor_index),
            descriptor_index,
            variable_name,
            _phantom: PhantomData,
        }
    }
}

impl<S, ES> MoveSelector<S, ChangeMove<S>> for ChangeMoveSelector<S, ES>
where
    S: PlanningSolution + VariableOperations,
    ES: EntitySelector<S>,
{
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = ChangeMove<S>> + 'a> {
        let descriptor_index = self.descriptor_index;
        let variable_name = self.variable_name;

        // Get value range from solution
        let solution = score_director.working_solution();
        let values: Vec<_> = solution.value_range();

        // Lazy iteration over all entity-value pairs
        let iter = self
            .entity_selector
            .iter(score_director)
            .flat_map(move |entity_ref| {
                values.clone().into_iter().map(move |value| {
                    ChangeMove::new(entity_ref.entity_index, value, variable_name, descriptor_index)
                })
            });

        Box::new(iter)
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        let entity_count = self.entity_selector.size(score_director);
        if entity_count == 0 {
            return 0;
        }

        let solution = score_director.working_solution();
        let value_count = solution.value_range().len();
        entity_count * value_count
    }
}

/// A swap move selector that generates `SwapMove` instances.
///
/// Uses `VariableOperations` trait for zero-erasure access.
/// No function pointers required.
///
/// # Type Parameters
/// * `S` - The solution type (must implement VariableOperations)
/// * `LES` - The left entity selector type
/// * `RES` - The right entity selector type
pub struct SwapMoveSelector<S, LES, RES> {
    left_entity_selector: LES,
    right_entity_selector: RES,
    descriptor_index: usize,
    variable_name: &'static str,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, LES: Debug, RES: Debug> Debug for SwapMoveSelector<S, LES, RES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SwapMoveSelector")
            .field("left_entity_selector", &self.left_entity_selector)
            .field("right_entity_selector", &self.right_entity_selector)
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, LES, RES> SwapMoveSelector<S, LES, RES> {
    /// Creates a new swap move selector.
    ///
    /// # Arguments
    /// * `left_entity_selector` - Selects left entities
    /// * `right_entity_selector` - Selects right entities
    /// * `variable_name` - Name of the variable to swap
    /// * `descriptor_index` - Index in the entity descriptor
    pub fn new(
        left_entity_selector: LES,
        right_entity_selector: RES,
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            left_entity_selector,
            right_entity_selector,
            descriptor_index,
            variable_name,
            _phantom: PhantomData,
        }
    }
}

impl<S: PlanningSolution> SwapMoveSelector<S, FromSolutionEntitySelector, FromSolutionEntitySelector>
{
    /// Creates a simple selector for swapping within a single entity type.
    ///
    /// # Arguments
    /// * `variable_name` - Name of the variable to swap
    /// * `descriptor_index` - Index in the entity descriptor
    pub fn simple(variable_name: &'static str, descriptor_index: usize) -> Self {
        Self {
            left_entity_selector: FromSolutionEntitySelector::new(descriptor_index),
            right_entity_selector: FromSolutionEntitySelector::new(descriptor_index),
            descriptor_index,
            variable_name,
            _phantom: PhantomData,
        }
    }
}

impl<S, LES, RES> MoveSelector<S, SwapMove<S>> for SwapMoveSelector<S, LES, RES>
where
    S: PlanningSolution + VariableOperations,
    LES: EntitySelector<S>,
    RES: EntitySelector<S>,
{
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = SwapMove<S>> + 'a> {
        let descriptor_index = self.descriptor_index;
        let variable_name = self.variable_name;

        // Collect entities - needed for triangular pairing with index tracking
        let left_entities: Vec<_> = self
            .left_entity_selector
            .iter(score_director)
            .map(|r| r.entity_index)
            .collect();
        let right_entities: Vec<_> = self
            .right_entity_selector
            .iter(score_director)
            .map(|r| r.entity_index)
            .collect();

        // Lazy triangular iteration over pairs (only upper triangle to avoid duplicates)
        let iter = left_entities
            .into_iter()
            .enumerate()
            .flat_map(move |(i, left)| {
                let right_slice = right_entities.clone();
                right_slice
                    .into_iter()
                    .skip(i + 1)
                    .map(move |right| SwapMove::new(left, right, variable_name, descriptor_index))
            });

        Box::new(iter)
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        let left_count = self.left_entity_selector.size(score_director);
        let right_count = self.right_entity_selector.size(score_director);

        if left_count == right_count {
            left_count * left_count.saturating_sub(1) / 2
        } else {
            left_count * right_count / 2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heuristic::r#move::Move;
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
        priorities: Vec<usize>,
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

    fn get_tasks(s: &TaskSolution) -> &Vec<Task> {
        &s.tasks
    }

    fn get_tasks_mut(s: &mut TaskSolution) -> &mut Vec<Task> {
        &mut s.tasks
    }

    fn create_director(
        tasks: Vec<Task>,
        priorities: Vec<usize>,
    ) -> SimpleScoreDirector<TaskSolution, impl Fn(&TaskSolution) -> SimpleScore> {
        let solution = TaskSolution {
            tasks,
            priorities,
            score: None,
        };

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
    fn test_change_move_selector() {
        let director = create_director(
            vec![
                Task {
                    id: 0,
                    priority: Some(1),
                },
                Task {
                    id: 1,
                    priority: Some(2),
                },
                Task {
                    id: 2,
                    priority: Some(3),
                },
            ],
            vec![10, 20, 30],
        );

        let selector = ChangeMoveSelector::<TaskSolution, _>::simple("priority", 0);

        let moves: Vec<_> = selector.iter_moves(&director).collect();

        // 3 entities * 3 values = 9 moves
        assert_eq!(moves.len(), 9);
        assert_eq!(selector.size(&director), 9);

        // Verify first move structure
        let first = &moves[0];
        assert_eq!(first.entity_index(), 0);
        assert_eq!(first.to_value(), 10);
    }

    #[test]
    fn test_swap_move_selector() {
        let director = create_director(
            vec![
                Task {
                    id: 0,
                    priority: Some(1),
                },
                Task {
                    id: 1,
                    priority: Some(2),
                },
                Task {
                    id: 2,
                    priority: Some(3),
                },
                Task {
                    id: 3,
                    priority: Some(4),
                },
            ],
            vec![1, 2, 3, 4],
        );

        let selector = SwapMoveSelector::<TaskSolution, _, _>::simple("priority", 0);

        let moves: Vec<_> = selector.iter_moves(&director).collect();

        // 4 entities: 4*3/2 = 6 pairs
        assert_eq!(moves.len(), 6);
        assert_eq!(selector.size(&director), 6);

        // Verify first swap
        let first = &moves[0];
        assert_eq!(first.left_entity_index(), 0);
        assert_eq!(first.right_entity_index(), 1);
    }

    #[test]
    fn test_change_do_and_undo() {
        let mut director = create_director(
            vec![Task {
                id: 0,
                priority: Some(1),
            }],
            vec![99],
        );

        let selector = ChangeMoveSelector::<TaskSolution, _>::simple("priority", 0);

        let moves: Vec<_> = selector.iter_moves(&director).collect();
        assert_eq!(moves.len(), 1);

        let m = &moves[0];
        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            // Verify change
            let val = recording.working_solution().tasks[0].priority;
            assert_eq!(val, Some(99));

            // Undo
            recording.undo_changes();
        }

        // Verify restored
        let val = director.working_solution().tasks[0].priority;
        assert_eq!(val, Some(1));
    }

    #[test]
    fn test_swap_do_and_undo() {
        let mut director = create_director(
            vec![
                Task {
                    id: 0,
                    priority: Some(10),
                },
                Task {
                    id: 1,
                    priority: Some(20),
                },
            ],
            vec![10, 20],
        );

        let selector = SwapMoveSelector::<TaskSolution, _, _>::simple("priority", 0);

        let moves: Vec<_> = selector.iter_moves(&director).collect();
        assert_eq!(moves.len(), 1);

        let m = &moves[0];
        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            // Verify swap
            let val0 = recording.working_solution().tasks[0].priority;
            let val1 = recording.working_solution().tasks[1].priority;
            assert_eq!(val0, Some(20));
            assert_eq!(val1, Some(10));

            // Undo
            recording.undo_changes();
        }

        // Verify restored
        let val0 = director.working_solution().tasks[0].priority;
        let val1 = director.working_solution().tasks[1].priority;
        assert_eq!(val0, Some(10));
        assert_eq!(val1, Some(20));
    }
}
