//! Ruin move selector for Large Neighborhood Search.
//!
//! Generates `RuinMove` instances that unassign subsets of entities,
//! enabling exploration of distant regions in the solution space.
//!
//! # Zero-Erasure Design
//!
//! Uses `fn` pointers for variable access and entity counting.
//! No `Arc<dyn Fn>`, no trait objects in hot paths.
//!
//! # Example
//!
//! ```
//! use solverforge_solver::heuristic::selector::{MoveSelector, RuinMoveSelector};
//! use solverforge_solver::heuristic::r#move::RuinMove;
//! use solverforge_core::domain::PlanningSolution;
//! use solverforge_core::score::SimpleScore;
//! use solverforge_scoring::{ScoreDirector, SimpleScoreDirector};
//! use solverforge_core::domain::SolutionDescriptor;
//! use std::any::TypeId;
//!
//! #[derive(Clone, Debug)]
//! struct Task { assigned_to: Option<i32> }
//!
//! #[derive(Clone, Debug)]
//! struct Schedule { tasks: Vec<Task>, score: Option<SimpleScore> }
//!
//! impl PlanningSolution for Schedule {
//!     type Score = SimpleScore;
//!     fn score(&self) -> Option<Self::Score> { self.score }
//!     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
//! }
//!
//! fn entity_count(s: &Schedule) -> usize { s.tasks.len() }
//! fn get_task(s: &Schedule, idx: usize) -> Option<i32> {
//!     s.tasks.get(idx).and_then(|t| t.assigned_to)
//! }
//! fn set_task(s: &mut Schedule, idx: usize, v: Option<i32>) {
//!     if let Some(t) = s.tasks.get_mut(idx) { t.assigned_to = v; }
//! }
//!
//! // Create selector that ruins 2-3 entities at a time
//! let selector = RuinMoveSelector::<Schedule, i32>::new(
//!     2, 3,
//!     entity_count,
//!     get_task, set_task,
//!     "assigned_to", 0,
//! );
//!
//! // Use with a score director
//! let solution = Schedule {
//!     tasks: vec![
//!         Task { assigned_to: Some(1) },
//!         Task { assigned_to: Some(2) },
//!         Task { assigned_to: Some(3) },
//!     ],
//!     score: None,
//! };
//! let descriptor = SolutionDescriptor::new("Schedule", TypeId::of::<Schedule>());
//! let director = SimpleScoreDirector::with_calculator(
//!     solution, descriptor, |_| SimpleScore::of(0)
//! );
//!
//! let moves: Vec<_> = selector.iter_moves(&director).collect();
//! assert!(!moves.is_empty());
//! ```

use std::fmt::Debug;
use std::marker::PhantomData;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::RuinMove;

use super::MoveSelector;

/// A move selector that generates `RuinMove` instances for Large Neighborhood Search.
///
/// Selects random subsets of entities to "ruin" (unassign), enabling a construction
/// heuristic to reassign them in potentially better configurations.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The variable value type
///
/// # Zero-Erasure
///
/// All variable access uses `fn` pointers:
/// - `getter: fn(&S, usize) -> Option<V>` - gets current value
/// - `setter: fn(&mut S, usize, Option<V>)` - sets value
/// - `entity_count: fn(&S) -> usize` - counts entities
pub struct RuinMoveSelector<S, V> {
    /// Minimum entities to include in each ruin move.
    min_ruin_count: usize,
    /// Maximum entities to include in each ruin move.
    max_ruin_count: usize,
    /// Random seed for reproducible subset selection.
    seed: Option<u64>,
    /// Function to get entity count from solution.
    entity_count: fn(&S) -> usize,
    /// Function to get current value.
    getter: fn(&S, usize) -> Option<V>,
    /// Function to set value.
    setter: fn(&mut S, usize, Option<V>),
    /// Variable name.
    variable_name: &'static str,
    /// Entity descriptor index.
    descriptor_index: usize,
    /// Number of ruin moves to generate per iteration.
    moves_per_step: usize,
    _phantom: PhantomData<V>,
}

impl<S, V: Debug> Debug for RuinMoveSelector<S, V> {
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

impl<S, V> RuinMoveSelector<S, V> {
    /// Creates a new ruin move selector with typed function pointers.
    ///
    /// # Arguments
    /// * `min_ruin_count` - Minimum entities to ruin (at least 1)
    /// * `max_ruin_count` - Maximum entities to ruin
    /// * `entity_count` - Function to get total entity count
    /// * `getter` - Function to get current value
    /// * `setter` - Function to set value
    /// * `variable_name` - Name of the planning variable
    /// * `descriptor_index` - Entity descriptor index
    ///
    /// # Panics
    /// Panics if `min_ruin_count` is 0 or `max_ruin_count < min_ruin_count`.
    pub fn new(
        min_ruin_count: usize,
        max_ruin_count: usize,
        entity_count: fn(&S) -> usize,
        getter: fn(&S, usize) -> Option<V>,
        setter: fn(&mut S, usize, Option<V>),
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
            entity_count,
            getter,
            setter,
            variable_name,
            descriptor_index,
            moves_per_step: 10, // Default: generate 10 ruin moves per step
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

impl<S, D, V> MoveSelector<S, D, RuinMove<S, D, V>> for RuinMoveSelector<S, V>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    V: Clone + Send + Sync + Debug + 'static,
{
    fn iter_moves<'a>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = RuinMove<S, D, V>> + 'a> {
        let total_entities = (self.entity_count)(score_director.working_solution());
        let getter = self.getter;
        let setter = self.setter;
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
            RuinMove::new(&indices, getter, setter, variable_name, descriptor_index)
        }))
    }

    fn size(&self, score_director: &D) -> usize {
        let total = (self.entity_count)(score_director.working_solution());
        if total == 0 {
            return 0;
        }
        // Return configured moves per step (not combinatorial count)
        self.moves_per_step
    }

    fn is_never_ending(&self) -> bool {
        // Random selection means we could generate moves indefinitely
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solverforge_core::domain::SolutionDescriptor;
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::SimpleScoreDirector;
    use std::any::TypeId;

    #[derive(Clone, Debug)]
    struct Task {
        assigned_to: Option<i32>,
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

    fn entity_count(s: &Schedule) -> usize {
        s.tasks.len()
    }
    fn get_assigned(s: &Schedule, idx: usize) -> Option<i32> {
        s.tasks.get(idx).and_then(|t| t.assigned_to)
    }
    fn set_assigned(s: &mut Schedule, idx: usize, v: Option<i32>) {
        if let Some(t) = s.tasks.get_mut(idx) {
            t.assigned_to = v;
        }
    }

    fn create_director(
        assignments: &[Option<i32>],
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

        let selector = RuinMoveSelector::<Schedule, i32>::new(
            2,
            3,
            entity_count,
            get_assigned,
            set_assigned,
            "assigned_to",
            0,
        )
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
        let selector = RuinMoveSelector::<Schedule, i32>::new(
            5,
            10,
            entity_count,
            get_assigned,
            set_assigned,
            "assigned_to",
            0,
        )
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

        let selector = RuinMoveSelector::<Schedule, i32>::new(
            1,
            2,
            entity_count,
            get_assigned,
            set_assigned,
            "assigned_to",
            0,
        );

        let moves: Vec<_> = selector.iter_moves(&director).collect();

        // Moves are generated but they're empty (no entities to ruin)
        for m in &moves {
            assert_eq!(m.ruin_count(), 0);
        }
    }

    #[test]
    fn size_returns_moves_per_step() {
        let director = create_director(&[Some(1), Some(2), Some(3)]);

        let selector = RuinMoveSelector::<Schedule, i32>::new(
            1,
            2,
            entity_count,
            get_assigned,
            set_assigned,
            "assigned_to",
            0,
        )
        .with_moves_per_step(7);

        assert_eq!(selector.size(&director), 7);
    }

    #[test]
    #[should_panic(expected = "min_ruin_count must be at least 1")]
    fn panics_on_zero_min() {
        let _selector = RuinMoveSelector::<Schedule, i32>::new(
            0,
            2,
            entity_count,
            get_assigned,
            set_assigned,
            "assigned_to",
            0,
        );
    }

    #[test]
    #[should_panic(expected = "max_ruin_count must be >= min_ruin_count")]
    fn panics_on_invalid_range() {
        let _selector = RuinMoveSelector::<Schedule, i32>::new(
            5,
            2,
            entity_count,
            get_assigned,
            set_assigned,
            "assigned_to",
            0,
        );
    }
}
