//! CVRP domain helpers for SolverForge.
//!
//! Provides `ProblemData` with its `RouteMetric` distance sources
//! (`DenseMatrix`, `EuclideanMetric`), `MatrixDistanceMeter`,
//! `MatrixIntraDistanceMeter`, the `VrpSolution` trait, and the stock helpers
//! behind `#[planning_list_variable(domain = "cvrp")]`.

mod helpers;
mod meters;
mod metric;
mod problem_data;
mod solution;

pub use helpers::{
    depot_for_entity, get_route, replace_route, route_distance, route_feasible, route_hooks,
    savings_depot_for_entity, savings_distance, savings_feasible, savings_hooks,
    savings_metric_class,
};
pub use meters::{MatrixDistanceMeter, MatrixIntraDistanceMeter};
pub use metric::{DenseMatrix, EuclideanMetric, RouteMetric};
pub use problem_data::{ProblemData, UNREACHABLE};
pub use solution::VrpSolution;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod metric_tests;
