/* List reverse move selector for 2-opt optimization.

Generates `ListReverseMove`s that reverse contiguous segments within a single
list. This is the fundamental 2-opt move for TSP and VRP: reversing a segment
of the tour can eliminate crossing edges and reduce total distance.

For VRP, 2-opt is applied independently within each route (intra-route 2-opt).
Cross-route 2-opt would require inter-entity reversal, which is a different
operation modeled by `SublistSwapMove` with same-size segments.

# Complexity

For n entities with average route length m:
O(n * m²) — all (start, end) pairs per entity where end > start + 1.

# Example

```
use solverforge_solver::heuristic::selector::list_reverse::ListReverseMoveSelector;
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
fn list_reverse(s: &mut Solution, entity_idx: usize, start: usize, end: usize) {
if let Some(v) = s.vehicles.get_mut(entity_idx) {
v.visits[start..end].reverse();
}
}

let selector = ListReverseMoveSelector::<Solution, i32, _>::new(
FromSolutionEntitySelector::new(0),
list_len,
list_reverse,
"visits",
0,
);
```
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::ListReverseMove;

use super::entity::EntitySelector;
use super::list_support::ordered_index;
use super::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};

/// A move selector that generates 2-opt segment reversal moves.
///
/// For each entity, enumerates all valid (start, end) pairs where
/// `end > start + 1` (at least 2 elements in the reversed segment).
///
/// # Type Parameters
/// * `S` - The solution type
/// * `V` - The list element type (phantom — only used for type safety)
/// * `ES` - The entity selector type
pub struct ListReverseMoveSelector<S, V, ES> {
    entity_selector: ES,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    list_reverse: fn(&mut S, usize, usize, usize),
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

pub struct ListReverseMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    store: CandidateStore<S, ListReverseMove<S, V>>,
    entities: Vec<usize>,
    route_lens: Vec<usize>,
    context: MoveStreamContext,
    entity_idx: usize,
    start_offset: usize,
    end_offset: usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    list_reverse: fn(&mut S, usize, usize, usize),
    variable_name: &'static str,
    descriptor_index: usize,
}

impl<S, V> ListReverseMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    #[allow(clippy::too_many_arguments)]
    fn new(
        entities: Vec<usize>,
        route_lens: Vec<usize>,
        context: MoveStreamContext,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_reverse: fn(&mut S, usize, usize, usize),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            store: CandidateStore::new(),
            entities,
            route_lens,
            context,
            entity_idx: 0,
            start_offset: 0,
            end_offset: 0,
            list_len,
            list_get,
            list_reverse,
            variable_name,
            descriptor_index,
        }
    }

    fn push_move(&mut self, entity: usize, start: usize, end: usize) -> CandidateId {
        self.store.push(ListReverseMove::new(
            entity,
            start,
            end,
            self.list_len,
            self.list_get,
            self.list_reverse,
            self.variable_name,
            self.descriptor_index,
        ))
    }
}

impl<S, V> MoveCursor<S, ListReverseMove<S, V>> for ListReverseMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        loop {
            let entity = *self.entities.get(self.entity_idx)?;
            let len = self.route_lens[self.entity_idx];
            if len < 2 {
                self.entity_idx += 1;
                self.start_offset = 0;
                self.end_offset = 0;
                continue;
            }

            while self.start_offset < len {
                let start = ordered_index(
                    self.start_offset,
                    len,
                    self.context,
                    0x1157_2A07_0000_0002 ^ entity as u64 ^ self.descriptor_index as u64,
                );
                let end_count = len.saturating_sub(start + 1);
                if self.end_offset < end_count {
                    let end = start
                        + 2
                        + ordered_index(
                            self.end_offset,
                            end_count,
                            self.context,
                            0x1157_2A07_0000_0003 ^ entity as u64 ^ start as u64,
                        );
                    self.end_offset += 1;
                    return Some(self.push_move(entity, start, end));
                }
                self.start_offset += 1;
                self.end_offset = 0;
            }

            self.entity_idx += 1;
            self.start_offset = 0;
            self.end_offset = 0;
        }
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, ListReverseMove<S, V>>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> ListReverseMove<S, V> {
        self.store.take_candidate(id)
    }
}

impl<S, V> Iterator for ListReverseMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    type Item = ListReverseMove<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.next_candidate()?;
        Some(self.take_candidate(id))
    }
}

impl<S, V: Debug, ES: Debug> Debug for ListReverseMoveSelector<S, V, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListReverseMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V, ES> ListReverseMoveSelector<S, V, ES> {
    /// Creates a new list reverse move selector.
    ///
    /// # Arguments
    /// * `entity_selector` - Selects entities (routes) to apply 2-opt to
    /// * `list_len` - Function to get route length
    /// * `list_reverse` - Function to reverse `[start, end)` in-place
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    pub fn new(
        entity_selector: ES,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_reverse: fn(&mut S, usize, usize, usize),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_selector,
            list_len,
            list_get,
            list_reverse,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V, ES> MoveSelector<S, ListReverseMove<S, V>> for ListReverseMoveSelector<S, V, ES>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
{
    type Cursor<'a>
        = ListReverseMoveCursor<S, V>
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
        let mut entities: Vec<usize> = self
            .entity_selector
            .iter(score_director)
            .map(|r| r.entity_index)
            .collect();
        let entity_start = context.start_offset(
            entities.len(),
            0x1157_2A07_0000_0001 ^ self.descriptor_index as u64,
        );
        entities.rotate_left(entity_start);

        let solution = score_director.working_solution();
        let route_lens = entities
            .iter()
            .map(|&entity| (self.list_len)(solution, entity))
            .collect();

        ListReverseMoveCursor::new(
            entities,
            route_lens,
            context,
            self.list_len,
            self.list_get,
            self.list_reverse,
            self.variable_name,
            self.descriptor_index,
        )
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        let solution = score_director.working_solution();
        let list_len = self.list_len;

        self.entity_selector
            .iter(score_director)
            .map(|r| {
                let m = list_len(solution, r.entity_index);
                // Number of valid (start, end) pairs: m*(m-1)/2 - m = m*(m-1)/2 - m
                // For start in 0..m, end in start+2..=m: sum = m*(m-1)/2
                if m >= 2 {
                    m * (m - 1) / 2
                } else {
                    0
                }
            })
            .sum()
    }
}
