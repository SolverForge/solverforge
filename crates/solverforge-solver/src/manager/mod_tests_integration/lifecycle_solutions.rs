use std::any::TypeId;
use std::time::Duration;

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::SoftScore;
use solverforge_scoring::Director;

use super::super::{
    Analyzable, ConstraintAnalysis, ScoreAnalysis, Solvable, SolverRuntime, SolverTerminalReason,
};
use super::common::NoOpPhase;
use super::gates::LifecycleStepGate;
use super::runtime_helpers::{telemetry_with_steps, zero_telemetry};
use crate::scope::SolverScope;

#[derive(Clone, Debug)]
pub(super) struct LifecycleSolution {
    pub(super) gate: LifecycleStepGate,
    value: i64,
    score: Option<SoftScore>,
}

impl LifecycleSolution {
    pub(super) fn new(value: i64) -> Self {
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
            solver_scope
                .stats_mut()
                .record_generated_move(Duration::ZERO);
            solver_scope
                .stats_mut()
                .record_evaluated_move(Duration::ZERO);
            solver_scope.stats_mut().record_move_accepted();
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
pub(super) struct PauseRequestedProgressSolution {
    pub(super) gate: LifecycleStepGate,
    value: i64,
    score: Option<SoftScore>,
}

impl PauseRequestedProgressSolution {
    pub(super) fn new(value: i64) -> Self {
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
pub(super) struct PauseOrderingSolution {
    value: i64,
    score: Option<SoftScore>,
}

impl PauseOrderingSolution {
    pub(super) fn new(value: i64) -> Self {
        Self { value, score: None }
    }
}

impl PlanningSolution for PauseOrderingSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

impl Solvable for PauseOrderingSolution {
    fn solve(mut self, runtime: SolverRuntime<Self>) {
        let score = SoftScore::of(self.value);
        self.set_score(Some(score));
        runtime.emit_best_solution(self.clone(), Some(score), score, zero_telemetry());

        while !runtime.is_pause_requested() {
            std::hint::spin_loop();
        }

        for step_count in 1..=8 {
            runtime.emit_progress(Some(score), Some(score), telemetry_with_steps(step_count));
        }

        if runtime.pause_with_snapshot(
            self.clone(),
            Some(score),
            Some(score),
            telemetry_with_steps(9),
        ) {
            runtime.emit_completed(
                self,
                Some(score),
                score,
                telemetry_with_steps(10),
                SolverTerminalReason::Completed,
            );
        } else {
            runtime.emit_cancelled(Some(score), Some(score), telemetry_with_steps(9));
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct DeleteReservationSolution {
    pub(super) release_return: LifecycleStepGate,
    score: Option<SoftScore>,
}

impl DeleteReservationSolution {
    pub(super) fn new() -> Self {
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
pub(super) struct TrivialLifecycleSolution {
    pub(super) gate: LifecycleStepGate,
    score: Option<SoftScore>,
}

impl TrivialLifecycleSolution {
    pub(super) fn new() -> Self {
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

    fn update_entity_shadows(&mut self, _descriptor_index: usize, _entity_index: usize) {}
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
