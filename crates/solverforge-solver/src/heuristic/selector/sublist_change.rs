/* Sublist change move selector for segment relocation (Or-opt).

Generates `SubListChangeMove`s that relocate contiguous segments within or
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
use solverforge_solver::heuristic::selector::sublist_change::SubListChangeMoveSelector;
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
let selector = SubListChangeMoveSelector::<Solution, i32, _>::new(
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

use crate::heuristic::r#move::{ListMoveImpl, SubListChangeMove};

use super::entity::EntitySelector;
use super::list_support::collect_selected_entities;
use super::move_selector::MoveSelector;
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
pub struct SubListChangeMoveSelector<S, V, ES> {
    entity_selector: ES,
    // Minimum segment size (inclusive). Usually 1.
    min_sublist_size: usize,
    // Maximum segment size (inclusive). Usually 3-5.
    max_sublist_size: usize,
    list_len: fn(&S, usize) -> usize,
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

impl<S, V: Debug, ES: Debug> Debug for SubListChangeMoveSelector<S, V, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubListChangeMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("min_sublist_size", &self.min_sublist_size)
            .field("max_sublist_size", &self.max_sublist_size)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V, ES> SubListChangeMoveSelector<S, V, ES> {
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
            sublist_remove,
            sublist_insert,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V, ES> MoveSelector<S, SubListChangeMove<S, V>> for SubListChangeMoveSelector<S, V, ES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = SubListChangeMove<S, V>> + 'a {
        let list_len = self.list_len;
        let sublist_remove = self.sublist_remove;
        let sublist_insert = self.sublist_insert;
        let variable_name = self.variable_name;
        let descriptor_index = self.descriptor_index;
        let min_seg = self.min_sublist_size;
        let max_seg = self.max_sublist_size;

        let selected = collect_selected_entities(&self.entity_selector, score_director, list_len);
        let entities = selected.entities;
        let route_lens = selected.route_lens;
        let mut moves = Vec::new();

        for (src_idx, &src_entity) in entities.iter().enumerate() {
            let src_len = route_lens[src_idx];

            for seg_start in 0..src_len {
                for seg_size in min_seg..=max_seg {
                    let seg_end = seg_start + seg_size;
                    if seg_end > src_len {
                        break;
                    }

                    let post_removal_len = src_len - seg_size;
                    for dst_pos in 0..=post_removal_len {
                        if dst_pos == seg_start {
                            continue;
                        }
                        moves.push(SubListChangeMove::new(
                            src_entity,
                            seg_start,
                            seg_end,
                            src_entity,
                            dst_pos,
                            list_len,
                            sublist_remove,
                            sublist_insert,
                            variable_name,
                            descriptor_index,
                        ));
                    }

                    for (dst_idx, &dst_entity) in entities.iter().enumerate() {
                        if dst_idx == src_idx {
                            continue;
                        }
                        let dst_len = route_lens[dst_idx];
                        for dst_pos in 0..=dst_len {
                            moves.push(SubListChangeMove::new(
                                src_entity,
                                seg_start,
                                seg_end,
                                dst_entity,
                                dst_pos,
                                list_len,
                                sublist_remove,
                                sublist_insert,
                                variable_name,
                                descriptor_index,
                            ));
                        }
                    }
                }
            }
        }

        moves.into_iter()
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

/// Wraps a `SubListChangeMoveSelector` to yield `ListMoveImpl::SubListChange`.
pub struct ListMoveSubListChangeSelector<S, V, ES> {
    inner: SubListChangeMoveSelector<S, V, ES>,
}

impl<S, V: Debug, ES: Debug> Debug for ListMoveSubListChangeSelector<S, V, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListMoveSubListChangeSelector")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S, V, ES> ListMoveSubListChangeSelector<S, V, ES> {
    /// Wraps an existing [`SubListChangeMoveSelector`].
    pub fn new(inner: SubListChangeMoveSelector<S, V, ES>) -> Self {
        Self { inner }
    }
}

impl<S, V, ES> MoveSelector<S, ListMoveImpl<S, V>> for ListMoveSubListChangeSelector<S, V, ES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
{
    fn open_cursor<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
    ) -> impl Iterator<Item = ListMoveImpl<S, V>> + 'a {
        self.inner
            .open_cursor(score_director)
            .map(ListMoveImpl::SubListChange)
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director)
    }
}
