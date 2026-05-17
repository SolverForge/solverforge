#[test]
fn grouped_assignment_vnd_improves_soft_assignment_cost_after_feasibility() {
    let descriptor = coverage_plan_descriptor();
    let director = CoverageDirector {
        working_solution: coverage_plan(
            2,
            vec![coverage_slot_with_worker_penalties(
                true,
                0,
                Some(0),
                &[0, 1],
                &[10, 0],
            )],
        ),
        descriptor,
    };
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();
    solver_scope.calculate_score();

    assert_eq!(
        solver_scope.current_score().copied(),
        Some(HardSoftScore::of(0, -10))
    );

    let config = solverforge_config::LocalSearchConfig {
        local_search_type: solverforge_config::LocalSearchType::VariableNeighborhoodDescent,
        neighborhoods: vec![solverforge_config::MoveSelectorConfig::GroupedScalarMoveSelector(
            solverforge_config::GroupedScalarMoveSelectorConfig {
                group_name: "slot_assignment".to_string(),
                value_candidate_limit: None,
                max_moves_per_step: None,
                require_hard_improvement: false,
            },
        )],
        ..solverforge_config::LocalSearchConfig::default()
    };
    let mut phase =
        crate::builder::build_local_search(Some(&config), &assignment_model(), Some(7));
    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().slots[0].assigned, Some(1));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(HardSoftScore::of(0, 0))
    );
}
