use std::any::TypeId;

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::SoftScore;
use solverforge_scoring::Director;

use super::super::{
    Solvable, SolverEvent, SolverLifecycleState, SolverManager, SolverRuntime, SolverTerminalReason,
};
use super::common::recv_event;
use super::gates::BlockingPoint;
use crate::phase::partitioned::{FunctionalPartitioner, PartitionedSearchPhase};
use crate::phase::Phase;
use crate::scope::{ProgressCallback, SolverScope};

#[derive(Clone, Debug)]
struct PartitionedRetainedSolution {
    blocker: BlockingPoint,
    value: i64,
    origin: &'static str,
    score: Option<SoftScore>,
}

impl PartitionedRetainedSolution {
    fn new(value: i64) -> Self {
        Self {
            blocker: BlockingPoint::new(),
            value,
            origin: "parent",
            score: None,
        }
    }
}

impl PlanningSolution for PartitionedRetainedSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[derive(Clone, Debug)]
struct PartitionedRetainedDirector {
    solution: PartitionedRetainedSolution,
    descriptor: SolutionDescriptor,
}

impl PartitionedRetainedDirector {
    fn new(solution: PartitionedRetainedSolution) -> Self {
        Self {
            solution,
            descriptor: SolutionDescriptor::new(
                "PartitionedRetainedSolution",
                TypeId::of::<PartitionedRetainedSolution>(),
            ),
        }
    }
}

impl Director<PartitionedRetainedSolution> for PartitionedRetainedDirector {
    fn working_solution(&self) -> &PartitionedRetainedSolution {
        &self.solution
    }

    fn working_solution_mut(&mut self) -> &mut PartitionedRetainedSolution {
        &mut self.solution
    }

    fn calculate_score(&mut self) -> SoftScore {
        let score = SoftScore::of(self.solution.value);
        self.solution.set_score(Some(score));
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.descriptor
    }

    fn clone_working_solution(&self) -> PartitionedRetainedSolution {
        self.solution.clone()
    }

    fn before_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn after_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

    fn entity_count(&self, _descriptor_index: usize) -> Option<usize> {
        Some(1)
    }

    fn total_entity_count(&self) -> Option<usize> {
        Some(1)
    }

    fn constraint_metadata(&self) -> Vec<solverforge_scoring::ConstraintMetadata<'_>> {
        Vec::new()
    }
}

#[derive(Debug)]
struct PausePollingChildPhase;

impl<D, BestCb> Phase<PartitionedRetainedSolution, D, BestCb> for PausePollingChildPhase
where
    D: Director<PartitionedRetainedSolution>,
    BestCb: ProgressCallback<PartitionedRetainedSolution>,
{
    fn solve(
        &mut self,
        solver_scope: &mut SolverScope<'_, PartitionedRetainedSolution, D, BestCb>,
    ) {
        solver_scope.increment_step_count();
        let blocker = solver_scope.working_solution().blocker.clone();
        blocker.block();
        if solver_scope.should_terminate() {
            return;
        }

        solver_scope.mutate(|director| {
            let solution = director.working_solution_mut();
            solution.value = 23;
            solution.origin = "child";
        });
        solver_scope.update_best_solution();
    }

    fn phase_type_name(&self) -> &'static str {
        "PausePollingChild"
    }
}

impl Solvable for PartitionedRetainedSolution {
    fn solve(self, runtime: SolverRuntime<Self>) {
        let mut solver_scope = SolverScope::new_with_callback(
            PartitionedRetainedDirector::new(self),
            (),
            None,
            Some(runtime),
        );
        solver_scope.start_solving();
        let score = solver_scope.calculate_score();
        let solution = solver_scope.score_director().clone_working_solution();
        solver_scope.set_best_solution(solution.clone(), score);
        runtime.emit_best_solution(
            solution,
            solver_scope.current_score().copied(),
            score,
            solver_scope.stats().snapshot(),
        );

        let partitioner = FunctionalPartitioner::new(
            |solution: &PartitionedRetainedSolution| {
                let mut child = solution.clone();
                child.origin = "child";
                vec![child]
            },
            |original: &PartitionedRetainedSolution, mut partitions| {
                let child = partitions.pop().expect("partition should exist");
                let mut merged = original.clone();
                merged.value = child.value;
                merged.origin = "parent";
                merged.score = None;
                merged
            },
        );
        let mut phase =
            PartitionedSearchPhase::new(partitioner, PartitionedRetainedDirector::new, || {
                (PausePollingChildPhase,)
            });
        phase.solve(&mut solver_scope);

        let telemetry = solver_scope.stats().snapshot();
        let current_score = solver_scope.current_score().copied();
        let best_score = solver_scope.best_score().copied().unwrap_or(score);
        match solver_scope.terminal_reason() {
            SolverTerminalReason::Completed | SolverTerminalReason::TerminatedByConfig => {
                let solution = solver_scope
                    .best_solution()
                    .cloned()
                    .unwrap_or_else(|| solver_scope.score_director().clone_working_solution());
                runtime.emit_completed(
                    solution,
                    current_score,
                    best_score,
                    telemetry,
                    solver_scope.terminal_reason(),
                );
            }
            SolverTerminalReason::Cancelled => {
                runtime.emit_cancelled(current_score, Some(best_score), telemetry);
            }
            SolverTerminalReason::Failed => unreachable!("test solver scope cannot fail"),
        }
    }
}

#[test]
fn partitioned_child_pause_publishes_parent_snapshot_only() {
    static MANAGER: SolverManager<PartitionedRetainedSolution> = SolverManager::new();

    let solution = PartitionedRetainedSolution::new(5);
    let blocker = solution.blocker.clone();
    let (job_id, mut receiver) = MANAGER.solve(solution).expect("job should start");

    match recv_event(&mut receiver, "best solution event") {
        SolverEvent::BestSolution { metadata, solution } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Solving);
            assert_eq!(solution.origin, "parent");
            assert_eq!(solution.value, 5);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    blocker.wait_until_blocked();
    MANAGER.pause(job_id).expect("pause should be accepted");
    match recv_event(&mut receiver, "pause requested event") {
        SolverEvent::PauseRequested { metadata } => {
            assert_eq!(
                metadata.lifecycle_state,
                SolverLifecycleState::PauseRequested
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }

    blocker.release();
    match recv_event(&mut receiver, "paused event") {
        SolverEvent::Paused { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Paused);
            assert_eq!(metadata.snapshot_revision, Some(2));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let paused = MANAGER
        .get_snapshot(job_id, None)
        .expect("paused parent snapshot");
    assert_eq!(paused.solution.origin, "parent");
    assert_eq!(paused.solution.value, 5);

    MANAGER.resume(job_id).expect("resume should be accepted");
    match recv_event(&mut receiver, "resumed event") {
        SolverEvent::Resumed { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Solving);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match recv_event(&mut receiver, "completed event") {
        SolverEvent::Completed { metadata, solution } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Completed);
            assert_eq!(
                metadata.terminal_reason,
                Some(SolverTerminalReason::Completed)
            );
            assert_eq!(solution.origin, "parent");
            assert_eq!(solution.value, 23);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.delete(job_id).expect("delete completed job");
}
