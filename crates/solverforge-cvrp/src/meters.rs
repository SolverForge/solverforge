use solverforge_solver::CrossEntityDistanceMeter;

use crate::helpers::problem_data_for_entity;
use crate::VrpSolution;

// Cross-entity distance meter backed by the solution's distance matrix.
#[derive(Clone, Debug, Default)]
pub struct MatrixDistanceMeter;

impl<S: VrpSolution> CrossEntityDistanceMeter<S> for MatrixDistanceMeter {
    fn distance(
        &self,
        solution: &S,
        src_entity: usize,
        src_pos: usize,
        dst_entity: usize,
        dst_pos: usize,
    ) -> f64 {
        let src_visits = solution.vehicle_visits(src_entity);
        let dst_visits = solution.vehicle_visits(dst_entity);
        if src_pos >= src_visits.len() || dst_pos >= dst_visits.len() {
            return f64::INFINITY;
        }
        problem_data_for_entity(solution, src_entity).map_or(f64::INFINITY, |data| {
            data.distance_matrix[src_visits[src_pos]][dst_visits[dst_pos]] as f64
        })
    }
}

// Intra-entity distance meter backed by the solution's distance matrix.
#[derive(Clone, Debug, Default)]
pub struct MatrixIntraDistanceMeter;

impl<S: VrpSolution> CrossEntityDistanceMeter<S> for MatrixIntraDistanceMeter {
    fn distance(
        &self,
        solution: &S,
        src_entity: usize,
        src_pos: usize,
        _dst_entity: usize,
        dst_pos: usize,
    ) -> f64 {
        let visits = solution.vehicle_visits(src_entity);
        if src_pos >= visits.len() || dst_pos >= visits.len() {
            return f64::INFINITY;
        }
        problem_data_for_entity(solution, src_entity).map_or(f64::INFINITY, |data| {
            data.distance_matrix[visits[src_pos]][visits[dst_pos]] as f64
        })
    }
}
