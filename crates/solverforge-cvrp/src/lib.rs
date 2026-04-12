/* CVRP domain helpers for SolverForge.

Provides `ProblemData`, `MatrixDistanceMeter`, `MatrixIntraDistanceMeter`,
the `VrpSolution` trait, and a suite of free functions for Clarke-Wright
and k-opt construction phases.
*/

mod helpers;
mod meters;
mod problem_data;
mod solution;

pub use helpers::{
    capacity, depot_for_cw, depot_for_entity, distance, element_load, get_route, is_kopt_feasible,
    is_time_feasible, replace_route,
};
pub use meters::{MatrixDistanceMeter, MatrixIntraDistanceMeter};
pub use problem_data::ProblemData;
pub use solution::VrpSolution;

#[cfg(test)]
mod tests;
