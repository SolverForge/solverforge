/* Nearby list swap move selector for distance-pruned element exchange.

A distance-biased variant of [`ListSwapMoveSelector`] that only considers
swap partners within a configurable distance of the source element.
Reduces the move space from O(n²m²) to O(nm × k).

# Example

```
use solverforge_solver::heuristic::selector::nearby_list_swap::NearbyListSwapMoveSelector;
use solverforge_solver::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use solverforge_solver::heuristic::selector::entity::FromSolutionEntitySelector;
use solverforge_solver::heuristic::selector::MoveSelector;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SoftScore;

#[derive(Clone, Debug)]
struct Visit { x: f64, y: f64 }

#[derive(Clone, Debug)]
struct Vehicle { visits: Vec<Visit> }

#[derive(Clone, Debug)]
struct Solution { vehicles: Vec<Vehicle>, score: Option<SoftScore> }

impl PlanningSolution for Solution {
type Score = SoftScore;
fn score(&self) -> Option<Self::Score> { self.score }
fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
}

fn list_len(s: &Solution, e: usize) -> usize {
s.vehicles.get(e).map_or(0, |v| v.visits.len())
}
fn list_get(s: &Solution, e: usize, pos: usize) -> Option<Visit> {
s.vehicles.get(e).and_then(|v| v.visits.get(pos).cloned())
}
fn list_set(s: &mut Solution, e: usize, pos: usize, val: Visit) {
if let Some(v) = s.vehicles.get_mut(e) {
if let Some(elem) = v.visits.get_mut(pos) { *elem = val; }
}
}

#[derive(Debug)]
struct EuclideanMeter;

impl CrossEntityDistanceMeter<Solution> for EuclideanMeter {
fn distance(
&self,
solution: &Solution,
src_entity: usize, src_pos: usize,
dst_entity: usize, dst_pos: usize,
) -> f64 {
let src = &solution.vehicles[src_entity].visits[src_pos];
let dst = &solution.vehicles[dst_entity].visits[dst_pos];
let dx = src.x - dst.x;
let dy = src.y - dst.y;
(dx * dx + dy * dy).sqrt()
}
}

let selector = NearbyListSwapMoveSelector::<Solution, Visit, _, _>::new(
FromSolutionEntitySelector::new(0),
EuclideanMeter,
10,
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
use super::nearby_list_change::CrossEntityDistanceMeter;
use super::nearby_list_support::{sort_and_limit_nearby_candidates, NearbyCandidate};

/// A distance-pruned list swap move selector.
///
/// For each source (entity, position), generates swap moves only to the
/// `max_nearby` nearest positions (measured by `CrossEntityDistanceMeter`).
///
/// # Type Parameters
/// * `S` - The solution type
/// * `V` - The list element type
/// * `D` - The cross-entity distance meter type
/// * `ES` - The entity selector type
pub struct NearbyListSwapMoveSelector<S, V, D, ES> {
    entity_selector: ES,
    distance_meter: D,
    max_nearby: usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    list_set: fn(&mut S, usize, usize, V),
    element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

pub struct NearbyListSwapMoveCursor<'a, S, V, D>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    D: CrossEntityDistanceMeter<S>,
{
    store: CandidateStore<S, ListSwapMove<S, V>>,
    solution: S,
    distance_meter: &'a D,
    entities: Vec<usize>,
    route_lens: Vec<usize>,
    context: MoveStreamContext,
    source_idx: usize,
    source_pos_offset: usize,
    current_source: Option<(usize, usize)>,
    destinations: Vec<(usize, usize)>,
    destination_offset: usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    list_set: fn(&mut S, usize, usize, V),
    owner_context: Option<(fn(&S, &V) -> Option<usize>, usize)>,
    max_nearby: usize,
    variable_name: &'static str,
    descriptor_index: usize,
}

impl<'a, S, V, D> NearbyListSwapMoveCursor<'a, S, V, D>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    D: CrossEntityDistanceMeter<S>,
{
    #[allow(clippy::too_many_arguments)]
    fn new(
        solution: S,
        distance_meter: &'a D,
        entities: Vec<usize>,
        route_lens: Vec<usize>,
        context: MoveStreamContext,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_set: fn(&mut S, usize, usize, V),
        owner_context: Option<(fn(&S, &V) -> Option<usize>, usize)>,
        max_nearby: usize,
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            store: CandidateStore::new(),
            solution,
            distance_meter,
            entities,
            route_lens,
            context,
            source_idx: 0,
            source_pos_offset: 0,
            current_source: None,
            destinations: Vec::new(),
            destination_offset: 0,
            list_len,
            list_get,
            list_set,
            owner_context,
            max_nearby,
            variable_name,
            descriptor_index,
        }
    }

    fn load_next_source(&mut self) -> bool {
        let mut candidates: Vec<NearbyCandidate> = Vec::new();

        while self.source_idx < self.entities.len() {
            let src_entity = self.entities[self.source_idx];
            let src_len = self.route_lens[self.source_idx];
            if src_len == 0 {
                self.source_idx += 1;
                self.source_pos_offset = 0;
                continue;
            }

            while self.source_pos_offset < src_len {
                let src_pos = ordered_index(
                    self.source_pos_offset,
                    src_len,
                    self.context,
                    0xA1EA_25A0_9000_0002 ^ src_entity as u64 ^ self.descriptor_index as u64,
                );
                self.source_pos_offset += 1;

                let source_restriction = if let Some((owner_fn, entity_count)) = self.owner_context
                {
                    let Some(source_element) = (self.list_get)(&self.solution, src_entity, src_pos)
                    else {
                        continue;
                    };
                    Some(crate::list_placement::owner_restriction(
                        Some(owner_fn),
                        &self.solution,
                        entity_count,
                        &source_element,
                    ))
                } else {
                    None
                };

                candidates.clear();

                for dst_pos in src_pos + 1..src_len {
                    if let Some((owner_fn, entity_count)) = self.owner_context {
                        let Some(dst_element) =
                            (self.list_get)(&self.solution, src_entity, dst_pos)
                        else {
                            continue;
                        };
                        let dst_restriction = crate::list_placement::owner_restriction(
                            Some(owner_fn),
                            &self.solution,
                            entity_count,
                            &dst_element,
                        );
                        if source_restriction
                            .is_some_and(|restriction| !restriction.allows(src_entity))
                            || !dst_restriction.allows(src_entity)
                        {
                            continue;
                        }
                    }
                    let dist = self.distance_meter.distance(
                        &self.solution,
                        src_entity,
                        src_pos,
                        src_entity,
                        dst_pos,
                    );
                    if dist.is_finite() {
                        candidates.push((src_entity, dst_pos, dist));
                    }
                }

                for (dst_idx, &dst_entity) in self.entities.iter().enumerate() {
                    if dst_idx <= self.source_idx {
                        continue;
                    }
                    let dst_len = self.route_lens[dst_idx];
                    if dst_len == 0 {
                        continue;
                    }

                    for dst_pos in 0..dst_len {
                        if let Some((owner_fn, entity_count)) = self.owner_context {
                            let Some(dst_element) =
                                (self.list_get)(&self.solution, dst_entity, dst_pos)
                            else {
                                continue;
                            };
                            let dst_restriction = crate::list_placement::owner_restriction(
                                Some(owner_fn),
                                &self.solution,
                                entity_count,
                                &dst_element,
                            );
                            if source_restriction
                                .is_some_and(|restriction| !restriction.allows(dst_entity))
                                || !dst_restriction.allows(src_entity)
                            {
                                continue;
                            }
                        }
                        let dist = self.distance_meter.distance(
                            &self.solution,
                            src_entity,
                            src_pos,
                            dst_entity,
                            dst_pos,
                        );
                        if dist.is_finite() {
                            candidates.push((dst_entity, dst_pos, dist));
                        }
                    }
                }

                sort_and_limit_nearby_candidates(&mut candidates, self.max_nearby);
                if candidates.is_empty() {
                    continue;
                }

                self.current_source = Some((src_entity, src_pos));
                self.destinations.clear();
                self.destinations.extend(
                    candidates
                        .iter()
                        .map(|&(dst_entity, dst_pos, _)| (dst_entity, dst_pos)),
                );
                self.destination_offset = 0;
                return true;
            }

            self.source_idx += 1;
            self.source_pos_offset = 0;
        }

        false
    }
}

impl<S, V, D> MoveCursor<S, ListSwapMove<S, V>> for NearbyListSwapMoveCursor<'_, S, V, D>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    D: CrossEntityDistanceMeter<S>,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        loop {
            if self.destination_offset >= self.destinations.len() && !self.load_next_source() {
                return None;
            }
            let Some((src_entity, src_pos)) = self.current_source else {
                continue;
            };
            let (dst_entity, dst_pos) = self.destinations[self.destination_offset];
            self.destination_offset += 1;
            return Some(self.store.push(ListSwapMove::new(
                src_entity,
                src_pos,
                dst_entity,
                dst_pos,
                self.list_len,
                self.list_get,
                self.list_set,
                self.variable_name,
                self.descriptor_index,
            )));
        }
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, ListSwapMove<S, V>>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> ListSwapMove<S, V> {
        self.store.take_candidate(id)
    }
}

impl<S, V, D> Iterator for NearbyListSwapMoveCursor<'_, S, V, D>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    D: CrossEntityDistanceMeter<S>,
{
    type Item = ListSwapMove<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.next_candidate()?;
        Some(self.take_candidate(id))
    }
}

impl<S, V: Debug, D, ES: Debug> Debug for NearbyListSwapMoveSelector<S, V, D, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NearbyListSwapMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("distance_meter", &"<distance_meter>")
            .field("max_nearby", &self.max_nearby)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V, D, ES> NearbyListSwapMoveSelector<S, V, D, ES> {
    /* Creates a new nearby list swap move selector.

    # Arguments
    * `entity_selector` - Selects entities to consider for swaps
    * `distance_meter` - Measures distance between position pairs
    * `max_nearby` - Maximum partner positions to consider per source
    * `list_len` - Function to get list length
    * `list_get` - Function to get element at position
    * `list_set` - Function to set element at position
    * `variable_name` - Name of the list variable
    * `descriptor_index` - Entity descriptor index
    */
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_selector: ES,
        distance_meter: D,
        max_nearby: usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_set: fn(&mut S, usize, usize, V),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_selector,
            distance_meter,
            max_nearby,
            list_len,
            list_get,
            list_set,
            element_owner_fn: None,
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
}

impl<S, V, D, ES> MoveSelector<S, ListSwapMove<S, V>> for NearbyListSwapMoveSelector<S, V, D, ES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    D: CrossEntityDistanceMeter<S>,
    ES: EntitySelector<S>,
{
    type Cursor<'a>
        = NearbyListSwapMoveCursor<'a, S, V, D>
    where
        Self: 'a;

    fn open_cursor<'a, SD: Director<S>>(&'a self, score_director: &SD) -> Self::Cursor<'a> {
        self.open_cursor_with_context(score_director, MoveStreamContext::default())
    }

    fn open_cursor_with_context<'a, SD: Director<S>>(
        &'a self,
        score_director: &SD,
        context: MoveStreamContext,
    ) -> Self::Cursor<'a> {
        let max_nearby = self.max_nearby;
        let solution = score_director.working_solution();
        let owner_context = self.element_owner_fn.map(|owner_fn| {
            (
                owner_fn,
                score_director
                    .entity_count(self.descriptor_index)
                    .unwrap_or(0),
            )
        });

        let mut selected =
            collect_selected_entities(&self.entity_selector, score_director, self.list_len);
        selected.apply_stream_order(
            context,
            0xA1EA_25A0_9000_0001 ^ self.descriptor_index as u64,
        );
        let entities = selected.entities;
        let route_lens = selected.route_lens;

        NearbyListSwapMoveCursor::new(
            solution.clone(),
            &self.distance_meter,
            entities,
            route_lens,
            context,
            self.list_len,
            self.list_get,
            self.list_set,
            owner_context,
            max_nearby,
            self.variable_name,
            self.descriptor_index,
        )
    }

    fn size<SD: Director<S>>(&self, score_director: &SD) -> usize {
        let selected =
            collect_selected_entities(&self.entity_selector, score_director, self.list_len);

        // Each element generates at most max_nearby canonical swap partners.
        selected.total_elements() * self.max_nearby / 2
    }
}
