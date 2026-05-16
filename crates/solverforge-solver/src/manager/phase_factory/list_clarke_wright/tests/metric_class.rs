use std::any::TypeId;
use std::sync::atomic::{AtomicUsize, Ordering};

use solverforge_core::domain::SolutionDescriptor;
use solverforge_scoring::ScoreDirector;

use super::*;

static DISTANCE_CALLS: AtomicUsize = AtomicUsize::new(0);

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

fn grouped_owner_feasible(_: &Plan, entity_idx: usize, route: &[usize]) -> bool {
    let mut sorted = route.to_vec();
    sorted.sort_unstable();

    match entity_idx {
        0 => route.len() <= 1,
        1 => route.len() <= 1 || sorted == [1, 2],
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
    DISTANCE_CALLS.store(0, Ordering::SeqCst);
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
