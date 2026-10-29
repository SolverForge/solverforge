use super::*;

#[test]
fn vnd_rejects_hard_neutral_repair_move_when_hard_improvement_required() {
    let director = HardRepairDirector::new();
    let mut solver_scope = SolverScope::new(director);
    let mut neighborhoods = vec![SelectorCursorSource::new(HardRepairSelector {
        moves: vec![HardRepairMove {
            hard: -1,
            soft: 10,
            require_hard: true,
        }],
    })];
    let mut resources = ();
    solve_vnd_with_resources::<_, _, _, HardRepairMove, _>(
        &mut neighborhoods,
        &mut resources,
        None,
        &mut solver_scope,
    );

    let solution = solver_scope.working_solution();
    assert_eq!(solution.hard, -1);
    assert_eq!(solution.soft, 0);
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(HardSoftScore::of(-1, 0))
    );
    let stats = solver_scope.stats();
    assert_eq!(stats.moves_generated(), 1);
    assert_eq!(stats.moves_evaluated(), 1);
    assert_eq!(stats.moves_acceptor_rejected(), 1);
    assert_eq!(stats.moves_hard_neutral(), 1);
}
