/* List ruin move selector for Large Neighborhood Search on list variables.

Generates `ListRuinMove` instances that remove elements from list variables,
enabling exploration of distant regions in the solution space.

# Zero-Erasure Design

Uses `fn` pointers for list operations. No `Arc<dyn Fn>`, no trait objects
in hot paths.

# Example

```
use solverforge_solver::heuristic::selector::{MoveSelector, ListRuinMoveSelector};
use solverforge_solver::heuristic::r#move::ListRuinMove;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SoftScore;
use solverforge_scoring::{Director, ScoreDirector};
use solverforge_core::domain::SolutionDescriptor;
use std::any::TypeId;

#[derive(Clone, Debug)]
struct Route { stops: Vec<i32> }

#[derive(Clone, Debug)]
struct VrpSolution { routes: Vec<Route>, score: Option<SoftScore> }

impl PlanningSolution for VrpSolution {
type Score = SoftScore;
fn score(&self) -> Option<Self::Score> { self.score }
fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
}

fn entity_count(s: &VrpSolution) -> usize { s.routes.len() }
fn list_len(s: &VrpSolution, idx: usize) -> usize {
s.routes.get(idx).map_or(0, |r| r.stops.len())
}
fn list_remove(s: &mut VrpSolution, entity_idx: usize, idx: usize) -> i32 {
s.routes.get_mut(entity_idx).map(|r| r.stops.remove(idx)).unwrap_or(0)
}
fn list_insert(s: &mut VrpSolution, entity_idx: usize, idx: usize, v: i32) {
if let Some(r) = s.routes.get_mut(entity_idx) { r.stops.insert(idx, v); }
}

// Create selector that removes 2-3 elements at a time
let selector = ListRuinMoveSelector::<VrpSolution, i32>::new(
2, 3,
entity_count,
list_len, list_remove, list_insert,
"stops", 0,
);

// Use with a score director
let solution = VrpSolution {
routes: vec![Route { stops: vec![1, 2, 3, 4, 5] }],
score: None,
};
let descriptor = SolutionDescriptor::new("VrpSolution", TypeId::of::<VrpSolution>());
let director = ScoreDirector::simple(
solution, descriptor, |s, _| s.routes.len()
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

use crate::heuristic::r#move::ListRuinMove;

use super::move_selector::{ArenaMoveCursor, MoveSelector};

/// A move selector that generates `ListRuinMove` instances for Large Neighborhood Search.
///
/// Selects random subsets of elements from list variables to "ruin" (remove),
/// enabling a construction heuristic to reinsert them in better positions.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The list element value type
///
/// # Zero-Erasure
///
/// All list access uses `fn` pointers:
/// - `list_len: fn(&S, usize) -> usize` - gets list length
/// - `list_remove: fn(&mut S, usize, usize) -> V` - removes element
/// - `list_insert: fn(&mut S, usize, usize, V)` - inserts element
/// - `entity_count: fn(&S) -> usize` - counts entities
pub struct ListRuinMoveSelector<S, V> {
    // Minimum elements to remove per move.
    min_ruin_count: usize,
    // Maximum elements to remove per move.
    max_ruin_count: usize,
    // RNG state for reproducible subset selection.
    rng: RefCell<SmallRng>,
    // Function to get entity count from solution.
    entity_count: fn(&S) -> usize,
    // Function to get list length for an entity.
    list_len: fn(&S, usize) -> usize,
    // Function to read a list element by position for move metadata.
    list_get: fn(&S, usize, usize) -> Option<V>,
    // Function to remove element at index, returning it.
    list_remove: fn(&mut S, usize, usize) -> V,
    // Function to insert element at index.
    list_insert: fn(&mut S, usize, usize, V),
    element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    // Variable name.
    variable_name: &'static str,
    // Entity descriptor index.
    descriptor_index: usize,
    // Number of ruin moves to generate per iteration.
    moves_per_step: usize,
    // Optional cap on source list length for specialized route-emptying selectors.
    max_source_list_len: Option<usize>,
    // Optional recreate pruning for domains where opening empty lists is undesirable.
    skip_empty_destinations: bool,
    _phantom: PhantomData<fn() -> V>,
}

// SAFETY: RefCell<SmallRng> is only accessed while pre-generating a move batch
// inside `iter_moves`, and selectors are consumed from a single thread at a time.
unsafe impl<S, V> Send for ListRuinMoveSelector<S, V> {}

impl<S, V: Debug> Debug for ListRuinMoveSelector<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListRuinMoveSelector")
            .field("min_ruin_count", &self.min_ruin_count)
            .field("max_ruin_count", &self.max_ruin_count)
            .field("moves_per_step", &self.moves_per_step)
            .field("max_source_list_len", &self.max_source_list_len)
            .field("skip_empty_destinations", &self.skip_empty_destinations)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V> ListRuinMoveSelector<S, V> {
    /* Creates a new list ruin move selector with concrete function pointers.

    # Arguments
    * `min_ruin_count` - Minimum elements to remove (at least 1)
    * `max_ruin_count` - Maximum elements to remove
    * `entity_count` - Function to get total entity count
    * `list_len` - Function to get list length for an entity
    * `list_remove` - Function to remove element at index
    * `list_insert` - Function to insert element at index
    * `variable_name` - Name of the list variable
    * `descriptor_index` - Entity descriptor index

    # Panics
    Panics if `min_ruin_count` is 0 or `max_ruin_count < min_ruin_count`.
    */
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        min_ruin_count: usize,
        max_ruin_count: usize,
        entity_count: fn(&S) -> usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_remove: fn(&mut S, usize, usize) -> V,
        list_insert: fn(&mut S, usize, usize, V),
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
            rng: RefCell::new(SmallRng::from_rng(&mut rand::rng())),
            entity_count,
            list_len,
            list_get,
            list_remove,
            list_insert,
            element_owner_fn: None,
            variable_name,
            descriptor_index,
            moves_per_step: 10,
            max_source_list_len: None,
            skip_empty_destinations: false,
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

    pub fn with_max_source_list_len(mut self, max_source_list_len: Option<usize>) -> Self {
        self.max_source_list_len = max_source_list_len;
        self
    }

    pub fn with_skip_empty_destinations(mut self, skip_empty_destinations: bool) -> Self {
        self.skip_empty_destinations = skip_empty_destinations;
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.rng = RefCell::new(SmallRng::seed_from_u64(seed));
        self
    }

    pub fn with_element_owner_fn(
        mut self,
        element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    ) -> Self {
        self.element_owner_fn = element_owner_fn;
        self
    }
}

impl<S, V> MoveSelector<S, ListRuinMove<S, V>> for ListRuinMoveSelector<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    type Cursor<'a>
        = ArenaMoveCursor<S, ListRuinMove<S, V>>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        let solution = score_director.working_solution();
        let total_entities = (self.entity_count)(solution);
        let list_len = self.list_len;
        let list_get = self.list_get;
        let list_remove = self.list_remove;
        let list_insert = self.list_insert;
        let element_owner_fn = self.element_owner_fn;
        let variable_name = self.variable_name;
        let descriptor_index = self.descriptor_index;
        let min_ruin = self.min_ruin_count;
        let max_ruin = self.max_ruin_count;
        let moves_count = self.moves_per_step;

        let non_empty_entities: Vec<(usize, usize)> = (0..total_entities)
            .filter_map(|entity_idx| {
                let len = list_len(solution, entity_idx);
                (len > 0
                    && self
                        .max_source_list_len
                        .is_none_or(|max_len| len <= max_len))
                .then_some((entity_idx, len))
            })
            .collect();

        // Pre-generate moves using RNG (empty if no entities)
        let mut rng = self.rng.borrow_mut();
        let moves: Vec<ListRuinMove<S, V>> = if non_empty_entities.is_empty() {
            Vec::new()
        } else if let Some(owner_fn) = element_owner_fn {
            let owner_eligible_entities: Vec<(usize, SmallVec<[usize; 8]>)> = non_empty_entities
                .iter()
                .filter_map(|&(entity_idx, list_length)| {
                    let mut eligible_indices = SmallVec::new();
                    for pos in 0..list_length {
                        let Some(element) = list_get(solution, entity_idx, pos) else {
                            continue;
                        };
                        if crate::list_placement::candidate_entity_indices(
                            Some(owner_fn),
                            solution,
                            total_entities,
                            &element,
                        )
                        .next()
                        .is_some()
                        {
                            eligible_indices.push(pos);
                        }
                    }
                    (!eligible_indices.is_empty()).then_some((entity_idx, eligible_indices))
                })
                .collect();

            if owner_eligible_entities.is_empty() {
                Vec::new()
            } else {
                (0..moves_count)
                    .map(|_| {
                        // Pick a random entity that has at least one recreatable element.
                        let (entity_idx, eligible_indices) = &owner_eligible_entities
                            [rng.random_range(0..owner_eligible_entities.len())];
                        let eligible_len = eligible_indices.len();

                        // Clamp ruin count to owner-eligible elements.
                        let min = min_ruin.min(eligible_len);
                        let max = max_ruin.min(eligible_len);
                        let ruin_count = if min == max {
                            min
                        } else {
                            rng.random_range(min..=max)
                        };

                        // Fisher-Yates partial shuffle over owner-eligible indices only.
                        let mut indices: SmallVec<[usize; 8]> =
                            eligible_indices.iter().copied().collect();
                        for i in 0..ruin_count {
                            let j = rng.random_range(i..eligible_len);
                            indices.swap(i, j);
                        }
                        indices.truncate(ruin_count);

                        ListRuinMove::new(
                            *entity_idx,
                            &indices,
                            self.entity_count,
                            list_len,
                            list_get,
                            list_remove,
                            list_insert,
                            variable_name,
                            descriptor_index,
                        )
                        .with_element_owner_fn(element_owner_fn)
                        .with_skip_empty_destinations(self.skip_empty_destinations)
                    })
                    .collect()
            }
        } else {
            (0..moves_count)
                .filter_map(|_| {
                    // Pick a random entity
                    let (entity_idx, list_length) =
                        non_empty_entities[rng.random_range(0..non_empty_entities.len())];

                    if list_length == 0 {
                        return None;
                    }

                    // Clamp ruin count to available elements
                    let min = min_ruin.min(list_length);
                    let max = max_ruin.min(list_length);
                    let ruin_count = if min == max {
                        min
                    } else {
                        rng.random_range(min..=max)
                    };

                    // Fisher-Yates partial shuffle to select random indices
                    let mut indices: SmallVec<[usize; 8]> = (0..list_length).collect();
                    for i in 0..ruin_count {
                        let j = rng.random_range(i..list_length);
                        indices.swap(i, j);
                    }
                    indices.truncate(ruin_count);

                    Some(
                        ListRuinMove::new(
                            entity_idx,
                            &indices,
                            self.entity_count,
                            list_len,
                            list_get,
                            list_remove,
                            list_insert,
                            variable_name,
                            descriptor_index,
                        )
                        .with_element_owner_fn(element_owner_fn)
                        .with_skip_empty_destinations(self.skip_empty_destinations),
                    )
                })
                .collect()
        };

        ArenaMoveCursor::from_moves(moves)
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        let total = (self.entity_count)(score_director.working_solution());
        if total == 0 {
            return 0;
        }
        self.moves_per_step
    }

    fn is_never_ending(&self) -> bool {
        false
    }
}
