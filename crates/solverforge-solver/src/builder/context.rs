// Context types that carry domain function pointers into the builder layer.

use std::marker::PhantomData;

use crate::heuristic::selector::k_opt::ListPositionDistanceMeter;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;

/* Adapts a `CrossEntityDistanceMeter` to `ListPositionDistanceMeter` for intra-entity use.

`NearbyKOptMoveSelector` requires a `ListPositionDistanceMeter` (4-param intra-entity),
but `ListContext.intra_distance_meter` is a `CrossEntityDistanceMeter` (5-param).
This adapter bridges the two by always calling with `src_entity_idx == dst_entity_idx`.
*/
#[derive(Debug, Clone)]
pub struct IntraDistanceAdapter<T>(pub T);

impl<S, T: CrossEntityDistanceMeter<S>> ListPositionDistanceMeter<S> for IntraDistanceAdapter<T> {
    fn distance(&self, solution: &S, entity_idx: usize, pos_a: usize, pos_b: usize) -> f64 {
        self.0
            .distance(solution, entity_idx, pos_a, entity_idx, pos_b)
    }
}

/// Function-pointer context for basic (non-list) variable solvers.
///
/// Carries all domain callbacks needed to construct move selectors
/// without requiring `dyn` or closures.
pub struct BasicContext<S> {
    pub get_variable: fn(&S, usize) -> Option<usize>,
    pub set_variable: fn(&mut S, usize, Option<usize>),
    // All valid values for the variable.
    pub values: Vec<usize>,
    // Descriptor index for the entity collection.
    pub descriptor_index: usize,
    // Variable field name.
    pub variable_field: &'static str,
}

impl<S> std::fmt::Debug for BasicContext<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BasicContext")
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_field", &self.variable_field)
            .field("values_len", &self.values.len())
            .finish()
    }
}

/// Function-pointer context for list variable solvers.
///
/// Carries all domain callbacks and distance meters needed to construct
/// list move selectors without requiring `dyn` or closures.
pub struct ListContext<S, V, DM, IDM> {
    pub list_len: fn(&S, usize) -> usize,
    // Removes element at `pos` from entity `i`, returning it (returns `None` if out of bounds).
    pub list_remove: fn(&mut S, usize, usize) -> Option<V>,
    // Inserts `val` at `pos` in entity `i`.
    pub list_insert: fn(&mut S, usize, usize, V),
    pub list_get: fn(&S, usize, usize) -> Option<V>,
    // Replaces element at `pos` in entity `i`.
    pub list_set: fn(&mut S, usize, usize, V),
    // Reverses the segment `[start, end)` in entity `i`.
    pub list_reverse: fn(&mut S, usize, usize, usize),
    // Removes segment `[start, end)` from entity `i`.
    pub sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    // Inserts `items` at `pos` in entity `i`.
    pub sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    // Removes element at `pos` from entity `i` for ruin moves (panics if out of bounds).
    pub ruin_remove: fn(&mut S, usize, usize) -> V,
    // Inserts `val` at `pos` in entity `i` for ruin reinsertion.
    pub ruin_insert: fn(&mut S, usize, usize, V),
    pub entity_count: fn(&S) -> usize,
    // Cross-entity (inter-route) distance meter.
    pub cross_distance_meter: DM,
    // Intra-entity (intra-route) distance meter.
    pub intra_distance_meter: IDM,
    // List variable field name.
    pub variable_name: &'static str,
    // Descriptor index for the list owner entity collection.
    pub descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

impl<S, V, DM, IDM> ListContext<S, V, DM, IDM> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        list_len: fn(&S, usize) -> usize,
        list_remove: fn(&mut S, usize, usize) -> Option<V>,
        list_insert: fn(&mut S, usize, usize, V),
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_set: fn(&mut S, usize, usize, V),
        list_reverse: fn(&mut S, usize, usize, usize),
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        ruin_remove: fn(&mut S, usize, usize) -> V,
        ruin_insert: fn(&mut S, usize, usize, V),
        entity_count: fn(&S) -> usize,
        cross_distance_meter: DM,
        intra_distance_meter: IDM,
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            list_len,
            list_remove,
            list_insert,
            list_get,
            list_set,
            list_reverse,
            sublist_remove,
            sublist_insert,
            ruin_remove,
            ruin_insert,
            entity_count,
            cross_distance_meter,
            intra_distance_meter,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V, DM: std::fmt::Debug, IDM: std::fmt::Debug> std::fmt::Debug
    for ListContext<S, V, DM, IDM>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListContext")
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}
