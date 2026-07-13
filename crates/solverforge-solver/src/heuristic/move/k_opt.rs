/* K-opt move for tour optimization.

K-opt removes k edges from a tour and reconnects the resulting segments
in a different order, potentially reversing some segments. This is a
fundamental move for TSP and VRP optimization.

# Zero-Erasure Design

- Fixed arrays for cut points (no SmallVec for static data)
- Reconnection pattern stored by value (`KOptReconnection` is `Copy`)
- Concrete function pointers for all list operations

# Example

```
use solverforge_solver::heuristic::r#move::{KOptMove, CutPoint};
use solverforge_solver::heuristic::r#move::k_opt_reconnection::THREE_OPT_RECONNECTIONS;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SoftScore;

#[derive(Clone, Debug)]
struct Tour { cities: Vec<i32>, score: Option<SoftScore> }

impl PlanningSolution for Tour {
type Score = SoftScore;
fn score(&self) -> Option<Self::Score> { self.score }
fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
}

fn list_len(s: &Tour, _: usize) -> usize { s.cities.len() }
fn sublist_remove(s: &mut Tour, _: usize, start: usize, end: usize) -> Vec<i32> {
s.cities.drain(start..end).collect()
}
fn sublist_insert(s: &mut Tour, _: usize, pos: usize, items: Vec<i32>) {
for (i, item) in items.into_iter().enumerate() {
s.cities.insert(pos + i, item);
}
}

// Create a 3-opt move with cuts at positions 2, 4, 6
// This creates 4 segments: [0..2), [2..4), [4..6), [6..)
let cuts = [
CutPoint::new(0, 2),
CutPoint::new(0, 4),
CutPoint::new(0, 6),
];
let reconnection = &THREE_OPT_RECONNECTIONS[3]; // Swap middle segments

let m = KOptMove::<Tour, i32>::new(
&cuts,
reconnection,
list_len,
sublist_remove,
sublist_insert,
"cities",
0,
);
```
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::k_opt_reconnection::KOptReconnection;
use super::list_kernel::{
    k_opt_do_move, k_opt_is_doable, k_opt_tabu_signature, k_opt_undo_move, StaticListWindowAccess,
};
use super::metadata::hash_str;
use super::{Move, MoveTabuSignature};

/* A cut point in a route, defining where an edge is removed.

For k-opt, we have k cut points which divide the route into k+1 segments.

# Example

```
use solverforge_solver::heuristic::r#move::CutPoint;

// Cut at position 5 in entity 0
let cut = CutPoint::new(0, 5);
assert_eq!(cut.entity_index(), 0);
assert_eq!(cut.position(), 5);
```
*/
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CutPoint {
    // Entity (route/vehicle) index.
    entity_index: usize,
    // Position in the route where the cut occurs.
    // The edge between position-1 and position is removed.
    position: usize,
}

impl CutPoint {
    #[inline]
    pub const fn new(entity_index: usize, position: usize) -> Self {
        Self {
            entity_index,
            position,
        }
    }

    #[inline]
    pub const fn entity_index(&self) -> usize {
        self.entity_index
    }

    #[inline]
    pub const fn position(&self) -> usize {
        self.position
    }
}

/// A k-opt move that removes k edges and reconnects segments.
///
/// This is the generalized k-opt move supporting k=2,3,4,5.
/// For k=2, this is equivalent to a 2-opt (segment reversal).
///
/// # Zero-Erasure Design
///
/// - Fixed array `[CutPoint; 5]` for up to 5 cuts (5-opt)
/// - `KOptReconnection` stored by value (`Copy` type, no heap allocation)
/// - Concrete function pointers for list operations
///
/// # Type Parameters
///
/// * `S` - The planning solution type
/// * `V` - The list element value type
pub struct KOptMove<S, V> {
    // Cut points (up to 5 for 5-opt).
    cuts: [CutPoint; 5],
    // Number of actual cuts (k value).
    cut_count: u8,
    // Reconnection pattern to apply (stored by value — `KOptReconnection` is `Copy`).
    reconnection: KOptReconnection,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    // Remove sublist [start, end), returns removed elements.
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    // Insert elements at position.
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    // Variable name.
    variable_name: &'static str,
    variable_id: u64,
    // Descriptor index.
    descriptor_index: usize,
    // Entity index (for intra-route moves).
    entity_index: usize,
    _phantom: PhantomData<fn() -> V>,
}

impl<S, V> Clone for KOptMove<S, V> {
    fn clone(&self) -> Self {
        Self {
            cuts: self.cuts,
            cut_count: self.cut_count,
            reconnection: self.reconnection,
            list_len: self.list_len,
            list_get: self.list_get,
            sublist_remove: self.sublist_remove,
            sublist_insert: self.sublist_insert,
            variable_name: self.variable_name,
            variable_id: self.variable_id,
            descriptor_index: self.descriptor_index,
            entity_index: self.entity_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V: Debug> Debug for KOptMove<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let cuts: Vec<_> = self.cuts[..self.cut_count as usize]
            .iter()
            .map(|c| c.position)
            .collect();
        f.debug_struct("KOptMove")
            .field("k", &self.cut_count)
            .field("entity", &self.entity_index)
            .field("cuts", &cuts)
            .field("reconnection", &self.reconnection)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, V> KOptMove<S, V> {
    /* Creates a new k-opt move.

    # Arguments

    * `cuts` - Slice of cut points (must be sorted by position for intra-route)
    * `reconnection` - How to reconnect the segments
    * `list_len` - Function to get list length
    * `sublist_remove` - Function to remove a range
    * `sublist_insert` - Function to insert elements
    * `variable_name` - Name of the list variable
    * `descriptor_index` - Entity descriptor index

    # Panics

    Panics if cuts is empty or has more than 5 elements.
    */
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cuts: &[CutPoint],
        reconnection: &KOptReconnection,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self::new_with_variable_id(
            cuts,
            reconnection,
            list_len,
            list_get,
            sublist_remove,
            sublist_insert,
            variable_name,
            hash_str(variable_name),
            descriptor_index,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_with_variable_id(
        cuts: &[CutPoint],
        reconnection: &KOptReconnection,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        variable_name: &'static str,
        variable_id: u64,
        descriptor_index: usize,
    ) -> Self {
        assert!(!cuts.is_empty() && cuts.len() <= 5, "k must be 1-5");

        let mut cut_array = [CutPoint::default(); 5];
        for (i, cut) in cuts.iter().enumerate() {
            cut_array[i] = *cut;
        }

        // For now, assume intra-route (all cuts on same entity)
        let entity_index = cuts[0].entity_index;

        Self {
            cuts: cut_array,
            cut_count: cuts.len() as u8,
            reconnection: *reconnection,
            list_len,
            list_get,
            sublist_remove,
            sublist_insert,
            variable_name,
            variable_id,
            descriptor_index,
            entity_index,
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub fn k(&self) -> usize {
        self.cut_count as usize
    }

    #[inline]
    pub fn cuts(&self) -> &[CutPoint] {
        &self.cuts[..self.cut_count as usize]
    }

    pub fn is_intra_route(&self) -> bool {
        let first = self.cuts[0].entity_index;
        self.cuts[..self.cut_count as usize]
            .iter()
            .all(|c| c.entity_index == first)
    }

    fn access(&self) -> StaticListWindowAccess<S, V> {
        StaticListWindowAccess {
            list_len: self.list_len,
            list_get: self.list_get,
            sublist_remove: self.sublist_remove,
            sublist_insert: self.sublist_insert,
            variable_name: self.variable_name,
            descriptor_index: self.descriptor_index,
        }
    }
}

impl<S, V> Move<S> for KOptMove<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    type Undo = Vec<V>;

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        k_opt_is_doable(
            &self.access(),
            self.cuts(),
            &self.reconnection,
            self.entity_index,
            score_director,
        )
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        k_opt_do_move(
            &self.access(),
            self.cuts(),
            &self.reconnection,
            self.entity_index,
            score_director,
        )
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, undo: Self::Undo) {
        k_opt_undo_move(&self.access(), self.entity_index, undo, score_director);
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        std::slice::from_ref(&self.entity_index)
    }

    fn variable_name(&self) -> &str {
        self.variable_name
    }

    fn telemetry_label(&self) -> &'static str {
        "k_opt"
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        k_opt_tabu_signature(
            &self.access(),
            self.cuts(),
            &self.reconnection,
            self.variable_id,
            self.entity_index,
            score_director,
        )
    }
}
