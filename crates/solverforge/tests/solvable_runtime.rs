use solverforge::__internal::PlanningSolution as PlanningSolutionTrait;
use solverforge::{
    SoftScore, Solvable, SolverEvent, SolverManager, SolverRuntime, SolverTelemetry,
    SolverTerminalReason,
};

#[derive(Clone, Debug)]
struct ManualRuntimeSolution {
    value: i64,
    score: Option<SoftScore>,
}

impl PlanningSolutionTrait for ManualRuntimeSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

impl Solvable for ManualRuntimeSolution {
    fn solve(mut self, runtime: SolverRuntime<Self>) {
        let score = SoftScore::of(self.value);
        self.set_score(Some(score));

        runtime.emit_best_solution(self.clone(), Some(score), score, SolverTelemetry::default());
        runtime.emit_progress(Some(score), Some(score), SolverTelemetry::default());
        runtime.emit_completed(
            self,
            Some(score),
            score,
            SolverTelemetry::default(),
            SolverTerminalReason::Completed,
        );
    }
}

#[test]
fn manual_solvable_impl_can_emit_public_runtime_events() {
    static MANAGER: SolverManager<ManualRuntimeSolution> = SolverManager::new();

    let (job_id, mut receiver) = MANAGER
        .solve(ManualRuntimeSolution {
            value: 9,
            score: None,
        })
        .expect("job should start");

    match receiver.blocking_recv().expect("best solution event") {
        SolverEvent::BestSolution { metadata, solution } => {
            assert_eq!(metadata.current_score, Some(SoftScore::of(9)));
            assert_eq!(solution.score(), Some(SoftScore::of(9)));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match receiver.blocking_recv().expect("progress event") {
        SolverEvent::Progress { metadata } => {
            assert_eq!(metadata.best_score, Some(SoftScore::of(9)));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match receiver.blocking_recv().expect("completed event") {
        SolverEvent::Completed { metadata, solution } => {
            assert_eq!(
                metadata.terminal_reason,
                Some(SolverTerminalReason::Completed)
            );
            assert_eq!(solution.score(), Some(SoftScore::of(9)));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.delete(job_id).expect("delete completed job");
}
