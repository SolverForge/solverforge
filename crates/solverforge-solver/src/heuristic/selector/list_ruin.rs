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

use super::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};

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
    precedence_element_count: Option<fn(&S) -> usize>,
    precedence_index_to_element: Option<fn(&S, usize) -> V>,
    precedence_successors_fn: Option<fn(&S, V, &mut Vec<V>)>,
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

// SAFETY: RefCell<SmallRng> is accessed only while opening a cursor to derive
// that cursor's private seed. Each cursor owns its SmallRng and is consumed from
// a single thread at a time.
unsafe impl<S, V> Send for ListRuinMoveSelector<S, V> {}

impl<S, V: Debug> Debug for ListRuinMoveSelector<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListRuinMoveSelector")
            .field("min_ruin_count", &self.min_ruin_count)
            .field("max_ruin_count", &self.max_ruin_count)
            .field("moves_per_step", &self.moves_per_step)
            .field("max_source_list_len", &self.max_source_list_len)
            .field("skip_empty_destinations", &self.skip_empty_destinations)
            .field(
                "has_precedence_recreate_filter",
                &self.precedence_successors_fn.is_some(),
            )
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
            precedence_element_count: None,
            precedence_index_to_element: None,
            precedence_successors_fn: None,
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

    pub(crate) fn with_precedence_hooks(
        mut self,
        element_count: Option<fn(&S) -> usize>,
        index_to_element: Option<fn(&S, usize) -> V>,
        successors_fn: Option<fn(&S, V, &mut Vec<V>)>,
    ) -> Self {
        self.precedence_element_count = element_count;
        self.precedence_index_to_element = index_to_element;
        self.precedence_successors_fn = successors_fn;
        self
    }
}

enum ListRuinSourcePool {
    Unrestricted(Vec<(usize, usize)>),
    OwnerRestricted(Vec<(usize, SmallVec<[usize; 8]>)>),
}

impl ListRuinSourcePool {
    fn is_empty(&self) -> bool {
        match self {
            Self::Unrestricted(entities) => entities.is_empty(),
            Self::OwnerRestricted(entities) => entities.is_empty(),
        }
    }
}

pub struct ListRuinMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    store: CandidateStore<S, ListRuinMove<S, V>>,
    rng: SmallRng,
    source_pool: ListRuinSourcePool,
    remaining_moves: usize,
    min_ruin_count: usize,
    max_ruin_count: usize,
    entity_count: fn(&S) -> usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    list_remove: fn(&mut S, usize, usize) -> V,
    list_insert: fn(&mut S, usize, usize, V),
    element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    precedence_element_count: Option<fn(&S) -> usize>,
    precedence_index_to_element: Option<fn(&S, usize) -> V>,
    precedence_successors_fn: Option<fn(&S, V, &mut Vec<V>)>,
    skip_empty_destinations: bool,
    variable_name: &'static str,
    descriptor_index: usize,
}

impl<S, V> ListRuinMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    #[allow(clippy::too_many_arguments)]
    fn new(
        rng: SmallRng,
        source_pool: ListRuinSourcePool,
        remaining_moves: usize,
        min_ruin_count: usize,
        max_ruin_count: usize,
        entity_count: fn(&S) -> usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_remove: fn(&mut S, usize, usize) -> V,
        list_insert: fn(&mut S, usize, usize, V),
        element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
        precedence_element_count: Option<fn(&S) -> usize>,
        precedence_index_to_element: Option<fn(&S, usize) -> V>,
        precedence_successors_fn: Option<fn(&S, V, &mut Vec<V>)>,
        skip_empty_destinations: bool,
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            store: CandidateStore::new(),
            rng,
            source_pool,
            remaining_moves,
            min_ruin_count,
            max_ruin_count,
            entity_count,
            list_len,
            list_get,
            list_remove,
            list_insert,
            element_owner_fn,
            precedence_element_count,
            precedence_index_to_element,
            precedence_successors_fn,
            skip_empty_destinations,
            variable_name,
            descriptor_index,
        }
    }

    fn choose_ruin_count(&mut self, eligible_len: usize) -> usize {
        let min = self.min_ruin_count.min(eligible_len);
        let max = self.max_ruin_count.min(eligible_len);
        if min == max {
            min
        } else {
            self.rng.random_range(min..=max)
        }
    }

    fn next_unrestricted_move(&mut self) -> Option<ListRuinMove<S, V>> {
        let ListRuinSourcePool::Unrestricted(entities) = &self.source_pool else {
            return None;
        };
        let &(entity_idx, list_length) = entities.get(self.rng.random_range(0..entities.len()))?;
        let ruin_count = self.choose_ruin_count(list_length);
        let mut indices: SmallVec<[usize; 8]> = (0..list_length).collect();
        for i in 0..ruin_count {
            let j = self.rng.random_range(i..list_length);
            indices.swap(i, j);
        }
        indices.truncate(ruin_count);
        Some(self.build_move(entity_idx, &indices))
    }

    fn next_owner_restricted_move(&mut self) -> Option<ListRuinMove<S, V>> {
        let (entity_idx, mut indices) = {
            let ListRuinSourcePool::OwnerRestricted(entities) = &self.source_pool else {
                return None;
            };
            let (entity_idx, eligible_indices) =
                entities.get(self.rng.random_range(0..entities.len()))?;
            (*entity_idx, eligible_indices.clone())
        };
        let eligible_len = indices.len();
        let ruin_count = self.choose_ruin_count(eligible_len);
        for i in 0..ruin_count {
            let j = self.rng.random_range(i..eligible_len);
            indices.swap(i, j);
        }
        indices.truncate(ruin_count);
        Some(self.build_move(entity_idx, &indices))
    }

    fn build_move(&self, entity_idx: usize, indices: &[usize]) -> ListRuinMove<S, V> {
        ListRuinMove::new(
            entity_idx,
            indices,
            self.entity_count,
            self.list_len,
            self.list_get,
            self.list_remove,
            self.list_insert,
            self.variable_name,
            self.descriptor_index,
        )
        .with_element_owner_fn(self.element_owner_fn)
        .with_precedence_hooks(
            self.precedence_element_count,
            self.precedence_index_to_element,
            self.precedence_successors_fn,
        )
        .with_skip_empty_destinations(self.skip_empty_destinations)
    }
}

impl<S, V> MoveCursor<S, ListRuinMove<S, V>> for ListRuinMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        if self.remaining_moves == 0 || self.source_pool.is_empty() {
            return None;
        }
        self.remaining_moves -= 1;

        let next_move = match self.source_pool {
            ListRuinSourcePool::Unrestricted(_) => self.next_unrestricted_move(),
            ListRuinSourcePool::OwnerRestricted(_) => self.next_owner_restricted_move(),
        }?;
        Some(self.store.push(next_move))
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, ListRuinMove<S, V>>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> ListRuinMove<S, V> {
        self.store.take_candidate(id)
    }
}

impl<S, V> MoveSelector<S, ListRuinMove<S, V>> for ListRuinMoveSelector<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    type Cursor<'a>
        = ListRuinMoveCursor<S, V>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        self.open_cursor_with_context(score_director, MoveStreamContext::default())
    }

    fn open_cursor_with_context<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
        context: MoveStreamContext,
    ) -> Self::Cursor<'a> {
        let solution = score_director.working_solution();
        let total_entities = (self.entity_count)(solution);
        let non_empty_entities: Vec<(usize, usize)> = (0..total_entities)
            .filter_map(|entity_idx| {
                let len = (self.list_len)(solution, entity_idx);
                (len > 0
                    && self
                        .max_source_list_len
                        .is_none_or(|max_len| len <= max_len))
                .then_some((entity_idx, len))
            })
            .collect();

        let source_pool = if let Some(owner_fn) = self.element_owner_fn {
            let owner_eligible_entities = non_empty_entities
                .iter()
                .filter_map(|&(entity_idx, list_length)| {
                    let mut eligible_indices = SmallVec::new();
                    for pos in 0..list_length {
                        let Some(element) = (self.list_get)(solution, entity_idx, pos) else {
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
            ListRuinSourcePool::OwnerRestricted(owner_eligible_entities)
        } else {
            ListRuinSourcePool::Unrestricted(non_empty_entities)
        };

        let seed = self.rng.borrow_mut().random::<u64>()
            ^ context.offset_seed(0x7157_8011_C0DE_0001) as u64;
        ListRuinMoveCursor::new(
            SmallRng::seed_from_u64(seed),
            source_pool,
            self.moves_per_step,
            self.min_ruin_count,
            self.max_ruin_count,
            self.entity_count,
            self.list_len,
            self.list_get,
            self.list_remove,
            self.list_insert,
            self.element_owner_fn,
            self.precedence_element_count,
            self.precedence_index_to_element,
            self.precedence_successors_fn,
            self.skip_empty_destinations,
            self.variable_name,
            self.descriptor_index,
        )
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
