//! Ruin move selector for Large Neighborhood Search.
//!
//! Generates `RuinMove` instances that unassign subsets of entities,
//! enabling exploration of distant regions in the solution space.
//!
//! # Zero-Erasure Design
//!
//! No value type parameter. Uses VariableOperations trait for entity access.

use std::fmt::Debug;
use std::marker::PhantomData;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::ruin::RuinMove;
use crate::operations::VariableOperations;

use super::MoveSelector;

/// A move selector that generates `RuinMove` instances for Large Neighborhood Search.
///
/// Selects random subsets of entities to "ruin" (unassign), enabling a construction
/// heuristic to reassign them in potentially better configurations.
///
/// # Type Parameters
/// * `S` - The planning solution type (must implement VariableOperations)
///
/// # Zero-Erasure
///
/// Uses VariableOperations trait for entity counting. No function pointers required.
pub struct RuinMoveSelector<S> {
    /// Minimum entities to include in each ruin move.
    min_ruin_count: usize,
    /// Maximum entities to include in each ruin move.
    max_ruin_count: usize,
    /// Random seed for reproducible subset selection.
    seed: Option<u64>,
    /// Variable name.
    variable_name: &'static str,
    /// Entity descriptor index.
    descriptor_index: usize,
    /// Number of ruin moves to generate per iteration.
    moves_per_step: usize,
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Debug for RuinMoveSelector<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuinMoveSelector")
            .field("min_ruin_count", &self.min_ruin_count)
            .field("max_ruin_count", &self.max_ruin_count)
            .field("moves_per_step", &self.moves_per_step)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S> RuinMoveSelector<S> {
    /// Creates a new ruin move selector.
    ///
    /// # Arguments
    /// * `min_ruin_count` - Minimum entities to ruin (at least 1)
    /// * `max_ruin_count` - Maximum entities to ruin
    /// * `variable_name` - Name of the planning variable
    /// * `descriptor_index` - Entity descriptor index
    ///
    /// # Panics
    /// Panics if `min_ruin_count` is 0 or `max_ruin_count < min_ruin_count`.
    pub fn new(
        min_ruin_count: usize,
        max_ruin_count: usize,
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        assert!(min_ruin_count >= 1, "min_ruin_count must be at least 1");
        assert!(
            max_ruin_count >= min_ruin_count,
            "max_ruin_count must be >= min_ruin_count"
        );

        Self {
            min_ruin_count,
            max_ruin_count,
            seed: None,
            variable_name,
            descriptor_index,
            moves_per_step: 10,
            _phantom: PhantomData,
        }
    }

    /// Sets the number of ruin moves to generate per iteration.
    ///
    /// Default is 10.
    pub fn with_moves_per_step(mut self, count: usize) -> Self {
        self.moves_per_step = count;
        self
    }

    /// Sets the random seed for reproducible subset selection.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Creates a random number generator.
    fn create_rng(&self) -> StdRng {
        match self.seed {
            Some(seed) => StdRng::seed_from_u64(seed),
            None => StdRng::from_os_rng(),
        }
    }
}

impl<S> MoveSelector<S, RuinMove<S>> for RuinMoveSelector<S>
where
    S: PlanningSolution + VariableOperations,
{
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = RuinMove<S>> + 'a> {
        let solution = score_director.working_solution();
        let total_entities = solution.entity_count();
        let variable_name = self.variable_name;
        let descriptor_index = self.descriptor_index;

        let min = self.min_ruin_count.min(total_entities);
        let max = self.max_ruin_count.min(total_entities);
        let moves_count = self.moves_per_step;

        // Pre-generate subsets using RNG
        let mut rng = self.create_rng();
        let subsets: Vec<SmallVec<[usize; 8]>> = (0..moves_count)
            .map(|_| {
                if total_entities == 0 {
                    return SmallVec::new();
                }
                let ruin_count = if min == max {
                    min
                } else {
                    rng.random_range(min..=max)
                };
                let mut indices: SmallVec<[usize; 8]> = (0..total_entities).collect();
                for i in 0..ruin_count {
                    let j = rng.random_range(i..total_entities);
                    indices.swap(i, j);
                }
                indices.truncate(ruin_count);
                indices
            })
            .collect();

        Box::new(subsets.into_iter().map(move |indices| {
            RuinMove::new(&indices, variable_name, descriptor_index)
        }))
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        let total = score_director.working_solution().entity_count();
        if total == 0 {
            return 0;
        }
        self.moves_per_step
    }

    fn is_never_ending(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operations::VariableOperations;
    use solverforge_core::domain::SolutionDescriptor;
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::SimpleScoreDirector;
    use std::any::TypeId;

    #[derive(Clone, Debug)]
    struct Task {
        assigned_to: Option<usize>,
    }

    #[derive(Clone, Debug)]
    struct Schedule {
        tasks: Vec<Task>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for Schedule {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    impl VariableOperations for Schedule {
        type Element = usize;

        fn element_count(&self) -> usize {
            10 // 10 possible assignments
        }

        fn entity_count(&self) -> usize {
            self.tasks.len()
        }

        fn assigned_elements(&self) -> Vec<Self::Element> {
            self.tasks.iter().filter_map(|t| t.assigned_to).collect()
        }

        fn assign(&mut self, entity_idx: usize, elem: Self::Element) {
            self.tasks[entity_idx].assigned_to = Some(elem);
        }

        fn list_len(&self, entity_idx: usize) -> usize {
            if self.tasks[entity_idx].assigned_to.is_some() {
                1
            } else {
                0
            }
        }

        fn remove(&mut self, entity_idx: usize, _pos: usize) -> Self::Element {
            self.tasks[entity_idx].assigned_to.take().unwrap()
        }

        fn insert(&mut self, entity_idx: usize, _pos: usize, elem: Self::Element) {
            self.tasks[entity_idx].assigned_to = Some(elem);
        }

        fn get(&self, entity_idx: usize, _pos: usize) -> Self::Element {
            self.tasks[entity_idx].assigned_to.unwrap()
        }

        fn descriptor_index() -> usize {
            0
        }

        fn variable_name() -> &'static str {
            "assigned_to"
        }

        fn is_list_variable() -> bool {
            false
        }
    }

    fn create_director(
        assignments: &[Option<usize>],
    ) -> SimpleScoreDirector<Schedule, impl Fn(&Schedule) -> SimpleScore> {
        let tasks: Vec<Task> = assignments
            .iter()
            .map(|&a| Task { assigned_to: a })
            .collect();
        let solution = Schedule { tasks, score: None };
        let descriptor = SolutionDescriptor::new("Schedule", TypeId::of::<Schedule>());
        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn generates_ruin_moves() {
        let director = create_director(&[Some(1), Some(2), Some(3), Some(4), Some(5)]);

        let selector = RuinMoveSelector::<Schedule>::new(2, 3, "assigned_to", 0)
            .with_moves_per_step(5);

        let moves: Vec<_> = selector.iter_moves(&director).collect();

        assert_eq!(moves.len(), 5);
        for m in &moves {
            let count = m.ruin_count();
            assert!((2..=3).contains(&count));
        }
    }

    #[test]
    fn clamps_to_available_entities() {
        let director = create_director(&[Some(1), Some(2)]);

        // Request more entities than available
        let selector = RuinMoveSelector::<Schedule>::new(5, 10, "assigned_to", 0)
            .with_moves_per_step(3);

        let moves: Vec<_> = selector.iter_moves(&director).collect();

        assert_eq!(moves.len(), 3);
        for m in &moves {
            // Should clamp to max 2 entities
            assert!(m.ruin_count() <= 2);
        }
    }

    #[test]
    fn empty_solution_yields_empty_moves() {
        let director = create_director(&[]);

        let selector = RuinMoveSelector::<Schedule>::new(1, 2, "assigned_to", 0);

        let moves: Vec<_> = selector.iter_moves(&director).collect();

        // Moves are generated but they're empty (no entities to ruin)
        for m in &moves {
            assert_eq!(m.ruin_count(), 0);
        }
    }

    #[test]
    fn size_returns_moves_per_step() {
        let director = create_director(&[Some(1), Some(2), Some(3)]);

        let selector = RuinMoveSelector::<Schedule>::new(1, 2, "assigned_to", 0)
            .with_moves_per_step(7);

        assert_eq!(selector.size(&director), 7);
    }

    #[test]
    #[should_panic(expected = "min_ruin_count must be at least 1")]
    fn panics_on_zero_min() {
        let _selector = RuinMoveSelector::<Schedule>::new(0, 2, "assigned_to", 0);
    }

    #[test]
    #[should_panic(expected = "max_ruin_count must be >= min_ruin_count")]
    fn panics_on_invalid_range() {
        let _selector = RuinMoveSelector::<Schedule>::new(5, 2, "assigned_to", 0);
    }
}
