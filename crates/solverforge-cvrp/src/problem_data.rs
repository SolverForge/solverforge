use crate::{DenseMatrix, RouteMetric};

/// Matrix sentinel for a leg that cannot be traversed.
///
/// Stock CVRP helpers treat this as non-evaluable route-local travel and as a
/// very large construction distance. It is intentionally the same sentinel used
/// by `solverforge-maps` road-network matrices.
pub const UNREACHABLE: i64 = i64::MAX;

pub(crate) const MAX_SAFE_LEG_COST: i64 = i64::MAX / 4;

/// Immutable problem data shared by all vehicles.
///
/// A solution typically holds it once (for example behind an `Arc`) and
/// returns a reference from [`VrpSolution::vehicle_data`](crate::VrpSolution::vehicle_data),
/// so cloning vehicles during local search never copies the metric. Leg
/// distances and travel times come from `metric`: a [`DenseMatrix`] holds
/// supplied tables, an [`EuclideanMetric`](crate::EuclideanMetric) computes
/// legs from coordinates.
#[derive(Clone, Debug)]
pub struct ProblemData<M = DenseMatrix> {
    /// Vehicle capacity, in the units of `demands`.
    pub capacity: i64,
    /// Location index of the depot every route starts and ends at.
    pub depot: usize,
    /// Demand of each location, indexed by location.
    pub demands: Vec<i32>,
    /// Leg distances and travel times between locations.
    pub metric: M,
    /// `(earliest, latest)` service start of each location.
    pub time_windows: Vec<(i64, i64)>,
    /// Service time spent at each location.
    pub service_durations: Vec<i64>,
    /// Time each vehicle leaves the depot.
    pub vehicle_departure_time: i64,
}

impl<M: RouteMetric> ProblemData<M> {
    #[inline]
    pub(crate) fn distance_cost(&self, from: usize, to: usize) -> i64 {
        self.finite_distance(from, to).unwrap_or(MAX_SAFE_LEG_COST)
    }

    #[inline]
    pub(crate) fn finite_distance(&self, from: usize, to: usize) -> Option<i64> {
        finite_leg(self.metric.distance(from, to)?)
    }

    #[inline]
    pub(crate) fn travel_time(&self, from: usize, to: usize) -> Option<i64> {
        finite_leg(self.metric.travel_time(from, to)?)
    }
}

/// A leg value, or `None` when it marks a leg that cannot be traversed.
#[inline]
pub(crate) fn finite_leg(value: i64) -> Option<i64> {
    (value >= 0 && value != UNREACHABLE).then_some(value)
}
