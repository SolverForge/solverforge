/* Sublist change move selector for segment relocation (Or-opt).

Generates `SublistChangeMove`s that relocate contiguous segments within or
between list variables. The Or-opt family of moves (segments of size 1, 2, 3, …)
is among the most effective VRP improvements after basic 2-opt.

# Complexity

For n entities with average route length m and max segment size k:
- Intra-entity: O(n * m * k) sources × O(m) destinations
- Inter-entity: O(n * m * k) sources × O(n * m) destinations
- Total: O(n² * m² * k)

Use a forager that quits early (`FirstAccepted`, `AcceptedCount`) to keep
iteration practical for large instances.

# Example

```
use solverforge_solver::heuristic::selector::sublist_change::SublistChangeMoveSelector;
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

// Or-opt: relocate segments of size 1..=3
let selector = SublistChangeMoveSelector::<Solution, i32, _>::new(
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

use crate::heuristic::r#move::SublistChangeMove;

use super::entity::EntitySelector;
use super::list_support::{collect_selected_entities, ordered_index};
use super::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};
use super::sublist_support::count_sublist_change_moves_for_len;

/// A move selector that generates sublist change (Or-opt) moves.
///
/// For each source entity and each valid segment `[start, start+len)`, generates
/// moves that insert the segment at every valid destination position in every
/// entity (including the source entity for intra-route relocation).
///
/// # Type Parameters
/// * `S` - The solution type
/// * `V` - The list element type
/// * `ES` - The entity selector type
pub struct SublistChangeMoveSelector<S, V, ES> {
    entity_selector: ES,
    // Minimum segment size (inclusive). Usually 1.
    min_sublist_size: usize,
    // Maximum segment size (inclusive). Usually 3-5.
    max_sublist_size: usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

enum SublistChangeStage {
    Intra,
    Inter,
}

pub struct SublistChangeMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    store: CandidateStore<S, SublistChangeMove<S, V>>,
    entities: Vec<usize>,
    route_lens: Vec<usize>,
    context: MoveStreamContext,
    src_idx: usize,
    seg_start_offset: usize,
    seg_size_offset: usize,
    stage: SublistChangeStage,
    intra_dst_offset: usize,
    dst_idx: usize,
    inter_dst_offset: usize,
    min_seg: usize,
    max_seg: usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    variable_name: &'static str,
    descriptor_index: usize,
}

impl<S, V> SublistChangeMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    #[allow(clippy::too_many_arguments)]
    fn new(
        entities: Vec<usize>,
        route_lens: Vec<usize>,
        context: MoveStreamContext,
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
            src_idx: 0,
            seg_start_offset: 0,
            seg_size_offset: 0,
            stage: SublistChangeStage::Intra,
            intra_dst_offset: 0,
            dst_idx: 0,
            inter_dst_offset: 0,
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

    fn segment_size_count(&self, src_len: usize, seg_start: usize) -> usize {
        let max_valid = self.max_seg.min(src_len.saturating_sub(seg_start));
        max_valid.saturating_sub(self.min_seg) + usize::from(max_valid >= self.min_seg)
    }

    fn current_segment(&self) -> Option<(usize, usize, usize, usize, usize)> {
        let src_entity = *self.entities.get(self.src_idx)?;
        let src_len = self.route_lens[self.src_idx];
        if src_len < self.min_seg {
            return Some((src_entity, src_len, 0, 0, 0));
        }
        let seg_start = ordered_index(
            self.seg_start_offset,
            src_len,
            self.context,
            0x5B15_7C4A_46E0_0002 ^ src_entity as u64 ^ self.descriptor_index as u64,
        );
        let size_count = self.segment_size_count(src_len, seg_start);
        if size_count == 0 {
            return Some((src_entity, src_len, seg_start, 0, 0));
        }
        let size_offset = ordered_index(
            self.seg_size_offset,
            size_count,
            self.context,
            0x5B15_7C4A_46E0_0003 ^ src_entity as u64 ^ seg_start as u64,
        );
        let seg_size = self.min_seg + size_offset;
        Some((
            src_entity,
            src_len,
            seg_start,
            seg_start + seg_size,
            seg_size,
        ))
    }

    fn advance_segment(&mut self) {
        let Some((_, src_len, seg_start, _, _)) = self.current_segment() else {
            return;
        };
        let size_count = self.segment_size_count(src_len, seg_start);
        self.seg_size_offset += 1;
        if self.seg_size_offset >= size_count {
            self.seg_size_offset = 0;
            self.seg_start_offset += 1;
        }
        while self.src_idx < self.route_lens.len()
            && self.seg_start_offset >= self.route_lens[self.src_idx]
        {
            self.src_idx += 1;
            self.seg_start_offset = 0;
            self.seg_size_offset = 0;
        }
        self.stage = SublistChangeStage::Intra;
        self.intra_dst_offset = 0;
        self.dst_idx = 0;
        self.inter_dst_offset = 0;
    }

    fn push_move(
        &mut self,
        src_entity: usize,
        seg_start: usize,
        seg_end: usize,
        dst_entity: usize,
        dst_pos: usize,
    ) -> CandidateId {
        self.store.push(SublistChangeMove::new(
            src_entity,
            seg_start,
            seg_end,
            dst_entity,
            dst_pos,
            self.list_len,
            self.list_get,
            self.sublist_remove,
            self.sublist_insert,
            self.variable_name,
            self.descriptor_index,
        ))
    }
}

impl<S, V> MoveCursor<S, SublistChangeMove<S, V>> for SublistChangeMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        loop {
            let (src_entity, src_len, seg_start, seg_end, seg_size) = self.current_segment()?;
            if src_len < self.min_seg || seg_size == 0 {
                self.advance_segment();
                continue;
            }

            match self.stage {
                SublistChangeStage::Intra => {
                    let post_removal_len = src_len - seg_size;
                    while self.intra_dst_offset <= post_removal_len {
                        let dst_pos = ordered_index(
                            self.intra_dst_offset,
                            post_removal_len + 1,
                            self.context,
                            0x5B15_7C4A_46E0_0004 ^ src_entity as u64 ^ seg_start as u64,
                        );
                        self.intra_dst_offset += 1;
                        if dst_pos == seg_start {
                            continue;
                        }
                        return Some(
                            self.push_move(src_entity, seg_start, seg_end, src_entity, dst_pos),
                        );
                    }
                    self.stage = SublistChangeStage::Inter;
                    self.dst_idx = 0;
                    self.inter_dst_offset = 0;
                }
                SublistChangeStage::Inter => {
                    while self.dst_idx < self.entities.len() {
                        if self.dst_idx == self.src_idx {
                            self.dst_idx += 1;
                            continue;
                        }
                        let dst_entity = self.entities[self.dst_idx];
                        let dst_len = self.route_lens[self.dst_idx];
                        if self.inter_dst_offset <= dst_len {
                            let dst_pos = ordered_index(
                                self.inter_dst_offset,
                                dst_len + 1,
                                self.context,
                                0x5B15_7C4A_46E0_0005
                                    ^ src_entity as u64
                                    ^ dst_entity as u64
                                    ^ seg_start as u64,
                            );
                            self.inter_dst_offset += 1;
                            return Some(
                                self.push_move(src_entity, seg_start, seg_end, dst_entity, dst_pos),
                            );
                        }
                        self.dst_idx += 1;
                        self.inter_dst_offset = 0;
                    }
                    self.advance_segment();
                }
            }
        }
    }

    fn candidate(
        &self,
        id: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, SublistChangeMove<S, V>>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> SublistChangeMove<S, V> {
        self.store.take_candidate(id)
    }
}

impl<S, V> Iterator for SublistChangeMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    type Item = SublistChangeMove<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.next_candidate()?;
        Some(self.take_candidate(id))
    }
}

impl<S, V: Debug, ES: Debug> Debug for SublistChangeMoveSelector<S, V, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SublistChangeMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("min_sublist_size", &self.min_sublist_size)
            .field("max_sublist_size", &self.max_sublist_size)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V, ES> SublistChangeMoveSelector<S, V, ES> {
    /* Creates a new sublist change move selector.

    # Arguments
    * `entity_selector` - Selects entities to generate moves for
    * `min_sublist_size` - Minimum segment length (must be ≥ 1)
    * `max_sublist_size` - Maximum segment length
    * `list_len` - Function to get list length
    * `sublist_remove` - Function to drain a range `[start, end)`, returning removed elements
    * `sublist_insert` - Function to insert a slice at a position
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

impl<S, V, ES> MoveSelector<S, SublistChangeMove<S, V>> for SublistChangeMoveSelector<S, V, ES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
{
    type Cursor<'a>
        = SublistChangeMoveCursor<S, V>
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
        let mut selected =
            collect_selected_entities(&self.entity_selector, score_director, self.list_len);
        selected.apply_stream_order(
            context,
            0x5B15_7C4A_46E0_0001 ^ self.descriptor_index as u64,
        );
        SublistChangeMoveCursor::new(
            selected.entities,
            selected.route_lens,
            context,
            self.min_sublist_size,
            self.max_sublist_size,
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
        let total_elements = selected.route_lens.iter().sum::<usize>();
        let entity_count = selected.entities.len();

        selected
            .route_lens
            .iter()
            .map(|&route_len| {
                let inter_destinations =
                    total_elements.saturating_sub(route_len) + entity_count.saturating_sub(1);
                count_sublist_change_moves_for_len(
                    route_len,
                    inter_destinations,
                    self.min_sublist_size,
                    self.max_sublist_size,
                )
            })
            .sum()
    }
}
