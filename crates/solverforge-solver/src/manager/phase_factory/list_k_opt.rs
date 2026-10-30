//! Per-route k-opt polishing phase for list-variable problems.
//!
//! Applies 2-opt local search to each entity's route independently,
//! improving solution quality after construction without modifying the
//! overall assignment structure.

use solverforge_core::domain::PlanningSolution;

use crate::builder::context::list_access::{ListAccess, RouteAccess, RouteSequenceAccess};
use crate::builder::context::RuntimeListSlot;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;

#[cfg(test)]
mod facade;
mod kernel;

#[cfg(test)]
pub(crate) use facade::ListKOptPhase;
pub(crate) use kernel::{run_list_k_opt, ListKOptAccess};

impl<S, V, DM, IDM> ListKOptAccess<S> for RuntimeListSlot<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Into<usize> + Send + Sync + std::fmt::Debug + 'static,
    DM: Clone + Send + Sync + std::fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + std::fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn descriptor_index(&self) -> usize {
        ListAccess::descriptor_index(self)
    }

    fn entity_count(&self, solution: &S) -> usize {
        ListAccess::entity_count(self, solution)
    }

    fn route_values(&self, solution: &S, entity_index: usize) -> Vec<usize> {
        RouteSequenceAccess::route_values(self, solution, entity_index)
            .expect("compiled list K-opt route access must be validated before execution")
    }

    fn replace_route(&self, solution: &mut S, entity_index: usize, route: Vec<usize>) {
        RouteSequenceAccess::replace_route(self, solution, entity_index, route)
            .expect("compiled list K-opt route replacement must be validated before execution");
    }

    fn route_depot(&self, solution: &S, entity_index: usize) -> usize {
        RouteAccess::route_depot(self, solution, entity_index)
            .expect("compiled list K-opt route metadata must be validated before execution")
    }

    fn route_distance(&self, solution: &S, entity_index: usize, from: usize, to: usize) -> i64 {
        RouteAccess::route_distance(self, solution, entity_index, from, to)
            .expect("compiled list K-opt route metadata must be validated before execution")
    }

    fn route_feasible(&self, solution: &S, entity_index: usize, route: &[usize]) -> bool {
        RouteAccess::route_feasible(self, solution, entity_index, route)
            .expect("compiled list K-opt route metadata must be validated before execution")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase::Phase;
    use crate::scope::solver::SolverScope;
    use std::any::TypeId;
    use std::sync::{Mutex, OnceLock};

    use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
    use solverforge_core::score::SoftScore;
    use solverforge_scoring::ScoreDirector;

    #[derive(Clone, Debug)]
    struct Route {
        visits: Vec<usize>,
    }

    #[derive(Clone, Debug)]
    struct Plan {
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

    fn entity_count(plan: &Plan) -> usize {
        plan.routes.len()
    }

    fn get_route(plan: &Plan, entity_idx: usize) -> Vec<usize> {
        plan.routes[entity_idx].visits.clone()
    }

    fn set_route(plan: &mut Plan, entity_idx: usize, route: Vec<usize>) {
        plan.routes[entity_idx].visits = route;
    }

    fn depot(_: &Plan, _: usize) -> usize {
        0
    }

    fn line_distance(_: &Plan, _: usize, left: usize, right: usize) -> i64 {
        (left as i64 - right as i64).abs()
    }

    fn extreme_distance(_: &Plan, _: usize, left: usize, right: usize) -> i64 {
        if left == right {
            0
        } else {
            i64::MAX
        }
    }

    fn owner_distance_calls() -> &'static Mutex<Vec<usize>> {
        static CALLS: OnceLock<Mutex<Vec<usize>>> = OnceLock::new();
        CALLS.get_or_init(|| Mutex::new(Vec::new()))
    }

    fn recording_line_distance(_: &Plan, entity_idx: usize, left: usize, right: usize) -> i64 {
        owner_distance_calls()
            .lock()
            .expect("owner distance recorder should lock")
            .push(entity_idx);
        (left as i64 - right as i64).abs()
    }

    fn keep_original_middle_pair(_: &Plan, _: usize, route: &[usize]) -> bool {
        route.get(1) == Some(&3) && route.get(2) == Some(&2)
    }

    fn director(plan: Plan) -> ScoreDirector<Plan, ()> {
        ScoreDirector::simple(
            plan,
            SolutionDescriptor::new("Plan", TypeId::of::<Plan>()),
            |s, descriptor_index| {
                if descriptor_index == 0 {
                    s.routes.len()
                } else {
                    0
                }
            },
        )
    }

    #[test]
    fn list_k_opt_improves_route_with_unified_route_hooks() {
        let plan = Plan {
            routes: vec![Route {
                visits: vec![1, 3, 2, 4],
            }],
            score: None,
        };
        let mut solver_scope = SolverScope::new(director(plan));
        let mut phase = ListKOptPhase::<Plan, usize>::new(
            2,
            entity_count,
            get_route,
            set_route,
            depot,
            line_distance,
            None,
            0,
        );

        phase.solve(&mut solver_scope);

        assert_eq!(
            solver_scope.working_solution().routes[0].visits,
            vec![1, 2, 3, 4]
        );
    }

    #[test]
    fn list_k_opt_feasibility_hook_rejects_improving_reversal() {
        let plan = Plan {
            routes: vec![Route {
                visits: vec![1, 3, 2, 4],
            }],
            score: None,
        };
        let mut solver_scope = SolverScope::new(director(plan));
        let mut phase = ListKOptPhase::<Plan, usize>::new(
            2,
            entity_count,
            get_route,
            set_route,
            depot,
            line_distance,
            Some(keep_original_middle_pair),
            0,
        );

        phase.solve(&mut solver_scope);

        assert_eq!(
            solver_scope.working_solution().routes[0].visits,
            vec![1, 3, 2, 4]
        );
    }

    #[test]
    fn list_k_opt_extreme_distances_do_not_overflow_or_accept_wrapped_improvement() {
        let plan = Plan {
            routes: vec![Route {
                visits: vec![1, 3, 2, 4],
            }],
            score: None,
        };
        let mut solver_scope = SolverScope::new(director(plan));
        let mut phase = ListKOptPhase::<Plan, usize>::new(
            2,
            entity_count,
            get_route,
            set_route,
            depot,
            extreme_distance,
            None,
            0,
        );

        phase.solve(&mut solver_scope);

        assert_eq!(
            solver_scope.working_solution().routes[0].visits,
            vec![1, 3, 2, 4]
        );
    }

    #[test]
    fn list_k_opt_uses_owner_specific_distance_hook() {
        owner_distance_calls()
            .lock()
            .expect("owner distance recorder should lock")
            .clear();
        let plan = Plan {
            routes: vec![
                Route {
                    visits: vec![1, 3, 2, 4],
                },
                Route {
                    visits: vec![2, 4, 3, 5],
                },
            ],
            score: None,
        };
        let mut solver_scope = SolverScope::new(director(plan));
        let mut phase = ListKOptPhase::<Plan, usize>::new(
            2,
            entity_count,
            get_route,
            set_route,
            depot,
            recording_line_distance,
            None,
            0,
        );

        phase.solve(&mut solver_scope);

        let calls = owner_distance_calls()
            .lock()
            .expect("owner distance recorder should lock")
            .clone();
        assert!(calls.contains(&0));
        assert!(calls.contains(&1));
    }

    #[test]
    fn list_k_opt_interruption_discards_buffered_reversal_telemetry() {
        let original_route = vec![3, 1, 2, 4];
        let plan = Plan {
            routes: vec![Route {
                visits: original_route.clone(),
            }],
            score: None,
        };
        let mut solver_scope = SolverScope::new(director(plan));
        solver_scope.start_solving();
        solver_scope.inphase_move_count_limit = Some(1);
        let mut phase = ListKOptPhase::<Plan, usize>::new(
            2,
            entity_count,
            get_route,
            set_route,
            depot,
            line_distance,
            None,
            0,
        );

        phase.solve(&mut solver_scope);

        assert_eq!(
            solver_scope.working_solution().routes[0].visits,
            original_route
        );
        assert_eq!(solver_scope.stats().moves_accepted(), 1);
        assert_eq!(solver_scope.stats().moves_applied(), 0);
        assert_eq!(
            solver_scope.terminal_reason(),
            crate::manager::SolverTerminalReason::TerminatedByConfig
        );
    }
}
