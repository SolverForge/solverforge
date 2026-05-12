use super::*;

use std::any::TypeId;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{ArenaMoveCursor, MoveStreamContext};
use crate::heuristic::selector::{ChangeMoveSelector, MoveSelector};
use crate::phase::localsearch::{AcceptedCountForager, BestScoreForager, HillClimbingAcceptor};
use crate::test_utils::{
    create_minimal_descriptor, create_minimal_director, create_nqueens_director, get_queen_row,
    set_queen_row, NQueensSolution, TestSolution,
};

type NQueensMove = crate::heuristic::r#move::ChangeMove<NQueensSolution, i64>;

fn create_move_selector(
    values: Vec<i64>,
) -> ChangeMoveSelector<
    NQueensSolution,
    i64,
    crate::heuristic::selector::FromSolutionEntitySelector,
    crate::heuristic::selector::StaticValueSelector<NQueensSolution, i64>,
> {
    ChangeMoveSelector::simple(get_queen_row, set_queen_row, 0, 0, "row", values)
}

#[derive(Clone, Debug)]
struct OptionalTask {
    worker: Option<i64>,
}

#[derive(Clone, Debug)]
struct OptionalTaskSolution {
    tasks: Vec<OptionalTask>,
    score: Option<SoftScore>,
}

impl PlanningSolution for OptionalTaskSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[derive(Clone, Debug)]
struct OptionalScoreDirector {
    working_solution: OptionalTaskSolution,
    descriptor: SolutionDescriptor,
}

impl OptionalScoreDirector {
    fn new(solution: OptionalTaskSolution, descriptor: SolutionDescriptor) -> Self {
        Self {
            working_solution: solution,
            descriptor,
        }
    }
}

impl Director<OptionalTaskSolution> for OptionalScoreDirector {
    fn working_solution(&self) -> &OptionalTaskSolution {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut OptionalTaskSolution {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> SoftScore {
        let score = self.working_solution.tasks.iter().fold(0, |acc, task| {
            acc + match task.worker {
                Some(worker) => -worker,
                None => 0,
            }
        });
        let score = SoftScore::of(score);
        self.working_solution.set_score(Some(score));
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> OptionalTaskSolution {
        self.working_solution.clone()
    }

    fn before_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn after_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        (descriptor_index == 0).then_some(self.working_solution.tasks.len())
    }

    fn total_entity_count(&self) -> Option<usize> {
        Some(self.working_solution.tasks.len())
    }

    fn constraint_metadata(&self) -> Vec<solverforge_scoring::ConstraintMetadata<'_>> {
        Vec::new()
    }
}

fn get_optional_tasks(solution: &OptionalTaskSolution) -> &Vec<OptionalTask> {
    &solution.tasks
}

fn get_optional_tasks_mut(solution: &mut OptionalTaskSolution) -> &mut Vec<OptionalTask> {
    &mut solution.tasks
}

fn get_optional_worker(
    solution: &OptionalTaskSolution,
    entity_index: usize,
    _variable_index: usize,
) -> Option<i64> {
    solution.tasks[entity_index].worker
}

fn set_optional_worker(
    solution: &mut OptionalTaskSolution,
    entity_index: usize,
    _variable_index: usize,
    value: Option<i64>,
) {
    solution.tasks[entity_index].worker = value;
}

fn create_optional_director(solution: OptionalTaskSolution) -> OptionalScoreDirector {
    let descriptor =
        SolutionDescriptor::new("OptionalTaskSolution", TypeId::of::<OptionalTaskSolution>())
            .with_entity(
                EntityDescriptor::new("OptionalTask", TypeId::of::<OptionalTask>(), "tasks")
                    .with_extractor(Box::new(EntityCollectionExtractor::new(
                        "OptionalTask",
                        "tasks",
                        get_optional_tasks,
                        get_optional_tasks_mut,
                    ))),
            );

    OptionalScoreDirector::new(solution, descriptor)
}

#[test]
fn test_local_search_hill_climbing() {
    let director = create_nqueens_director(&[0, 0, 0, 0]);
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let initial_score = solver_scope.calculate_score();

    let values: Vec<i64> = (0..4).collect();
    let move_selector = create_move_selector(values);
    let acceptor = HillClimbingAcceptor::new();
    let forager: AcceptedCountForager<_> = AcceptedCountForager::new(1);
    let mut phase: LocalSearchPhase<_, NQueensMove, _, _, _> =
        LocalSearchPhase::new(move_selector, acceptor, forager, Some(100));

    phase.solve(&mut solver_scope);

    let final_score = solver_scope.best_score().copied().unwrap_or(initial_score);
    assert!(final_score >= initial_score);
    assert!(solver_scope.stats().moves_evaluated > 0);
}

#[test]
fn test_local_search_reaches_optimal() {
    let director = create_nqueens_director(&[0, 2, 1, 3]);
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let initial_score = solver_scope.calculate_score();

    let values: Vec<i64> = (0..4).collect();
    let move_selector = create_move_selector(values);
    let acceptor = HillClimbingAcceptor::new();
    let forager: AcceptedCountForager<_> = AcceptedCountForager::new(1);
    let mut phase: LocalSearchPhase<_, NQueensMove, _, _, _> =
        LocalSearchPhase::new(move_selector, acceptor, forager, Some(50));

    phase.solve(&mut solver_scope);

    let final_score = solver_scope.best_score().copied().unwrap_or(initial_score);
    assert!(final_score >= initial_score);
}

#[test]
fn local_search_can_improve_by_unassigning_optional_variable() {
    type OptionalMove = crate::heuristic::r#move::ChangeMove<OptionalTaskSolution, i64>;

    let director = create_optional_director(OptionalTaskSolution {
        tasks: vec![OptionalTask { worker: Some(5) }],
        score: None,
    });
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let initial_score = solver_scope.calculate_score();

    let move_selector = ChangeMoveSelector::simple(
        get_optional_worker,
        set_optional_worker,
        0,
        0,
        "worker",
        vec![5],
    )
    .with_allows_unassigned(true);
    let acceptor = HillClimbingAcceptor::new();
    let forager: AcceptedCountForager<_> = AcceptedCountForager::new(1);
    let mut phase: LocalSearchPhase<_, OptionalMove, _, _, _> =
        LocalSearchPhase::new(move_selector, acceptor, forager, Some(5));

    phase.solve(&mut solver_scope);

    let final_score = solver_scope.best_score().copied().unwrap_or(initial_score);
    assert!(final_score > initial_score);
    assert_eq!(solver_scope.working_solution().tasks[0].worker, None);
}

#[test]
fn test_local_search_step_limit() {
    let director = create_nqueens_director(&[0, 0, 0, 0]);
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let values: Vec<i64> = (0..4).collect();
    let move_selector = create_move_selector(values);
    let acceptor = HillClimbingAcceptor::new();
    let forager: AcceptedCountForager<_> = AcceptedCountForager::new(1);
    let mut phase: LocalSearchPhase<_, NQueensMove, _, _, _> =
        LocalSearchPhase::new(move_selector, acceptor, forager, Some(3));

    phase.solve(&mut solver_scope);

    assert!(solver_scope.stats().step_count <= 3);
}

#[derive(Debug)]
struct NoopMove;

impl Move<TestSolution> for NoopMove {
    type Undo = ();

    fn is_doable<D: Director<TestSolution>>(&self, _score_director: &D) -> bool {
        true
    }

    fn do_move<D: Director<TestSolution>>(&self, _score_director: &mut D) -> Self::Undo {}

    fn undo_move<D: Director<TestSolution>>(&self, _score_director: &mut D, _undo: Self::Undo) {}

    fn descriptor_index(&self) -> usize {
        0
    }

    fn entity_indices(&self) -> &[usize] {
        &[]
    }

    fn variable_name(&self) -> &str {
        "noop"
    }

    fn tabu_signature<D: Director<TestSolution>>(
        &self,
        _score_director: &D,
    ) -> crate::heuristic::r#move::MoveTabuSignature {
        let scope = crate::heuristic::r#move::metadata::MoveTabuScope::new(0, "noop");
        let identity = crate::heuristic::r#move::metadata::hash_str("phase_tests_noop_move");
        crate::heuristic::r#move::MoveTabuSignature::new(
            scope,
            smallvec::smallvec![identity],
            smallvec::smallvec![identity],
        )
    }
}

#[derive(Debug)]
struct SlowOpenSelector;

impl MoveSelector<TestSolution, NoopMove> for SlowOpenSelector {
    type Cursor<'a>
        = ArenaMoveCursor<TestSolution, NoopMove>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<TestSolution>>(
        &'a self,
        _score_director: &D,
    ) -> Self::Cursor<'a> {
        thread::sleep(Duration::from_millis(20));
        ArenaMoveCursor::from_moves(std::iter::once(NoopMove))
    }

    fn size<D: Director<TestSolution>>(&self, _score_director: &D) -> usize {
        1
    }
}

#[derive(Clone, Debug)]
struct ScoreFieldDirector {
    working_solution: TestSolution,
    descriptor: SolutionDescriptor,
}

impl ScoreFieldDirector {
    fn new() -> Self {
        Self {
            working_solution: TestSolution::with_score(SoftScore::of(0)),
            descriptor: create_minimal_descriptor(),
        }
    }
}

impl Director<TestSolution> for ScoreFieldDirector {
    fn working_solution(&self) -> &TestSolution {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut TestSolution {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> SoftScore {
        self.working_solution.score.unwrap_or(SoftScore::ZERO)
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> TestSolution {
        self.working_solution.clone()
    }

    fn before_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn after_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        (descriptor_index == 0).then_some(1)
    }

    fn total_entity_count(&self) -> Option<usize> {
        Some(1)
    }

    fn constraint_metadata(&self) -> Vec<solverforge_scoring::ConstraintMetadata<'_>> {
        Vec::new()
    }
}

#[derive(Clone, Copy, Debug)]
struct ScoreFieldMove(i64);

impl Move<TestSolution> for ScoreFieldMove {
    type Undo = Option<SoftScore>;

    fn is_doable<D: Director<TestSolution>>(&self, _score_director: &D) -> bool {
        true
    }

    fn do_move<D: Director<TestSolution>>(&self, score_director: &mut D) -> Self::Undo {
        let old_score = score_director.working_solution().score;
        score_director.before_variable_changed(0, 0);
        score_director.working_solution_mut().score = Some(SoftScore::of(self.0));
        score_director.after_variable_changed(0, 0);
        old_score
    }

    fn undo_move<D: Director<TestSolution>>(&self, score_director: &mut D, undo: Self::Undo) {
        score_director.before_variable_changed(0, 0);
        score_director.working_solution_mut().score = undo;
        score_director.after_variable_changed(0, 0);
    }

    fn descriptor_index(&self) -> usize {
        0
    }

    fn entity_indices(&self) -> &[usize] {
        &[0]
    }

    fn variable_name(&self) -> &str {
        "score"
    }

    fn tabu_signature<D: Director<TestSolution>>(
        &self,
        _score_director: &D,
    ) -> crate::heuristic::r#move::MoveTabuSignature {
        let scope = crate::heuristic::r#move::metadata::MoveTabuScope::new(0, "score");
        crate::heuristic::r#move::MoveTabuSignature::new(
            scope,
            smallvec::smallvec![self.0 as u64],
            smallvec::smallvec![self.0 as u64],
        )
    }
}

#[derive(Debug)]
struct ScoreFieldSelector {
    scores: Vec<i64>,
}

impl ScoreFieldSelector {
    fn new(scores: impl Into<Vec<i64>>) -> Self {
        Self {
            scores: scores.into(),
        }
    }
}

#[derive(Debug)]
struct ContextSpySelector {
    saw_context: &'static AtomicBool,
    step_index: &'static AtomicU64,
    accepted_limit: &'static AtomicUsize,
}

impl MoveSelector<TestSolution, ScoreFieldMove> for ContextSpySelector {
    type Cursor<'a>
        = ArenaMoveCursor<TestSolution, ScoreFieldMove>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<TestSolution>>(
        &'a self,
        _score_director: &D,
    ) -> Self::Cursor<'a> {
        ArenaMoveCursor::from_moves([ScoreFieldMove(1)])
    }

    fn open_cursor_with_context<'a, D: Director<TestSolution>>(
        &'a self,
        _score_director: &D,
        context: MoveStreamContext,
    ) -> Self::Cursor<'a> {
        self.saw_context.store(true, Ordering::SeqCst);
        self.step_index
            .store(context.step_index(), Ordering::SeqCst);
        self.accepted_limit.store(
            context.accepted_count_limit().unwrap_or(usize::MAX),
            Ordering::SeqCst,
        );
        ArenaMoveCursor::from_moves([ScoreFieldMove(1)])
    }

    fn size<D: Director<TestSolution>>(&self, _score_director: &D) -> usize {
        1
    }
}

impl MoveSelector<TestSolution, ScoreFieldMove> for ScoreFieldSelector {
    type Cursor<'a>
        = ArenaMoveCursor<TestSolution, ScoreFieldMove>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<TestSolution>>(
        &'a self,
        _score_director: &D,
    ) -> Self::Cursor<'a> {
        ArenaMoveCursor::from_moves(self.scores.iter().copied().map(ScoreFieldMove))
    }

    fn size<D: Director<TestSolution>>(&self, _score_director: &D) -> usize {
        self.scores.len()
    }
}

#[derive(Debug)]
struct CancelOnDoableMove {
    score: i64,
    terminate: &'static AtomicBool,
}

impl Move<TestSolution> for CancelOnDoableMove {
    type Undo = Option<SoftScore>;

    fn is_doable<D: Director<TestSolution>>(&self, _score_director: &D) -> bool {
        self.terminate.store(true, Ordering::SeqCst);
        true
    }

    fn do_move<D: Director<TestSolution>>(&self, score_director: &mut D) -> Self::Undo {
        let old_score = score_director.working_solution().score;
        score_director.before_variable_changed(0, 0);
        score_director.working_solution_mut().score = Some(SoftScore::of(self.score));
        score_director.after_variable_changed(0, 0);
        old_score
    }

    fn undo_move<D: Director<TestSolution>>(&self, score_director: &mut D, undo: Self::Undo) {
        score_director.before_variable_changed(0, 0);
        score_director.working_solution_mut().score = undo;
        score_director.after_variable_changed(0, 0);
    }

    fn descriptor_index(&self) -> usize {
        0
    }

    fn entity_indices(&self) -> &[usize] {
        &[0]
    }

    fn variable_name(&self) -> &str {
        "score"
    }

    fn tabu_signature<D: Director<TestSolution>>(
        &self,
        _score_director: &D,
    ) -> crate::heuristic::r#move::MoveTabuSignature {
        let scope = crate::heuristic::r#move::metadata::MoveTabuScope::new(0, "score");
        crate::heuristic::r#move::MoveTabuSignature::new(
            scope,
            smallvec::smallvec![self.score as u64],
            smallvec::smallvec![self.score as u64],
        )
    }
}

#[derive(Debug)]
struct CancelOnDoableSelector {
    terminate: &'static AtomicBool,
}

impl MoveSelector<TestSolution, CancelOnDoableMove> for CancelOnDoableSelector {
    type Cursor<'a>
        = ArenaMoveCursor<TestSolution, CancelOnDoableMove>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<TestSolution>>(
        &'a self,
        _score_director: &D,
    ) -> Self::Cursor<'a> {
        ArenaMoveCursor::from_moves([
            CancelOnDoableMove {
                score: 1,
                terminate: self.terminate,
            },
            CancelOnDoableMove {
                score: 3,
                terminate: self.terminate,
            },
        ])
    }

    fn size<D: Director<TestSolution>>(&self, _score_director: &D) -> usize {
        2
    }
}

#[test]
fn test_local_search_records_selector_open_time_as_generation_time() {
    let director = create_minimal_director();
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let move_selector = SlowOpenSelector;
    let acceptor = HillClimbingAcceptor::new();
    let forager: AcceptedCountForager<_> = AcceptedCountForager::new(1);
    let mut phase: LocalSearchPhase<_, NoopMove, _, _, _> =
        LocalSearchPhase::new(move_selector, acceptor, forager, Some(1));

    phase.solve(&mut solver_scope);

    assert!(solver_scope.stats().generation_time() >= Duration::from_millis(20));
    assert_eq!(solver_scope.stats().moves_generated, 1);
}

#[test]
fn local_search_opens_selector_with_stream_context() {
    let director = ScoreFieldDirector::new();
    let mut solver_scope = SolverScope::new(director).with_seed(7);
    solver_scope.start_solving();

    let saw_context = Box::leak(Box::new(AtomicBool::new(false)));
    let step_index = Box::leak(Box::new(AtomicU64::new(u64::MAX)));
    let accepted_limit = Box::leak(Box::new(AtomicUsize::new(usize::MAX)));
    let move_selector = ContextSpySelector {
        saw_context,
        step_index,
        accepted_limit,
    };
    let acceptor = HillClimbingAcceptor::new();
    let forager: AcceptedCountForager<_> = AcceptedCountForager::new(2);
    let mut phase: LocalSearchPhase<_, ScoreFieldMove, _, _, _> =
        LocalSearchPhase::new(move_selector, acceptor, forager, Some(1));

    phase.solve(&mut solver_scope);

    assert!(saw_context.load(Ordering::SeqCst));
    assert_eq!(step_index.load(Ordering::SeqCst), 0);
    assert_eq!(accepted_limit.load(Ordering::SeqCst), 2);
}

#[test]
fn accepted_count_one_evaluates_one_accepted_move_per_step() {
    let director = ScoreFieldDirector::new();
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let move_selector = ScoreFieldSelector::new([1, 2, 3]);
    let acceptor = HillClimbingAcceptor::new();
    let forager: AcceptedCountForager<_> = AcceptedCountForager::new(1);
    let mut phase: LocalSearchPhase<_, ScoreFieldMove, _, _, _> =
        LocalSearchPhase::new(move_selector, acceptor, forager, Some(1));

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.stats().moves_evaluated, 1);
    assert_eq!(solver_scope.stats().moves_applied, 1);
    assert_eq!(
        solver_scope.working_solution().score,
        Some(SoftScore::of(1))
    );
}

#[test]
fn cancellation_before_next_candidate_does_not_commit_selected_move() {
    let terminate = Box::leak(Box::new(AtomicBool::new(false)));
    let director = ScoreFieldDirector::new();
    let mut solver_scope = SolverScope::new(director).with_terminate(Some(terminate));
    solver_scope.start_solving();

    let move_selector = CancelOnDoableSelector { terminate };
    let acceptor = HillClimbingAcceptor::new();
    let forager: AcceptedCountForager<_> = AcceptedCountForager::new(2);
    let mut phase: LocalSearchPhase<_, CancelOnDoableMove, _, _, _> =
        LocalSearchPhase::new(move_selector, acceptor, forager, Some(1));

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.stats().moves_evaluated, 1);
    assert_eq!(solver_scope.stats().moves_applied, 0);
    assert_eq!(
        solver_scope.working_solution().score,
        Some(SoftScore::of(0))
    );
}

#[test]
fn accepted_count_limit_picks_best_of_accepted_horizon() {
    let director = ScoreFieldDirector::new();
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let move_selector = ScoreFieldSelector::new([1, 3, 2]);
    let acceptor = HillClimbingAcceptor::new();
    let forager: AcceptedCountForager<_> = AcceptedCountForager::new(2);
    let mut phase: LocalSearchPhase<_, ScoreFieldMove, _, _, _> =
        LocalSearchPhase::new(move_selector, acceptor, forager, Some(1));

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.stats().moves_evaluated, 2);
    assert_eq!(solver_scope.stats().moves_applied, 1);
    assert_eq!(
        solver_scope.working_solution().score,
        Some(SoftScore::of(3))
    );
}

#[test]
fn best_score_forager_still_scans_full_neighborhood() {
    let director = ScoreFieldDirector::new();
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let move_selector = ScoreFieldSelector::new([1, 3, 2]);
    let acceptor = HillClimbingAcceptor::new();
    let forager: BestScoreForager<_> = BestScoreForager::new();
    let mut phase: LocalSearchPhase<_, ScoreFieldMove, _, _, _> =
        LocalSearchPhase::new(move_selector, acceptor, forager, Some(1));

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.stats().moves_evaluated, 3);
    assert_eq!(solver_scope.stats().moves_applied, 1);
    assert_eq!(
        solver_scope.working_solution().score,
        Some(SoftScore::of(3))
    );
}
