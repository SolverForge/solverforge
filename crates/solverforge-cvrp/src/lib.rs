/* CVRP domain helpers for SolverForge.

Provides `ProblemData`, `MatrixDistanceMeter`, `MatrixIntraDistanceMeter`,
the `VrpSolution` trait, and route hook functions for Clarke-Wright and k-opt
construction phases.
*/

mod helpers;
mod meters;
mod problem_data;
mod solution;

pub use helpers::{depot_for_entity, get_route, replace_route, route_distance, route_feasible};
pub use meters::{MatrixDistanceMeter, MatrixIntraDistanceMeter};
pub use problem_data::ProblemData;
pub use solution::VrpSolution;

#[cfg(test)]
mod tests;
