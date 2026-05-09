/* Sublist swap move selector for segment exchange.

Generates `SublistSwapMove`s that swap contiguous segments within or between
list variables. Useful for balanced inter-route segment exchanges in VRP.

# Complexity

For n entities with average route length m and max segment size k:
- Intra-entity pairs: O(n * m² * k²) — triangular over non-overlapping segments
- Inter-entity pairs: O(n² * m² * k²) — all pairs across entities

Use a forager that quits early for large instances.

# Example

```
use solverforge_solver::heuristic::selector::sublist_swap::SublistSwapMoveSelector;
use solverforge_solver::heuristic::selector::entity::FromSolutionEntitySelector;
use solverforge_solver::heuristic::selector::MoveSelector;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SoftScore;

#[derive(Clone, Debug)]
struct Vehicle { visits: Vec<i32> }

#[derive(Clone, Debug)]
struct Solution { vehicles: Vec<Vehicle>, score: Option<SoftScore> }

impl PlanningSolution for Solution {
type Score = SoftScore;
fn score(&self) -> Option<Self::Score> { self.score }
fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
}

fn list_len(s: &Solution, entity_idx: usize) -> usize {
s.vehicles.get(entity_idx).map_or(0, |v| v.visits.len())
}
fn sublist_remove(s: &mut Solution, entity_idx: usize, start: usize, end: usize) -> Vec<i32> {
s.vehicles.get_mut(entity_idx)
.map(|v| v.visits.drain(start..end).collect())
.unwrap_or_default()
}
fn sublist_insert(s: &mut Solution, entity_idx: usize, pos: usize, items: Vec<i32>) {
if let Some(v) = s.vehicles.get_mut(entity_idx) {
for (i, item) in items.into_iter().enumerate() {
v.visits.insert(pos + i, item);
}
}
}

// Swap segments of size 1..=3 between routes
let selector = SublistSwapMoveSelector::<Solution, i32, _>::new(
FromSolutionEntitySelector::new(0),
1, 3,
list_len,
sublist_remove,
sublist_insert,
"visits",
0,
);
```
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::SublistSwapMove;

use super::entity::EntitySelector;
use super::list_support::{collect_selected_entities, ordered_index};
use super::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};
use super::sublist_support::{count_intra_sublist_swap_moves_for_len, count_sublist_segments};

/// A move selector that generates sublist swap moves.
///
/// For each pair of segments (which may span different entities), generates
/// a swap move. Intra-entity swaps require non-overlapping segments.
///
/// # Type Parameters
/// * `S` - The solution type
/// * `V` - The list element type
/// * `ES` - The entity selector type
pub struct SublistSwapMoveSelector<S, V, ES> {
    entity_selector: ES,
    // Minimum segment size (inclusive).
    min_sublist_size: usize,
    // Maximum segment size (inclusive).
    max_sublist_size: usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

#[derive(Clone, Copy)]
struct ListSegment {
    start: usize,
    end: usize,
}

pub struct SublistSwapMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    store: CandidateStore<S, SublistSwapMove<S, V>>,
    entities: Vec<usize>,
    segments_by_entity: Vec<Vec<ListSegment>>,
    entity_a_idx: usize,
    segment_a_idx: usize,
    entity_b_idx: usize,
    segment_b_idx: usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    variable_name: &'static str,
    descriptor_index: usize,
}

impl<S, V> SublistSwapMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    #[allow(clippy::too_many_arguments)]
    fn new(
        entities: Vec<usize>,
        segments_by_entity: Vec<Vec<ListSegment>>,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            store: CandidateStore::new(),
            entities,
            segments_by_entity,
            entity_a_idx: 0,
            segment_a_idx: 0,
            entity_b_idx: 0,
            segment_b_idx: 0,
            list_len,
            list_get,
            sublist_remove,
            sublist_insert,
            variable_name,
            descriptor_index,
        }
    }

    fn push_move(
        &mut self,
        entity_a: usize,
        first: ListSegment,
        entity_b: usize,
        second: ListSegment,
    ) -> CandidateId {
        self.store.push(SublistSwapMove::new(
            entity_a,
            first.start,
            first.end,
            entity_b,
            second.start,
            second.end,
            self.list_len,
            self.list_get,
            self.sublist_remove,
            self.sublist_insert,
            self.variable_name,
            self.descriptor_index,
        ))
    }
}

impl<S, V> MoveCursor<S, SublistSwapMove<S, V>> for SublistSwapMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        loop {
            if self.entity_a_idx >= self.entities.len() {
                return None;
            }
            if self.segment_a_idx >= self.segments_by_entity[self.entity_a_idx].len() {
                self.entity_a_idx += 1;
                self.segment_a_idx = 0;
                self.entity_b_idx = self.entity_a_idx;
                self.segment_b_idx = 0;
                continue;
            }

            let entity_a = self.entities[self.entity_a_idx];
            let first = self.segments_by_entity[self.entity_a_idx][self.segment_a_idx];
            if self.entity_b_idx < self.entity_a_idx {
                self.entity_b_idx = self.entity_a_idx;
                self.segment_b_idx = 0;
            }

            while self.entity_b_idx < self.entities.len() {
                let entity_b = self.entities[self.entity_b_idx];
                while self.segment_b_idx < self.segments_by_entity[self.entity_b_idx].len() {
                    let second = self.segments_by_entity[self.entity_b_idx][self.segment_b_idx];
                    self.segment_b_idx += 1;
                    if self.entity_a_idx == self.entity_b_idx {
                        if second.start < first.end {
                            continue;
                        }
                        if first.start == second.start && first.end == second.end {
                            continue;
                        }
                    }
                    return Some(self.push_move(entity_a, first, entity_b, second));
                }
                self.entity_b_idx += 1;
                self.segment_b_idx = 0;
            }

            self.segment_a_idx += 1;
            self.entity_b_idx = self.entity_a_idx;
            self.segment_b_idx = 0;
        }
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, SublistSwapMove<S, V>>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> SublistSwapMove<S, V> {
        self.store.take_candidate(id)
    }
}

impl<S, V> Iterator for SublistSwapMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    type Item = SublistSwapMove<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.next_candidate()?;
        Some(self.take_candidate(id))
    }
}

fn build_segments_for_entity(
    entity: usize,
    route_len: usize,
    min_seg: usize,
    max_seg: usize,
    context: MoveStreamContext,
    descriptor_index: usize,
) -> Vec<ListSegment> {
    if route_len < min_seg {
        return Vec::new();
    }

    let mut segments = Vec::new();
    for start_offset in 0..route_len {
        let start = ordered_index(
            start_offset,
            route_len,
            context,
            0x5B15_75A0_9000_0002 ^ entity as u64 ^ descriptor_index as u64,
        );
        let max_valid = max_seg.min(route_len - start);
        if max_valid < min_seg {
            continue;
        }
        let size_count = max_valid - min_seg + 1;
        for size_offset in 0..size_count {
            let segment_size = min_seg
                + ordered_index(
                    size_offset,
                    size_count,
                    context,
                    0x5B15_75A0_9000_0003 ^ entity as u64 ^ start as u64,
                );
            segments.push(ListSegment {
                start,
                end: start + segment_size,
            });
        }
    }
    segments
}

impl<S, V: Debug, ES: Debug> Debug for SublistSwapMoveSelector<S, V, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SublistSwapMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("min_sublist_size", &self.min_sublist_size)
            .field("max_sublist_size", &self.max_sublist_size)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V, ES> SublistSwapMoveSelector<S, V, ES> {
    /* Creates a new sublist swap move selector.

    # Arguments
    * `entity_selector` - Selects entities to consider for swaps
    * `min_sublist_size` - Minimum segment length (must be ≥ 1)
    * `max_sublist_size` - Maximum segment length
    * `list_len` - Function to get list length for an entity
    * `sublist_remove` - Function to drain range `[start, end)`, returning elements
    * `sublist_insert` - Function to insert elements at a position
    * `variable_name` - Name of the list variable
    * `descriptor_index` - Entity descriptor index

    # Panics
    Panics if `min_sublist_size == 0` or `max_sublist_size < min_sublist_size`.
    */
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_selector: ES,
        min_sublist_size: usize,
        max_sublist_size: usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        assert!(min_sublist_size >= 1, "min_sublist_size must be at least 1");
        assert!(
            max_sublist_size >= min_sublist_size,
            "max_sublist_size must be >= min_sublist_size"
        );
        Self {
            entity_selector,
            min_sublist_size,
            max_sublist_size,
            list_len,
            list_get,
            sublist_remove,
            sublist_insert,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V, ES> MoveSelector<S, SublistSwapMove<S, V>> for SublistSwapMoveSelector<S, V, ES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
{
    type Cursor<'a>
        = SublistSwapMoveCursor<S, V>
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
        let min_seg = self.min_sublist_size;
        let max_seg = self.max_sublist_size;

        let mut selected =
            collect_selected_entities(&self.entity_selector, score_director, self.list_len);
        selected.apply_stream_order(
            context,
            0x5B15_75A0_9000_0001 ^ self.descriptor_index as u64,
        );
        let segments_by_entity = selected
            .route_lens
            .iter()
            .enumerate()
            .map(|(entity_offset, &route_len)| {
                let entity = selected.entities[entity_offset];
                build_segments_for_entity(
                    entity,
                    route_len,
                    min_seg,
                    max_seg,
                    context,
                    self.descriptor_index,
                )
            })
            .collect();

        SublistSwapMoveCursor::new(
            selected.entities,
            segments_by_entity,
            self.list_len,
            self.list_get,
            self.sublist_remove,
            self.sublist_insert,
            self.variable_name,
            self.descriptor_index,
        )
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        let selected =
            collect_selected_entities(&self.entity_selector, score_director, self.list_len);
        let segment_counts: Vec<usize> = selected
            .route_lens
            .iter()
            .map(|&route_len| {
                count_sublist_segments(route_len, self.min_sublist_size, self.max_sublist_size)
            })
            .collect();
        let intra: usize = selected
            .route_lens
            .iter()
            .map(|&route_len| {
                count_intra_sublist_swap_moves_for_len(
                    route_len,
                    self.min_sublist_size,
                    self.max_sublist_size,
                )
            })
            .sum();
        let inter: usize = (0..selected.route_lens.len())
            .flat_map(|left| (left + 1..selected.route_lens.len()).map(move |right| (left, right)))
            .map(|(left, right)| segment_counts[left] * segment_counts[right])
            .sum();
        intra + inter
    }
}
