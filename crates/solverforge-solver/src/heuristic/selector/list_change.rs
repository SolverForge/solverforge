/* List change move selector for element relocation.

Generates `ListChangeMove`s that relocate elements within or between list variables.
Essential for vehicle routing and scheduling problems.

# Example

```
use solverforge_solver::heuristic::selector::list_change::ListChangeMoveSelector;
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
fn list_remove(s: &mut Solution, entity_idx: usize, pos: usize) -> Option<i32> {
s.vehicles.get_mut(entity_idx).map(|v| v.visits.remove(pos))
}
fn list_insert(s: &mut Solution, entity_idx: usize, pos: usize, val: i32) {
if let Some(v) = s.vehicles.get_mut(entity_idx) { v.visits.insert(pos, val); }
}

let selector = ListChangeMoveSelector::<Solution, i32, _>::new(
FromSolutionEntitySelector::new(0),
list_len,
list_remove,
list_insert,
"visits",
0,
);
```
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::ListChangeMove;

use super::entity::EntitySelector;
use super::list_support::{collect_selected_entities, ordered_index};
use super::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};

/// A move selector that generates list change moves.
///
/// Enumerates all valid (source_entity, source_pos, dest_entity, dest_pos)
/// combinations for relocating elements within or between list variables.
///
/// # Type Parameters
/// * `S` - The solution type
/// * `V` - The list element type
///
/// # Complexity
///
/// For n entities with average route length m:
/// - Intra-entity moves: O(n * m * m)
/// - Inter-entity moves: O(n * n * m * m)
/// - Total: O(n² * m²) worst case
///
/// Use with a forager that quits early for better performance.
pub struct ListChangeMoveSelector<S, V, ES> {
    // Selects entities (vehicles) for moves.
    entity_selector: ES,
    list_len: fn(&S, usize) -> usize,
    // Read element by position for exact move/value tabu metadata.
    list_get: fn(&S, usize, usize) -> Option<V>,
    // Remove element at position.
    list_remove: fn(&mut S, usize, usize) -> Option<V>,
    // Insert element at position.
    list_insert: fn(&mut S, usize, usize, V),
    // Variable name for notifications.
    variable_name: &'static str,
    // Entity descriptor index.
    descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

enum ListChangeStage {
    Intra,
    Inter,
}

pub struct ListChangeMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    store: CandidateStore<S, ListChangeMove<S, V>>,
    entities: Vec<usize>,
    route_lens: Vec<usize>,
    context: MoveStreamContext,
    src_idx: usize,
    src_pos_offset: usize,
    stage: ListChangeStage,
    intra_dst_offset: usize,
    dst_idx: usize,
    inter_dst_pos_offset: usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    list_remove: fn(&mut S, usize, usize) -> Option<V>,
    list_insert: fn(&mut S, usize, usize, V),
    variable_name: &'static str,
    descriptor_index: usize,
}

impl<S, V> ListChangeMoveCursor<S, V>
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
        list_remove: fn(&mut S, usize, usize) -> Option<V>,
        list_insert: fn(&mut S, usize, usize, V),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            store: CandidateStore::new(),
            entities,
            route_lens,
            context,
            src_idx: 0,
            src_pos_offset: 0,
            stage: ListChangeStage::Intra,
            intra_dst_offset: 0,
            dst_idx: 0,
            inter_dst_pos_offset: 0,
            list_len,
            list_get,
            list_remove,
            list_insert,
            variable_name,
            descriptor_index,
        }
    }

    fn current_source(&self) -> Option<(usize, usize, usize)> {
        let src_entity = *self.entities.get(self.src_idx)?;
        let src_len = self.route_lens[self.src_idx];
        if src_len == 0 {
            return Some((src_entity, src_len, 0));
        }
        let src_pos = ordered_index(
            self.src_pos_offset,
            src_len,
            self.context,
            0x1157_C4A4_6E00_0002 ^ src_entity as u64 ^ self.descriptor_index as u64,
        );
        Some((src_entity, src_len, src_pos))
    }

    fn advance_source_position(&mut self) {
        self.src_pos_offset += 1;
        self.stage = ListChangeStage::Intra;
        self.intra_dst_offset = 0;
        self.dst_idx = 0;
        self.inter_dst_pos_offset = 0;

        while self.src_idx < self.route_lens.len()
            && self.src_pos_offset >= self.route_lens[self.src_idx]
        {
            self.src_idx += 1;
            self.src_pos_offset = 0;
        }
    }

    fn push_move(
        &mut self,
        src_entity: usize,
        src_pos: usize,
        dst_entity: usize,
        dst_pos: usize,
    ) -> CandidateId {
        self.store.push(ListChangeMove::new(
            src_entity,
            src_pos,
            dst_entity,
            dst_pos,
            self.list_len,
            self.list_get,
            self.list_remove,
            self.list_insert,
            self.variable_name,
            self.descriptor_index,
        ))
    }
}

impl<S, V> MoveCursor<S, ListChangeMove<S, V>> for ListChangeMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        loop {
            let (src_entity, src_len, src_pos) = self.current_source()?;
            if src_len == 0 {
                self.src_idx += 1;
                continue;
            }

            match self.stage {
                ListChangeStage::Intra => {
                    while self.intra_dst_offset < src_len {
                        let dst_pos = ordered_index(
                            self.intra_dst_offset,
                            src_len,
                            self.context,
                            0x1157_C4A4_6E00_0003 ^ src_entity as u64 ^ src_pos as u64,
                        );
                        self.intra_dst_offset += 1;
                        if src_pos == dst_pos || dst_pos == src_pos + 1 {
                            continue;
                        }
                        return Some(self.push_move(src_entity, src_pos, src_entity, dst_pos));
                    }
                    self.stage = ListChangeStage::Inter;
                    self.dst_idx = 0;
                    self.inter_dst_pos_offset = 0;
                }
                ListChangeStage::Inter => {
                    while self.dst_idx < self.entities.len() {
                        if self.dst_idx == self.src_idx {
                            self.dst_idx += 1;
                            self.inter_dst_pos_offset = 0;
                            continue;
                        }
                        let dst_entity = self.entities[self.dst_idx];
                        let dst_len = self.route_lens[self.dst_idx];
                        if self.inter_dst_pos_offset <= dst_len {
                            let dst_pos = ordered_index(
                                self.inter_dst_pos_offset,
                                dst_len + 1,
                                self.context,
                                0x1157_C4A4_6E00_0004
                                    ^ src_entity as u64
                                    ^ dst_entity as u64
                                    ^ src_pos as u64,
                            );
                            self.inter_dst_pos_offset += 1;
                            return Some(self.push_move(src_entity, src_pos, dst_entity, dst_pos));
                        }
                        self.dst_idx += 1;
                        self.inter_dst_pos_offset = 0;
                    }
                    self.advance_source_position();
                }
            }
        }
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, ListChangeMove<S, V>>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> ListChangeMove<S, V> {
        self.store.take_candidate(id)
    }
}

impl<S, V> Iterator for ListChangeMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    type Item = ListChangeMove<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.next_candidate()?;
        Some(self.take_candidate(id))
    }
}

impl<S, V: Debug, ES: Debug> Debug for ListChangeMoveSelector<S, V, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListChangeMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V, ES> ListChangeMoveSelector<S, V, ES> {
    /// Creates a new list change move selector.
    ///
    /// # Arguments
    /// * `entity_selector` - Selects entities to consider for moves
    /// * `list_len` - Function to get list length for an entity
    /// * `list_remove` - Function to remove element at position
    /// * `list_insert` - Function to insert element at position
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    pub fn new(
        entity_selector: ES,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_remove: fn(&mut S, usize, usize) -> Option<V>,
        list_insert: fn(&mut S, usize, usize, V),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_selector,
            list_len,
            list_get,
            list_remove,
            list_insert,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V, ES> MoveSelector<S, ListChangeMove<S, V>> for ListChangeMoveSelector<S, V, ES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
{
    type Cursor<'a>
        = ListChangeMoveCursor<S, V>
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
            0x1157_C4A4_6E00_0001 ^ self.descriptor_index as u64,
        );
        ListChangeMoveCursor::new(
            selected.entities,
            selected.route_lens,
            context,
            self.list_len,
            self.list_get,
            self.list_remove,
            self.list_insert,
            self.variable_name,
            self.descriptor_index,
        )
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        collect_selected_entities(&self.entity_selector, score_director, self.list_len)
            .list_change_move_capacity()
    }
}
