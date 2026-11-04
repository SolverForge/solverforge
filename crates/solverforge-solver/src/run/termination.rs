use super::*;

/// Parsed solver termination policy shared by runtime phase assembly and the
/// top-level termination builder: every configured criterion, combined with
/// OR, so the solve stops at whichever is reached first.
#[derive(Clone, Copy)]
pub(crate) struct ConfiguredTermination<Sc> {
    pub(super) time_limit: Option<Duration>,
    best_score: Option<Sc>,
    step_count: Option<u64>,
    unimproved_step_count: Option<u64>,
    unimproved_time: Option<Duration>,
}

impl<Sc> ConfiguredTermination<Sc> {
    pub(crate) fn has_effective_limit(&self) -> bool {
        self.time_limit.is_some()
            || self.best_score.is_some()
            || self.step_count.is_some()
            || self.unimproved_step_count.is_some()
            || self.unimproved_time.is_some()
    }
}

/// Returns a termination's `best_score_limit` as the model's score.
///
/// `compile_runtime_graph` rejects a limit that does not parse before any
/// solve starts, so every reader after compilation sees either no limit or a
/// valid one.
pub(crate) fn configured_best_score_limit<Sc: ParseableScore>(
    config: &TerminationConfig,
) -> Option<Sc> {
    config.best_score_limit.as_deref().map(|score| {
        Sc::parse(score).unwrap_or_else(|error| {
            panic!("best_score_limit `{score}` reached a solve unvalidated: {error}")
        })
    })
}

pub(crate) fn parse_configured_termination<S>(
    config: Option<&TerminationConfig>,
) -> ConfiguredTermination<S::Score>
where
    S: PlanningSolution,
    S::Score: ParseableScore,
{
    ConfiguredTermination {
        time_limit: config.and_then(TerminationConfig::time_limit),
        best_score: config.and_then(configured_best_score_limit::<S::Score>),
        step_count: config.and_then(|config| config.step_count_limit),
        unimproved_step_count: config.and_then(|config| config.unimproved_step_count_limit),
        unimproved_time: config.and_then(TerminationConfig::unimproved_time_limit),
    }
}

/// The solver-wide termination: every criterion of `[termination]`, combined
/// with OR. Each criterion is its own concrete termination; an absent one is
/// never checked.
pub(crate) struct ConfiguredSolverTermination<S: PlanningSolution> {
    time: Option<TimeTermination>,
    best_score: Option<BestScoreTermination<S::Score>>,
    step_count: Option<StepCountTermination>,
    unimproved_step_count: Option<UnimprovedStepCountTermination<S>>,
    unimproved_time: Option<UnimprovedTimeTermination<S>>,
}

impl<S: PlanningSolution> fmt::Debug for ConfiguredSolverTermination<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConfiguredSolverTermination")
            .field("time", &self.time)
            .field("best_score", &self.best_score.is_some())
            .field("step_count", &self.step_count)
            .field("unimproved_step_count", &self.unimproved_step_count)
            .field("unimproved_time", &self.unimproved_time.is_some())
            .finish()
    }
}

impl<S: PlanningSolution> ConfiguredSolverTermination<S> {
    /// Whether no criterion is configured, so the solve runs until its phases
    /// end or it is cancelled.
    #[cfg(test)]
    pub(crate) fn is_unbounded(&self) -> bool {
        self.time.is_none()
            && self.best_score.is_none()
            && self.step_count.is_none()
            && self.unimproved_step_count.is_none()
            && self.unimproved_time.is_none()
    }
}

impl<S, D, ProgressCb> Termination<S, D, ProgressCb> for ConfiguredSolverTermination<S>
where
    S: PlanningSolution,
    S::Score: Score,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    fn is_terminated(&mut self, solver_scope: &SolverScope<S, D, ProgressCb>) -> bool {
        // Every criterion is checked, so stateful ones (the unimproved
        // trackers) observe each step even when another one fires first.
        let mut terminated = false;
        if let Some(time) = &mut self.time {
            terminated |= Termination::<S, D, ProgressCb>::is_terminated(time, solver_scope);
        }
        if let Some(best_score) = &mut self.best_score {
            terminated |= Termination::<S, D, ProgressCb>::is_terminated(best_score, solver_scope);
        }
        if let Some(step_count) = &mut self.step_count {
            terminated |= Termination::<S, D, ProgressCb>::is_terminated(step_count, solver_scope);
        }
        if let Some(unimproved) = &mut self.unimproved_step_count {
            terminated |= Termination::<S, D, ProgressCb>::is_terminated(unimproved, solver_scope);
        }
        if let Some(unimproved) = &mut self.unimproved_time {
            terminated |= Termination::<S, D, ProgressCb>::is_terminated(unimproved, solver_scope);
        }
        terminated
    }

    fn install_inphase_limits(&self, solver_scope: &mut SolverScope<S, D, ProgressCb>) {
        if let Some(time) = &self.time {
            Termination::<S, D, ProgressCb>::install_inphase_limits(time, solver_scope);
        }
        if let Some(best_score) = &self.best_score {
            Termination::<S, D, ProgressCb>::install_inphase_limits(best_score, solver_scope);
        }
        if let Some(step_count) = &self.step_count {
            Termination::<S, D, ProgressCb>::install_inphase_limits(step_count, solver_scope);
        }
        if let Some(unimproved) = &self.unimproved_step_count {
            Termination::<S, D, ProgressCb>::install_inphase_limits(unimproved, solver_scope);
        }
        if let Some(unimproved) = &self.unimproved_time {
            Termination::<S, D, ProgressCb>::install_inphase_limits(unimproved, solver_scope);
        }
    }
}

/// Builds the solver-wide termination from config, returning it and the
/// configured time limit. Nothing is added that the configuration does not
/// state: without a time limit there is no time guard.
pub(crate) fn build_solver_termination<S>(
    config: &SolverConfig,
) -> (ConfiguredSolverTermination<S>, Option<Duration>)
where
    S: PlanningSolution,
    S::Score: Score + ParseableScore,
{
    let configured = parse_configured_termination::<S>(config.termination.as_ref());
    (
        ConfiguredSolverTermination {
            time: configured.time_limit.map(TimeTermination::new),
            best_score: configured.best_score.map(BestScoreTermination::new),
            step_count: configured.step_count.map(StepCountTermination::new),
            unimproved_step_count: configured
                .unimproved_step_count
                .map(UnimprovedStepCountTermination::new),
            unimproved_time: configured
                .unimproved_time
                .map(UnimprovedTimeTermination::new),
        },
        configured.time_limit,
    )
}

/// Records the termination policy the configured runtime installed: the
/// configured time limit and every configured criterion, all combined with OR.
pub(crate) fn configured_execution_policy<S>(config: &SolverConfig) -> CandidateTraceExecutionPolicy
where
    S: PlanningSolution,
    S::Score: ParseableScore + std::fmt::Display,
{
    let configured = parse_configured_termination::<S>(config.termination.as_ref());
    let none = || "none".to_string();
    let mut criteria = Vec::new();
    if configured.time_limit.is_some() {
        criteria.push("time");
    }
    if configured.best_score.is_some() {
        criteria.push("best_score");
    }
    if configured.step_count.is_some() {
        criteria.push("step_count");
    }
    if configured.unimproved_step_count.is_some() {
        criteria.push("unimproved_step_count");
    }
    if configured.unimproved_time.is_some() {
        criteria.push("unimproved_time");
    }
    let composition = if criteria.is_empty() {
        "unbounded".to_string()
    } else {
        criteria.join("_or_")
    };
    let attributes = vec![
        ("entrypoint".to_string(), "configured_runtime".to_string()),
        (
            "configured_time_limit_ns".to_string(),
            configured.time_limit.map_or_else(none, duration_nanos),
        ),
        (
            "best_score_limit".to_string(),
            configured
                .best_score
                .map_or_else(none, |score| score.to_string()),
        ),
        (
            "step_count_limit".to_string(),
            configured
                .step_count
                .map_or_else(none, |limit| limit.to_string()),
        ),
        (
            "unimproved_step_count_limit".to_string(),
            configured
                .unimproved_step_count
                .map_or_else(none, |limit| limit.to_string()),
        ),
        (
            "unimproved_time_limit_ns".to_string(),
            configured.unimproved_time.map_or_else(none, duration_nanos),
        ),
        ("termination_composition".to_string(), composition),
    ];
    CandidateTraceExecutionPolicy::known("solverforge.execution_policy", attributes)
}

fn duration_nanos(duration: Duration) -> String {
    duration.as_nanos().to_string()
}
