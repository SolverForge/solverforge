// Integration tests for SolverFactory with termination and solving.

use super::*;
use std::any::TypeId;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use solverforge_core::domain::SolutionDescriptor;
use solverforge_core::score::SoftScore;
use solverforge_core::PlanningSolution;
use solverforge_scoring::{Director, ScoreDirector};

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::MoveSelector;
use crate::phase::localsearch::{BestScoreForager, HillClimbingAcceptor, LocalSearchPhase};
use crate::phase::Phase;
use crate::scope::SolverScope;

/* ============================================================================
Type Aliases for Score Directors
============================================================================
*/

// Score director type for TestSolution
type TestDirector = ScoreDirector<TestSolution, ()>;

// Score director type for EntityTestSolution
type EntityTestDirector = ScoreDirector<EntityTestSolution, ()>;

/* ============================================================================
Test Solution Types
============================================================================
*/

#[derive(Clone, Debug)]
struct TestSolution {
    value: i64,
    score: Option<SoftScore>,
}

impl PlanningSolution for TestSolution {
    type Score = SoftScore;
    fn score(&self) -> Option<Self::Score> {
        self.score
    }
    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[derive(Clone, Debug)]
struct TestEntity {
    #[allow(dead_code)]
    id: i64,
    value: Option<i64>,
}

#[derive(Clone, Debug)]
struct EntityTestSolution {
    entities: Vec<TestEntity>,
    target_sum: i64,
    score: Option<SoftScore>,
}

impl PlanningSolution for EntityTestSolution {
    type Score = SoftScore;
    fn score(&self) -> Option<Self::Score> {
        self.score
    }
    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn calculate_entity_score(solution: &EntityTestSolution) -> SoftScore {
    let sum: i64 = solution.entities.iter().filter_map(|e| e.value).sum();
    let diff = (sum - solution.target_sum).abs();
    SoftScore::of(-diff)
}

/* ============================================================================
Test with Termination Conditions
============================================================================
*/

// A simple test phase that just sets best solution
#[derive(Debug, Clone)]
struct NoOpPhase;

impl<
        S: PlanningSolution,
        D: solverforge_scoring::Director<S>,
        ProgressCb: crate::scope::ProgressCallback<S>,
    > crate::phase::Phase<S, D, ProgressCb> for NoOpPhase
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D, ProgressCb>) {
        solver_scope.update_best_solution();
    }

    fn phase_type_name(&self) -> &'static str {
        "NoOpPhase"
    }
}

#[test]
fn test_solver_with_time_limit_termination() {
    let solution = EntityTestSolution {
        entities: vec![
            TestEntity {
                id: 0,
                value: Some(1),
            },
            TestEntity {
                id: 1,
                value: Some(2),
            },
            TestEntity {
                id: 2,
                value: Some(2),
            },
        ],
        target_sum: 5,
        score: None,
    };

    let factory =
        solver_factory_builder::<EntityTestSolution, EntityTestDirector, _>(calculate_entity_score)
            .with_time_limit(Duration::from_millis(100))
            .build()
            .expect("Failed to build factory");

    // Verify the factory can calculate scores
    let score = factory.calculate_score(&solution);
    assert_eq!(score, SoftScore::of(0)); // 1 + 2 + 2 = 5, target is 5, diff = 0
}

#[test]
fn test_solver_with_step_limit_termination() {
    let solution = EntityTestSolution {
        entities: vec![
            TestEntity {
                id: 0,
                value: Some(0),
            },
            TestEntity {
                id: 1,
                value: Some(0),
            },
        ],
        target_sum: 6,
        score: None,
    };

    let factory =
        solver_factory_builder::<EntityTestSolution, EntityTestDirector, _>(calculate_entity_score)
            .with_step_limit(5)
            .build()
            .expect("Failed to build factory");

    // Verify the factory can calculate scores
    let score = factory.calculate_score(&solution);
    assert_eq!(score, SoftScore::of(-6)); // sum = 0, target = 6, diff = 6
}

#[test]
fn test_solver_factory_with_entity_solution() {
    let solution = EntityTestSolution {
        entities: vec![
            TestEntity {
                id: 0,
                value: Some(2),
            },
            TestEntity {
                id: 1,
                value: Some(3),
            },
        ],
        target_sum: 5,
        score: None,
    };

    let factory =
        solver_factory_builder::<EntityTestSolution, EntityTestDirector, _>(calculate_entity_score)
            .build()
            .expect("Failed to build factory");

    let score = factory.calculate_score(&solution);
    assert_eq!(score, SoftScore::of(0));
}

#[test]
fn test_solver_factory_with_phases() {
    let factory = solver_factory_builder::<TestSolution, TestDirector, _>(|s: &TestSolution| {
        SoftScore::of(-s.value)
    })
    .with_phase(NoOpPhase)
    .with_step_limit(10)
    .build()
    .expect("Failed to build factory");

    let solution = TestSolution {
        value: 5,
        score: None,
    };
    let score = factory.calculate_score(&solution);
    assert_eq!(score, SoftScore::of(-5));
}

#[test]
fn test_solver_factory_with_multiple_phases() {
    let factory = solver_factory_builder::<TestSolution, TestDirector, _>(|s: &TestSolution| {
        SoftScore::of(-s.value)
    })
    .with_phase(NoOpPhase)
    .with_phase(NoOpPhase)
    .with_time_limit(Duration::from_secs(1))
    .build()
    .expect("Failed to build factory");

    let solution = TestSolution {
        value: 7,
        score: None,
    };
    let score = factory.calculate_score(&solution);
    assert_eq!(score, SoftScore::of(-7));
}

#[test]
fn test_construction_and_local_search_types_exist() {
    // Just verify the enum variants exist
    assert_eq!(ConstructionType::default(), ConstructionType::FirstFit);
    assert_eq!(LocalSearchType::default(), LocalSearchType::HillClimbing);

    let _tabu = LocalSearchType::TabuSearch { tabu_size: 10 };
    let _sa = LocalSearchType::SimulatedAnnealing {
        starting_temp: 1.0,
        decay_rate: 0.99,
    };
    let _la = LocalSearchType::LateAcceptance { size: 100 };
    let _bf = ConstructionType::BestFit;
}

/* ============================================================================
4. Retained Job Lifecycle Tests
============================================================================
*/

#[derive(Clone, Debug)]
struct LifecycleStepGate {
    permit: Arc<(Mutex<bool>, Condvar)>,
}

impl LifecycleStepGate {
    fn new_closed() -> Self {
        Self {
            permit: Arc::new((Mutex::new(false), Condvar::new())),
        }
    }

    fn allow_next_step(&self) {
        let (lock, condvar) = &*self.permit;
        let mut open = lock.lock().unwrap();
        *open = true;
        condvar.notify_all();
    }

    fn wait_for_permit(&self) {
        let (lock, condvar) = &*self.permit;
        let mut open = lock.lock().unwrap();
        while !*open {
            open = condvar.wait(open).unwrap();
        }
        *open = false;
    }
}

#[derive(Clone, Debug)]
struct LifecycleSolution {
    gate: LifecycleStepGate,
    value: i64,
    score: Option<SoftScore>,
}

impl LifecycleSolution {
    fn new(value: i64) -> Self {
        Self {
            gate: LifecycleStepGate::new_closed(),
            value,
            score: None,
        }
    }
}

impl PlanningSolution for LifecycleSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

impl Analyzable for LifecycleSolution {
    fn analyze(&self) -> ScoreAnalysis<Self::Score> {
        let score = SoftScore::of(self.value);
        ScoreAnalysis {
            score,
            constraints: vec![ConstraintAnalysis {
                name: "value".to_string(),
                weight: SoftScore::of(1),
                score,
                match_count: 1,
            }],
        }
    }
}

#[derive(Clone, Debug)]
struct LifecycleDirector {
    working_solution: LifecycleSolution,
    descriptor: SolutionDescriptor,
}

impl LifecycleDirector {
    fn new(solution: LifecycleSolution) -> Self {
        Self {
            working_solution: solution,
            descriptor: SolutionDescriptor::new(
                "LifecycleSolution",
                TypeId::of::<LifecycleSolution>(),
            ),
        }
    }
}

impl solverforge_scoring::Director<LifecycleSolution> for LifecycleDirector {
    fn working_solution(&self) -> &LifecycleSolution {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut LifecycleSolution {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> SoftScore {
        let score = SoftScore::of(self.working_solution.value);
        self.working_solution.set_score(Some(score));
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> LifecycleSolution {
        self.working_solution.clone()
    }

    fn before_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn after_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn entity_count(&self, _descriptor_index: usize) -> Option<usize> {
        Some(0)
    }

    fn total_entity_count(&self) -> Option<usize> {
        Some(0)
    }
}

fn zero_telemetry() -> crate::SolverTelemetry {
    crate::SolverTelemetry::default()
}

fn telemetry_with_steps(step_count: u64) -> crate::SolverTelemetry {
    crate::SolverTelemetry {
        step_count,
        ..crate::SolverTelemetry::default()
    }
}

impl Solvable for LifecycleSolution {
    fn solve(self, runtime: SolverRuntime<Self>) {
        let mut solver_scope =
            SolverScope::new_with_callback(LifecycleDirector::new(self), (), None, Some(runtime));

        solver_scope.start_solving();
        let score = solver_scope.calculate_score();
        let solution = solver_scope.score_director().clone_working_solution();
        solver_scope.set_best_solution(solution.clone(), score);
        runtime.emit_best_solution(
            solution.clone(),
            Some(score),
            score,
            solver_scope.stats().snapshot(),
        );

        for step_index in 0..2 {
            solver_scope.increment_step_count();
            solver_scope.stats_mut().record_move(true);
            runtime.emit_progress(
                solver_scope.current_score().copied(),
                solver_scope.best_score().copied(),
                solver_scope.stats().snapshot(),
            );

            if step_index == 0 {
                solution.gate.wait_for_permit();
                solver_scope.pause_if_requested();
                if runtime.is_cancel_requested() {
                    break;
                }
            }
        }

        let telemetry = solver_scope.stats().snapshot();
        let current_score = solver_scope.current_score().copied();
        let best_score = solver_scope.best_score().copied().unwrap_or(score);

        if runtime.is_cancel_requested() {
            runtime.emit_cancelled(current_score, Some(best_score), telemetry);
        } else {
            runtime.emit_completed(
                solver_scope.score_director().clone_working_solution(),
                current_score,
                best_score,
                telemetry,
                SolverTerminalReason::Completed,
            );
        }
    }
}

#[derive(Clone, Debug)]
struct PauseRequestedProgressSolution {
    gate: LifecycleStepGate,
    value: i64,
    score: Option<SoftScore>,
}

impl PauseRequestedProgressSolution {
    fn new(value: i64) -> Self {
        Self {
            gate: LifecycleStepGate::new_closed(),
            value,
            score: None,
        }
    }
}

impl PlanningSolution for PauseRequestedProgressSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

impl Solvable for PauseRequestedProgressSolution {
    fn solve(mut self, runtime: SolverRuntime<Self>) {
        let score = SoftScore::of(self.value);
        self.set_score(Some(score));
        runtime.emit_best_solution(self.clone(), Some(score), score, zero_telemetry());

        self.gate.wait_for_permit();
        runtime.emit_progress(Some(score), Some(score), zero_telemetry());

        if runtime.pause_with_snapshot(self.clone(), Some(score), Some(score), zero_telemetry()) {
            runtime.emit_completed(
                self,
                Some(score),
                score,
                zero_telemetry(),
                SolverTerminalReason::Completed,
            );
        } else {
            runtime.emit_cancelled(Some(score), Some(score), zero_telemetry());
        }
    }
}

#[derive(Clone, Debug)]
struct DeleteReservationSolution {
    release_return: LifecycleStepGate,
    score: Option<SoftScore>,
}

impl DeleteReservationSolution {
    fn new() -> Self {
        Self {
            release_return: LifecycleStepGate::new_closed(),
            score: None,
        }
    }
}

impl PlanningSolution for DeleteReservationSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

impl Solvable for DeleteReservationSolution {
    fn solve(mut self, runtime: SolverRuntime<Self>) {
        let score = SoftScore::of(1);
        self.set_score(Some(score));
        runtime.emit_completed(
            self.clone(),
            Some(score),
            score,
            zero_telemetry(),
            SolverTerminalReason::Completed,
        );
        self.release_return.wait_for_permit();
    }
}

#[derive(Clone, Debug)]
struct TrivialLifecycleSolution {
    gate: LifecycleStepGate,
    score: Option<SoftScore>,
}

impl TrivialLifecycleSolution {
    fn new() -> Self {
        Self {
            gate: LifecycleStepGate::new_closed(),
            score: None,
        }
    }
}

impl PlanningSolution for TrivialLifecycleSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn trivial_solution_descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new(
        "TrivialLifecycleSolution",
        TypeId::of::<TrivialLifecycleSolution>(),
    )
}

fn trivial_entity_count(_solution: &TrivialLifecycleSolution, _descriptor_index: usize) -> usize {
    0
}

fn trivial_log_scale(solution: &TrivialLifecycleSolution) {
    solution.gate.wait_for_permit();
}

fn empty_noop_phase_sequence(
    _config: &solverforge_config::SolverConfig,
) -> crate::phase::PhaseSequence<NoOpPhase> {
    crate::phase::PhaseSequence::new(Vec::new())
}

impl Solvable for TrivialLifecycleSolution {
    fn solve(self, runtime: SolverRuntime<Self>) {
        let _ = crate::run::run_solver(
            self,
            || (),
            trivial_solution_descriptor,
            trivial_entity_count,
            runtime,
            30,
            |_| true,
            trivial_log_scale,
            empty_noop_phase_sequence,
        );
    }
}

#[derive(Clone, Debug)]
struct DeterministicResumeSolution {
    gate: LifecycleStepGate,
    value: i64,
    score: Option<SoftScore>,
}

impl DeterministicResumeSolution {
    fn new() -> Self {
        Self {
            gate: LifecycleStepGate::new_closed(),
            value: 0,
            score: None,
        }
    }
}

impl PlanningSolution for DeterministicResumeSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

impl Solvable for DeterministicResumeSolution {
    fn solve(mut self, runtime: SolverRuntime<Self>) {
        self.value = 10;
        let initial_score = SoftScore::of(self.value);
        self.set_score(Some(initial_score));
        runtime.emit_best_solution(
            self.clone(),
            Some(initial_score),
            initial_score,
            telemetry_with_steps(0),
        );
        runtime.emit_progress(
            Some(initial_score),
            Some(initial_score),
            telemetry_with_steps(1),
        );

        self.gate.wait_for_permit();

        self.value = 12;
        let boundary_score = SoftScore::of(self.value);
        self.set_score(Some(boundary_score));
        if !runtime.pause_with_snapshot(
            self.clone(),
            Some(boundary_score),
            Some(boundary_score),
            telemetry_with_steps(2),
        ) {
            if runtime.is_cancel_requested() {
                runtime.emit_cancelled(
                    Some(boundary_score),
                    Some(boundary_score),
                    telemetry_with_steps(2),
                );
                return;
            }

            runtime.emit_best_solution(
                self.clone(),
                Some(boundary_score),
                boundary_score,
                telemetry_with_steps(2),
            );
        }

        runtime.emit_progress(
            Some(boundary_score),
            Some(boundary_score),
            telemetry_with_steps(3),
        );

        self.value = 15;
        let final_score = SoftScore::of(self.value);
        self.set_score(Some(final_score));
        runtime.emit_completed(
            self,
            Some(final_score),
            final_score,
            telemetry_with_steps(4),
            SolverTerminalReason::Completed,
        );
    }
}

#[derive(Clone, Debug)]
struct FailureAfterSnapshotSolution {
    value: i64,
    score: Option<SoftScore>,
}

impl FailureAfterSnapshotSolution {
    fn new(value: i64) -> Self {
        Self { value, score: None }
    }
}

impl PlanningSolution for FailureAfterSnapshotSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

impl Analyzable for FailureAfterSnapshotSolution {
    fn analyze(&self) -> ScoreAnalysis<Self::Score> {
        let score = SoftScore::of(self.value);
        ScoreAnalysis {
            score,
            constraints: vec![ConstraintAnalysis {
                name: "value".to_string(),
                weight: SoftScore::of(1),
                score,
                match_count: 1,
            }],
        }
    }
}

impl Solvable for FailureAfterSnapshotSolution {
    fn solve(mut self, runtime: SolverRuntime<Self>) {
        let score = SoftScore::of(self.value);
        self.set_score(Some(score));
        runtime.emit_best_solution(self, Some(score), score, zero_telemetry());
        panic!("expected retained lifecycle failure");
    }
}

#[derive(Clone, Debug)]
struct ConfigTerminatedSolution {
    value: i64,
    score: Option<SoftScore>,
}

impl ConfigTerminatedSolution {
    fn new(value: i64) -> Self {
        Self { value, score: None }
    }
}

impl PlanningSolution for ConfigTerminatedSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

impl Analyzable for ConfigTerminatedSolution {
    fn analyze(&self) -> ScoreAnalysis<Self::Score> {
        let score = SoftScore::of(self.value);
        ScoreAnalysis {
            score,
            constraints: vec![ConstraintAnalysis {
                name: "value".to_string(),
                weight: SoftScore::of(1),
                score,
                match_count: 1,
            }],
        }
    }
}

impl Solvable for ConfigTerminatedSolution {
    fn solve(mut self, runtime: SolverRuntime<Self>) {
        let score = SoftScore::of(self.value);
        self.set_score(Some(score));
        runtime.emit_best_solution(self.clone(), Some(score), score, zero_telemetry());
        runtime.emit_completed(
            self,
            Some(score),
            score,
            zero_telemetry(),
            SolverTerminalReason::TerminatedByConfig,
        );
    }
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
struct PromptControlSolution {
    value: i64,
    score: Option<SoftScore>,
    selector: PromptControlSelector,
    time_limit: Option<Duration>,
}

impl PromptControlSolution {
    fn generation_blocked(
        total_moves: usize,
        block_at: usize,
        blocker: BlockingPoint,
        time_limit: Option<Duration>,
    ) -> Self {
        Self {
            value: 0,
            score: None,
            selector: PromptControlSelector::Generation(BlockingGenerationSelector {
                total_moves,
                block_at,
                blocker,
            }),
            time_limit,
        }
    }

    fn evaluation_blocked(total_moves: usize, gate: BlockingEvaluationGate) -> Self {
        Self {
            value: 0,
            score: None,
            selector: PromptControlSelector::Evaluation(BlockingEvaluationSelector {
                total_moves,
                gate,
            }),
            time_limit: None,
        }
    }
}

impl PlanningSolution for PromptControlSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[derive(Clone, Debug)]
struct PromptControlDirector {
    working_solution: PromptControlSolution,
    descriptor: SolutionDescriptor,
}

impl PromptControlDirector {
    fn new(solution: PromptControlSolution) -> Self {
        Self {
            working_solution: solution,
            descriptor: SolutionDescriptor::new(
                "PromptControlSolution",
                TypeId::of::<PromptControlSolution>(),
            ),
        }
    }
}

impl Director<PromptControlSolution> for PromptControlDirector {
    fn working_solution(&self) -> &PromptControlSolution {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut PromptControlSolution {
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> SoftScore {
        let score = SoftScore::of(self.working_solution.value);
        self.working_solution.set_score(Some(score));
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> PromptControlSolution {
        self.working_solution.clone()
    }

    fn before_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn after_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn entity_count(&self, _descriptor_index: usize) -> Option<usize> {
        Some(0)
    }

    fn total_entity_count(&self) -> Option<usize> {
        Some(0)
    }
}

#[derive(Clone, Debug)]
struct NoOpMove {
    eval_gate: Option<BlockingEvaluationGate>,
}

impl NoOpMove {
    fn new(eval_gate: Option<BlockingEvaluationGate>) -> Self {
        Self { eval_gate }
    }
}

impl Move<PromptControlSolution> for NoOpMove {
    fn is_doable<D: Director<PromptControlSolution>>(&self, _score_director: &D) -> bool {
        true
    }

    fn do_move<D: Director<PromptControlSolution>>(&self, _score_director: &mut D) {
        if let Some(gate) = &self.eval_gate {
            gate.on_evaluation();
        }
    }

    fn descriptor_index(&self) -> usize {
        0
    }

    fn entity_indices(&self) -> &[usize] {
        &[]
    }

    fn variable_name(&self) -> &str {
        "noop"
    }
}

#[derive(Clone, Debug)]
struct BlockingGenerationSelector {
    total_moves: usize,
    block_at: usize,
    blocker: BlockingPoint,
}

#[derive(Clone, Debug)]
struct BlockingEvaluationSelector {
    total_moves: usize,
    gate: BlockingEvaluationGate,
}

#[derive(Clone, Debug)]
enum PromptControlSelector {
    Generation(BlockingGenerationSelector),
    Evaluation(BlockingEvaluationSelector),
}

impl MoveSelector<PromptControlSolution, NoOpMove> for BlockingGenerationSelector {
    fn iter_moves<'a, D: Director<PromptControlSolution>>(
        &'a self,
        _score_director: &'a D,
    ) -> impl Iterator<Item = NoOpMove> + 'a {
        (0..self.total_moves).map(move |index| {
            if index == self.block_at {
                self.blocker.block();
            }
            NoOpMove::new(None)
        })
    }

    fn size<D: Director<PromptControlSolution>>(&self, _score_director: &D) -> usize {
        self.total_moves
    }
}

impl MoveSelector<PromptControlSolution, NoOpMove> for BlockingEvaluationSelector {
    fn iter_moves<'a, D: Director<PromptControlSolution>>(
        &'a self,
        _score_director: &'a D,
    ) -> impl Iterator<Item = NoOpMove> + 'a {
        (0..self.total_moves).map(move |_| NoOpMove::new(Some(self.gate.clone())))
    }

    fn size<D: Director<PromptControlSolution>>(&self, _score_director: &D) -> usize {
        self.total_moves
    }
}

impl MoveSelector<PromptControlSolution, NoOpMove> for PromptControlSelector {
    fn iter_moves<'a, D: Director<PromptControlSolution>>(
        &'a self,
        score_director: &'a D,
    ) -> impl Iterator<Item = NoOpMove> + 'a {
        enum PromptControlIter<A, B> {
            Generation(A),
            Evaluation(B),
        }

        impl<T, A, B> Iterator for PromptControlIter<A, B>
        where
            A: Iterator<Item = T>,
            B: Iterator<Item = T>,
        {
            type Item = T;

            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    Self::Generation(iter) => iter.next(),
                    Self::Evaluation(iter) => iter.next(),
                }
            }
        }

        match self {
            Self::Generation(selector) => {
                PromptControlIter::Generation(selector.iter_moves(score_director))
            }
            Self::Evaluation(selector) => {
                PromptControlIter::Evaluation(selector.iter_moves(score_director))
            }
        }
    }

    fn size<D: Director<PromptControlSolution>>(&self, score_director: &D) -> usize {
        match self {
            Self::Generation(selector) => selector.size(score_director),
            Self::Evaluation(selector) => selector.size(score_director),
        }
    }
}

impl Solvable for PromptControlSolution {
    fn solve(self, runtime: SolverRuntime<Self>) {
        let mut solver_scope =
            SolverScope::new_with_callback(PromptControlDirector::new(self), (), None, Some(runtime));

        solver_scope.start_solving();
        if let Some(time_limit) = solver_scope.working_solution().time_limit {
            solver_scope.set_time_limit(time_limit);
        }
        let score = solver_scope.calculate_score();
        let solution = solver_scope.score_director().clone_working_solution();
        solver_scope.set_best_solution(solution.clone(), score);
        runtime.emit_best_solution(solution, Some(score), score, solver_scope.stats().snapshot());

        let selector = solver_scope.working_solution().selector.clone();
        let mut phase = LocalSearchPhase::new(
            selector,
            HillClimbingAcceptor::new(),
            BestScoreForager::new(),
            Some(1),
        );
        phase.solve(&mut solver_scope);

        let terminal_reason = solver_scope.terminal_reason();
        let telemetry = solver_scope.stats().snapshot();
        let current_score = solver_scope.current_score().copied();
        let best_score = solver_scope.best_score().copied().unwrap_or(score);

        match terminal_reason {
            SolverTerminalReason::Cancelled => {
                runtime.emit_cancelled(current_score, Some(best_score), telemetry);
            }
            reason => runtime.emit_completed(
                solver_scope.score_director().clone_working_solution(),
                current_score,
                best_score,
                telemetry,
                reason,
            ),
        }
    }
}

#[test]
fn retained_job_pause_settles_promptly_during_generation() {
    static MANAGER: SolverManager<PromptControlSolution> = SolverManager::new();

    let blocker = BlockingPoint::new();
    let solution = PromptControlSolution::generation_blocked(8_000, 512, blocker.clone(), None);
    let (job_id, mut receiver) = MANAGER.solve(solution).expect("job should start");

    match receiver.blocking_recv().expect("best solution event") {
        SolverEvent::BestSolution { .. } => {}
        other => panic!("unexpected event: {other:?}"),
    }

    blocker.wait_until_blocked();
    MANAGER.pause(job_id).expect("pause should be accepted");

    match receiver.blocking_recv().expect("pause requested event") {
        SolverEvent::PauseRequested { .. } => {}
        other => panic!("unexpected event: {other:?}"),
    }

    let resumed_at = Instant::now();
    blocker.release();

    match receiver.blocking_recv().expect("paused event") {
        SolverEvent::Paused { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Paused);
        }
        other => panic!("unexpected event: {other:?}"),
    }
    assert!(
        resumed_at.elapsed() < Duration::from_secs(1),
        "pause settlement after generation block took too long: {:?}",
        resumed_at.elapsed()
    );

    MANAGER.cancel(job_id).expect("cancel should be accepted");
    match receiver.blocking_recv().expect("cancelled event") {
        SolverEvent::Cancelled { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Cancelled);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.delete(job_id).expect("delete cancelled job");
}

#[test]
fn retained_job_cancel_settles_promptly_during_evaluation() {
    static MANAGER: SolverManager<PromptControlSolution> = SolverManager::new();

    let gate = BlockingEvaluationGate::new(96);
    let solution = PromptControlSolution::evaluation_blocked(8_000, gate.clone());
    let (job_id, mut receiver) = MANAGER.solve(solution).expect("job should start");

    match receiver.blocking_recv().expect("best solution event") {
        SolverEvent::BestSolution { .. } => {}
        other => panic!("unexpected event: {other:?}"),
    }

    gate.wait_until_blocked();
    MANAGER.cancel(job_id).expect("cancel should be accepted");

    let released_at = Instant::now();
    gate.release();

    match receiver.blocking_recv().expect("cancelled event") {
        SolverEvent::Cancelled { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Cancelled);
        }
        other => panic!("unexpected event: {other:?}"),
    }
    assert!(
        released_at.elapsed() < Duration::from_secs(1),
        "cancel settlement after evaluation block took too long: {:?}",
        released_at.elapsed()
    );

    MANAGER.delete(job_id).expect("delete cancelled job");
}

#[test]
fn retained_job_time_limit_settles_promptly_during_generation() {
    static MANAGER: SolverManager<PromptControlSolution> = SolverManager::new();

    let blocker = BlockingPoint::new();
    let solution = PromptControlSolution::generation_blocked(
        8_000,
        512,
        blocker.clone(),
        Some(Duration::from_millis(20)),
    );
    let (job_id, mut receiver) = MANAGER.solve(solution).expect("job should start");

    match receiver.blocking_recv().expect("best solution event") {
        SolverEvent::BestSolution { .. } => {}
        other => panic!("unexpected event: {other:?}"),
    }

    blocker.wait_until_blocked();
    std::thread::sleep(Duration::from_millis(40));

    let released_at = Instant::now();
    blocker.release();

    match receiver.blocking_recv().expect("completed event") {
        SolverEvent::Completed { metadata, .. } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Completed);
            assert_eq!(
                metadata.terminal_reason,
                Some(SolverTerminalReason::TerminatedByConfig)
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }
    assert!(
        released_at.elapsed() < Duration::from_secs(1),
        "config termination after generation block took too long: {:?}",
        released_at.elapsed()
    );

    MANAGER.delete(job_id).expect("delete completed job");
}

#[test]
fn retained_job_pause_resume_completion_flow() {
    static MANAGER: SolverManager<LifecycleSolution> = SolverManager::new();

    let solution = LifecycleSolution::new(7);
    let gate = solution.gate.clone();
    let (job_id, mut receiver) = MANAGER.solve(solution).expect("job should start");

    match receiver.blocking_recv().expect("best solution event") {
        SolverEvent::BestSolution { metadata, .. } => {
            assert_eq!(metadata.event_sequence, 1);
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Solving);
            assert_eq!(metadata.snapshot_revision, Some(1));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match receiver.blocking_recv().expect("progress event") {
        SolverEvent::Progress { metadata } => {
            assert_eq!(metadata.event_sequence, 2);
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Solving);
            assert_eq!(metadata.snapshot_revision, Some(1));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.pause(job_id).expect("pause should be accepted");

    match receiver.blocking_recv().expect("pause requested event") {
        SolverEvent::PauseRequested { metadata } => {
            assert_eq!(metadata.event_sequence, 3);
            assert_eq!(
                metadata.lifecycle_state,
                SolverLifecycleState::PauseRequested
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }

    gate.allow_next_step();

    match receiver.blocking_recv().expect("paused event") {
        SolverEvent::Paused { metadata } => {
            assert_eq!(metadata.event_sequence, 4);
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Paused);
            assert_eq!(metadata.snapshot_revision, Some(2));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let status = MANAGER.get_status(job_id).expect("status while paused");
    assert_eq!(status.lifecycle_state, SolverLifecycleState::Paused);
    assert!(status.checkpoint_available);
    assert_eq!(status.event_sequence, 4);
    assert_eq!(status.latest_snapshot_revision, Some(2));

    let paused_snapshot = MANAGER.get_snapshot(job_id, None).expect("paused snapshot");
    assert_eq!(
        paused_snapshot.lifecycle_state,
        SolverLifecycleState::Paused
    );
    assert_eq!(paused_snapshot.snapshot_revision, 2);

    let analysis = MANAGER
        .analyze_snapshot(job_id, Some(paused_snapshot.snapshot_revision))
        .expect("analysis for paused snapshot");
    assert_eq!(
        analysis.snapshot_revision,
        paused_snapshot.snapshot_revision
    );
    assert_eq!(analysis.lifecycle_state, SolverLifecycleState::Paused);
    assert_eq!(analysis.analysis.score, SoftScore::of(7));

    MANAGER.resume(job_id).expect("resume should be accepted");

    match receiver.blocking_recv().expect("resumed event") {
        SolverEvent::Resumed { metadata } => {
            assert_eq!(metadata.event_sequence, 5);
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Solving);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match receiver.blocking_recv().expect("progress after resume") {
        SolverEvent::Progress { metadata } => {
            assert_eq!(metadata.event_sequence, 6);
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Solving);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match receiver.blocking_recv().expect("completed event") {
        SolverEvent::Completed { metadata, .. } => {
            assert_eq!(metadata.event_sequence, 7);
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Completed);
            assert_eq!(
                metadata.terminal_reason,
                Some(SolverTerminalReason::Completed)
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let status = MANAGER.get_status(job_id).expect("status after completion");
    assert_eq!(status.lifecycle_state, SolverLifecycleState::Completed);
    assert_eq!(
        status.terminal_reason,
        Some(SolverTerminalReason::Completed)
    );
    assert_eq!(status.event_sequence, 7);
    assert!(!status.checkpoint_available);

    MANAGER.delete(job_id).expect("delete terminal job");
    assert!(matches!(
        MANAGER.get_status(job_id),
        Err(SolverManagerError::JobNotFound { .. })
    ));
}

#[test]
fn retained_job_invalid_transitions_cancel_and_delete() {
    static MANAGER: SolverManager<LifecycleSolution> = SolverManager::new();

    let solution = LifecycleSolution::new(3);
    let gate = solution.gate.clone();
    let (job_id, mut receiver) = MANAGER.solve(solution).expect("job should start");

    assert!(matches!(
        MANAGER.resume(job_id),
        Err(SolverManagerError::InvalidStateTransition { action, .. }) if action == "resume"
    ));

    assert!(matches!(
        MANAGER.delete(job_id),
        Err(SolverManagerError::InvalidStateTransition { action, .. }) if action == "delete"
    ));

    match receiver.blocking_recv().expect("best solution event") {
        SolverEvent::BestSolution { .. } => {}
        other => panic!("unexpected event: {other:?}"),
    }
    match receiver.blocking_recv().expect("progress event") {
        SolverEvent::Progress { .. } => {}
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.cancel(job_id).expect("cancel should be accepted");

    gate.allow_next_step();

    match receiver.blocking_recv().expect("cancelled event") {
        SolverEvent::Cancelled { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Cancelled);
            assert_eq!(
                metadata.terminal_reason,
                Some(SolverTerminalReason::Cancelled)
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let status = MANAGER.get_status(job_id).expect("status after cancel");
    assert_eq!(status.lifecycle_state, SolverLifecycleState::Cancelled);
    assert_eq!(
        status.terminal_reason,
        Some(SolverTerminalReason::Cancelled)
    );

    MANAGER.delete(job_id).expect("delete cancelled job");
    assert!(matches!(
        MANAGER.get_status(job_id),
        Err(SolverManagerError::JobNotFound { .. })
    ));
}

#[test]
fn retained_job_progress_reflects_pause_requested_state() {
    static MANAGER: SolverManager<PauseRequestedProgressSolution> = SolverManager::new();

    let solution = PauseRequestedProgressSolution::new(11);
    let gate = solution.gate.clone();
    let (job_id, mut receiver) = MANAGER.solve(solution).expect("job should start");

    match receiver.blocking_recv().expect("best solution event") {
        SolverEvent::BestSolution { metadata, .. } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Solving);
        }
        other => panic!("unexpected event: {other:?}"),
    }

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

    gate.allow_next_step();

    match receiver.blocking_recv().expect("progress event") {
        SolverEvent::Progress { metadata } => {
            assert_eq!(
                metadata.lifecycle_state,
                SolverLifecycleState::PauseRequested
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match receiver.blocking_recv().expect("paused event") {
        SolverEvent::Paused { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Paused);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.cancel(job_id).expect("cancel should be accepted");

    match receiver.blocking_recv().expect("cancelled event") {
        SolverEvent::Cancelled { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Cancelled);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.delete(job_id).expect("delete cancelled job");
}

#[test]
fn retained_job_delete_keeps_slot_reserved_until_worker_exit() {
    static MANAGER: SolverManager<DeleteReservationSolution> = SolverManager::new();

    let solution = DeleteReservationSolution::new();
    let release_return = solution.release_return.clone();
    let (job_id, mut receiver) = MANAGER.solve(solution).expect("job should start");

    match receiver.blocking_recv().expect("completed event") {
        SolverEvent::Completed { metadata, .. } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Completed);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.delete(job_id).expect("delete completed job");
    assert!(matches!(
        MANAGER.get_status(job_id),
        Err(SolverManagerError::JobNotFound { .. })
    ));
    assert_eq!(MANAGER.active_job_count(), 0);
    assert!(!MANAGER.slot_is_free_for_test(job_id));

    release_return.allow_next_step();

    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    while std::time::Instant::now() < deadline {
        if MANAGER.slot_is_free_for_test(job_id) {
            return;
        }
        std::thread::yield_now();
    }

    panic!("slot {job_id} was not released after the worker exited");
}

#[test]
fn trivial_job_cancelled_while_paused_reports_cancelled() {
    static MANAGER: SolverManager<TrivialLifecycleSolution> = SolverManager::new();

    let solution = TrivialLifecycleSolution::new();
    let gate = solution.gate.clone();
    let (job_id, mut receiver) = MANAGER.solve(solution).expect("job should start");

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

    gate.allow_next_step();

    match receiver.blocking_recv().expect("paused event") {
        SolverEvent::Paused { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Paused);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.cancel(job_id).expect("cancel should be accepted");

    match receiver.blocking_recv().expect("cancelled event") {
        SolverEvent::Cancelled { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Cancelled);
            assert_eq!(
                metadata.terminal_reason,
                Some(SolverTerminalReason::Cancelled)
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.delete(job_id).expect("delete cancelled job");
}

#[test]
fn retained_job_exact_resume_matches_uninterrupted_execution_after_boundary() {
    static MANAGER: SolverManager<DeterministicResumeSolution> = SolverManager::new();

    let uninterrupted = DeterministicResumeSolution::new();
    let uninterrupted_gate = uninterrupted.gate.clone();
    let (uninterrupted_job_id, mut uninterrupted_receiver) = MANAGER
        .solve(uninterrupted)
        .expect("uninterrupted job should start");

    match uninterrupted_receiver
        .blocking_recv()
        .expect("uninterrupted best solution event")
    {
        SolverEvent::BestSolution { metadata, solution } => {
            assert_eq!(metadata.snapshot_revision, Some(1));
            assert_eq!(solution.value, 10);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match uninterrupted_receiver
        .blocking_recv()
        .expect("uninterrupted progress event")
    {
        SolverEvent::Progress { metadata } => {
            assert_eq!(metadata.snapshot_revision, Some(1));
            assert_eq!(metadata.current_score, Some(SoftScore::of(10)));
            assert_eq!(metadata.best_score, Some(SoftScore::of(10)));
            assert_eq!(metadata.telemetry.step_count, 1);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    uninterrupted_gate.allow_next_step();

    let uninterrupted_boundary_snapshot = match uninterrupted_receiver
        .blocking_recv()
        .expect("uninterrupted boundary snapshot event")
    {
        SolverEvent::BestSolution { metadata, solution } => {
            assert_eq!(metadata.snapshot_revision, Some(2));
            assert_eq!(metadata.current_score, Some(SoftScore::of(12)));
            assert_eq!(metadata.best_score, Some(SoftScore::of(12)));
            assert_eq!(metadata.telemetry.step_count, 2);
            assert_eq!(solution.value, 12);
            solution
        }
        other => panic!("unexpected event: {other:?}"),
    };

    let uninterrupted_post_boundary = match uninterrupted_receiver
        .blocking_recv()
        .expect("uninterrupted post-boundary progress")
    {
        SolverEvent::Progress { metadata } => (
            metadata.snapshot_revision,
            metadata.current_score,
            metadata.best_score,
            metadata.telemetry.step_count,
        ),
        other => panic!("unexpected event: {other:?}"),
    };

    let uninterrupted_completed = match uninterrupted_receiver
        .blocking_recv()
        .expect("uninterrupted completed event")
    {
        SolverEvent::Completed { metadata, solution } => (
            metadata.snapshot_revision,
            metadata.current_score,
            metadata.best_score,
            metadata.terminal_reason,
            metadata.telemetry.step_count,
            solution.value,
        ),
        other => panic!("unexpected event: {other:?}"),
    };

    let resumed = DeterministicResumeSolution::new();
    let resumed_gate = resumed.gate.clone();
    let (resumed_job_id, mut resumed_receiver) =
        MANAGER.solve(resumed).expect("resumed job should start");

    match resumed_receiver
        .blocking_recv()
        .expect("resumed best solution event")
    {
        SolverEvent::BestSolution { metadata, solution } => {
            assert_eq!(metadata.snapshot_revision, Some(1));
            assert_eq!(solution.value, 10);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match resumed_receiver
        .blocking_recv()
        .expect("resumed progress event")
    {
        SolverEvent::Progress { metadata } => {
            assert_eq!(metadata.snapshot_revision, Some(1));
            assert_eq!(metadata.current_score, Some(SoftScore::of(10)));
            assert_eq!(metadata.best_score, Some(SoftScore::of(10)));
            assert_eq!(metadata.telemetry.step_count, 1);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER
        .pause(resumed_job_id)
        .expect("pause should be accepted");

    match resumed_receiver
        .blocking_recv()
        .expect("pause requested event")
    {
        SolverEvent::PauseRequested { metadata } => {
            assert_eq!(
                metadata.lifecycle_state,
                SolverLifecycleState::PauseRequested
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }

    resumed_gate.allow_next_step();

    match resumed_receiver
        .blocking_recv()
        .expect("paused boundary snapshot event")
    {
        SolverEvent::Paused { metadata } => {
            assert_eq!(metadata.snapshot_revision, Some(2));
            assert_eq!(metadata.current_score, Some(SoftScore::of(12)));
            assert_eq!(metadata.best_score, Some(SoftScore::of(12)));
            assert_eq!(metadata.telemetry.step_count, 2);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let resumed_boundary_snapshot = MANAGER
        .get_snapshot(resumed_job_id, Some(2))
        .expect("paused boundary snapshot");
    assert_eq!(resumed_boundary_snapshot.snapshot_revision, 2);
    assert_eq!(
        resumed_boundary_snapshot.current_score,
        Some(SoftScore::of(12))
    );
    assert_eq!(
        resumed_boundary_snapshot.best_score,
        Some(SoftScore::of(12))
    );
    assert_eq!(resumed_boundary_snapshot.telemetry.step_count, 2);
    assert_eq!(resumed_boundary_snapshot.solution.value, 12);
    assert_eq!(
        resumed_boundary_snapshot.solution.score(),
        Some(SoftScore::of(12))
    );

    assert_eq!(
        uninterrupted_boundary_snapshot.value,
        resumed_boundary_snapshot.solution.value
    );
    assert_eq!(
        uninterrupted_boundary_snapshot.score(),
        resumed_boundary_snapshot.solution.score()
    );

    MANAGER
        .resume(resumed_job_id)
        .expect("resume should be accepted");

    match resumed_receiver.blocking_recv().expect("resumed event") {
        SolverEvent::Resumed { metadata } => {
            assert_eq!(metadata.snapshot_revision, Some(2));
            assert_eq!(metadata.best_score, Some(SoftScore::of(12)));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let resumed_post_boundary = match resumed_receiver
        .blocking_recv()
        .expect("resumed post-boundary progress")
    {
        SolverEvent::Progress { metadata } => (
            metadata.snapshot_revision,
            metadata.current_score,
            metadata.best_score,
            metadata.telemetry.step_count,
        ),
        other => panic!("unexpected event: {other:?}"),
    };

    let resumed_completed = match resumed_receiver
        .blocking_recv()
        .expect("resumed completed event")
    {
        SolverEvent::Completed { metadata, solution } => (
            metadata.snapshot_revision,
            metadata.current_score,
            metadata.best_score,
            metadata.terminal_reason,
            metadata.telemetry.step_count,
            solution.value,
        ),
        other => panic!("unexpected event: {other:?}"),
    };

    assert_eq!(resumed_post_boundary, uninterrupted_post_boundary);
    assert_eq!(resumed_completed, uninterrupted_completed);

    let uninterrupted_final_snapshot = MANAGER
        .get_snapshot(uninterrupted_job_id, None)
        .expect("uninterrupted final snapshot");
    let resumed_final_snapshot = MANAGER
        .get_snapshot(resumed_job_id, None)
        .expect("resumed final snapshot");

    assert_eq!(
        uninterrupted_final_snapshot.snapshot_revision,
        resumed_final_snapshot.snapshot_revision
    );
    assert_eq!(
        uninterrupted_final_snapshot.current_score,
        resumed_final_snapshot.current_score
    );
    assert_eq!(
        uninterrupted_final_snapshot.best_score,
        resumed_final_snapshot.best_score
    );
    assert_eq!(
        uninterrupted_final_snapshot.solution.value,
        resumed_final_snapshot.solution.value
    );

    MANAGER
        .delete(uninterrupted_job_id)
        .expect("delete uninterrupted job");
    MANAGER.delete(resumed_job_id).expect("delete resumed job");
}

#[test]
fn retained_job_analysis_is_snapshot_bound_across_live_states_and_completion() {
    static MANAGER: SolverManager<LifecycleSolution> = SolverManager::new();

    let solution = LifecycleSolution::new(13);
    let gate = solution.gate.clone();
    let (job_id, mut receiver) = MANAGER.solve(solution).expect("job should start");

    match receiver.blocking_recv().expect("best solution event") {
        SolverEvent::BestSolution { metadata, .. } => {
            assert_eq!(metadata.snapshot_revision, Some(1));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let solving_analysis = MANAGER
        .analyze_snapshot(job_id, None)
        .expect("analysis while solving");
    assert_eq!(
        solving_analysis.lifecycle_state,
        SolverLifecycleState::Solving
    );
    assert_eq!(solving_analysis.snapshot_revision, 1);
    assert_eq!(solving_analysis.analysis.score, SoftScore::of(13));

    match receiver.blocking_recv().expect("progress event") {
        SolverEvent::Progress { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Solving);
        }
        other => panic!("unexpected event: {other:?}"),
    }

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

    let pause_requested_status = MANAGER
        .get_status(job_id)
        .expect("status while pause is requested");
    assert_eq!(
        pause_requested_status.lifecycle_state,
        SolverLifecycleState::PauseRequested
    );

    let pause_requested_analysis = MANAGER
        .analyze_snapshot(job_id, None)
        .expect("analysis while pause is requested");
    assert_eq!(
        pause_requested_analysis.lifecycle_state,
        SolverLifecycleState::Solving
    );
    assert_eq!(pause_requested_analysis.snapshot_revision, 1);
    assert_eq!(pause_requested_analysis.analysis.score, SoftScore::of(13));
    assert!(!pause_requested_analysis.lifecycle_state.is_terminal());

    gate.allow_next_step();

    match receiver.blocking_recv().expect("paused event") {
        SolverEvent::Paused { metadata } => {
            assert_eq!(metadata.snapshot_revision, Some(2));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let paused_analysis = MANAGER
        .analyze_snapshot(job_id, None)
        .expect("analysis while paused");
    assert_eq!(
        paused_analysis.lifecycle_state,
        SolverLifecycleState::Paused
    );
    assert_eq!(paused_analysis.snapshot_revision, 2);
    assert_eq!(paused_analysis.analysis.score, SoftScore::of(13));

    MANAGER.resume(job_id).expect("resume should be accepted");

    match receiver.blocking_recv().expect("resumed event") {
        SolverEvent::Resumed { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Solving);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match receiver
        .blocking_recv()
        .expect("post-resume progress event")
    {
        SolverEvent::Progress { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Solving);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match receiver.blocking_recv().expect("completed event") {
        SolverEvent::Completed { metadata, .. } => {
            assert_eq!(metadata.snapshot_revision, Some(3));
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Completed);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let completed_analysis = MANAGER
        .analyze_snapshot(job_id, None)
        .expect("analysis after completion");
    assert_eq!(
        completed_analysis.lifecycle_state,
        SolverLifecycleState::Completed
    );
    assert_eq!(
        completed_analysis.terminal_reason,
        Some(SolverTerminalReason::Completed)
    );
    assert_eq!(completed_analysis.snapshot_revision, 3);
    assert_eq!(completed_analysis.analysis.score, SoftScore::of(13));

    MANAGER.delete(job_id).expect("delete completed job");
}

#[test]
fn retained_job_analysis_remains_available_after_cancel_failure_and_config_termination() {
    static CANCEL_MANAGER: SolverManager<LifecycleSolution> = SolverManager::new();
    static FAILURE_MANAGER: SolverManager<FailureAfterSnapshotSolution> = SolverManager::new();
    static TERMINATED_MANAGER: SolverManager<ConfigTerminatedSolution> = SolverManager::new();

    let cancelled = LifecycleSolution::new(5);
    let cancel_gate = cancelled.gate.clone();
    let (cancelled_job_id, mut cancelled_receiver) = CANCEL_MANAGER
        .solve(cancelled)
        .expect("cancelled job should start");

    match cancelled_receiver
        .blocking_recv()
        .expect("cancelled job best solution event")
    {
        SolverEvent::BestSolution { .. } => {}
        other => panic!("unexpected event: {other:?}"),
    }
    match cancelled_receiver
        .blocking_recv()
        .expect("cancelled job progress event")
    {
        SolverEvent::Progress { .. } => {}
        other => panic!("unexpected event: {other:?}"),
    }

    CANCEL_MANAGER
        .pause(cancelled_job_id)
        .expect("pause should be accepted");
    match cancelled_receiver
        .blocking_recv()
        .expect("cancelled job pause requested event")
    {
        SolverEvent::PauseRequested { .. } => {}
        other => panic!("unexpected event: {other:?}"),
    }

    cancel_gate.allow_next_step();

    match cancelled_receiver
        .blocking_recv()
        .expect("cancelled job paused event")
    {
        SolverEvent::Paused { metadata } => {
            assert_eq!(metadata.snapshot_revision, Some(2));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    CANCEL_MANAGER
        .cancel(cancelled_job_id)
        .expect("cancel should be accepted");
    match cancelled_receiver
        .blocking_recv()
        .expect("cancelled job cancelled event")
    {
        SolverEvent::Cancelled { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Cancelled);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let cancelled_analysis = CANCEL_MANAGER
        .analyze_snapshot(cancelled_job_id, None)
        .expect("analysis after cancellation");
    assert_eq!(
        cancelled_analysis.lifecycle_state,
        SolverLifecycleState::Paused
    );
    assert_eq!(cancelled_analysis.snapshot_revision, 2);
    assert_eq!(cancelled_analysis.analysis.score, SoftScore::of(5));

    let (failed_job_id, mut failed_receiver) = FAILURE_MANAGER
        .solve(FailureAfterSnapshotSolution::new(17))
        .expect("failed job should start");

    match failed_receiver
        .blocking_recv()
        .expect("failed job best solution event")
    {
        SolverEvent::BestSolution { metadata, .. } => {
            assert_eq!(metadata.snapshot_revision, Some(1));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match failed_receiver
        .blocking_recv()
        .expect("failed job failed event")
    {
        SolverEvent::Failed { metadata, error } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Failed);
            assert!(error.contains("expected retained lifecycle failure"));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let failed_analysis = FAILURE_MANAGER
        .analyze_snapshot(failed_job_id, None)
        .expect("analysis after failure");
    assert_eq!(
        failed_analysis.lifecycle_state,
        SolverLifecycleState::Solving
    );
    assert_eq!(failed_analysis.snapshot_revision, 1);
    assert_eq!(failed_analysis.analysis.score, SoftScore::of(17));

    let (terminated_job_id, mut terminated_receiver) = TERMINATED_MANAGER
        .solve(ConfigTerminatedSolution::new(23))
        .expect("configured-termination job should start");

    match terminated_receiver
        .blocking_recv()
        .expect("configured-termination best solution event")
    {
        SolverEvent::BestSolution { metadata, .. } => {
            assert_eq!(metadata.snapshot_revision, Some(1));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match terminated_receiver
        .blocking_recv()
        .expect("configured-termination completed event")
    {
        SolverEvent::Completed { metadata, .. } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Completed);
            assert_eq!(metadata.snapshot_revision, Some(2));
            assert_eq!(
                metadata.terminal_reason,
                Some(SolverTerminalReason::TerminatedByConfig)
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let terminated_analysis = TERMINATED_MANAGER
        .analyze_snapshot(terminated_job_id, None)
        .expect("analysis after configured termination");
    assert_eq!(
        terminated_analysis.lifecycle_state,
        SolverLifecycleState::Completed
    );
    assert_eq!(
        terminated_analysis.terminal_reason,
        Some(SolverTerminalReason::TerminatedByConfig)
    );
    assert_eq!(terminated_analysis.snapshot_revision, 2);
    assert_eq!(terminated_analysis.analysis.score, SoftScore::of(23));

    CANCEL_MANAGER
        .delete(cancelled_job_id)
        .expect("delete cancelled job");
    FAILURE_MANAGER
        .delete(failed_job_id)
        .expect("delete failed job");
    TERMINATED_MANAGER
        .delete(terminated_job_id)
        .expect("delete configured-termination job");
}
