// Tests for parallel local-search evaluation.

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SoftScore;
use solverforge_core::{ConstraintRef, ImpactType};
use solverforge_scoring::api::constraint_set::IncrementalConstraint;
use solverforge_scoring::stream::collection_extract::{source, ChangeSource};
use solverforge_scoring::{ConstraintSet, IncrementalUniConstraint, ScoreDirector};

use super::EvaluatorPool;
use crate::heuristic::r#move::ChangeMove;
use crate::heuristic::selector::ChangeMoveSelector;
use crate::phase::localsearch::{
    solve_local_search_with_resources, AcceptedCountForager, BestScoreForager,
    HillClimbingAcceptor, LocalSearchForager, SelectorCursorSource,
};
use crate::scope::SolverScope;
use crate::test_utils::{
    create_nqueens_descriptor, get_queen_row, nqueens_clone_count, reset_nqueens_clone_count,
    set_queen_row, NQueensSolution, Queen,
};

type QueenMove = ChangeMove<NQueensSolution, i64>;

fn queen_weight(queen: &Queen) -> SoftScore {
    match queen.row {
        Some(row) => SoftScore::of((row * 3 + queen.column * 7) % 11),
        None => SoftScore::of(20),
    }
}

fn queen_constraints(
    _solution: &NQueensSolution,
) -> (impl IncrementalConstraint<NQueensSolution, SoftScore>,) {
    (IncrementalUniConstraint::new(
        ConstraintRef::new("", "Queen placement"),
        ImpactType::Penalty,
        source(
            (|s: &NQueensSolution| s.queens.as_slice()) as fn(&NQueensSolution) -> &[Queen],
            ChangeSource::Descriptor(0),
        ),
        |_s: &NQueensSolution, _queen: &Queen| true,
        queen_weight,
        false,
    ),)
}

fn starting_solution() -> NQueensSolution {
    NQueensSolution::with_rows(&[3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5, 8])
}

fn forkable_director(
    solution: NQueensSolution,
) -> ScoreDirector<NQueensSolution, impl ConstraintSet<NQueensSolution, SoftScore>> {
    let constraints = queen_constraints(&solution);
    ScoreDirector::with_descriptor(
        solution,
        constraints,
        create_nqueens_descriptor(),
        |s, _| s.queens.len(),
    )
    .with_replica_constraints(queen_constraints)
}

type QueenSelector = ChangeMoveSelector<
    NQueensSolution,
    i64,
    crate::heuristic::selector::FromSolutionEntitySelector,
    crate::heuristic::selector::StaticValueSelector<NQueensSolution, i64>,
>;

fn selector() -> SelectorCursorSource<QueenSelector> {
    SelectorCursorSource::new(ChangeMoveSelector::simple(
        get_queen_row,
        set_queen_row,
        0,
        0,
        "row",
        (0..10).collect(),
    ))
}

struct RunResult {
    rows: Vec<Option<i64>>,
    score: Option<SoftScore>,
    moves_evaluated: u64,
    moves_applied: u64,
    moves_speculative: u64,
    steps: u64,
}

fn run<Fo>(
    evaluator_count: usize,
    mut forager: Fo,
    steps: u64,
    move_limit: Option<u64>,
) -> RunResult
where
    Fo: LocalSearchForager<NQueensSolution, QueenMove>,
{
    let mut solver_scope = SolverScope::new(forkable_director(starting_solution())).with_seed(7);
    solver_scope.start_solving();
    solver_scope.inphase_move_count_limit = move_limit;
    let mut source = selector();
    let mut acceptor = HillClimbingAcceptor::new();
    solve_local_search_with_resources(
        &mut source,
        &mut (),
        &mut acceptor,
        &mut forager,
        Some(steps),
        evaluator_count,
        &mut solver_scope,
    );
    let stats = solver_scope.stats();
    RunResult {
        rows: solver_scope
            .working_solution()
            .queens
            .iter()
            .map(|queen| queen.row)
            .collect(),
        score: solver_scope.best_score().copied(),
        moves_evaluated: stats.moves_evaluated,
        moves_applied: stats.moves_applied,
        moves_speculative: stats.moves_speculative,
        steps: stats.step_count,
    }
}

#[test]
fn pool_with_one_evaluator_never_forks() {
    let director =
        ScoreDirector::simple(starting_solution(), create_nqueens_descriptor(), |s, _| {
            s.queens.len()
        });
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();
    let mut pool = EvaluatorPool::<NQueensSolution, _, QueenMove>::new(1);

    pool.sync(&solver_scope);

    assert!(!pool.is_parallel());
    assert_eq!(pool.window_capacity(None), 1);
}

#[test]
#[should_panic(expected = "constraint-set constructor")]
fn pool_requires_a_forkable_director() {
    let director =
        ScoreDirector::simple(starting_solution(), create_nqueens_descriptor(), |s, _| {
            s.queens.len()
        });
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();
    let mut pool = EvaluatorPool::<NQueensSolution, _, QueenMove>::new(2);

    pool.sync(&solver_scope);
}

#[test]
fn pool_forks_one_replica_per_extra_evaluator_and_resyncs_after_writes() {
    let mut solver_scope = SolverScope::new(forkable_director(starting_solution()));
    solver_scope.start_solving();
    solver_scope.calculate_score();
    let mut pool = EvaluatorPool::<NQueensSolution, _, QueenMove>::new(4);

    reset_nqueens_clone_count();
    pool.sync(&solver_scope);
    assert_eq!(nqueens_clone_count(), 3);
    assert_eq!(pool.replicas().count(), 3);
    assert_eq!(pool.window_capacity(None), 32);
    assert_eq!(pool.window_capacity(Some(5)), 5);

    pool.sync(&solver_scope);
    assert_eq!(
        nqueens_clone_count(),
        3,
        "an unchanged generation must not fork again"
    );

    solver_scope.mutate(|director| {
        director.before_variable_changed(0, 0);
        director.working_solution_mut().queens[0].row = Some(7);
        director.after_variable_changed(0, 0);
    });
    let main_score = solver_scope.calculate_score();
    pool.sync(&solver_scope);

    assert_eq!(nqueens_clone_count(), 6);
    for replica in pool.replicas() {
        assert_eq!(replica.working_solution().queens[0].row, Some(7));
        assert_eq!(replica.working_solution().score(), Some(main_score));
    }
}

#[test]
fn sequential_and_parallel_reach_identical_trajectories() {
    let sequential = run(1, AcceptedCountForager::new(4, false), 25, None);
    for evaluator_count in [2, 4] {
        let parallel = run(
            evaluator_count,
            AcceptedCountForager::new(4, false),
            25,
            None,
        );
        assert_eq!(parallel.rows, sequential.rows);
        assert_eq!(parallel.score, sequential.score);
        assert_eq!(parallel.moves_evaluated, sequential.moves_evaluated);
        assert_eq!(parallel.moves_applied, sequential.moves_applied);
        assert_eq!(parallel.steps, sequential.steps);
    }
    assert!(sequential.moves_applied > 0);
}

#[test]
fn speculative_evaluations_are_reported_and_not_counted() {
    let sequential = run(1, AcceptedCountForager::new(1, false), 10, None);
    let parallel = run(2, AcceptedCountForager::new(1, false), 10, None);

    assert_eq!(parallel.rows, sequential.rows);
    assert_eq!(parallel.moves_evaluated, sequential.moves_evaluated);
    assert_eq!(sequential.moves_speculative, 0);
    assert!(parallel.moves_speculative > 0);
}

#[test]
fn in_phase_move_limit_reserves_the_window() {
    let parallel = run(2, BestScoreForager::new(false), 10, Some(5));

    assert_eq!(parallel.moves_evaluated, 5);
    assert_eq!(parallel.moves_speculative, 0);
}

#[test]
fn replicas_follow_solution_replacement() {
    let run_with_replacement = |evaluator_count: usize| {
        let mut solver_scope =
            SolverScope::new(forkable_director(starting_solution())).with_seed(11);
        solver_scope.start_solving();
        let mut source = selector();
        let mut acceptor = HillClimbingAcceptor::new();
        let mut forager = AcceptedCountForager::new(3, false);
        solve_local_search_with_resources(
            &mut source,
            &mut (),
            &mut acceptor,
            &mut forager,
            Some(5),
            evaluator_count,
            &mut solver_scope,
        );
        solver_scope.replace_working_solution_and_reinitialize(NQueensSolution::with_rows(&[
            9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
        ]));
        solve_local_search_with_resources(
            &mut source,
            &mut (),
            &mut acceptor,
            &mut forager,
            Some(5),
            evaluator_count,
            &mut solver_scope,
        );
        solver_scope
            .working_solution()
            .queens
            .iter()
            .map(|queen| queen.row)
            .collect::<Vec<_>>()
    };

    assert_eq!(run_with_replacement(2), run_with_replacement(1));
}
