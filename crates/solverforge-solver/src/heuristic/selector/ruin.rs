/* Ruin move selector for Large Neighborhood Search.

Generates `RuinMove` instances that unassign subsets of entities,
enabling exploration of distant regions in the solution space.

# Zero-Erasure Design

Uses `fn` pointers for variable access and entity counting.
No `Arc<dyn Fn>`, no trait objects in hot paths.

# Example

```
use solverforge_solver::heuristic::selector::{MoveSelector, RuinMoveSelector, RuinVariableAccess};
use solverforge_solver::heuristic::r#move::RuinMove;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SoftScore;
use solverforge_scoring::{Director, ScoreDirector};
use solverforge_core::domain::SolutionDescriptor;
use std::any::TypeId;

#[derive(Clone, Debug)]
struct Task { assigned_to: Option<i32> }

#[derive(Clone, Debug)]
struct Schedule { tasks: Vec<Task>, score: Option<SoftScore> }

impl PlanningSolution for Schedule {
type Score = SoftScore;
fn score(&self) -> Option<Self::Score> { self.score }
fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
}

fn entity_count(s: &Schedule) -> usize { s.tasks.len() }
fn get_task(s: &Schedule, idx: usize, _variable_index: usize) -> Option<i32> {
s.tasks.get(idx).and_then(|t| t.assigned_to)
}
fn set_task(s: &mut Schedule, idx: usize, _variable_index: usize, v: Option<i32>) {
if let Some(t) = s.tasks.get_mut(idx) { t.assigned_to = v; }
}

// Create selector that ruins 2-3 entities at a time
let access = RuinVariableAccess::new(
entity_count,
get_task, set_task,
0,
"assigned_to", 0,
);
let selector = RuinMoveSelector::<Schedule, i32>::new(
2, 3,
access,
);

// Use with a score director
let solution = Schedule {
tasks: vec![
Task { assigned_to: Some(1) },
Task { assigned_to: Some(2) },
Task { assigned_to: Some(3) },
],
score: None,
};
let descriptor = SolutionDescriptor::new("Schedule", TypeId::of::<Schedule>());
let director = ScoreDirector::simple(
solution, descriptor, |s, _| s.tasks.len()
);

let moves: Vec<_> = selector.iter_moves(&director).collect();
assert!(!moves.is_empty());
```
*/

use std::cell::RefCell;
use std::fmt::Debug;
use std::marker::PhantomData;

use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};
use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::RuinMove;

use super::move_selector::{ArenaMoveCursor, MoveSelector};

pub struct RuinVariableAccess<S, V> {
    // Function to get entity count from solution.
    entity_count: fn(&S) -> usize,
    // Function to get current value.
    getter: fn(&S, usize, usize) -> Option<V>,
    // Function to set value.
    setter: fn(&mut S, usize, usize, Option<V>),
    variable_index: usize,
    // Variable name.
    variable_name: &'static str,
    // Entity descriptor index.
    descriptor_index: usize,
    _phantom: PhantomData<fn() -> V>,
}

impl<S, V> Clone for RuinVariableAccess<S, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, V> Copy for RuinVariableAccess<S, V> {}

impl<S, V> RuinVariableAccess<S, V> {
    /// Creates scalar-variable access metadata for scalar ruin moves.
    pub fn new(
        entity_count: fn(&S) -> usize,
        getter: fn(&S, usize, usize) -> Option<V>,
        setter: fn(&mut S, usize, usize, Option<V>),
        variable_index: usize,
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_count,
            getter,
            setter,
            variable_index,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }
}

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
/// - `getter: fn(&S, usize, usize) -> Option<V>` - gets current value
/// - `setter: fn(&mut S, usize, usize, Option<V>)` - sets value
/// - `entity_count: fn(&S) -> usize` - counts entities
pub struct RuinMoveSelector<S, V> {
    // Minimum entities to include in each ruin move.
    min_ruin_count: usize,
    // Maximum entities to include in each ruin move.
    max_ruin_count: usize,
    // RNG state for reproducible subset selection.
    rng: RefCell<SmallRng>,
    access: RuinVariableAccess<S, V>,
    // Number of ruin moves to generate per iteration.
    moves_per_step: usize,
}

// SAFETY: RefCell<SmallRng> is only accessed while pre-generating a move batch
// inside `iter_moves`, and selectors are consumed from a single thread at a time.
unsafe impl<S, V> Send for RuinMoveSelector<S, V> {}

impl<S, V: Debug> Debug for RuinMoveSelector<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuinMoveSelector")
            .field("min_ruin_count", &self.min_ruin_count)
            .field("max_ruin_count", &self.max_ruin_count)
            .field("moves_per_step", &self.moves_per_step)
            .field("variable_name", &self.access.variable_name)
            .field("descriptor_index", &self.access.descriptor_index)
            .finish()
    }
}

impl<S, V> RuinMoveSelector<S, V> {
    /// Creates a new ruin move selector with concrete function pointers.
    ///
    /// # Arguments
    /// * `min_ruin_count` - Minimum entities to ruin (at least 1)
    /// * `max_ruin_count` - Maximum entities to ruin
    /// * `access` - Concrete variable access metadata for the scalar variable
    ///
    /// # Panics
    /// Panics if `min_ruin_count` is 0 or `max_ruin_count < min_ruin_count`.
    pub fn new(
        min_ruin_count: usize,
        max_ruin_count: usize,
        access: RuinVariableAccess<S, V>,
    ) -> Self {
        assert!(min_ruin_count >= 1, "min_ruin_count must be at least 1");
        assert!(
            max_ruin_count >= min_ruin_count,
            "max_ruin_count must be >= min_ruin_count"
        );

        Self {
            min_ruin_count,
            max_ruin_count,
            rng: RefCell::new(SmallRng::from_rng(&mut rand::rng())),
            access,
            moves_per_step: 10, // Default: generate 10 ruin moves per step
        }
    }

    /// Sets the number of ruin moves to generate per iteration.
    ///
    /// Default is 10.
    pub fn with_moves_per_step(mut self, count: usize) -> Self {
        self.moves_per_step = count;
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.rng = RefCell::new(SmallRng::seed_from_u64(seed));
        self
    }
}

impl<S, V> MoveSelector<S, RuinMove<S, V>> for RuinMoveSelector<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    type Cursor<'a>
        = ArenaMoveCursor<S, RuinMove<S, V>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        let access = self.access;
        let total_entities = (access.entity_count)(score_director.working_solution());

        let min = self.min_ruin_count.min(total_entities);
        let max = self.max_ruin_count.min(total_entities);
        let moves_count = self.moves_per_step;

        // Pre-generate subsets using RNG
        let mut rng = self.rng.borrow_mut();
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

        ArenaMoveCursor::from_moves(subsets.into_iter().map(move |indices| {
            RuinMove::new(
                &indices,
                access.getter,
                access.setter,
                access.variable_index,
                access.variable_name,
                access.descriptor_index,
            )
        }))
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        let total = (self.access.entity_count)(score_director.working_solution());
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
