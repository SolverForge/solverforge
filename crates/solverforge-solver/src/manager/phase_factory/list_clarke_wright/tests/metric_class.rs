use crate::phase::Phase;
use crate::scope::solver::SolverScope;
use std::any::TypeId;
use std::sync::atomic::{AtomicUsize, Ordering};

use solverforge_core::domain::SolutionDescriptor;
use solverforge_scoring::ScoreDirector;

use super::*;

static DISTANCE_CALLS: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static FEASIBLE_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn counting_feasible(plan: &Plan, entity_idx: usize, route: &[usize]) -> bool {
    FEASIBLE_CALLS.with(|calls| calls.set(calls.get() + 1));
    feasible(plan, entity_idx, route)
}

fn shared_metric_class(_: &Plan, _: usize) -> usize {
    0
}

fn counting_distance(_: &Plan, _: usize, a: usize, b: usize) -> i64 {
    DISTANCE_CALLS.fetch_add(1, Ordering::SeqCst);
    (a as i64 - b as i64).abs()
}

fn simple_distance(_: &Plan, _: usize, a: usize, b: usize) -> i64 {
    (a as i64 - b as i64).abs()
}

fn conflict_distance(_: &Plan, _: usize, a: usize, b: usize) -> i64 {
    if a == 0 || b == 0 {
        100
    } else {
        match (a.min(b), a.max(b)) {
            (3, 4) => 0,
            (1, 2) => 1,
            _ => 100,
        }
    }
}

fn grouped_owner_feasible(_: &Plan, entity_idx: usize, route: &[usize]) -> bool {
    let mut sorted = route.to_vec();
    sorted.sort_unstable();

    match entity_idx {
        0 => route.len() <= 1,
        1 => route.len() <= 1 || sorted == [1, 2],
        _ => false,
    }
}

fn conflicting_shared_class_feasible(_: &Plan, entity_idx: usize, route: &[usize]) -> bool {
    let mut sorted = route.to_vec();
    sorted.sort_unstable();

    match sorted.as_slice() {
        [1, 2] | [3, 4] => entity_idx == 0,
        [1] | [2] | [3] | [4] => true,
        _ => false,
    }
}

#[test]
fn clarke_wright_computes_savings_once_per_metric_class() {
    DISTANCE_CALLS.store(0, Ordering::SeqCst);
    let plan = Plan {
        customer_values: vec![1, 2, 3, 4],
        routes: vec![
            Route { visits: Vec::new() },
            Route { visits: Vec::new() },
            Route { visits: Vec::new() },
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
        crate::builder::context::list::usize_element_source_key,
        depot,
        counting_distance,
        feasible,
        0,
    )
    .with_metric_class_fn(shared_metric_class);

    phase.solve(&mut solver_scope);

    let pair_count = 4 * 3 / 2;
    assert_eq!(DISTANCE_CALLS.load(Ordering::SeqCst), pair_count * 3);
}

#[test]
fn clarke_wright_keeps_feasibility_owner_specific_with_shared_metric_class() {
    let plan = Plan {
        customer_values: vec![1, 2],
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
        crate::builder::context::list::usize_element_source_key,
        depot,
        simple_distance,
        grouped_owner_feasible,
        0,
    )
    .with_metric_class_fn(shared_metric_class);

    phase.solve(&mut solver_scope);

    let solution = solver_scope.working_solution();
    assert!(solution.routes[0].visits.len() <= 1);

    let mut owner_one_visits = solution.routes[1].visits.clone();
    owner_one_visits.sort_unstable();
    assert_eq!(owner_one_visits, vec![1, 2]);
}

#[test]
fn clarke_wright_checks_owner_matching_inside_shared_metric_class() {
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
        crate::builder::context::list::usize_element_source_key,
        depot,
        conflict_distance,
        conflicting_shared_class_feasible,
        0,
    )
    .with_metric_class_fn(shared_metric_class);

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
        visits == [3, 4]
    }));
    assert!(!solution.routes.iter().any(|route| {
        let mut visits = route.visits.clone();
        visits.sort_unstable();
        visits == [1, 2]
    }));
}

fn solve_counting_feasibility(shared_by_metric_class: bool) -> (Vec<Vec<usize>>, usize) {
    FEASIBLE_CALLS.with(|calls| calls.set(0));
    let plan = Plan {
        customer_values: vec![1, 2, 3, 4, 5, 6],
        routes: (0..4).map(|_| Route { visits: Vec::new() }).collect(),
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
        crate::builder::context::list::usize_element_source_key,
        depot,
        simple_distance,
        counting_feasible,
        0,
    )
    .with_metric_class_fn(shared_metric_class)
    .with_savings_feasible_shared_by_metric_class(shared_by_metric_class);

    phase.solve(&mut solver_scope);

    let routes = solver_scope
        .working_solution()
        .routes
        .iter()
        .map(|route| route.visits.clone())
        .collect();
    (routes, FEASIBLE_CALLS.with(std::cell::Cell::get))
}

#[test]
fn shared_savings_feasibility_asks_one_owner_per_metric_class() {
    let (per_owner_routes, per_owner_calls) = solve_counting_feasibility(false);
    let (shared_routes, shared_calls) = solve_counting_feasibility(true);

    assert_eq!(shared_routes, per_owner_routes);
    assert!(
        shared_calls * 2 < per_owner_calls,
        "shared {shared_calls} vs per-owner {per_owner_calls}"
    );
}
