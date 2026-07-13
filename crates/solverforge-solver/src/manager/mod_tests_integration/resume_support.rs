use solverforge_core::score::SoftScore;
use solverforge_core::PlanningSolution;

use super::super::{
    Analyzable, ConstraintAnalysis, ScoreAnalysis, Solvable, SolverRuntime, SolverTerminalReason,
};
use super::gates::LifecycleStepGate;
use super::runtime_helpers::{telemetry_with_steps, zero_telemetry};
use crate::stats::{
    CandidateTraceExecutionPolicy, CandidateTraceHeader, CandidateTracePhasePlan,
    CandidateTraceTelemetry,
};

fn telemetry_with_trace(step_count: u64, configured_input: &str) -> crate::SolverTelemetry {
    let header = CandidateTraceHeader::new(
        configured_input.to_string(),
        CandidateTraceExecutionPolicy::known("test-policy", std::iter::empty::<(String, String)>()),
        CandidateTracePhasePlan::known(
            "test-phase",
            std::iter::empty::<(String, String)>(),
            Vec::new(),
        ),
        None,
    );
    crate::SolverTelemetry {
        step_count,
        candidate_trace: Some(CandidateTraceTelemetry::new(header, 1)),
        ..crate::SolverTelemetry::default()
    }
}

#[derive(Clone, Debug)]
pub(super) struct DeterministicResumeSolution {
    pub(super) gate: LifecycleStepGate,
    pub(super) value: i64,
    score: Option<SoftScore>,
}

impl DeterministicResumeSolution {
    pub(super) fn new() -> Self {
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
    fn solve(
        mut self,
        runtime: SolverRuntime<Self>,
        _provenance: Option<crate::stats::QualifiedCandidateTraceRunProvenance>,
    ) {
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

/// A retained solve whose gates make every trace-bearing and trace-free
/// publication observable before the worker can publish the next one.
///
/// It exists to prove that `get_telemetry_detail()` never combines a paused
/// diagnostic prefix with a later resume/progress/terminal status.
#[derive(Clone, Debug)]
pub(super) struct TracePairingResumeSolution {
    pub(super) pause_boundary: LifecycleStepGate,
    pub(super) resumed_boundary: LifecycleStepGate,
    pub(super) progress_boundary: LifecycleStepGate,
    score: Option<SoftScore>,
}

impl TracePairingResumeSolution {
    pub(super) fn new() -> Self {
        Self {
            pause_boundary: LifecycleStepGate::new_closed(),
            resumed_boundary: LifecycleStepGate::new_closed(),
            progress_boundary: LifecycleStepGate::new_closed(),
            score: None,
        }
    }
}

impl PlanningSolution for TracePairingResumeSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

impl Solvable for TracePairingResumeSolution {
    fn solve(
        mut self,
        runtime: SolverRuntime<Self>,
        _provenance: Option<crate::stats::QualifiedCandidateTraceRunProvenance>,
    ) {
        let score = SoftScore::of(17);
        self.set_score(Some(score));
        runtime.emit_best_solution(self.clone(), Some(score), score, telemetry_with_steps(0));

        self.pause_boundary.wait_for_permit();
        if !runtime.pause_with_snapshot(
            self.clone(),
            Some(score),
            Some(score),
            telemetry_with_trace(1, "retained-trace-pairing-pause"),
        ) {
            runtime.emit_cancelled(Some(score), Some(score), zero_telemetry());
            return;
        }

        // `pause_with_snapshot` has already published Resumed at this point.
        self.resumed_boundary.wait_for_permit();
        runtime.emit_progress(Some(score), Some(score), telemetry_with_steps(2));
        self.progress_boundary.wait_for_permit();
        runtime.emit_completed(
            self,
            Some(score),
            score,
            telemetry_with_trace(3, "retained-trace-pairing-terminal"),
            SolverTerminalReason::Completed,
        );
    }
}

#[derive(Clone, Debug)]
pub(super) struct FailureAfterSnapshotSolution {
    value: i64,
    score: Option<SoftScore>,
}

impl FailureAfterSnapshotSolution {
    pub(super) fn new(value: i64) -> Self {
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
    fn solve(
        mut self,
        runtime: SolverRuntime<Self>,
        _provenance: Option<crate::stats::QualifiedCandidateTraceRunProvenance>,
    ) {
        let score = SoftScore::of(self.value);
        self.set_score(Some(score));
        runtime.emit_best_solution(self, Some(score), score, zero_telemetry());
        panic!("expected retained lifecycle failure");
    }
}

#[derive(Clone, Debug)]
pub(super) struct ConfigTerminatedSolution {
    value: i64,
    score: Option<SoftScore>,
}

impl ConfigTerminatedSolution {
    pub(super) fn new(value: i64) -> Self {
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
    fn solve(
        mut self,
        runtime: SolverRuntime<Self>,
        _provenance: Option<crate::stats::QualifiedCandidateTraceRunProvenance>,
    ) {
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
