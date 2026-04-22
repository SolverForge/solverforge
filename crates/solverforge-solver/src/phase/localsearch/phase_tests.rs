use super::*;

use std::any::TypeId;
use std::thread;
use std::time::Duration;

use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::ChangeMoveSelector;
use crate::heuristic::selector::MoveSelector;
use crate::phase::localsearch::{AcceptedCountForager, HillClimbingAcceptor};
use crate::test_utils::{
    create_minimal_director, create_nqueens_director, get_queen_row, set_queen_row,
    NQueensSolution, TestSolution,
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
    ChangeMoveSelector::simple(get_queen_row, set_queen_row, 0, "row", values)
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
}

fn get_optional_tasks(solution: &OptionalTaskSolution) -> &Vec<OptionalTask> {
    &solution.tasks
}

fn get_optional_tasks_mut(solution: &mut OptionalTaskSolution) -> &mut Vec<OptionalTask> {
    &mut solution.tasks
}

fn get_optional_worker(solution: &OptionalTaskSolution, entity_index: usize) -> Option<i64> {
    solution.tasks[entity_index].worker
}

fn set_optional_worker(
    solution: &mut OptionalTaskSolution,
    entity_index: usize,
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
    fn is_doable<D: Director<TestSolution>>(&self, _score_director: &D) -> bool {
        true
    }

    fn do_move<D: Director<TestSolution>>(&self, _score_director: &mut D) {}

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
    fn open_cursor<'a, D: Director<TestSolution>>(
        &'a self,
        _score_director: &D,
    ) -> impl Iterator<Item = NoopMove> + 'a {
        thread::sleep(Duration::from_millis(20));
        std::iter::once(NoopMove)
    }

    fn size<D: Director<TestSolution>>(&self, _score_director: &D) -> usize {
        1
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
