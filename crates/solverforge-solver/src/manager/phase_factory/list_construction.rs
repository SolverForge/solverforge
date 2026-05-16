/* List construction phase for assigning list elements to entities.

Provides several construction strategies for list variables
(e.g., assigning visits to vehicles in VRP):

- [`ListConstructionPhase`]: Simple round-robin assignment
- [`ListCheapestInsertionPhase`]: Score-guided greedy insertion
- [`ListRegretInsertionPhase`]: Regret-based insertion (reduces greedy myopia)
*/

use std::fmt::Debug;
use std::hash::Hash;

use solverforge_config::ConstructionHeuristicType;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::{ListClarkeWrightPhase, ListKOptPhase};
use crate::builder::ListVariableSlot;
use crate::phase::Phase;
use crate::scope::{ProgressCallback, SolverScope};

mod cheapest;
mod regret;
mod round_robin;
mod state;

pub use cheapest::ListCheapestInsertionPhase;
pub use regret::ListRegretInsertionPhase;
pub use round_robin::{ListConstructionPhase, ListConstructionPhaseBuilder};

fn list_work_remaining<S, V, DM, IDM>(ctx: &ListVariableSlot<S, V, DM, IDM>, solution: &S) -> bool
where
    S: PlanningSolution,
    V: Copy + PartialEq + Eq + Hash + Send + Sync + 'static,
{
    (ctx.assigned_elements)(solution).len() < (ctx.element_count)(solution)
}

pub(crate) fn solve_specialized_list_construction<S, V, DM, IDM, D, ProgressCb>(
    heuristic: ConstructionHeuristicType,
    k: usize,
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
    list_variables: &[ListVariableSlot<S, V, DM, IDM>],
) -> bool
where
    S: PlanningSolution,
    V: Clone + Copy + PartialEq + Eq + Hash + Into<usize> + Send + Sync + Debug + 'static,
    DM: Clone + Debug + Send + 'static,
    IDM: Clone + Debug + Send + 'static,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let mut ran_phase = false;

    for ctx in list_variables {
        if !list_work_remaining(ctx, solver_scope.working_solution()) {
            continue;
        }

        match heuristic {
            ConstructionHeuristicType::ListRoundRobin => {
                ListConstructionPhase::from_variable_slot(ctx).solve(solver_scope);
            }
            ConstructionHeuristicType::ListCheapestInsertion => {
                ListCheapestInsertionPhase::new(
                    ctx.element_count,
                    ctx.assigned_elements,
                    ctx.entity_count,
                    ctx.list_len,
                    ctx.list_insert,
                    ctx.construction_list_remove,
                    ctx.index_to_element,
                    ctx.descriptor_index,
                )
                .solve(solver_scope);
            }
            ConstructionHeuristicType::ListRegretInsertion => {
                ListRegretInsertionPhase::new(
                    ctx.element_count,
                    ctx.assigned_elements,
                    ctx.entity_count,
                    ctx.list_len,
                    ctx.list_insert,
                    ctx.construction_list_remove,
                    ctx.index_to_element,
                    ctx.descriptor_index,
                )
                .solve(solver_scope);
            }
            ConstructionHeuristicType::ListClarkeWright => {
                let (Some(set_route), Some(depot), Some(dist), Some(feasible)) = (
                    ctx.route_set_fn,
                    ctx.route_depot_fn,
                    ctx.route_distance_fn,
                    ctx.route_feasible_fn,
                ) else {
                    unreachable!("validated list_clarke_wright hooks must be present");
                };
                let mut phase = ListClarkeWrightPhase::new(
                    ctx.element_count,
                    ctx.assigned_elements,
                    ctx.entity_count,
                    ctx.list_len,
                    set_route,
                    ctx.index_to_element,
                    depot,
                    dist,
                    feasible,
                    ctx.descriptor_index,
                );
                if let Some(metric_class) = ctx.route_metric_class_fn {
                    phase = phase.with_metric_class_fn(metric_class);
                }
                phase.solve(solver_scope);
            }
            ConstructionHeuristicType::ListKOpt => {
                let (Some(get_route), Some(set_route), Some(route_depot), Some(route_dist)) = (
                    ctx.route_get_fn,
                    ctx.route_set_fn,
                    ctx.route_depot_fn,
                    ctx.route_distance_fn,
                ) else {
                    unreachable!("validated list_k_opt hooks must be present");
                };
                ListKOptPhase::<S, V>::new(
                    k,
                    ctx.entity_count,
                    get_route,
                    set_route,
                    route_depot,
                    route_dist,
                    ctx.route_feasible_fn,
                    ctx.descriptor_index,
                )
                .solve(solver_scope);
            }
            other => unreachable!(
                "specialized list construction only dispatches list heuristics, got {:?}",
                other
            ),
        }

        ran_phase = true;
    }

    ran_phase
}
