use std::any::TypeId;

use solverforge_core::domain::SolutionDescriptor;
use solverforge_scoring::ScoreDirector;

use super::*;

fn owner_binding_distance(_: &Plan, entity_idx: usize, a: usize, b: usize) -> i64 {
    if a == 0 || b == 0 {
        return 1_000;
    }

    match (entity_idx, a.min(b), a.max(b)) {
        (0, 1, 2) => 1,
        (1, 2, 3) => 10,
        (1, 1, 2) => 500,
        _ => 900,
    }
}

fn owner_binding_feasible(_: &Plan, entity_idx: usize, route: &[usize]) -> bool {
    let mut sorted = route.to_vec();
    sorted.sort_unstable();

    match entity_idx {
        0 => route.len() <= 1,
        1 => route.len() <= 1 || sorted == [1, 2] || sorted == [2, 3],
        _ => false,
    }
}

#[test]
fn clarke_wright_skips_savings_for_owner_that_cannot_take_merge() {
    let plan = Plan {
        customer_values: vec![1, 2, 3],
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
        owner_binding_distance,
        owner_binding_feasible,
        0,
    );

    phase.solve(&mut solver_scope);

    let solution = solver_scope.working_solution();
    assert!(solution.routes.iter().any(|route| {
        let mut visits = route.visits.clone();
        visits.sort_unstable();
        visits == [2, 3]
    }));
    assert!(!solution.routes.iter().any(|route| {
        let mut visits = route.visits.clone();
        visits.sort_unstable();
        visits == [1, 2]
    }));
}
