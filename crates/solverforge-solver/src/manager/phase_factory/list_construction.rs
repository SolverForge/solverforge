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
                ListConstructionPhase::from_variable_slot(ctx)
                    .with_element_order_key(ctx.construction_element_order_key)
                    .solve(solver_scope);
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
                .with_element_owner_fn(ctx.element_owner_fn)
                .with_element_order_key(ctx.construction_element_order_key)
                .with_precedence_hooks(ctx.precedence_duration_fn, ctx.precedence_successors_fn)
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
                .with_element_owner_fn(ctx.element_owner_fn)
                .with_element_order_key(ctx.construction_element_order_key)
                .with_precedence_hooks(ctx.precedence_duration_fn, ctx.precedence_successors_fn)
                .solve(solver_scope);
            }
            ConstructionHeuristicType::ListClarkeWright => {
                let (Some(set_route), Some(depot), Some(dist), Some(feasible)) = (
                    ctx.route_set_fn,
                    ctx.savings_depot_fn,
                    ctx.savings_distance_fn,
                    ctx.savings_feasible_fn,
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
                )
                .with_element_owner_fn(ctx.element_owner_fn);
                if let Some(metric_class) = ctx.savings_metric_class_fn {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DefaultCrossEntityDistanceMeter;
    use solverforge_core::domain::SolutionDescriptor;
    use solverforge_core::score::SoftScore;
    use solverforge_scoring::ScoreDirector;
    use std::any::TypeId;
    use std::sync::atomic::{AtomicUsize, Ordering};

    type DefaultMeter = DefaultCrossEntityDistanceMeter;

    static CLARKE_WRIGHT_DISTANCE_CALLS: AtomicUsize = AtomicUsize::new(0);
    static ROUTE_DISTANCE_CALLS: AtomicUsize = AtomicUsize::new(0);

    #[derive(Clone, Debug)]
    struct Route {
        visits: Vec<usize>,
    }

    #[derive(Clone, Debug)]
    struct Plan {
        elements: Vec<usize>,
        routes: Vec<Route>,
        score: Option<SoftScore>,
    }

    impl PlanningSolution for Plan {
        type Score = SoftScore;

        fn score(&self) -> Option<Self::Score> {
            self.score
        }

        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    fn element_count(plan: &Plan) -> usize {
        plan.elements.len()
    }

    fn assigned_elements(plan: &Plan) -> Vec<usize> {
        plan.routes
            .iter()
            .flat_map(|route| route.visits.iter().copied())
            .collect()
    }

    fn entity_count(plan: &Plan) -> usize {
        plan.routes.len()
    }

    fn list_len(plan: &Plan, entity_idx: usize) -> usize {
        plan.routes[entity_idx].visits.len()
    }

    fn list_remove(plan: &mut Plan, entity_idx: usize, pos: usize) -> Option<usize> {
        (pos < plan.routes[entity_idx].visits.len())
            .then(|| plan.routes[entity_idx].visits.remove(pos))
    }

    fn construction_list_remove(plan: &mut Plan, entity_idx: usize, pos: usize) -> usize {
        plan.routes[entity_idx].visits.remove(pos)
    }

    fn list_insert(plan: &mut Plan, entity_idx: usize, pos: usize, value: usize) {
        plan.routes[entity_idx].visits.insert(pos, value);
    }

    fn list_get(plan: &Plan, entity_idx: usize, pos: usize) -> Option<usize> {
        plan.routes[entity_idx].visits.get(pos).copied()
    }

    fn list_set(plan: &mut Plan, entity_idx: usize, pos: usize, value: usize) {
        plan.routes[entity_idx].visits[pos] = value;
    }

    fn list_reverse(plan: &mut Plan, entity_idx: usize, start: usize, end: usize) {
        plan.routes[entity_idx].visits[start..end].reverse();
    }

    fn sublist_remove(plan: &mut Plan, entity_idx: usize, start: usize, end: usize) -> Vec<usize> {
        plan.routes[entity_idx].visits.drain(start..end).collect()
    }

    fn sublist_insert(plan: &mut Plan, entity_idx: usize, pos: usize, values: Vec<usize>) {
        plan.routes[entity_idx].visits.splice(pos..pos, values);
    }

    fn ruin_remove(plan: &mut Plan, entity_idx: usize, pos: usize) -> usize {
        plan.routes[entity_idx].visits.remove(pos)
    }

    fn ruin_insert(plan: &mut Plan, entity_idx: usize, pos: usize, value: usize) {
        plan.routes[entity_idx].visits.insert(pos, value);
    }

    fn index_to_element(plan: &Plan, idx: usize) -> usize {
        plan.elements[idx]
    }

    fn route_get(plan: &Plan, entity_idx: usize) -> Vec<usize> {
        plan.routes[entity_idx].visits.clone()
    }

    fn route_set(plan: &mut Plan, entity_idx: usize, route: Vec<usize>) {
        plan.routes[entity_idx].visits = route;
    }

    fn route_depot(_: &Plan, _: usize) -> usize {
        0
    }

    fn route_distance(_: &Plan, _: usize, from: usize, to: usize) -> i64 {
        ROUTE_DISTANCE_CALLS.fetch_add(1, Ordering::SeqCst);
        from.abs_diff(to) as i64
    }

    fn route_feasible(_: &Plan, _: usize, _: &[usize]) -> bool {
        true
    }

    fn savings_depot(_: &Plan, _: usize) -> usize {
        0
    }

    fn savings_distance(_: &Plan, _: usize, from: usize, to: usize) -> i64 {
        CLARKE_WRIGHT_DISTANCE_CALLS.fetch_add(1, Ordering::SeqCst);
        if from == 0 || to == 0 {
            100
        } else {
            from.abs_diff(to) as i64
        }
    }

    fn savings_feasible(_: &Plan, _: usize, _: &[usize]) -> bool {
        true
    }

    fn panic_route_distance(_: &Plan, _: usize, _: usize, _: usize) -> i64 {
        panic!("route_distance_fn must not be used by Clarke-Wright construction")
    }

    fn panic_route_feasible(_: &Plan, _: usize, _: &[usize]) -> bool {
        panic!("route_feasible_fn must not be used by Clarke-Wright construction")
    }

    fn panic_savings_depot(_: &Plan, _: usize) -> usize {
        panic!("savings_depot_fn must not be used by k-opt")
    }

    fn panic_savings_distance(_: &Plan, _: usize, _: usize, _: usize) -> i64 {
        panic!("savings_distance_fn must not be used by k-opt")
    }

    fn panic_savings_feasible(_: &Plan, _: usize, _: &[usize]) -> bool {
        panic!("savings_feasible_fn must not be used by k-opt")
    }

    fn descriptor() -> SolutionDescriptor {
        SolutionDescriptor::new("Plan", TypeId::of::<Plan>())
    }

    fn descriptor_entity_count(plan: &Plan, descriptor_index: usize) -> usize {
        if descriptor_index == 0 {
            plan.routes.len()
        } else {
            0
        }
    }

    fn director(plan: Plan) -> ScoreDirector<Plan, ()> {
        ScoreDirector::simple(plan, descriptor(), descriptor_entity_count)
    }

    #[allow(clippy::too_many_arguments)]
    fn list_slot(
        route_get_fn: Option<fn(&Plan, usize) -> Vec<usize>>,
        route_set_fn: Option<fn(&mut Plan, usize, Vec<usize>)>,
        route_depot_fn: Option<fn(&Plan, usize) -> usize>,
        route_distance_fn: Option<fn(&Plan, usize, usize, usize) -> i64>,
        route_feasible_fn: Option<fn(&Plan, usize, &[usize]) -> bool>,
        savings_depot_fn: Option<fn(&Plan, usize) -> usize>,
        savings_distance_fn: Option<fn(&Plan, usize, usize, usize) -> i64>,
        savings_feasible_fn: Option<fn(&Plan, usize, &[usize]) -> bool>,
    ) -> ListVariableSlot<Plan, usize, DefaultMeter, DefaultMeter> {
        ListVariableSlot::new(
            "Route",
            element_count,
            assigned_elements,
            list_len,
            list_remove,
            construction_list_remove,
            list_insert,
            list_get,
            list_set,
            list_reverse,
            sublist_remove,
            sublist_insert,
            ruin_remove,
            ruin_insert,
            index_to_element,
            entity_count,
            DefaultMeter::default(),
            DefaultMeter::default(),
            "visits",
            0,
            route_get_fn,
            route_set_fn,
            route_depot_fn,
            route_distance_fn,
            route_feasible_fn,
            savings_depot_fn,
            None,
            savings_distance_fn,
            savings_feasible_fn,
        )
    }

    #[test]
    fn dispatcher_clarke_wright_uses_construction_distance_not_route_distance() {
        CLARKE_WRIGHT_DISTANCE_CALLS.store(0, Ordering::SeqCst);
        ROUTE_DISTANCE_CALLS.store(0, Ordering::SeqCst);

        let plan = Plan {
            elements: vec![1, 2, 3],
            routes: vec![Route { visits: Vec::new() }],
            score: None,
        };
        let mut solver_scope = SolverScope::new(director(plan));
        let slot = list_slot(
            None,
            Some(route_set),
            Some(route_depot),
            Some(panic_route_distance),
            Some(panic_route_feasible),
            Some(savings_depot),
            Some(savings_distance),
            Some(savings_feasible),
        );

        let ran = solve_specialized_list_construction(
            ConstructionHeuristicType::ListClarkeWright,
            2,
            &mut solver_scope,
            &[slot],
        );

        assert!(ran);
        assert!(CLARKE_WRIGHT_DISTANCE_CALLS.load(Ordering::SeqCst) > 0);
        let mut visits = solver_scope.working_solution().routes[0].visits.clone();
        visits.sort_unstable();
        assert_eq!(visits, vec![1, 2, 3]);
    }

    #[test]
    fn dispatcher_k_opt_uses_route_distance_not_construction_distance() {
        CLARKE_WRIGHT_DISTANCE_CALLS.store(0, Ordering::SeqCst);
        ROUTE_DISTANCE_CALLS.store(0, Ordering::SeqCst);

        let plan = Plan {
            elements: vec![1, 2, 3, 4, 5],
            routes: vec![Route {
                visits: vec![1, 3, 2, 4],
            }],
            score: None,
        };
        let mut solver_scope = SolverScope::new(director(plan));
        let slot = list_slot(
            Some(route_get),
            Some(route_set),
            Some(route_depot),
            Some(route_distance),
            Some(route_feasible),
            Some(panic_savings_depot),
            Some(panic_savings_distance),
            Some(panic_savings_feasible),
        );

        let ran = solve_specialized_list_construction(
            ConstructionHeuristicType::ListKOpt,
            2,
            &mut solver_scope,
            &[slot],
        );

        assert!(ran);
        assert!(ROUTE_DISTANCE_CALLS.load(Ordering::SeqCst) > 0);
    }
}
