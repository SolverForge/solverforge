//! Shared distance and ownership probes for nearby list cursors.
//!
//! The probe is intentionally immutable and borrowed by static adapters so
//! opening a cursor does not clone or pre-observe user distance callbacks.

use std::fmt::{self, Debug};

use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::list_placement::{owner_restriction, OwnerRestriction};

pub(crate) trait NearbyChangeProbe<S>: Debug {
    fn distance(
        &self,
        solution: &S,
        source_entity: usize,
        source_position: usize,
        destination_entity: usize,
        destination_position: usize,
    ) -> f64;

    fn has_owner_binding(&self) -> bool;

    fn owner_restriction(
        &self,
        solution: &S,
        entity_count: usize,
        entity: usize,
        position: usize,
    ) -> Option<OwnerRestriction>;
}

pub(crate) trait NearbySwapProbe<S>: Debug {
    fn distance(
        &self,
        solution: &S,
        source_entity: usize,
        source_position: usize,
        destination_entity: usize,
        destination_position: usize,
    ) -> f64;

    fn has_owner_binding(&self) -> bool;

    fn owner_restriction(
        &self,
        solution: &S,
        entity_count: usize,
        entity: usize,
        position: usize,
    ) -> Option<OwnerRestriction>;
}

/// Static adapter over the existing distance meter and owner callback.
pub(crate) struct NativeNearbyProbe<'a, S, V, D> {
    distance_meter: &'a D,
    list_get: fn(&S, usize, usize) -> Option<V>,
    element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
}

impl<'a, S, V, D> NativeNearbyProbe<'a, S, V, D> {
    pub(crate) fn new(
        distance_meter: &'a D,
        list_get: fn(&S, usize, usize) -> Option<V>,
        element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    ) -> Self {
        Self {
            distance_meter,
            list_get,
            element_owner_fn,
        }
    }

    fn owner_at(
        &self,
        solution: &S,
        entity_count: usize,
        entity: usize,
        position: usize,
    ) -> Option<OwnerRestriction> {
        let owner_fn = self.element_owner_fn?;
        let element = (self.list_get)(solution, entity, position)?;
        Some(owner_restriction(
            Some(owner_fn),
            solution,
            entity_count,
            &element,
        ))
    }
}

impl<S, V, D> Debug for NativeNearbyProbe<'_, S, V, D> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeNearbyProbe")
            .field("has_element_owner", &self.element_owner_fn.is_some())
            .finish()
    }
}

impl<S, V, D> NearbyChangeProbe<S> for NativeNearbyProbe<'_, S, V, D>
where
    D: CrossEntityDistanceMeter<S>,
{
    fn distance(
        &self,
        solution: &S,
        source_entity: usize,
        source_position: usize,
        destination_entity: usize,
        destination_position: usize,
    ) -> f64 {
        self.distance_meter.distance(
            solution,
            source_entity,
            source_position,
            destination_entity,
            destination_position,
        )
    }

    fn has_owner_binding(&self) -> bool {
        self.element_owner_fn.is_some()
    }

    fn owner_restriction(
        &self,
        solution: &S,
        entity_count: usize,
        entity: usize,
        position: usize,
    ) -> Option<OwnerRestriction> {
        self.owner_at(solution, entity_count, entity, position)
    }
}

impl<S, V, D> NearbySwapProbe<S> for NativeNearbyProbe<'_, S, V, D>
where
    D: CrossEntityDistanceMeter<S>,
{
    fn distance(
        &self,
        solution: &S,
        source_entity: usize,
        source_position: usize,
        destination_entity: usize,
        destination_position: usize,
    ) -> f64 {
        self.distance_meter.distance(
            solution,
            source_entity,
            source_position,
            destination_entity,
            destination_position,
        )
    }

    fn has_owner_binding(&self) -> bool {
        self.element_owner_fn.is_some()
    }

    fn owner_restriction(
        &self,
        solution: &S,
        entity_count: usize,
        entity: usize,
        position: usize,
    ) -> Option<OwnerRestriction> {
        self.owner_at(solution, entity_count, entity, position)
    }
}
