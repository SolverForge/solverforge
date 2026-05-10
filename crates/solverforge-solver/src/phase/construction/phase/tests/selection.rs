
#[test]
fn test_construction_first_fit() {
    let director = create_simple_nqueens_director(4);
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let values: Vec<i64> = (0..4).collect();
    let placer = create_placer(values);
    let forager = FirstFitForager::new();
    let mut phase = ConstructionHeuristicPhase::new(placer, forager);

    phase.solve(&mut solver_scope);

    let solution = solver_scope.working_solution();
    assert_eq!(solution.queens.len(), 4);
    for queen in &solution.queens {
        assert!(queen.row.is_some(), "Queen should have a row assigned");
    }

    assert!(solver_scope.best_solution().is_some());
    assert!(solver_scope.stats().moves_evaluated > 0);
}

#[test]
fn test_construction_best_fit() {
    let director = create_simple_nqueens_director(4);
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let values: Vec<i64> = (0..4).collect();
    let placer = create_placer(values);
    let forager = BestFitForager::new();
    let mut phase = ConstructionHeuristicPhase::new(placer, forager);

    phase.solve(&mut solver_scope);

    let solution = solver_scope.working_solution();
    for queen in &solution.queens {
        assert!(queen.row.is_some(), "Queen should have a row assigned");
    }

    assert!(solver_scope.best_solution().is_some());
    assert!(solver_scope.best_score().is_some());
    assert_eq!(solver_scope.stats().moves_evaluated, 16);
}

#[test]
fn best_fit_keeps_current_when_every_assignment_is_worse() {
    let director = ConstructionPauseDirector::new(ConstructionPauseSolution::new(None));
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let placer = ScoredConstructionPlacer::new(vec![-5, -1], true);
    let mut phase = ConstructionHeuristicPhase::new(placer, BestFitForager::new());

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().entities[0].value, None);
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(SoftScore::of(0))
    );
    assert_eq!(solver_scope.stats().moves_accepted, 0);
    assert_eq!(solver_scope.stats().step_count, 1);
}

#[test]
fn best_fit_assigns_when_candidate_is_strictly_better_than_none() {
    let director = ConstructionPauseDirector::new(ConstructionPauseSolution::new(None));
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let placer = ScoredConstructionPlacer::new(vec![-5, 7], true);
    let mut phase = ConstructionHeuristicPhase::new(placer, BestFitForager::new());

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().entities[0].value, Some(7));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(SoftScore::of(7))
    );
    assert_eq!(solver_scope.stats().moves_accepted, 1);
}

#[test]
fn first_fit_optional_construction_keeps_current_when_baseline_is_not_beaten() {
    let director = ConstructionPauseDirector::new(ConstructionPauseSolution::new(None));
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let placer = ScoredConstructionPlacer::new(vec![-5, -1], true);
    let mut phase = ConstructionHeuristicPhase::new(placer, FirstFitForager::new());

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().entities[0].value, None);
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(SoftScore::of(0))
    );
    assert_eq!(solver_scope.stats().moves_accepted, 0);
    assert_eq!(solver_scope.stats().step_count, 1);
}

#[test]
fn first_fit_forced_construction_assigns_first_doable_candidate_even_when_worse() {
    let director = ConstructionPauseDirector::new(ConstructionPauseSolution::new(None));
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let placer = ScoredConstructionPlacer::new(vec![-5, -1], true);
    let mut phase = ConstructionHeuristicPhase::new(placer, FirstFitForager::new())
        .with_construction_obligation(
            solverforge_config::ConstructionObligation::AssignWhenCandidateExists,
        );

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().entities[0].value, Some(-5));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(SoftScore::of(-5))
    );
    assert_eq!(solver_scope.stats().moves_accepted, 1);
    assert_eq!(solver_scope.stats().step_count, 1);
}

#[test]
fn best_fit_forced_construction_assigns_best_candidate_even_when_worse() {
    let director = ConstructionPauseDirector::new(ConstructionPauseSolution::new(None));
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let placer = ScoredConstructionPlacer::new(vec![-5, -1], true);
    let mut phase = ConstructionHeuristicPhase::new(placer, BestFitForager::new())
        .with_construction_obligation(
            solverforge_config::ConstructionObligation::AssignWhenCandidateExists,
        );

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().entities[0].value, Some(-1));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(SoftScore::of(-1))
    );
    assert_eq!(solver_scope.stats().moves_accepted, 1);
    assert_eq!(solver_scope.stats().step_count, 1);
}

#[test]
fn first_fit_optional_construction_skips_worse_candidate_and_takes_later_improvement() {
    let director = ConstructionPauseDirector::new(ConstructionPauseSolution::new(None));
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let placer = ScoredConstructionPlacer::new(vec![-5, 7], true);
    let mut phase = ConstructionHeuristicPhase::new(placer, FirstFitForager::new());

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().entities[0].value, Some(7));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(SoftScore::of(7))
    );
    assert_eq!(solver_scope.stats().moves_accepted, 1);
    assert_eq!(solver_scope.stats().step_count, 1);
}

#[test]
fn first_fit_optional_construction_takes_first_improving_candidate() {
    let director = ConstructionPauseDirector::new(ConstructionPauseSolution::new(None));
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let placer = ScoredConstructionPlacer::new(vec![7, -5], true);
    let mut phase = ConstructionHeuristicPhase::new(placer, FirstFitForager::new());

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().entities[0].value, Some(7));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(SoftScore::of(7))
    );
    assert_eq!(solver_scope.stats().moves_accepted, 1);
    assert_eq!(solver_scope.stats().step_count, 1);
}

#[test]
fn first_fit_required_construction_still_selects_first_doable_candidate() {
    let director = ConstructionPauseDirector::new(ConstructionPauseSolution::new(None));
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let placer = ScoredConstructionPlacer::new(vec![3, 4], false);
    let mut phase = ConstructionHeuristicPhase::new(placer, FirstFitForager::new());

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().entities[0].value, Some(3));
    assert_eq!(solver_scope.stats().moves_accepted, 1);
    assert_eq!(solver_scope.stats().step_count, 1);
}

#[derive(Debug)]
struct ReferencingConstructionForager<'a> {
    tail_offset: &'a usize,
}

impl<'a> ConstructionForager<ConstructionPauseSolution, ConstructionPauseMove>
    for ReferencingConstructionForager<'a>
{
    fn pick_move_index<D: Director<ConstructionPauseSolution>>(
        &self,
        placement: &Placement<ConstructionPauseSolution, ConstructionPauseMove>,
        _score_director: &mut D,
    ) -> ConstructionChoice {
        placement
            .moves
            .len()
            .checked_sub(self.tail_offset.saturating_add(1))
            .map(ConstructionChoice::Select)
            .unwrap_or(ConstructionChoice::KeepCurrent)
    }
}

#[test]
fn construction_phase_accepts_custom_forager_without_static_router() {
    let director = ConstructionPauseDirector::new(ConstructionPauseSolution::new(None));
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let offset = 0;
    let placer = ScoredConstructionPlacer::new(vec![3, 5, 8], false);
    let forager = ReferencingConstructionForager {
        tail_offset: &offset,
    };
    let mut phase = ConstructionHeuristicPhase::new(placer, forager);

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().entities[0].value, Some(8));
}

#[test]
fn best_fit_prefers_equal_score_candidate_over_keep_current() {
    let director = ConstructionPauseDirector::new(ConstructionPauseSolution::new(None));
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let placer = ScoredConstructionPlacer::new(vec![0, -1], true);
    let mut phase = ConstructionHeuristicPhase::new(placer, BestFitForager::new());

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().entities[0].value, Some(0));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(SoftScore::of(0))
    );
    assert_eq!(solver_scope.stats().moves_accepted, 1);
}

#[test]
fn best_fit_progresses_across_equal_score_plateau() {
    let director = ConstructionPauseDirector::with_score_mode(
        ConstructionPauseSolution::with_entity_count(2, None),
        ConstructionPauseScoreMode::CompletionBonus {
            incomplete_score: 0,
            complete_score: 5,
        },
    );
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let placer = ScoredConstructionPlacer::new(vec![1], true);
    let mut phase = ConstructionHeuristicPhase::new(placer, BestFitForager::new());

    phase.solve(&mut solver_scope);

    assert_eq!(
        solver_scope
            .working_solution()
            .entities
            .iter()
            .map(|entity| entity.value)
            .collect::<Vec<_>>(),
        vec![Some(1), Some(1)]
    );
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(SoftScore::of(5))
    );
    assert_eq!(solver_scope.stats().moves_accepted, 2);
}

#[test]
fn first_feasible_keeps_current_when_baseline_is_already_feasible() {
    let director = ConstructionPauseDirector::new(ConstructionPauseSolution::new(None));
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let placer = ScoredConstructionPlacer::new(vec![2, 4], true);
    let mut phase = ConstructionHeuristicPhase::new(placer, FirstFeasibleForager::new());

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().entities[0].value, None);
    assert_eq!(solver_scope.stats().moves_accepted, 0);
}

#[test]
fn first_feasible_selects_first_feasible_move_when_baseline_is_infeasible() {
    let director = ConstructionPauseDirector::with_score_mode(
        ConstructionPauseSolution::new(None),
        ConstructionPauseScoreMode::AssignedSum {
            unassigned_score: -2,
        },
    );
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let placer = ScoredConstructionPlacer::new(vec![-3, 1, 5], true);
    let mut phase = ConstructionHeuristicPhase::new(placer, FirstFeasibleForager::new());

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().entities[0].value, Some(1));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(SoftScore::of(1))
    );
    assert_eq!(solver_scope.stats().moves_accepted, 1);
}

#[test]
fn first_feasible_prefers_equal_score_candidate_over_infeasible_baseline() {
    let director = ConstructionPauseDirector::with_score_mode(
        ConstructionPauseSolution::new(None),
        ConstructionPauseScoreMode::AssignedSum {
            unassigned_score: -1,
        },
    );
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let placer = ScoredConstructionPlacer::new(vec![-1, -2], true);
    let mut phase = ConstructionHeuristicPhase::new(placer, FirstFeasibleForager::new());

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().entities[0].value, Some(-1));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(SoftScore::of(-1))
    );
    assert_eq!(solver_scope.stats().moves_accepted, 1);
}
