use super::*;
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
    customer_values: Vec<usize>,
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

fn observed_hook_values() -> &'static Mutex<Vec<usize>> {
    static VALUES: OnceLock<Mutex<Vec<usize>>> = OnceLock::new();
    VALUES.get_or_init(|| Mutex::new(Vec::new()))
}

fn record(value: usize) {
    observed_hook_values()
        .lock()
        .expect("hook recorder should lock")
        .push(value);
}

fn element_count(s: &Plan) -> usize {
    s.customer_values.len()
}

fn get_assigned(s: &Plan) -> Vec<usize> {
    s.routes
        .iter()
        .flat_map(|route| route.visits.iter().copied())
        .collect()
}

fn entity_count(s: &Plan) -> usize {
    s.routes.len()
}

fn route_len(s: &Plan, entity_idx: usize) -> usize {
    s.routes
        .get(entity_idx)
        .map_or(0, |route| route.visits.len())
}

fn assign_route(s: &mut Plan, entity_idx: usize, route: Vec<usize>) {
    s.routes[entity_idx].visits = route;
}

fn index_to_element(s: &Plan, idx: usize) -> usize {
    s.customer_values[idx]
}

fn depot(_: &Plan) -> usize {
    0
}

fn distance(_: &Plan, a: usize, b: usize) -> i64 {
    record(a);
    record(b);
    (a as i64 - b as i64).abs()
}

fn element_load(_: &Plan, elem: usize) -> i64 {
    record(elem);
    1
}

fn capacity(_: &Plan) -> i64 {
    3
}

#[test]
fn clarke_wright_hooks_receive_actual_list_values() {
    observed_hook_values()
        .lock()
        .expect("hook recorder should lock")
        .clear();

    let plan = Plan {
        customer_values: vec![10, 20, 30],
        routes: vec![Route { visits: Vec::new() }],
        score: None,
    };
    let director = ScoreDirector::simple(
        plan,
        SolutionDescriptor::new("Plan", TypeId::of::<Plan>()),
        |s, descriptor_index| {
            if descriptor_index == 0 {
                s.routes.len()
            } else {
                0
            }
        },
    );
    let mut solver_scope = SolverScope::new(director);
    let mut phase = ListClarkeWrightPhase::new(
        element_count,
        get_assigned,
        entity_count,
        route_len,
        assign_route,
        index_to_element,
        depot,
        distance,
        element_load,
        capacity,
        None,
        0,
    );

    phase.solve(&mut solver_scope);

    let mut route = solver_scope.working_solution().routes[0].visits.clone();
    route.sort_unstable();
    assert_eq!(route, vec![10, 20, 30]);

    let observed = observed_hook_values()
        .lock()
        .expect("hook recorder should lock")
        .clone();
    assert!(
        observed.iter().any(|&value| value >= 10),
        "expected Clarke-Wright hooks to observe actual customer values"
    );
    assert!(
        observed
            .iter()
            .copied()
            .filter(|&value| value != 0)
            .all(|value| matches!(value, 10 | 20 | 30)),
        "Clarke-Wright hooks must receive actual stored list values, not collection indices: {observed:?}"
    );
}

#[test]
fn clarke_wright_preserves_preassigned_routes() {
    let plan = Plan {
        customer_values: vec![0, 10, 20, 30],
        routes: vec![
            Route { visits: vec![20] },
            Route { visits: Vec::new() },
            Route { visits: Vec::new() },
        ],
        score: None,
    };
    let director = ScoreDirector::simple(
        plan,
        SolutionDescriptor::new("Plan", TypeId::of::<Plan>()),
        |s, descriptor_index| {
            if descriptor_index == 0 {
                s.routes.len()
            } else {
                0
            }
        },
    );
    let mut solver_scope = SolverScope::new(director);
    let mut phase = ListClarkeWrightPhase::new(
        element_count,
        get_assigned,
        entity_count,
        route_len,
        assign_route,
        index_to_element,
        depot,
        distance,
        element_load,
        capacity,
        None,
        0,
    );

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().routes[0].visits, vec![20]);

    let mut remaining: Vec<_> = solver_scope.working_solution().routes[1..]
        .iter()
        .flat_map(|route| route.visits.iter().copied())
        .collect();
    remaining.sort_unstable();
    assert_eq!(remaining, vec![10, 30]);
}
