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
use crate::builder::ListVariableContext;
use crate::phase::Phase;
use crate::scope::{ProgressCallback, SolverScope};

mod cheapest;
mod regret;
mod round_robin;
mod state;

pub use cheapest::ListCheapestInsertionPhase;
pub use regret::ListRegretInsertionPhase;
pub use round_robin::{ListConstructionPhase, ListConstructionPhaseBuilder};

fn list_work_remaining<S, V, DM, IDM>(
    ctx: &ListVariableContext<S, V, DM, IDM>,
    solution: &S,
) -> bool
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
    list_variables: &[ListVariableContext<S, V, DM, IDM>],
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
                ListConstructionPhase::from_variable_context(ctx).solve(solver_scope);
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
                let (Some(depot), Some(dist), Some(load), Some(cap), Some(assign)) = (
                    ctx.cw_depot_fn,
                    ctx.cw_distance_fn,
                    ctx.cw_element_load_fn,
                    ctx.cw_capacity_fn,
                    ctx.cw_assign_route_fn,
                ) else {
                    unreachable!("validated list_clarke_wright hooks must be present");
                };
                ListClarkeWrightPhase::new(
                    ctx.element_count,
                    ctx.assigned_elements,
                    ctx.entity_count,
                    ctx.list_len,
                    assign,
                    ctx.index_to_element,
                    depot,
                    dist,
                    load,
                    cap,
                    ctx.merge_feasible_fn,
                    ctx.descriptor_index,
                )
                .solve(solver_scope);
            }
            ConstructionHeuristicType::ListKOpt => {
                let (Some(get_route), Some(set_route), Some(ko_depot), Some(ko_dist)) = (
                    ctx.k_opt_get_route,
                    ctx.k_opt_set_route,
                    ctx.k_opt_depot_fn,
                    ctx.k_opt_distance_fn,
                ) else {
                    unreachable!("validated list_k_opt hooks must be present");
                };
                ListKOptPhase::<S, V>::new(
                    k,
                    ctx.entity_count,
                    get_route,
                    set_route,
                    ko_depot,
                    ko_dist,
                    ctx.k_opt_feasible_fn,
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
