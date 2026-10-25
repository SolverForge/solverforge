//! Shared fixture for parallel local-search evaluation tests.

use solverforge_core::score::SoftScore;
use solverforge_core::{ConstraintRef, ImpactType};
use solverforge_scoring::api::constraint_set::IncrementalConstraint;
use solverforge_scoring::stream::collection_extract::{source, ChangeSource};
use solverforge_scoring::IncrementalUniConstraint;

use crate::heuristic::r#move::ChangeMove;
use crate::heuristic::selector::ChangeMoveSelector;
use crate::phase::localsearch::{
    solve_local_search_with_resources, HillClimbingAcceptor, LocalSearchForager,
    SelectorCursorSource,
};
use crate::scope::SolverScope;
use crate::test_utils::{
    create_nqueens_descriptor, get_queen_row, set_queen_row, NQueensSolution, Queen,
};

pub(super) type QueenMove = ChangeMove<NQueensSolution, i64>;

pub(super) fn queen_weight(queen: &Queen) -> SoftScore {
    match queen.row {
        Some(row) => SoftScore::of((row * 3 + queen.column * 7) % 11),
        None => SoftScore::of(20),
    }
}

pub(super) fn queen_constraints(
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

pub(super) fn starting_solution() -> NQueensSolution {
    NQueensSolution::with_rows(&[3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5, 8])
}

/* Builds a director that can fork replicas. A macro, because the director's
type names the opaque constraint set and the borrowed descriptor.
*/
macro_rules! forkable_director {
    ($solution:expr, $descriptor:expr) => {{
        let solution: crate::test_utils::NQueensSolution = $solution;
        let constraints = super::support::queen_constraints(&solution);
        solverforge_scoring::ScoreDirector::with_descriptor(
            solution,
            constraints,
            solverforge_scoring::ReplicaDescriptor::new(
                $descriptor,
                super::support::queen_constraints,
            ),
            |s: &NQueensSolution, _| s.queens.len(),
        )
    }};
}
pub(super) use forkable_director;

type QueenSelector = ChangeMoveSelector<
    NQueensSolution,
    i64,
    crate::heuristic::selector::FromSolutionEntitySelector,
    crate::heuristic::selector::StaticValueSelector<NQueensSolution, i64>,
>;

pub(super) fn selector() -> SelectorCursorSource<QueenSelector> {
    SelectorCursorSource::new(ChangeMoveSelector::simple(
        get_queen_row,
        set_queen_row,
        0,
        0,
        "row",
        (0..10).collect(),
    ))
}

pub(super) struct RunResult {
    pub(super) rows: Vec<Option<i64>>,
    pub(super) score: Option<SoftScore>,
    pub(super) moves_generated: u64,
    pub(super) moves_evaluated: u64,
    pub(super) moves_applied: u64,
    pub(super) moves_speculative: u64,
    pub(super) steps: u64,
}

pub(super) fn run<Fo>(
    evaluator_count: usize,
    mut forager: Fo,
    steps: u64,
    move_limit: Option<u64>,
) -> RunResult
where
    Fo: LocalSearchForager<NQueensSolution, QueenMove>,
{
    let descriptor = create_nqueens_descriptor();
    let mut solver_scope =
        SolverScope::new(forkable_director!(starting_solution(), &descriptor)).with_seed(7);
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
        moves_generated: stats.moves_generated,
        moves_evaluated: stats.moves_evaluated,
        moves_applied: stats.moves_applied,
        moves_speculative: stats.moves_speculative,
        steps: stats.step_count,
    }
}
