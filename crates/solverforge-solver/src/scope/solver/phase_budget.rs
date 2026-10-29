#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Publication {
    Enabled,
    Disabled,
}

pub(crate) struct PhaseBudget {
    step_count_limit: Option<u64>,
    move_count_limit: Option<u64>,
    score_calc_count_limit: Option<u64>,
    step_count: AtomicU64,
    moves_evaluated: AtomicU64,
    score_calculations: AtomicU64,
}

impl PhaseBudget {
    fn from_scope<S, D, ProgressCb>(scope: &SolverScope<'_, S, D, ProgressCb>) -> Self
    where
        S: PlanningSolution,
        D: Director<S>,
        ProgressCb: ProgressCallback<S>,
    {
        Self {
            step_count_limit: remaining_limit(
                scope.inphase_step_count_limit,
                scope.total_step_count,
            ),
            move_count_limit: remaining_limit(
                scope.inphase_move_count_limit,
                scope.stats.moves_evaluated(),
            ),
            score_calc_count_limit: remaining_limit(
                scope.inphase_score_calc_count_limit,
                scope.stats.score_calculations(),
            ),
            step_count: AtomicU64::new(0),
            moves_evaluated: AtomicU64::new(0),
            score_calculations: AtomicU64::new(0),
        }
    }

    fn has_limits(&self) -> bool {
        self.step_count_limit.is_some()
            || self.move_count_limit.is_some()
            || self.score_calc_count_limit.is_some()
    }

    fn record_step(&self) {
        self.step_count.fetch_add(1, Ordering::SeqCst);
    }

    fn record_evaluated_move(&self) {
        self.moves_evaluated.fetch_add(1, Ordering::SeqCst);
    }

    fn record_score_calculation(&self) {
        self.score_calculations.fetch_add(1, Ordering::SeqCst);
    }

    fn limit_reached(&self) -> bool {
        limit_reached(self.step_count_limit, self.step_count.load(Ordering::SeqCst))
            || limit_reached(
                self.move_count_limit,
                self.moves_evaluated.load(Ordering::SeqCst),
            )
            || limit_reached(
                self.score_calc_count_limit,
                self.score_calculations.load(Ordering::SeqCst),
            )
    }
}

#[derive(Clone, Copy)]
struct ScopedPhaseTermination<S: PlanningSolution> {
    start_step_count: u64,
    start_elapsed: Duration,
    time_limit: Option<Duration>,
    step_count_limit: Option<u64>,
    best_score_limit: Option<S::Score>,
    unimproved_step_count_limit: Option<u64>,
    unimproved_time_limit: Option<Duration>,
    best_score: Option<S::Score>,
    improvement_score: Option<S::Score>,
    last_improvement_step_count: u64,
    last_improvement_elapsed: Duration,
}

impl<S> ScopedPhaseTermination<S>
where
    S: PlanningSolution,
{
    fn from_config<D, ProgressCb>(
        scope: &SolverScope<'_, S, D, ProgressCb>,
        config: &TerminationConfig,
    ) -> Option<Self>
    where
        D: Director<S>,
        ProgressCb: ProgressCallback<S>,
        S::Score: ParseableScore,
    {
        let best_score_limit = crate::run::configured_best_score_limit::<S::Score>(config);
        let time_limit = config.time_limit();
        let step_count_limit = config.step_count_limit;
        let unimproved_step_count_limit = config.unimproved_step_count_limit;
        let unimproved_time_limit = config.unimproved_time_limit();
        if time_limit.is_none()
            && step_count_limit.is_none()
            && best_score_limit.is_none()
            && unimproved_step_count_limit.is_none()
            && unimproved_time_limit.is_none()
        {
            return None;
        }
        let elapsed = scope.elapsed().unwrap_or_default();
        let best_score = match (scope.best_score, scope.current_score) {
            (Some(best), Some(current)) => Some(best.max(current)),
            (Some(best), None) | (None, Some(best)) => Some(best),
            (None, None) => None,
        };
        Some(Self {
            start_step_count: scope.total_step_count,
            start_elapsed: elapsed,
            time_limit,
            step_count_limit,
            best_score_limit,
            unimproved_step_count_limit,
            unimproved_time_limit,
            best_score,
            improvement_score: scope.current_score.or(scope.best_score),
            last_improvement_step_count: scope.total_step_count,
            last_improvement_elapsed: elapsed,
        })
    }

    fn needs_clock(&self) -> bool {
        self.time_limit.is_some() || self.unimproved_time_limit.is_some()
    }

    fn is_reached(
        &self,
        total_step_count: u64,
        elapsed: Duration,
    ) -> bool {
        let phase_steps = total_step_count.saturating_sub(self.start_step_count);
        let phase_elapsed = elapsed.saturating_sub(self.start_elapsed);
        self.time_limit.is_some_and(|limit| phase_elapsed >= limit)
            || self.step_count_limit.is_some_and(|limit| phase_steps >= limit)
            || self
                .best_score_limit
                .is_some_and(|limit| self.best_score.is_some_and(|score| score >= limit))
            || self.unimproved_step_count_limit.is_some_and(|limit| {
                total_step_count.saturating_sub(self.last_improvement_step_count) >= limit
            })
            || self.unimproved_time_limit.is_some_and(|limit| {
                elapsed.saturating_sub(self.last_improvement_elapsed) >= limit
            })
    }

    fn record_improvement(&mut self, total_step_count: u64, elapsed: Duration) {
        self.last_improvement_step_count = total_step_count;
        self.last_improvement_elapsed = elapsed;
    }

    fn observe_score(
        &mut self,
        score: S::Score,
        completed_step_count: u64,
        elapsed: Duration,
    ) {
        if self.best_score.is_none_or(|best| score > best) {
            self.best_score = Some(score);
        }
        if self.improvement_score.is_none_or(|best| score > best) {
            self.improvement_score = Some(score);
            self.record_improvement(completed_step_count, elapsed);
        }
    }

    fn needs_score_observation(&self) -> bool {
        self.best_score_limit.is_some()
            || self.unimproved_step_count_limit.is_some()
            || self.unimproved_time_limit.is_some()
    }
}

fn remaining_limit(limit: Option<u64>, used: u64) -> Option<u64> {
    limit.map(|limit| limit.saturating_sub(used))
}

fn limit_reached(limit: Option<u64>, used: u64) -> bool {
    limit.is_some_and(|limit| used >= limit)
}
