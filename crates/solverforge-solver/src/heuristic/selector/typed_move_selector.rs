//! Typed move selectors for zero-allocation move generation.
//!
//! Typed move selectors yield concrete move types directly, enabling
//! monomorphization and arena allocation.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::{ChangeMove, Move, SwapMove};

use super::entity::{EntityReference, EntitySelector, FromSolutionEntitySelector};
use super::typed_value::{StaticTypedValueSelector, TypedValueSelector};

/// A typed move selector that yields moves of type `M` directly.
///
/// Unlike erased selectors, this returns concrete moves inline,
/// eliminating heap allocation per move.
pub trait MoveSelector<S: PlanningSolution, M: Move<S>>: Send + Debug {
    /// Returns an iterator over typed moves.
    fn iter_moves<'a>(
        &'a self,
        score_director: &'a dyn ScoreDirector<S>,
    ) -> Box<dyn Iterator<Item = M> + 'a>;

    /// Returns the approximate number of moves.
    fn size(&self, score_director: &dyn ScoreDirector<S>) -> usize;

    /// Returns true if this selector may return the same move multiple times.
    fn is_never_ending(&self) -> bool {
        false
    }
}

/// A change move selector that generates `ChangeMove` instances.
///
/// Stores typed function pointers for zero-erasure move generation.
pub struct ChangeMoveSelector<S, V> {
    entity_selector: Box<dyn EntitySelector<S>>,
    value_selector: Box<dyn TypedValueSelector<S, V>>,
    getter: fn(&S, usize) -> Option<V>,
    setter: fn(&mut S, usize, Option<V>),
    descriptor_index: usize,
    variable_name: &'static str,
    _phantom: PhantomData<S>,
}

impl<S, V: Debug> Debug for ChangeMoveSelector<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChangeMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("value_selector", &self.value_selector)
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S: PlanningSolution, V: Clone> ChangeMoveSelector<S, V> {
    /// Creates a new change move selector with typed function pointers.
    ///
    /// # Arguments
    /// * `entity_selector` - Selects entities to modify
    /// * `value_selector` - Selects values to assign
    /// * `getter` - Function pointer to get current value from solution
    /// * `setter` - Function pointer to set value on solution
    /// * `descriptor_index` - Index of the entity descriptor
    /// * `variable_name` - Name of the variable
    pub fn new(
        entity_selector: Box<dyn EntitySelector<S>>,
        value_selector: Box<dyn TypedValueSelector<S, V>>,
        getter: fn(&S, usize) -> Option<V>,
        setter: fn(&mut S, usize, Option<V>),
        descriptor_index: usize,
        variable_name: &'static str,
    ) -> Self {
        Self {
            entity_selector,
            value_selector,
            getter,
            setter,
            descriptor_index,
            variable_name,
            _phantom: PhantomData,
        }
    }

    /// Creates a simple selector with static values.
    pub fn simple(
        getter: fn(&S, usize) -> Option<V>,
        setter: fn(&mut S, usize, Option<V>),
        descriptor_index: usize,
        variable_name: &'static str,
        values: Vec<V>,
    ) -> Self
    where
        V: Send + Sync + Debug + 'static,
    {
        Self {
            entity_selector: Box::new(FromSolutionEntitySelector::new(descriptor_index)),
            value_selector: Box::new(StaticTypedValueSelector::new(values)),
            getter,
            setter,
            descriptor_index,
            variable_name,
            _phantom: PhantomData,
        }
    }
}

impl<S, V> MoveSelector<S, ChangeMove<S, V>> for ChangeMoveSelector<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn iter_moves<'a>(
        &'a self,
        score_director: &'a dyn ScoreDirector<S>,
    ) -> Box<dyn Iterator<Item = ChangeMove<S, V>> + 'a> {
        let descriptor_index = self.descriptor_index;
        let variable_name = self.variable_name;
        let getter = self.getter;
        let setter = self.setter;
        let value_selector = &self.value_selector;

        // Lazy iteration: O(1) per .next() call, no upfront allocation
        let iter = self
            .entity_selector
            .iter(score_director)
            .flat_map(move |entity_ref| {
                value_selector
                    .iter_typed(
                        score_director,
                        entity_ref.descriptor_index,
                        entity_ref.entity_index,
                    )
                    .map(move |value| {
                        ChangeMove::new(
                            entity_ref.entity_index,
                            Some(value),
                            getter,
                            setter,
                            variable_name,
                            descriptor_index,
                        )
                    })
            });

        Box::new(iter)
    }

    fn size(&self, score_director: &dyn ScoreDirector<S>) -> usize {
        let entity_count = self.entity_selector.size(score_director);
        if entity_count == 0 {
            return 0;
        }

        if let Some(entity_ref) = self.entity_selector.iter(score_director).next() {
            let value_count = self.value_selector.size(
                score_director,
                entity_ref.descriptor_index,
                entity_ref.entity_index,
            );
            entity_count * value_count
        } else {
            0
        }
    }
}

/// A swap move selector that generates `SwapMove` instances.
///
/// Uses typed function pointers for zero-erasure access to variable values.
pub struct SwapMoveSelector<S, V> {
    left_entity_selector: Box<dyn EntitySelector<S>>,
    right_entity_selector: Box<dyn EntitySelector<S>>,
    /// Typed getter function pointer - zero erasure.
    getter: fn(&S, usize) -> Option<V>,
    /// Typed setter function pointer - zero erasure.
    setter: fn(&mut S, usize, Option<V>),
    descriptor_index: usize,
    variable_name: &'static str,
    _phantom: PhantomData<S>,
}

impl<S, V> Debug for SwapMoveSelector<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SwapMoveSelector")
            .field("left_entity_selector", &self.left_entity_selector)
            .field("right_entity_selector", &self.right_entity_selector)
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S: PlanningSolution, V> SwapMoveSelector<S, V> {
    /// Creates a new swap move selector with typed function pointers.
    pub fn new(
        left_entity_selector: Box<dyn EntitySelector<S>>,
        right_entity_selector: Box<dyn EntitySelector<S>>,
        getter: fn(&S, usize) -> Option<V>,
        setter: fn(&mut S, usize, Option<V>),
        descriptor_index: usize,
        variable_name: &'static str,
    ) -> Self {
        Self {
            left_entity_selector,
            right_entity_selector,
            getter,
            setter,
            descriptor_index,
            variable_name,
            _phantom: PhantomData,
        }
    }

    /// Creates a simple selector for swapping within a single entity type.
    ///
    /// # Arguments
    /// * `getter` - Typed getter function pointer
    /// * `setter` - Typed setter function pointer
    /// * `descriptor_index` - Index in the entity descriptor
    /// * `variable_name` - Name of the variable to swap
    pub fn simple(
        getter: fn(&S, usize) -> Option<V>,
        setter: fn(&mut S, usize, Option<V>),
        descriptor_index: usize,
        variable_name: &'static str,
    ) -> Self {
        Self {
            left_entity_selector: Box::new(FromSolutionEntitySelector::new(descriptor_index)),
            right_entity_selector: Box::new(FromSolutionEntitySelector::new(descriptor_index)),
            getter,
            setter,
            descriptor_index,
            variable_name,
            _phantom: PhantomData,
        }
    }
}

impl<S, V> MoveSelector<S, SwapMove<S, V>> for SwapMoveSelector<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn iter_moves<'a>(
        &'a self,
        score_director: &'a dyn ScoreDirector<S>,
    ) -> Box<dyn Iterator<Item = SwapMove<S, V>> + 'a> {
        let descriptor_index = self.descriptor_index;
        let variable_name = self.variable_name;
        let getter = self.getter;
        let setter = self.setter;

        // Collect entities - needed for triangular pairing with index tracking
        // (lazy would require re-iterating right_entities for each left, which is worse)
        let left_entities: Vec<EntityReference> =
            self.left_entity_selector.iter(score_director).collect();
        let right_entities: Vec<EntityReference> =
            self.right_entity_selector.iter(score_director).collect();

        // Lazy triangular iteration over pairs
        let iter = left_entities
            .into_iter()
            .enumerate()
            .flat_map(move |(i, left)| {
                let right_slice = right_entities.clone();
                right_slice
                    .into_iter()
                    .skip(i + 1)
                    .filter(move |right| {
                        left.descriptor_index == right.descriptor_index
                            && left.descriptor_index == descriptor_index
                    })
                    .map(move |right| {
                        SwapMove::new(
                            left.entity_index,
                            right.entity_index,
                            getter,
                            setter,
                            variable_name,
                            descriptor_index,
                        )
                    })
            });

        Box::new(iter)
    }

    fn size(&self, score_director: &dyn ScoreDirector<S>) -> usize {
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
    fn test_change_move_selector() {
        let director = create_director(vec![
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
        ]);

        // Verify entity IDs
        let solution = director.working_solution();
        assert_eq!(solution.tasks[0].id, 0);
        assert_eq!(solution.tasks[1].id, 1);
        assert_eq!(solution.tasks[2].id, 2);

        let selector = ChangeMoveSelector::<TaskSolution, i32>::simple(
            get_priority,
            set_priority,
            0,
            "priority",
            vec![10, 20, 30],
        );

        let moves: Vec<_> = selector.iter_moves(&director).collect();

        // 3 entities * 3 values = 9 moves
        assert_eq!(moves.len(), 9);
        assert_eq!(selector.size(&director), 9);

        // Verify first move structure
        let first = &moves[0];
        assert_eq!(first.entity_index(), 0);
        assert_eq!(first.to_value(), Some(&10));
    }

    #[test]
    fn test_swap_move_selector() {
        let director = create_director(vec![
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
        ]);

        let selector = SwapMoveSelector::<TaskSolution, i32>::simple(
            get_priority,
            set_priority,
            0,
            "priority",
        );

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
        let mut director = create_director(vec![Task {
            id: 0,
            priority: Some(1),
        }]);

        let selector = ChangeMoveSelector::<TaskSolution, i32>::simple(
            get_priority,
            set_priority,
            0,
            "priority",
            vec![99],
        );

        let moves: Vec<_> = selector.iter_moves(&director).collect();
        assert_eq!(moves.len(), 1);

        let m = &moves[0];
        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            // Verify change using typed getter - zero erasure
            let val = get_priority(recording.working_solution(), 0);
            assert_eq!(val, Some(99));

            // Undo
            recording.undo_changes();
        }

        // Verify restored using typed getter
        let val = get_priority(director.working_solution(), 0);
        assert_eq!(val, Some(1));
    }

    #[test]
    fn test_swap_do_and_undo() {
        let mut director = create_director(vec![
            Task {
                id: 0,
                priority: Some(10),
            },
            Task {
                id: 1,
                priority: Some(20),
            },
        ]);

        let selector = SwapMoveSelector::<TaskSolution, i32>::simple(
            get_priority,
            set_priority,
            0,
            "priority",
        );

        let moves: Vec<_> = selector.iter_moves(&director).collect();
        assert_eq!(moves.len(), 1);

        let m = &moves[0];
        assert!(m.is_doable(&director));

        {
            let mut recording = RecordingScoreDirector::new(&mut director);
            m.do_move(&mut recording);

            // Verify swap using typed getter
            let val0 = get_priority(recording.working_solution(), 0);
            let val1 = get_priority(recording.working_solution(), 1);
            assert_eq!(val0, Some(20));
            assert_eq!(val1, Some(10));

            // Undo
            recording.undo_changes();
        }

        // Verify restored using typed getter
        let val0 = get_priority(director.working_solution(), 0);
        let val1 = get_priority(director.working_solution(), 1);
        assert_eq!(val0, Some(10));
        assert_eq!(val1, Some(20));
    }
}
