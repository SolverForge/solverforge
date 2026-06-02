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
use crate::list_placement::{selected_segment_allows, OwnerRestriction, SelectedOwnerRestrictions};

use super::entity::EntitySelector;
use super::list_support::{collect_selected_entities, ordered_index};
use super::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};
use super::precedence_route::{PrecedenceRouteGraph, PrecedenceRouteHooks};
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
    element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    precedence_route_hooks: Option<PrecedenceRouteHooks<S, V>>,
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

#[derive(Clone, Copy)]
struct ListSegment {
    start: usize,
    end: usize,
}

#[derive(Clone)]
struct SublistSegmentCursor {
    entity: usize,
    route_len: usize,
    min_seg: usize,
    max_seg: usize,
    context: MoveStreamContext,
    descriptor_index: usize,
    start_offset: usize,
    size_offset: usize,
    current_start: Option<usize>,
    current_size_count: usize,
}

impl SublistSegmentCursor {
    fn new(
        entity: usize,
        route_len: usize,
        min_seg: usize,
        max_seg: usize,
        context: MoveStreamContext,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity,
            route_len,
            min_seg,
            max_seg,
            context,
            descriptor_index,
            start_offset: 0,
            size_offset: 0,
            current_start: None,
            current_size_count: 0,
        }
    }
}

impl Iterator for SublistSegmentCursor {
    type Item = ListSegment;

    fn next(&mut self) -> Option<Self::Item> {
        if self.route_len < self.min_seg {
            return None;
        }

        loop {
            if let Some(start) = self.current_start {
                if self.size_offset < self.current_size_count {
                    let segment_size = self.min_seg
                        + ordered_index(
                            self.size_offset,
                            self.current_size_count,
                            self.context,
                            0x5B15_75A0_9000_0003 ^ self.entity as u64 ^ start as u64,
                        );
                    self.size_offset += 1;
                    return Some(ListSegment {
                        start,
                        end: start + segment_size,
                    });
                }
                self.current_start = None;
            }

            if self.start_offset >= self.route_len {
                return None;
            }

            let start = ordered_index(
                self.start_offset,
                self.route_len,
                self.context,
                0x5B15_75A0_9000_0002 ^ self.entity as u64 ^ self.descriptor_index as u64,
            );
            self.start_offset += 1;
            let max_valid = self.max_seg.min(self.route_len - start);
            if max_valid < self.min_seg {
                continue;
            }
            self.current_start = Some(start);
            self.current_size_count = max_valid - self.min_seg + 1;
            self.size_offset = 0;
        }
    }
}

pub struct SublistSwapMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    store: CandidateStore<S, SublistSwapMove<S, V>>,
    entities: Vec<usize>,
    route_lens: Vec<usize>,
    context: MoveStreamContext,
    element_owners: Option<Vec<Vec<OwnerRestriction>>>,
    fixed_to_current_entity: bool,
    precedence_route_graph: Option<PrecedenceRouteGraph>,
    entity_a_idx: usize,
    segment_a_cursor: Option<SublistSegmentCursor>,
    first_segment: Option<ListSegment>,
    entity_b_idx: usize,
    segment_b_cursor: Option<SublistSegmentCursor>,
    min_seg: usize,
    max_seg: usize,
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
        route_lens: Vec<usize>,
        context: MoveStreamContext,
        element_owners: Option<Vec<Vec<OwnerRestriction>>>,
        fixed_to_current_entity: bool,
        precedence_route_graph: Option<PrecedenceRouteGraph>,
        min_seg: usize,
        max_seg: usize,
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
            route_lens,
            context,
            element_owners,
            fixed_to_current_entity,
            precedence_route_graph,
            entity_a_idx: 0,
            segment_a_cursor: None,
            first_segment: None,
            entity_b_idx: 0,
            segment_b_cursor: None,
            min_seg,
            max_seg,
            list_len,
            list_get,
            sublist_remove,
            sublist_insert,
            variable_name,
            descriptor_index,
        }
    }

    pub(crate) fn with_precedence_route_graph(
        mut self,
        precedence_route_graph: Option<PrecedenceRouteGraph>,
    ) -> Self {
        self.precedence_route_graph = precedence_route_graph;
        self
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

    fn segment_cursor(&self, entity_idx: usize) -> SublistSegmentCursor {
        SublistSegmentCursor::new(
            self.entities[entity_idx],
            self.route_lens[entity_idx],
            self.min_seg,
            self.max_seg,
            self.context,
            self.descriptor_index,
        )
    }

    fn segment_owner_allows(
        &self,
        entity_idx: usize,
        segment: ListSegment,
        dst_entity: usize,
    ) -> bool {
        self.element_owners.as_ref().is_none_or(|owners| {
            selected_segment_allows(owners, entity_idx, segment.start, segment.end, dst_entity)
        })
    }

    fn owner_allows_swap(
        &self,
        entity_a_idx: usize,
        first: ListSegment,
        entity_a: usize,
        entity_b_idx: usize,
        second: ListSegment,
        entity_b: usize,
    ) -> bool {
        if entity_a == entity_b {
            self.segment_owner_allows(entity_a_idx, first, entity_a)
                && self.segment_owner_allows(entity_b_idx, second, entity_a)
        } else {
            self.segment_owner_allows(entity_a_idx, first, entity_b)
                && self.segment_owner_allows(entity_b_idx, second, entity_a)
        }
    }

    fn next_first_segment(&mut self) -> Option<ListSegment> {
        loop {
            if self.entity_a_idx >= self.entities.len() {
                return None;
            }
            if self.segment_a_cursor.is_none() {
                self.segment_a_cursor = Some(self.segment_cursor(self.entity_a_idx));
            }
            if let Some(first) = self
                .segment_a_cursor
                .as_mut()
                .and_then(SublistSegmentCursor::next)
            {
                self.first_segment = Some(first);
                self.entity_b_idx = self.entity_a_idx;
                self.segment_b_cursor = None;
                return Some(first);
            }

            self.entity_a_idx += 1;
            self.segment_a_cursor = None;
            self.first_segment = None;
            self.entity_b_idx = self.entity_a_idx;
            self.segment_b_cursor = None;
        }
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

            let first = match self.first_segment {
                Some(first) => first,
                None => self.next_first_segment()?,
            };
            let entity_a = self.entities[self.entity_a_idx];
            if self.entity_b_idx < self.entity_a_idx {
                self.entity_b_idx = self.entity_a_idx;
                self.segment_b_cursor = None;
            }

            while self.entity_b_idx < self.entities.len() {
                if self.fixed_to_current_entity && self.entity_b_idx != self.entity_a_idx {
                    break;
                }
                let entity_b = self.entities[self.entity_b_idx];
                if self.segment_b_cursor.is_none() {
                    self.segment_b_cursor = Some(self.segment_cursor(self.entity_b_idx));
                }
                while let Some(second) = self
                    .segment_b_cursor
                    .as_mut()
                    .and_then(SublistSegmentCursor::next)
                {
                    if self.entity_a_idx == self.entity_b_idx {
                        if second.start < first.end {
                            continue;
                        }
                        if first.start == second.start && first.end == second.end {
                            continue;
                        }
                    }
                    if self.element_owners.is_some()
                        && !self.owner_allows_swap(
                            self.entity_a_idx,
                            first,
                            entity_a,
                            self.entity_b_idx,
                            second,
                            entity_b,
                        )
                    {
                        continue;
                    }
                    if entity_a == entity_b
                        && self.precedence_route_graph.as_ref().is_some_and(|graph| {
                            graph.intra_sublist_swap_introduces_cycle(
                                entity_a,
                                first.start,
                                first.end,
                                second.start,
                                second.end,
                            )
                        })
                    {
                        continue;
                    }
                    return Some(self.push_move(entity_a, first, entity_b, second));
                }
                self.entity_b_idx += 1;
                self.segment_b_cursor = None;
            }

            self.first_segment = None;
            self.entity_b_idx = self.entity_a_idx;
            self.segment_b_cursor = None;
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

fn sublist_segments_for_entity(
    entity: usize,
    route_len: usize,
    min_seg: usize,
    max_seg: usize,
    context: MoveStreamContext,
    descriptor_index: usize,
) -> impl Iterator<Item = ListSegment> {
    SublistSegmentCursor::new(
        entity,
        route_len,
        min_seg,
        max_seg,
        context,
        descriptor_index,
    )
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
            element_owner_fn: None,
            precedence_route_hooks: None,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }

    pub fn with_element_owner_fn(
        mut self,
        element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    ) -> Self {
        self.element_owner_fn = element_owner_fn;
        self
    }

    pub(crate) fn with_precedence_route_hooks(
        mut self,
        precedence_route_hooks: Option<PrecedenceRouteHooks<S, V>>,
    ) -> Self {
        self.precedence_route_hooks = precedence_route_hooks;
        self
    }

    pub(crate) fn precedence_route_hooks(&self) -> Option<PrecedenceRouteHooks<S, V>> {
        self.precedence_route_hooks
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
        let owner_restrictions = crate::list_placement::selected_owner_restrictions(
            self.element_owner_fn,
            score_director.working_solution(),
            score_director
                .entity_count(self.descriptor_index)
                .unwrap_or(0),
            &selected.entities,
            &selected.route_lens,
            self.list_get,
        );
        let fixed_to_current_entity = owner_restrictions
            .as_ref()
            .is_some_and(SelectedOwnerRestrictions::is_fixed_to_current);
        let element_owners = owner_restrictions.and_then(SelectedOwnerRestrictions::into_mixed);

        SublistSwapMoveCursor::new(
            selected.entities,
            selected.route_lens,
            context,
            element_owners,
            fixed_to_current_entity,
            None,
            min_seg,
            max_seg,
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
        let Some(owner_restrictions) = crate::list_placement::selected_owner_restrictions(
            self.element_owner_fn,
            score_director.working_solution(),
            score_director
                .entity_count(self.descriptor_index)
                .unwrap_or(0),
            &selected.entities,
            &selected.route_lens,
            self.list_get,
        ) else {
            return unfiltered_sublist_swap_size(
                &selected.route_lens,
                self.min_sublist_size,
                self.max_sublist_size,
            );
        };

        if owner_restrictions.is_fixed_to_current() {
            return selected
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
        }
        let element_owners = owner_restrictions
            .mixed()
            .expect("non-fixed owner restrictions must retain mixed owner matrix");

        let mut count = 0;
        for (left_idx, &left_entity) in selected.entities.iter().enumerate() {
            for left_segment in sublist_segments_for_entity(
                left_entity,
                selected.route_lens[left_idx],
                self.min_sublist_size,
                self.max_sublist_size,
                MoveStreamContext::default(),
                self.descriptor_index,
            ) {
                for (right_idx, &right_entity) in
                    selected.entities.iter().enumerate().skip(left_idx)
                {
                    for right_segment in sublist_segments_for_entity(
                        right_entity,
                        selected.route_lens[right_idx],
                        self.min_sublist_size,
                        self.max_sublist_size,
                        MoveStreamContext::default(),
                        self.descriptor_index,
                    ) {
                        if left_idx == right_idx {
                            if right_segment.start < left_segment.end {
                                continue;
                            }
                            if left_segment.start == right_segment.start
                                && left_segment.end == right_segment.end
                            {
                                continue;
                            }
                        }

                        let allowed = if left_entity == right_entity {
                            selected_segment_allows(
                                element_owners,
                                left_idx,
                                left_segment.start,
                                left_segment.end,
                                left_entity,
                            ) && selected_segment_allows(
                                element_owners,
                                right_idx,
                                right_segment.start,
                                right_segment.end,
                                left_entity,
                            )
                        } else {
                            selected_segment_allows(
                                element_owners,
                                left_idx,
                                left_segment.start,
                                left_segment.end,
                                right_entity,
                            ) && selected_segment_allows(
                                element_owners,
                                right_idx,
                                right_segment.start,
                                right_segment.end,
                                left_entity,
                            )
                        };
                        if allowed {
                            count += 1;
                        }
                    }
                }
            }
        }
        count
    }
}

fn unfiltered_sublist_swap_size(
    route_lens: &[usize],
    min_sublist_size: usize,
    max_sublist_size: usize,
) -> usize {
    let segment_counts: Vec<usize> = route_lens
        .iter()
        .map(|&route_len| count_sublist_segments(route_len, min_sublist_size, max_sublist_size))
        .collect();
    let intra: usize = route_lens
        .iter()
        .map(|&route_len| {
            count_intra_sublist_swap_moves_for_len(route_len, min_sublist_size, max_sublist_size)
        })
        .sum();
    let inter: usize = (0..route_lens.len())
        .flat_map(|left| (left + 1..route_lens.len()).map(move |right| (left, right)))
        .map(|(left, right)| segment_counts[left] * segment_counts[right])
        .sum();
    intra + inter
}
