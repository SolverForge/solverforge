use super::*;
use crate::phase::Phase;
use crate::scope::solver::SolverScope;

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
        crate::builder::context::list::usize_element_source_key,
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
        crate::builder::context::list::usize_element_source_key,
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
        crate::builder::context::list::usize_element_source_key,
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
        crate::builder::context::list::usize_element_source_key,
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

#[test]
fn clarke_wright_completes_unmatched_routes_with_savings_insertion() {
    let plan = Plan {
        customer_values: vec![1, 2, 3, 4, 5],
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
        scarce_capacity_distance,
        scarce_capacity_feasible,
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

    assert_eq!(assigned, vec![1, 2, 3, 4, 5]);
    for (entity_idx, route) in solution.routes.iter().enumerate() {
        assert!(scarce_capacity_feasible(
            solution,
            entity_idx,
            &route.visits
        ));
    }
}
