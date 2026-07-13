use solverforge::__internal::PlanningSolution as PlanningSolutionTrait;
use solverforge::{
    CandidateTraceConfig, CandidateTraceExternalDigest, QualifiedCandidateTraceRunProvenance,
    SoftScore, Solvable, SolverEvent, SolverManager, SolverRuntime, SolverTelemetry,
    SolverTelemetryDetail, SolverTerminalReason,
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
    fn solve(
        mut self,
        runtime: SolverRuntime<Self>,
        _provenance: Option<QualifiedCandidateTraceRunProvenance>,
    ) {
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
fn candidate_trace_configuration_and_provenance_are_facade_root_types() {
    let config = CandidateTraceConfig::new(std::num::NonZeroUsize::new(4).unwrap());
    assert_eq!(config.max_entries.get(), 4);

    let digest = |byte| CandidateTraceExternalDigest::sha256([byte; 32]);
    let provenance = QualifiedCandidateTraceRunProvenance::externally_attested(
        digest(1),
        digest(2),
        digest(3),
        digest(4),
        digest(5),
        "facade-only-test",
    )
    .expect("complete external provenance qualifies");
    assert_eq!(provenance.input_provenance().schema_digest, digest(1),);

    let _detail: Option<SolverTelemetryDetail<SoftScore>> = None;
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
