use std::fmt;

use solverforge_core::domain::{DynamicListMetadata, DynamicListVariableSlot};

use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;

use super::super::ListVariableSlot;
use super::dynamic::{dynamic_access_has, dynamic_error, dynamic_metadata_has};
use super::static_access::typed_error;
use super::{ListAccess, ListAccessCapability, ListAccessError};

/// The route sequence part of a route/savings phase.
///
/// This is intentionally separate from route and savings metadata. Reading a
/// route always comes from ordinary list access, while replacing it must be a
/// single direct list mutation. The two semantic metadata bundles are layered
/// below so a Clarke-Wright phase cannot accidentally consume K-opt hooks (or
/// vice versa).
pub(crate) trait RouteSequenceAccess<S>: ListAccess<S> {
    /// Verifies that one complete route replacement is structurally available.
    fn validate_route_sequence(&self) -> Result<(), ListAccessError>;

    /// Materializes a route through the base list-read surface. A missing
    /// position is an invalid access implementation, not an opportunity to
    /// synthesize a route from metadata or callbacks.
    fn route_values(&self, solution: &S, entity: usize) -> Result<Vec<usize>, ListAccessError>;

    /// Replaces a route through the slot's one logical list-mutation path.
    fn replace_route(
        &self,
        solution: &mut S,
        entity: usize,
        route: Vec<usize>,
    ) -> Result<(), ListAccessError>;
}

/// Strict route-local metadata used by K-opt and other route neighborhoods.
///
/// `validate_route` checks the complete bundle: base route read/replacement
/// plus depot, distance, and feasibility. Phase assembly calls it before any
/// candidate generation, so a missing capability can never silently turn into
/// a generic neighborhood or a wrapper-local path.
pub(crate) trait RouteAccess<S>: RouteSequenceAccess<S> {
    fn validate_route(&self) -> Result<(), ListAccessError>;
    fn route_depot(&self, solution: &S, entity: usize) -> Result<usize, ListAccessError>;
    fn route_distance(
        &self,
        solution: &S,
        entity: usize,
        from: usize,
        to: usize,
    ) -> Result<i64, ListAccessError>;
    fn route_feasible(
        &self,
        solution: &S,
        entity: usize,
        route: &[usize],
    ) -> Result<bool, ListAccessError>;
}

/// Savings-only metadata used by Clarke-Wright construction.
///
/// This does not inherit route metadata. It shares only the direct route
/// sequence mutation surface, and `validate_savings` verifies its own depot,
/// metric-class, distance, and feasibility bundle.
pub(crate) trait SavingsAccess<S>: RouteSequenceAccess<S> {
    fn validate_savings(&self) -> Result<(), ListAccessError>;
    fn savings_depot(&self, solution: &S, entity: usize) -> Result<usize, ListAccessError>;
    fn savings_metric_class(&self, solution: &S, entity: usize) -> Result<usize, ListAccessError>;
    fn savings_distance(
        &self,
        solution: &S,
        entity: usize,
        from: usize,
        to: usize,
    ) -> Result<i64, ListAccessError>;
    fn savings_feasible(
        &self,
        solution: &S,
        entity: usize,
        route: &[usize],
    ) -> Result<bool, ListAccessError>;
}

impl<S, DM, IDM> RouteSequenceAccess<S> for ListVariableSlot<S, usize, DM, IDM>
where
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn validate_route_sequence(&self) -> Result<(), ListAccessError> {
        self.route_set_fn
            .is_some()
            .then_some(())
            .ok_or_else(|| typed_error(self, ListAccessCapability::Replace))
    }

    fn route_values(&self, solution: &S, entity: usize) -> Result<Vec<usize>, ListAccessError> {
        (0..self.list_len(solution, entity))
            .map(|position| {
                self.list_get(solution, entity, position)
                    .ok_or_else(|| typed_error(self, ListAccessCapability::Route))
            })
            .collect()
    }

    fn replace_route(
        &self,
        solution: &mut S,
        entity: usize,
        route: Vec<usize>,
    ) -> Result<(), ListAccessError> {
        self.validate_route_sequence()?;
        let replace = self.route_set_fn.expect("validated route replacement");
        replace(solution, entity, route);
        Ok(())
    }
}

impl<S, DM, IDM> RouteAccess<S> for ListVariableSlot<S, usize, DM, IDM>
where
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn validate_route(&self) -> Result<(), ListAccessError> {
        self.validate_route_sequence()?;
        (self.route_depot_fn.is_some()
            && self.route_distance_fn.is_some()
            && self.route_feasible_fn.is_some())
        .then_some(())
        .ok_or_else(|| typed_error(self, ListAccessCapability::Route))
    }

    fn route_depot(&self, solution: &S, entity: usize) -> Result<usize, ListAccessError> {
        self.validate_route()?;
        let depot = self.route_depot_fn.expect("validated route depot");
        Ok(depot(solution, entity))
    }

    fn route_distance(
        &self,
        solution: &S,
        entity: usize,
        from: usize,
        to: usize,
    ) -> Result<i64, ListAccessError> {
        self.validate_route()?;
        let distance = self.route_distance_fn.expect("validated route distance");
        Ok(distance(solution, entity, from, to))
    }

    fn route_feasible(
        &self,
        solution: &S,
        entity: usize,
        route: &[usize],
    ) -> Result<bool, ListAccessError> {
        self.validate_route()?;
        let feasible = self.route_feasible_fn.expect("validated route feasibility");
        Ok(feasible(solution, entity, route))
    }
}

impl<S, DM, IDM> SavingsAccess<S> for ListVariableSlot<S, usize, DM, IDM>
where
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn validate_savings(&self) -> Result<(), ListAccessError> {
        self.validate_route_sequence()?;
        (self.savings_depot_fn.is_some()
            && self.savings_metric_class_fn.is_some()
            && self.savings_distance_fn.is_some()
            && self.savings_feasible_fn.is_some())
        .then_some(())
        .ok_or_else(|| typed_error(self, ListAccessCapability::Savings))
    }

    fn savings_depot(&self, solution: &S, entity: usize) -> Result<usize, ListAccessError> {
        self.validate_savings()?;
        let depot = self.savings_depot_fn.expect("validated savings depot");
        Ok(depot(solution, entity))
    }

    fn savings_metric_class(&self, solution: &S, entity: usize) -> Result<usize, ListAccessError> {
        self.validate_savings()?;
        let metric_class = self
            .savings_metric_class_fn
            .expect("validated savings metric class");
        Ok(metric_class(solution, entity))
    }

    fn savings_distance(
        &self,
        solution: &S,
        entity: usize,
        from: usize,
        to: usize,
    ) -> Result<i64, ListAccessError> {
        self.validate_savings()?;
        let distance = self
            .savings_distance_fn
            .expect("validated savings distance");
        Ok(distance(solution, entity, from, to))
    }

    fn savings_feasible(
        &self,
        solution: &S,
        entity: usize,
        route: &[usize],
    ) -> Result<bool, ListAccessError> {
        self.validate_savings()?;
        let feasible = self
            .savings_feasible_fn
            .expect("validated savings feasibility");
        Ok(feasible(solution, entity, route))
    }
}

fn dynamic_metadata<S>(
    slot: &DynamicListVariableSlot<S>,
    capability: ListAccessCapability,
) -> Result<&dyn DynamicListMetadata<S>, ListAccessError> {
    slot.metadata()
        .filter(|metadata| dynamic_metadata_has(metadata.capabilities(), capability))
        .ok_or_else(|| dynamic_error(slot, capability))
}

impl<S> RouteSequenceAccess<S> for DynamicListVariableSlot<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn validate_route_sequence(&self) -> Result<(), ListAccessError> {
        dynamic_access_has(self.access_capabilities(), ListAccessCapability::Replace)
            .then_some(())
            .ok_or_else(|| dynamic_error(self, ListAccessCapability::Replace))
    }

    fn route_values(&self, solution: &S, entity: usize) -> Result<Vec<usize>, ListAccessError> {
        (0..self.list_len(solution, entity))
            .map(|position| {
                self.list_get(solution, entity, position)
                    .ok_or_else(|| dynamic_error(self, ListAccessCapability::Route))
            })
            .collect()
    }

    fn replace_route(
        &self,
        solution: &mut S,
        entity: usize,
        route: Vec<usize>,
    ) -> Result<(), ListAccessError> {
        self.validate_route_sequence()?;
        self.list_replace(solution, entity, route)
            .then_some(())
            .ok_or_else(|| dynamic_error(self, ListAccessCapability::Replace))
    }
}

impl<S> RouteAccess<S> for DynamicListVariableSlot<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn validate_route(&self) -> Result<(), ListAccessError> {
        self.validate_route_sequence()?;
        let _ = dynamic_metadata(self, ListAccessCapability::Route)?;
        Ok(())
    }

    fn route_depot(&self, solution: &S, entity: usize) -> Result<usize, ListAccessError> {
        self.validate_route()?;
        dynamic_metadata(self, ListAccessCapability::Route)?
            .route_depot(solution, entity)
            .ok_or_else(|| dynamic_error(self, ListAccessCapability::Route))
    }

    fn route_distance(
        &self,
        solution: &S,
        entity: usize,
        from: usize,
        to: usize,
    ) -> Result<i64, ListAccessError> {
        self.validate_route()?;
        dynamic_metadata(self, ListAccessCapability::Route)?
            .route_distance(solution, entity, from, to)
            .ok_or_else(|| dynamic_error(self, ListAccessCapability::Route))
    }

    fn route_feasible(
        &self,
        solution: &S,
        entity: usize,
        route: &[usize],
    ) -> Result<bool, ListAccessError> {
        self.validate_route()?;
        dynamic_metadata(self, ListAccessCapability::Route)?
            .route_feasible(solution, entity, route)
            .ok_or_else(|| dynamic_error(self, ListAccessCapability::Route))
    }
}

impl<S> SavingsAccess<S> for DynamicListVariableSlot<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn validate_savings(&self) -> Result<(), ListAccessError> {
        self.validate_route_sequence()?;
        let _ = dynamic_metadata(self, ListAccessCapability::Savings)?;
        Ok(())
    }

    fn savings_depot(&self, solution: &S, entity: usize) -> Result<usize, ListAccessError> {
        self.validate_savings()?;
        dynamic_metadata(self, ListAccessCapability::Savings)?
            .savings_depot(solution, entity)
            .ok_or_else(|| dynamic_error(self, ListAccessCapability::Savings))
    }

    fn savings_metric_class(&self, solution: &S, entity: usize) -> Result<usize, ListAccessError> {
        self.validate_savings()?;
        dynamic_metadata(self, ListAccessCapability::Savings)?
            .savings_metric_class(solution, entity)
            .ok_or_else(|| dynamic_error(self, ListAccessCapability::Savings))
    }

    fn savings_distance(
        &self,
        solution: &S,
        entity: usize,
        from: usize,
        to: usize,
    ) -> Result<i64, ListAccessError> {
        self.validate_savings()?;
        dynamic_metadata(self, ListAccessCapability::Savings)?
            .savings_distance(solution, entity, from, to)
            .ok_or_else(|| dynamic_error(self, ListAccessCapability::Savings))
    }

    fn savings_feasible(
        &self,
        solution: &S,
        entity: usize,
        route: &[usize],
    ) -> Result<bool, ListAccessError> {
        self.validate_savings()?;
        dynamic_metadata(self, ListAccessCapability::Savings)?
            .savings_feasible(solution, entity, route)
            .ok_or_else(|| dynamic_error(self, ListAccessCapability::Savings))
    }
}
