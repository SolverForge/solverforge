//! Entity placers for construction heuristic
//!
//! Placers enumerate the entities that need values assigned and
//! generate candidate moves for each entity.
//!
//! # Zero-Erasure Design
//!
//! No value type parameter. Uses VariableOperations trait for value access.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::change::ChangeMove;
use crate::heuristic::r#move::traits::Move;
use crate::heuristic::selector::entity::{EntityReference, EntitySelector};
use crate::operations::VariableOperations;

/// A placement represents an entity that needs a value assigned,
/// along with the candidate moves to assign values.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M` - The move type
pub struct Placement<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    /// The entity reference.
    pub entity_ref: EntityReference,
    /// Candidate moves for this placement.
    pub moves: Vec<M>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, M> Placement<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    /// Creates a new placement.
    pub fn new(entity_ref: EntityReference, moves: Vec<M>) -> Self {
        Self {
            entity_ref,
            moves,
            _phantom: PhantomData,
        }
    }

    /// Returns true if there are no candidate moves.
    pub fn is_empty(&self) -> bool {
        self.moves.is_empty()
    }

    /// Returns the number of candidate moves.
    pub fn move_count(&self) -> usize {
        self.moves.len()
    }

    /// Takes ownership of a move at the given index.
    ///
    /// Uses swap_remove for O(1) removal.
    pub fn take_move(&mut self, index: usize) -> M {
        self.moves.swap_remove(index)
    }
}

impl<S, M> Debug for Placement<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Placement")
            .field("entity_ref", &self.entity_ref)
            .field("move_count", &self.moves.len())
            .finish()
    }
}

/// Trait for placing entities during construction.
///
/// Entity placers iterate over uninitialized entities and generate
/// candidate moves for each.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M` - The move type
pub trait EntityPlacer<S, M>: Send + Debug
where
    S: PlanningSolution,
    M: Move<S>,
{
    /// Returns all placements (entities + their candidate moves).
    fn get_placements<D: ScoreDirector<S>>(&self, score_director: &D) -> Vec<Placement<S, M>>;
}

/// A queued entity placer that processes entities in order.
///
/// For each uninitialized entity, generates change moves for all possible values.
/// Uses VariableOperations trait for zero-erasure access.
///
/// # Type Parameters
/// * `S` - The planning solution type (must implement VariableOperations)
/// * `ES` - The entity selector type
pub struct QueuedEntityPlacer<S, ES>
where
    S: PlanningSolution,
    ES: EntitySelector<S>,
{
    /// The entity selector.
    entity_selector: ES,
    /// The variable name.
    variable_name: &'static str,
    /// The descriptor index.
    descriptor_index: usize,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, ES> QueuedEntityPlacer<S, ES>
where
    S: PlanningSolution,
    ES: EntitySelector<S>,
{
    /// Creates a new queued entity placer.
    ///
    /// # Arguments
    /// * `entity_selector` - Selects entities to place
    /// * `variable_name` - Name of the variable
    /// * `descriptor_index` - Index of the entity descriptor
    pub fn new(entity_selector: ES, variable_name: &'static str, descriptor_index: usize) -> Self {
        Self {
            entity_selector,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, ES> Debug for QueuedEntityPlacer<S, ES>
where
    S: PlanningSolution,
    ES: EntitySelector<S> + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueuedEntityPlacer")
            .field("entity_selector", &self.entity_selector)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, ES> EntityPlacer<S, ChangeMove<S>> for QueuedEntityPlacer<S, ES>
where
    S: PlanningSolution + VariableOperations,
    ES: EntitySelector<S>,
{
    fn get_placements<D: ScoreDirector<S>>(&self, score_director: &D) -> Vec<Placement<S, ChangeMove<S>>> {
        let variable_name = self.variable_name;
        let descriptor_index = self.descriptor_index;
        let solution = score_director.working_solution();

        // Get value range from solution
        let values: Vec<_> = solution.value_range();

        self.entity_selector
            .iter(score_director)
            .filter_map(|entity_ref| {
                // Check if entity is uninitialized
                let is_assigned = solution.list_len(entity_ref.entity_index) > 0;

                // Only include uninitialized entities
                if is_assigned {
                    return None;
                }

                // Generate moves for all possible values
                let moves: Vec<ChangeMove<S>> = values
                    .iter()
                    .copied()
                    .map(|value| {
                        ChangeMove::new(entity_ref.entity_index, value, variable_name, descriptor_index)
                    })
                    .collect();

                if moves.is_empty() {
                    None
                } else {
                    Some(Placement::new(entity_ref, moves))
                }
            })
            .collect()
    }
}

/// Entity placer that sorts placements by a comparator function.
///
/// Wraps an inner placer and sorts its placements using a typed comparator.
/// This enables FIRST_FIT_DECREASING and similar construction variants.
pub struct SortedEntityPlacer<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
    Inner: EntityPlacer<S, M>,
{
    inner: Inner,
    /// Comparator function: takes (solution, entity_index_a, entity_index_b) -> Ordering
    comparator: fn(&S, usize, usize) -> std::cmp::Ordering,
    _phantom: PhantomData<fn() -> (S, M)>,
}

impl<S, M, Inner> SortedEntityPlacer<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
    Inner: EntityPlacer<S, M>,
{
    /// Creates a new sorted entity placer.
    ///
    /// # Arguments
    /// * `inner` - The inner placer to wrap
    /// * `comparator` - Function to compare entities: `(solution, idx_a, idx_b) -> Ordering`
    pub fn new(inner: Inner, comparator: fn(&S, usize, usize) -> std::cmp::Ordering) -> Self {
        Self {
            inner,
            comparator,
            _phantom: PhantomData,
        }
    }
}

impl<S, M, Inner> Debug for SortedEntityPlacer<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
    Inner: EntityPlacer<S, M>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SortedEntityPlacer")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S, M, Inner> EntityPlacer<S, M> for SortedEntityPlacer<S, M, Inner>
where
    S: PlanningSolution,
    M: Move<S>,
    Inner: EntityPlacer<S, M>,
{
    fn get_placements<D: ScoreDirector<S>>(&self, score_director: &D) -> Vec<Placement<S, M>> {
        let mut placements = self.inner.get_placements(score_director);
        let solution = score_director.working_solution();
        let cmp = self.comparator;

        placements.sort_by(|a, b| {
            cmp(
                solution,
                a.entity_ref.entity_index,
                b.entity_ref.entity_index,
            )
        });

        placements
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heuristic::selector::entity::FromSolutionEntitySelector;
    use crate::operations::VariableOperations;
    use solverforge_core::domain::{EntityDescriptor, SolutionDescriptor, TypedEntityExtractor};
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::SimpleScoreDirector;
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
    fn queued_placer_generates_placements() {
        let director = create_director(
            vec![
                Task { id: 0, priority: None },
                Task { id: 1, priority: None },
                Task { id: 2, priority: Some(5) }, // Already assigned
            ],
            vec![1, 2, 3],
        );

        let placer = QueuedEntityPlacer::<TaskSolution, _>::new(
            FromSolutionEntitySelector::new(0),
            "priority",
            0,
        );

        let placements = placer.get_placements(&director);

        // Only 2 placements (unassigned entities)
        assert_eq!(placements.len(), 2);

        // Each has 3 moves (one per value)
        for p in &placements {
            assert_eq!(p.move_count(), 3);
        }
    }

    #[test]
    fn sorted_placer_orders_by_comparator() {
        let director = create_director(
            vec![
                Task { id: 0, priority: None },
                Task { id: 1, priority: None },
                Task { id: 2, priority: None },
            ],
            vec![1],
        );

        let inner = QueuedEntityPlacer::<TaskSolution, _>::new(
            FromSolutionEntitySelector::new(0),
            "priority",
            0,
        );

        // Sort by descending entity index
        fn cmp(s: &TaskSolution, a: usize, b: usize) -> std::cmp::Ordering {
            s.tasks[b].id.cmp(&s.tasks[a].id)
        }

        let placer = SortedEntityPlacer::new(inner, cmp);

        let placements = placer.get_placements(&director);

        // Should be sorted in descending order
        assert_eq!(placements[0].entity_ref.entity_index, 2);
        assert_eq!(placements[1].entity_ref.entity_index, 1);
        assert_eq!(placements[2].entity_ref.entity_index, 0);
    }

    #[test]
    fn placement_take_move() {
        let director = create_director(
            vec![Task { id: 0, priority: None }],
            vec![10, 20, 30],
        );

        let placer = QueuedEntityPlacer::<TaskSolution, _>::new(
            FromSolutionEntitySelector::new(0),
            "priority",
            0,
        );

        let mut placements = placer.get_placements(&director);
        assert_eq!(placements.len(), 1);

        let placement = &mut placements[0];
        assert_eq!(placement.move_count(), 3);

        let m = placement.take_move(0);
        assert_eq!(placement.move_count(), 2);

        // Move was taken
        assert_eq!(m.entity_index(), 0);
    }
}
