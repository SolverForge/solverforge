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

fn observed_actual_value_hook_values() -> &'static Mutex<Vec<usize>> {
    static VALUES: OnceLock<Mutex<Vec<usize>>> = OnceLock::new();
    VALUES.get_or_init(|| Mutex::new(Vec::new()))
}

fn record_actual_value(value: usize) {
    observed_actual_value_hook_values()
        .lock()
        .expect("actual-value hook recorder should lock")
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

fn depot(_: &Plan, _: usize) -> usize {
    0
}

fn distance(_: &Plan, _: usize, a: usize, b: usize) -> i64 {
    record(a);
    record(b);
    (a as i64 - b as i64).abs()
}

fn feasible(_: &Plan, _: usize, route: &[usize]) -> bool {
    for &value in route {
        record(value);
    }
    route.len() <= 3
}

fn actual_value_distance(_: &Plan, _: usize, a: usize, b: usize) -> i64 {
    record_actual_value(a);
    record_actual_value(b);
    (a as i64 - b as i64).abs()
}

fn actual_value_feasible(_: &Plan, _: usize, route: &[usize]) -> bool {
    for &value in route {
        record_actual_value(value);
    }
    route.len() <= 3
}

fn owner_feasible(_: &Plan, entity_idx: usize, route: &[usize]) -> bool {
    route.len() <= 2
        && match entity_idx {
            0 => route.iter().all(|&value| value < 20),
            1 => route.iter().all(|&value| value >= 20),
            _ => false,
        }
}

fn observed_owner_distance_calls() -> &'static Mutex<Vec<(usize, usize, usize)>> {
    static VALUES: OnceLock<Mutex<Vec<(usize, usize, usize)>>> = OnceLock::new();
    VALUES.get_or_init(|| Mutex::new(Vec::new()))
}

fn depot_by_owner(_: &Plan, entity_idx: usize) -> usize {
    100 + entity_idx
}

fn distance_records_owner(_: &Plan, entity_idx: usize, a: usize, b: usize) -> i64 {
    observed_owner_distance_calls()
        .lock()
        .expect("owner hook recorder should lock")
        .push((entity_idx, a, b));
    (a as i64 - b as i64).abs()
}

fn single_visit_route(_: &Plan, _: usize, route: &[usize]) -> bool {
    route.len() <= 1
}

fn capacity_feasible(_: &Plan, _: usize, route: &[usize]) -> bool {
    route.iter().sum::<usize>() <= 30
}

fn scarce_owner_distance(_: &Plan, _: usize, a: usize, b: usize) -> i64 {
    if a == 0 || b == 0 {
        100
    } else {
        match (a.min(b), a.max(b)) {
            (1, 2) => 0,
            (2, 3) => 1,
            _ => 100,
        }
    }
}

fn scarce_owner_feasible(_: &Plan, entity_idx: usize, route: &[usize]) -> bool {
    let mut sorted = route.to_vec();
    sorted.sort_unstable();
    match entity_idx {
        0 => sorted == [4] || sorted == [1, 2, 3],
        1 => sorted == [3],
        2 => sorted == [1, 2],
        _ => false,
    }
}

#[test]
fn clarke_wright_hooks_receive_actual_list_values() {
    observed_actual_value_hook_values()
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
        actual_value_distance,
        actual_value_feasible,
        0,
    );

    phase.solve(&mut solver_scope);

    let mut route = solver_scope.working_solution().routes[0].visits.clone();
    route.sort_unstable();
    assert_eq!(route, vec![10, 20, 30]);

    let observed = observed_actual_value_hook_values()
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
fn clarke_wright_route_feasible_preserves_capacity_hook_behavior() {
    let plan = Plan {
        customer_values: vec![10, 20, 30],
        routes: vec![Route { visits: Vec::new() }, Route { visits: Vec::new() }],
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
        capacity_feasible,
        0,
    );

    phase.solve(&mut solver_scope);

    let solution = solver_scope.working_solution();
    let mut assigned: Vec<_> = solution
        .routes
        .iter()
        .flat_map(|route| route.visits.iter().copied())
        .collect();
    assigned.sort_unstable();

    assert_eq!(assigned, vec![10, 20, 30]);
    assert!(solution
        .routes
        .iter()
        .all(|route| route.visits.iter().sum::<usize>() <= 30));
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
        feasible,
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

#[test]
fn clarke_wright_assigns_constructed_routes_to_feasible_owners() {
    let plan = Plan {
        customer_values: vec![10, 11, 20, 21],
        routes: vec![Route { visits: Vec::new() }, Route { visits: Vec::new() }],
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
        owner_feasible,
        0,
    );

    phase.solve(&mut solver_scope);

    let solution = solver_scope.working_solution();
    let mut left = solution.routes[0].visits.clone();
    let mut right = solution.routes[1].visits.clone();
    left.sort_unstable();
    right.sort_unstable();

    assert_eq!(left, vec![10, 11]);
    assert_eq!(right, vec![20, 21]);
}

#[test]
fn clarke_wright_uses_owner_depots_for_savings() {
    observed_owner_distance_calls()
        .lock()
        .expect("owner hook recorder should lock")
        .clear();

    let plan = Plan {
        customer_values: vec![10, 20],
        routes: vec![Route { visits: Vec::new() }, Route { visits: Vec::new() }],
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
        depot_by_owner,
        distance_records_owner,
        single_visit_route,
        0,
    );

    phase.solve(&mut solver_scope);

    let observed = observed_owner_distance_calls()
        .lock()
        .expect("owner hook recorder should lock")
        .clone();

    assert!(observed.contains(&(0, 100, 10)));
    assert!(observed.contains(&(1, 101, 10)));
}

#[test]
fn clarke_wright_skips_merge_that_breaks_global_owner_matching() {
    let plan = Plan {
        customer_values: vec![1, 2, 3, 4],
        routes: vec![
            Route { visits: Vec::new() },
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
        scarce_owner_distance,
        scarce_owner_feasible,
        0,
    );

    phase.solve(&mut solver_scope);

    let solution = solver_scope.working_solution();
    let mut assigned: Vec<_> = solution
        .routes
        .iter()
        .flat_map(|route| route.visits.iter().copied())
        .collect();
    assigned.sort_unstable();

    assert_eq!(assigned, vec![1, 2, 3, 4]);
    assert!(solution.routes.iter().any(|route| {
        let mut visits = route.visits.clone();
        visits.sort_unstable();
        visits == [1, 2]
    }));
    assert!(solution.routes.iter().any(|route| route.visits == [3]));
    assert!(solution.routes.iter().any(|route| route.visits == [4]));
}
