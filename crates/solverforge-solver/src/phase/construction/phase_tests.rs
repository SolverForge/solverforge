use super::*;
use std::any::TypeId;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};

use solverforge_core::domain::SolutionDescriptor;
use solverforge_core::score::SoftScore;
use solverforge_scoring::Director;

use crate::heuristic::selector::EntityReference;
use crate::heuristic::selector::{FromSolutionEntitySelector, StaticValueSelector};
use crate::manager::{
    Solvable, SolverEvent, SolverLifecycleState, SolverManager, SolverRuntime, SolverTerminalReason,
};
use crate::phase::construction::{
    BestFitForager, FirstFeasibleForager, FirstFitForager, Placement, QueuedEntityPlacer,
};
use crate::test_utils::{
    create_simple_nqueens_director, get_queen_row, set_queen_row, NQueensSolution,
};

fn create_placer(
    values: Vec<i64>,
) -> QueuedEntityPlacer<
    NQueensSolution,
    i64,
    FromSolutionEntitySelector,
    StaticValueSelector<NQueensSolution, i64>,
> {
    let es = FromSolutionEntitySelector::new(0);
    let vs = StaticValueSelector::new(values);
    QueuedEntityPlacer::new(es, vs, get_queen_row, set_queen_row, 0, "row")
}

#[derive(Clone, Debug)]
struct BlockingPoint {
    state: Arc<(Mutex<BlockingPointState>, Condvar)>,
}

#[derive(Debug)]
struct BlockingPointState {
    blocked: bool,
    released: bool,
}

impl BlockingPoint {
    fn new() -> Self {
        Self {
            state: Arc::new((
                Mutex::new(BlockingPointState {
                    blocked: false,
                    released: false,
                }),
                Condvar::new(),
            )),
        }
    }

    fn block(&self) {
        let (lock, condvar) = &*self.state;
        let mut state = lock.lock().unwrap();
        state.blocked = true;
        condvar.notify_all();
        while !state.released {
            state = condvar.wait(state).unwrap();
        }
    }

    fn wait_until_blocked(&self) {
        let (lock, condvar) = &*self.state;
        let mut state = lock.lock().unwrap();
        while !state.blocked {
            state = condvar.wait(state).unwrap();
        }
    }

    fn release(&self) {
        let (lock, condvar) = &*self.state;
        let mut state = lock.lock().unwrap();
        state.released = true;
        condvar.notify_all();
    }
}

#[derive(Clone, Debug)]
struct BlockingEvaluationGate {
    block_at: usize,
    seen: Arc<AtomicUsize>,
    blocker: BlockingPoint,
}

impl BlockingEvaluationGate {
    fn new(block_at: usize) -> Self {
        Self {
            block_at,
            seen: Arc::new(AtomicUsize::new(0)),
            blocker: BlockingPoint::new(),
        }
    }

    fn on_evaluation(&self) {
        let seen = self.seen.fetch_add(1, Ordering::SeqCst) + 1;
        if seen == self.block_at {
            self.blocker.block();
        }
    }

    fn wait_until_blocked(&self) {
        self.blocker.wait_until_blocked();
    }

    fn release(&self) {
        self.blocker.release();
    }
}

#[derive(Clone, Debug)]
struct ConstructionPauseEntity {
    value: Option<i64>,
}

#[derive(Clone, Debug)]
struct ConstructionPauseSolution {
    entities: Vec<ConstructionPauseEntity>,
    score: Option<SoftScore>,
    eval_gate: Option<BlockingEvaluationGate>,
    solvable_mode: ConstructionPauseSolvableMode,
}

#[derive(Clone, Copy, Debug)]
enum ConstructionPauseSolvableMode {
    FirstFitMax64,
    BestFitKeepCurrent,
}

impl ConstructionPauseSolution {
    fn new(eval_gate: Option<BlockingEvaluationGate>) -> Self {
        Self::with_entity_count(1, eval_gate)
    }

    fn with_entity_count(entity_count: usize, eval_gate: Option<BlockingEvaluationGate>) -> Self {
        Self {
            entities: vec![ConstructionPauseEntity { value: None }; entity_count],
            score: None,
            eval_gate,
            solvable_mode: ConstructionPauseSolvableMode::FirstFitMax64,
        }
    }

    fn keep_current_pause(eval_gate: Option<BlockingEvaluationGate>) -> Self {
        Self {
            solvable_mode: ConstructionPauseSolvableMode::BestFitKeepCurrent,
            ..Self::new(eval_gate)
        }
    }
}

impl PlanningSolution for ConstructionPauseSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[derive(Clone, Debug)]
struct ConstructionPauseDirector {
    working_solution: ConstructionPauseSolution,
    descriptor: SolutionDescriptor,
    score_mode: ConstructionPauseScoreMode,
}

#[derive(Clone, Copy, Debug)]
enum ConstructionPauseScoreMode {
    AssignedSum {
        unassigned_score: i64,
    },
    CompletionBonus {
        incomplete_score: i64,
        complete_score: i64,
    },
}

impl ConstructionPauseDirector {
    fn new(solution: ConstructionPauseSolution) -> Self {
        Self::with_score_mode(
            solution,
            ConstructionPauseScoreMode::AssignedSum {
                unassigned_score: 0,
            },
        )
    }

    fn with_score_mode(
        solution: ConstructionPauseSolution,
        score_mode: ConstructionPauseScoreMode,
    ) -> Self {
        Self {
            working_solution: solution,
            descriptor: SolutionDescriptor::new(
                "ConstructionPauseSolution",
                TypeId::of::<ConstructionPauseSolution>(),
            ),
            score_mode,
        }
    }
}

impl Director<ConstructionPauseSolution> for ConstructionPauseDirector {
    fn working_solution(&self) -> &ConstructionPauseSolution {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut ConstructionPauseSolution {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> SoftScore {
        let score = match self.score_mode {
            ConstructionPauseScoreMode::AssignedSum { unassigned_score } => SoftScore::of(
                self.working_solution
                    .entities
                    .iter()
                    .map(|entity| entity.value.unwrap_or(unassigned_score))
                    .sum(),
            ),
            ConstructionPauseScoreMode::CompletionBonus {
                incomplete_score,
                complete_score,
            } => {
                let all_assigned = self
                    .working_solution
                    .entities
                    .iter()
                    .all(|entity| entity.value.is_some());
                SoftScore::of(if all_assigned {
                    complete_score
                } else {
                    incomplete_score
                })
            }
        };
        self.working_solution.set_score(Some(score));
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> ConstructionPauseSolution {
        self.working_solution.clone()
    }

    fn before_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn after_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        (descriptor_index == 0).then_some(self.working_solution.entities.len())
    }

    fn total_entity_count(&self) -> Option<usize> {
        Some(self.working_solution.entities.len())
    }
}

#[derive(Clone, Debug)]
struct ConstructionPauseMove {
    entity_index: usize,
    entity_indices: [usize; 1],
    value: i64,
    doable: bool,
    eval_gate: Option<BlockingEvaluationGate>,
}

impl ConstructionPauseMove {
    fn new(
        entity_index: usize,
        value: i64,
        doable: bool,
        eval_gate: Option<BlockingEvaluationGate>,
    ) -> Self {
        Self {
            entity_index,
            entity_indices: [entity_index],
            value,
            doable,
            eval_gate,
        }
    }
}

impl Move<ConstructionPauseSolution> for ConstructionPauseMove {
    fn is_doable<D: Director<ConstructionPauseSolution>>(&self, _score_director: &D) -> bool {
        if let Some(gate) = &self.eval_gate {
            gate.on_evaluation();
        }
        self.doable
    }

    fn do_move<D: Director<ConstructionPauseSolution>>(&self, score_director: &mut D) {
        let old_value = score_director.working_solution().entities[self.entity_index].value;
        score_director.working_solution_mut().entities[self.entity_index].value = Some(self.value);

        let entity_index = self.entity_index;
        score_director.register_undo(Box::new(move |solution: &mut ConstructionPauseSolution| {
            solution.entities[entity_index].value = old_value;
        }));
    }

    fn descriptor_index(&self) -> usize {
        0
    }

    fn entity_indices(&self) -> &[usize] {
        &self.entity_indices
    }

    fn variable_name(&self) -> &str {
        "value"
    }
}

#[derive(Clone, Debug)]
struct ConstructionPausePlacer {
    eval_gate: Option<BlockingEvaluationGate>,
}

impl ConstructionPausePlacer {
    fn new(eval_gate: Option<BlockingEvaluationGate>) -> Self {
        Self { eval_gate }
    }
}

impl EntityPlacer<ConstructionPauseSolution, ConstructionPauseMove> for ConstructionPausePlacer {
    fn get_placements<D: Director<ConstructionPauseSolution>>(
        &self,
        score_director: &D,
    ) -> Vec<Placement<ConstructionPauseSolution, ConstructionPauseMove>> {
        score_director
            .working_solution()
            .entities
            .iter()
            .enumerate()
            .filter_map(|(entity_index, entity)| {
                if entity.value.is_some() {
                    return None;
                }

                let moves = (0..65)
                    .map(|value| {
                        ConstructionPauseMove::new(
                            entity_index,
                            value as i64,
                            value == 64,
                            (value == 0).then(|| self.eval_gate.clone()).flatten(),
                        )
                    })
                    .collect();

                Some(Placement::new(EntityReference::new(0, entity_index), moves))
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
struct ScoredConstructionPlacer {
    values: Vec<i64>,
    keep_current_legal: bool,
    eval_gate: Option<BlockingEvaluationGate>,
}

impl ScoredConstructionPlacer {
    fn new(values: Vec<i64>, keep_current_legal: bool) -> Self {
        Self {
            values,
            keep_current_legal,
            eval_gate: None,
        }
    }

    fn with_eval_gate(mut self, eval_gate: Option<BlockingEvaluationGate>) -> Self {
        self.eval_gate = eval_gate;
        self
    }
}

impl EntityPlacer<ConstructionPauseSolution, ConstructionPauseMove> for ScoredConstructionPlacer {
    fn get_placements<D: Director<ConstructionPauseSolution>>(
        &self,
        score_director: &D,
    ) -> Vec<Placement<ConstructionPauseSolution, ConstructionPauseMove>> {
        score_director
            .working_solution()
            .entities
            .iter()
            .enumerate()
            .filter_map(|(entity_index, entity)| {
                if entity.value.is_some() {
                    return None;
                }

                let moves = self
                    .values
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(idx, value)| {
                        ConstructionPauseMove::new(
                            entity_index,
                            value,
                            true,
                            (idx == 0).then(|| self.eval_gate.clone()).flatten(),
                        )
                    })
                    .collect();

                Some(
                    Placement::new(EntityReference::new(0, entity_index), moves)
                        .with_keep_current_legal(self.keep_current_legal),
                )
            })
            .collect()
    }
}

impl Solvable for ConstructionPauseSolution {
    fn solve(self, runtime: SolverRuntime<Self>) {
        let eval_gate = self.eval_gate.clone();
        let solvable_mode = self.solvable_mode;
        let mut solver_scope = SolverScope::new_with_callback(
            ConstructionPauseDirector::new(self),
            (),
            None,
            Some(runtime),
        );

        solver_scope.start_solving();

        match solvable_mode {
            ConstructionPauseSolvableMode::FirstFitMax64 => {
                let mut phase = ConstructionHeuristicPhase::new(
                    ConstructionPausePlacer::new(eval_gate),
                    FirstFitForager::new(),
                );
                phase.solve(&mut solver_scope);
            }
            ConstructionPauseSolvableMode::BestFitKeepCurrent => {
                let mut phase = ConstructionHeuristicPhase::new(
                    ScoredConstructionPlacer::new(vec![-5], true).with_eval_gate(eval_gate),
                    BestFitForager::new(),
                );
                phase.solve(&mut solver_scope);
            }
        }

        let mut current_score = solver_scope.current_score().copied();
        let best_score = if let Some(best_score) = solver_scope.best_score().copied() {
            best_score
        } else {
            let score = solver_scope.calculate_score();
            current_score.get_or_insert(score);
            score
        };

        let telemetry = solver_scope.stats().snapshot();
        let solution = solver_scope.score_director().clone_working_solution();

        if runtime.is_cancel_requested() {
            runtime.emit_cancelled(current_score, Some(best_score), telemetry);
        } else {
            runtime.emit_completed(
                solution,
                current_score,
                best_score,
                telemetry,
                SolverTerminalReason::Completed,
            );
        }
    }
}

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
fn first_fit_optional_construction_selects_first_doable_move() {
    let director = ConstructionPauseDirector::new(ConstructionPauseSolution::new(None));
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let placer = ScoredConstructionPlacer::new(vec![3, 4], true);
    let mut phase = ConstructionHeuristicPhase::new(placer, FirstFitForager::new());

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().entities[0].value, Some(3));
    assert_eq!(solver_scope.stats().moves_accepted, 1);
    assert_eq!(solver_scope.stats().step_count, 1);
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

#[test]
fn test_construction_phase_reports_one_best_solution_on_improvement() {
    let director = create_simple_nqueens_director(4);
    let best_events = Arc::new(AtomicUsize::new(0));
    let best_events_for_callback = Arc::clone(&best_events);
    let mut solver_scope = SolverScope::new_with_callback(
        director,
        move |progress: crate::scope::SolverProgressRef<'_, NQueensSolution>| {
            if progress.kind == crate::scope::SolverProgressKind::BestSolution {
                best_events_for_callback.fetch_add(1, Ordering::SeqCst);
            }
        },
        None,
        None,
    );
    solver_scope.start_solving();

    let values: Vec<i64> = (0..4).collect();
    let placer = create_placer(values);
    let forager = FirstFitForager::new();
    let mut phase = ConstructionHeuristicPhase::new(placer, forager);

    phase.solve(&mut solver_scope);

    assert_eq!(best_events.load(Ordering::SeqCst), 1);
}

#[test]
fn test_construction_empty_solution() {
    let director = create_simple_nqueens_director(0);
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let values: Vec<i64> = vec![];
    let placer = create_placer(values);
    let forager = FirstFitForager::new();
    let mut phase = ConstructionHeuristicPhase::new(placer, forager);

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.stats().moves_evaluated, 0);
}

#[test]
fn keep_current_pause_snapshot_has_committed_score() {
    static MANAGER: SolverManager<ConstructionPauseSolution> = SolverManager::new();

    let gate = BlockingEvaluationGate::new(1);
    let (job_id, mut receiver) = MANAGER
        .solve(ConstructionPauseSolution::keep_current_pause(Some(
            gate.clone(),
        )))
        .expect("paused keep-current job should start");

    gate.wait_until_blocked();
    MANAGER.pause(job_id).expect("pause should be accepted");

    match receiver.blocking_recv().expect("pause requested event") {
        SolverEvent::PauseRequested { metadata } => {
            assert_eq!(
                metadata.lifecycle_state,
                SolverLifecycleState::PauseRequested
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }

    gate.release();

    let paused_snapshot_revision = match receiver.blocking_recv().expect("paused event") {
        SolverEvent::Paused { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Paused);
            metadata
                .snapshot_revision
                .expect("paused snapshot revision")
        }
        other => panic!("unexpected event: {other:?}"),
    };

    let paused_snapshot = MANAGER
        .get_snapshot(job_id, Some(paused_snapshot_revision))
        .expect("paused snapshot");
    assert_eq!(paused_snapshot.current_score, Some(SoftScore::of(0)));
    assert_eq!(paused_snapshot.best_score, None);
    assert_eq!(paused_snapshot.solution.entities[0].value, None);
    assert_eq!(paused_snapshot.solution.score(), Some(SoftScore::of(0)));

    MANAGER.resume(job_id).expect("resume should be accepted");

    match receiver.blocking_recv().expect("resumed event") {
        SolverEvent::Resumed { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Solving);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match receiver.blocking_recv().expect("completed event") {
        SolverEvent::Completed { metadata, solution } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Completed);
            assert_eq!(solution.entities[0].value, None);
            assert_eq!(solution.score(), Some(SoftScore::of(0)));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.delete(job_id).expect("delete paused job");
}

#[test]
fn test_construction_resume_retries_interrupted_placement() {
    static MANAGER: SolverManager<ConstructionPauseSolution> = SolverManager::new();

    let (uninterrupted_job_id, mut uninterrupted_receiver) = MANAGER
        .solve(ConstructionPauseSolution::new(None))
        .expect("uninterrupted job should start");

    let uninterrupted_value = match uninterrupted_receiver
        .blocking_recv()
        .expect("uninterrupted completed event")
    {
        SolverEvent::Completed { metadata, solution } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Completed);
            assert_eq!(solution.entities[0].value, Some(64));
            assert_eq!(solution.score(), Some(SoftScore::of(64)));
            solution.entities[0].value
        }
        other => panic!("unexpected event: {other:?}"),
    };

    MANAGER
        .delete(uninterrupted_job_id)
        .expect("delete uninterrupted job");

    let gate = BlockingEvaluationGate::new(1);
    let (job_id, mut receiver) = MANAGER
        .solve(ConstructionPauseSolution::new(Some(gate.clone())))
        .expect("paused job should start");

    gate.wait_until_blocked();
    MANAGER.pause(job_id).expect("pause should be accepted");

    match receiver.blocking_recv().expect("pause requested event") {
        SolverEvent::PauseRequested { metadata } => {
            assert_eq!(
                metadata.lifecycle_state,
                SolverLifecycleState::PauseRequested
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }

    gate.release();

    let paused_snapshot_revision = match receiver.blocking_recv().expect("paused event") {
        SolverEvent::Paused { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Paused);
            metadata
                .snapshot_revision
                .expect("paused snapshot revision")
        }
        other => panic!("unexpected event: {other:?}"),
    };

    let paused_snapshot = MANAGER
        .get_snapshot(job_id, Some(paused_snapshot_revision))
        .expect("paused snapshot");
    assert_eq!(paused_snapshot.solution.entities[0].value, None);

    MANAGER.resume(job_id).expect("resume should be accepted");

    match receiver.blocking_recv().expect("resumed event") {
        SolverEvent::Resumed { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Solving);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match receiver.blocking_recv().expect("completed event") {
        SolverEvent::Completed { metadata, solution } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Completed);
            assert_eq!(solution.entities[0].value, uninterrupted_value);
            assert_eq!(solution.score(), Some(SoftScore::of(64)));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.delete(job_id).expect("delete resumed job");
}
