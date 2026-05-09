/* List swap move selector for element exchange.

Generates `ListSwapMove`s that swap elements within or between list variables.
Useful for inter-route rebalancing in vehicle routing problems.

# Complexity

For n entities with average route length m:
- Intra-entity swaps: O(n * m * (m-1) / 2)
- Inter-entity swaps: O(n² * m²)
- Total: O(n² * m²) worst case (triangular optimization halves constant)

# Example

```
use solverforge_solver::heuristic::selector::list_swap::ListSwapMoveSelector;
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
fn list_get(s: &Solution, entity_idx: usize, pos: usize) -> Option<i32> {
s.vehicles.get(entity_idx).and_then(|v| v.visits.get(pos).copied())
}
fn list_set(s: &mut Solution, entity_idx: usize, pos: usize, val: i32) {
if let Some(v) = s.vehicles.get_mut(entity_idx) {
if let Some(elem) = v.visits.get_mut(pos) { *elem = val; }
}
}

let selector = ListSwapMoveSelector::<Solution, i32, _>::new(
FromSolutionEntitySelector::new(0),
list_len,
list_get,
list_set,
"visits",
0,
);
```
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::ListSwapMove;

use super::entity::EntitySelector;
use super::list_support::{collect_selected_entities, ordered_index};
use super::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};

/// A move selector that generates list swap moves.
///
/// Enumerates all valid (entity_a, pos_a, entity_b, pos_b) pairs for swapping
/// elements within or between list variables. Intra-entity swaps use a
/// triangular iteration to avoid duplicate pairs.
///
/// # Type Parameters
/// * `S` - The solution type
/// * `V` - The list element type
/// * `ES` - The entity selector type
pub struct ListSwapMoveSelector<S, V, ES> {
    entity_selector: ES,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    list_set: fn(&mut S, usize, usize, V),
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

enum ListSwapStage {
    Intra,
    Inter,
}

pub struct ListSwapMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    store: CandidateStore<S, ListSwapMove<S, V>>,
    entities: Vec<usize>,
    route_lens: Vec<usize>,
    context: MoveStreamContext,
    entity_idx: usize,
    stage: ListSwapStage,
    pos_a_offset: usize,
    pos_b_offset: usize,
    dst_idx: usize,
    inter_pos_a_offset: usize,
    inter_pos_b_offset: usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    list_set: fn(&mut S, usize, usize, V),
    variable_name: &'static str,
    descriptor_index: usize,
}

impl<S, V> ListSwapMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    #[allow(clippy::too_many_arguments)]
    fn new(
        entities: Vec<usize>,
        route_lens: Vec<usize>,
        context: MoveStreamContext,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_set: fn(&mut S, usize, usize, V),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            store: CandidateStore::new(),
            entities,
            route_lens,
            context,
            entity_idx: 0,
            stage: ListSwapStage::Intra,
            pos_a_offset: 0,
            pos_b_offset: 0,
            dst_idx: 1,
            inter_pos_a_offset: 0,
            inter_pos_b_offset: 0,
            list_len,
            list_get,
            list_set,
            variable_name,
            descriptor_index,
        }
    }

    fn push_move(
        &mut self,
        entity_a: usize,
        pos_a: usize,
        entity_b: usize,
        pos_b: usize,
    ) -> CandidateId {
        self.store.push(ListSwapMove::new(
            entity_a,
            pos_a,
            entity_b,
            pos_b,
            self.list_len,
            self.list_get,
            self.list_set,
            self.variable_name,
            self.descriptor_index,
        ))
    }

    fn advance_entity(&mut self) {
        self.entity_idx += 1;
        self.stage = ListSwapStage::Intra;
        self.pos_a_offset = 0;
        self.pos_b_offset = 0;
        self.dst_idx = self.entity_idx + 1;
        self.inter_pos_a_offset = 0;
        self.inter_pos_b_offset = 0;
    }
}

impl<S, V> MoveCursor<S, ListSwapMove<S, V>> for ListSwapMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        loop {
            if self.entity_idx >= self.entities.len() {
                return None;
            }

            let entity_a = self.entities[self.entity_idx];
            let len_a = self.route_lens[self.entity_idx];
            if len_a == 0 {
                self.advance_entity();
                continue;
            }

            match self.stage {
                ListSwapStage::Intra => {
                    while self.pos_a_offset < len_a {
                        let pos_a = ordered_index(
                            self.pos_a_offset,
                            len_a,
                            self.context,
                            0x1157_5A09_0000_0002 ^ entity_a as u64 ^ self.descriptor_index as u64,
                        );
                        let pos_b_count = len_a.saturating_sub(pos_a + 1);
                        if self.pos_b_offset < pos_b_count {
                            let pos_b = pos_a
                                + 1
                                + ordered_index(
                                    self.pos_b_offset,
                                    pos_b_count,
                                    self.context,
                                    0x1157_5A09_0000_0003 ^ entity_a as u64 ^ pos_a as u64,
                                );
                            self.pos_b_offset += 1;
                            return Some(self.push_move(entity_a, pos_a, entity_a, pos_b));
                        }
                        self.pos_a_offset += 1;
                        self.pos_b_offset = 0;
                    }
                    self.stage = ListSwapStage::Inter;
                    self.dst_idx = self.entity_idx + 1;
                    self.inter_pos_a_offset = 0;
                    self.inter_pos_b_offset = 0;
                }
                ListSwapStage::Inter => {
                    while self.dst_idx < self.entities.len() {
                        let entity_b = self.entities[self.dst_idx];
                        let len_b = self.route_lens[self.dst_idx];
                        if len_b == 0 {
                            self.dst_idx += 1;
                            continue;
                        }

                        while self.inter_pos_a_offset < len_a {
                            let pos_a = ordered_index(
                                self.inter_pos_a_offset,
                                len_a,
                                self.context,
                                0x1157_5A09_0000_0004 ^ entity_a as u64 ^ entity_b as u64,
                            );
                            if self.inter_pos_b_offset < len_b {
                                let pos_b = ordered_index(
                                    self.inter_pos_b_offset,
                                    len_b,
                                    self.context,
                                    0x1157_5A09_0000_0005
                                        ^ entity_a as u64
                                        ^ entity_b as u64
                                        ^ pos_a as u64,
                                );
                                self.inter_pos_b_offset += 1;
                                return Some(self.push_move(entity_a, pos_a, entity_b, pos_b));
                            }
                            self.inter_pos_a_offset += 1;
                            self.inter_pos_b_offset = 0;
                        }
                        self.dst_idx += 1;
                        self.inter_pos_a_offset = 0;
                        self.inter_pos_b_offset = 0;
                    }
                    self.advance_entity();
                }
            }
        }
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, ListSwapMove<S, V>>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> ListSwapMove<S, V> {
        self.store.take_candidate(id)
    }
}

impl<S, V> Iterator for ListSwapMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    type Item = ListSwapMove<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.next_candidate()?;
        Some(self.take_candidate(id))
    }
}

impl<S, V: Debug, ES: Debug> Debug for ListSwapMoveSelector<S, V, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListSwapMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V, ES> ListSwapMoveSelector<S, V, ES> {
    /// Creates a new list swap move selector.
    ///
    /// # Arguments
    /// * `entity_selector` - Selects entities to consider for swaps
    /// * `list_len` - Function to get list length for an entity
    /// * `list_get` - Function to get element at position
    /// * `list_set` - Function to set element at position
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    pub fn new(
        entity_selector: ES,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_set: fn(&mut S, usize, usize, V),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_selector,
            list_len,
            list_get,
            list_set,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V, ES> MoveSelector<S, ListSwapMove<S, V>> for ListSwapMoveSelector<S, V, ES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
{
    type Cursor<'a>
        = ListSwapMoveCursor<S, V>
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
            0x1157_5A09_0000_0001 ^ self.descriptor_index as u64,
        );
        ListSwapMoveCursor::new(
            selected.entities,
            selected.route_lens,
            context,
            self.list_len,
            self.list_get,
            self.list_set,
            self.variable_name,
            self.descriptor_index,
        )
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        collect_selected_entities(&self.entity_selector, score_director, self.list_len)
            .list_swap_move_capacity()
    }
}
