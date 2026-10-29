use super::*;

#[test]
fn compiled_clarke_wright_refreshes_current_assignment_before_a_second_execution() {
    let slot = static_slot();
    let plan = Plan {
        score: None,
        elements: vec![1, 2, 3, 4],
        routes: vec![Vec::new(), Vec::new()],
    };
    let binding = bind_runtime_list_source(&slot, &plan)
        .expect("well-formed source binds before phase construction");
    let source_index = binding.into_source_index();
    let unassigned = unassigned_from_current_assignment(&slot, &source_index, &plan)
        .expect("initial assignment resolves against the bound source");
    let mut scope = SolverScope::new(round_robin_director(plan));
    execute_runtime_list_clarke_wright(
        &slot,
        &source_index,
        &unassigned,
        crate::scope::StepControlPolicy::ObserveConfigLimits,
        &mut scope,
    )
    .expect("compiled Clarke-Wright executes");
    let first_routes = scope.working_solution().routes.clone();
    let unassigned =
        unassigned_from_current_assignment(&slot, &source_index, scope.working_solution())
            .expect("updated assignment resolves against the bound source");
    execute_runtime_list_clarke_wright(
        &slot,
        &source_index,
        &unassigned,
        crate::scope::StepControlPolicy::ObserveConfigLimits,
        &mut scope,
    )
    .expect("compiled Clarke-Wright refreshes current assignment");

    assert_eq!(scope.working_solution().routes, first_routes);
}

#[test]
fn compiled_round_robin_refreshes_current_assignment_before_a_second_execution() {
    let slot = static_slot();
    let plan = round_robin_plan();
    let binding = bind_runtime_list_source(&slot, &plan)
        .expect("well-formed source binds before phase construction");
    let source_index = binding.into_source_index();
    let unassigned = unassigned_from_current_assignment(&slot, &source_index, &plan)
        .expect("initial assignment resolves against the bound source");
    let mut scope = SolverScope::new(round_robin_director(plan));
    execute_runtime_list_round_robin(
        &slot,
        &source_index,
        &unassigned,
        crate::scope::StepControlPolicy::ObserveConfigLimits,
        &mut scope,
    )
    .expect("compiled round-robin executes");
    let first_routes = scope.working_solution().routes.clone();
    let unassigned =
        unassigned_from_current_assignment(&slot, &source_index, scope.working_solution())
            .expect("updated assignment resolves against the bound source");
    execute_runtime_list_round_robin(
        &slot,
        &source_index,
        &unassigned,
        crate::scope::StepControlPolicy::ObserveConfigLimits,
        &mut scope,
    )
    .expect("compiled round-robin refreshes current assignment");

    assert_eq!(scope.working_solution().routes, first_routes);
}
