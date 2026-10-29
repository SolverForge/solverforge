use super::*;

#[derive(Clone, Debug)]
pub(crate) struct PauseRequestedProgressSolution {
    pub(crate) gate: LifecycleStepGate,
    value: i64,
    score: Option<SoftScore>,
}

impl PauseRequestedProgressSolution {
    pub(crate) fn new(value: i64) -> Self {
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
    fn solve(
        mut self,
        runtime: SolverRuntime<Self>,
        _provenance: Option<crate::stats::QualifiedCandidateTraceRunProvenance>,
    ) -> crate::RuntimeBuildResult<()> {
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
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TerminalHookLifecycleSolution {
    pub(crate) terminal_hook: BlockingPoint,
    value: i64,
    score: Option<SoftScore>,
}

impl TerminalHookLifecycleSolution {
    pub(crate) fn new(value: i64) -> Self {
        Self {
            terminal_hook: BlockingPoint::new(),
            value,
            score: None,
        }
    }
}

impl PlanningSolution for TerminalHookLifecycleSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

impl Solvable for TerminalHookLifecycleSolution {
    fn solve(
        mut self,
        runtime: SolverRuntime<Self>,
        _provenance: Option<crate::stats::QualifiedCandidateTraceRunProvenance>,
    ) -> crate::RuntimeBuildResult<()> {
        let score = SoftScore::of(self.value);
        self.set_score(Some(score));
        self.terminal_hook.block();
        runtime.emit_completed(
            self,
            Some(score),
            score,
            zero_telemetry(),
            SolverTerminalReason::Completed,
        );
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PauseOrderingSolution {
    value: i64,
    score: Option<SoftScore>,
}

impl PauseOrderingSolution {
    pub(crate) fn new(value: i64) -> Self {
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
    fn solve(
        mut self,
        runtime: SolverRuntime<Self>,
        _provenance: Option<crate::stats::QualifiedCandidateTraceRunProvenance>,
    ) -> crate::RuntimeBuildResult<()> {
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
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DeleteReservationSolution {
    pub(crate) release_return: LifecycleStepGate,
    score: Option<SoftScore>,
}

impl DeleteReservationSolution {
    pub(crate) fn new() -> Self {
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
    fn solve(
        mut self,
        runtime: SolverRuntime<Self>,
        _provenance: Option<crate::stats::QualifiedCandidateTraceRunProvenance>,
    ) -> crate::RuntimeBuildResult<()> {
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
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ZeroVariableLifecycleSolution {
    pub(crate) gate: LifecycleStepGate,
    score: Option<SoftScore>,
}

impl ZeroVariableLifecycleSolution {
    pub(crate) fn new() -> Self {
        Self {
            gate: LifecycleStepGate::new_closed(),
            score: None,
        }
    }
}

impl PlanningSolution for ZeroVariableLifecycleSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }

    fn update_entity_shadows(&mut self, _descriptor_index: usize, _entity_index: usize) {}
}

fn zero_variable_solution_descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new(
        "ZeroVariableLifecycleSolution",
        TypeId::of::<ZeroVariableLifecycleSolution>(),
    )
}

fn zero_variable_entity_count(
    _solution: &ZeroVariableLifecycleSolution,
    _descriptor_index: usize,
) -> usize {
    0
}

fn zero_variable_log_scale(solution: &ZeroVariableLifecycleSolution) {
    solution.gate.wait_for_permit();
}

fn zero_variable_constraints(_solution: &ZeroVariableLifecycleSolution) {}

impl Solvable for ZeroVariableLifecycleSolution {
    fn solve(
        self,
        runtime: SolverRuntime<Self>,
        _provenance: Option<crate::stats::QualifiedCandidateTraceRunProvenance>,
    ) -> crate::RuntimeBuildResult<()> {
        let result = try_run_solver_with_config_and_search(
            self,
            zero_variable_constraints,
            zero_variable_solution_descriptor(),
            zero_variable_entity_count,
            runtime,
            solverforge_config::SolverConfig::default(),
            30,
            zero_variable_log_scale,
            None,
            |config, descriptor| {
                let model = RuntimeModel::<
                    ZeroVariableLifecycleSolution,
                    usize,
                    DefaultCrossEntityDistanceMeter,
                    DefaultCrossEntityDistanceMeter,
                >::new(Vec::new());
                Ok(SearchContext::new(descriptor, model, config.random_seed)?.defaults())
            },
        );
        assert!(
            result.is_ok(),
            "zero-variable compiled runtime solve must succeed"
        );
        Ok(())
    }
}
