/// What a [`SolverProgressRef`] reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverProgressKind {
    /// A periodic progress report; carries no solution.
    Progress,
    /// A new best solution; carries the solution.
    BestSolution,
}

/// A progress report passed to a [`ProgressCallback`]; borrows the scope's
/// solution and scores for the duration of the call.
#[derive(Debug, Clone)]
pub struct SolverProgressRef<'a, S: PlanningSolution> {
    /// Whether this is a progress or best-solution report.
    pub kind: SolverProgressKind,
    /// `Solving`, or `Completed` once the retained runtime is terminal.
    pub status: SolverLifecycleState,
    /// The new best solution for [`SolverProgressKind::BestSolution`];
    /// `None` for progress reports.
    pub solution: Option<&'a S>,
    /// Score of the working solution, when calculated.
    pub current_score: Option<&'a S::Score>,
    /// Best score so far; `None` while best-solution publication is deferred.
    pub best_score: Option<&'a S::Score>,
    /// Solve telemetry at the time of the report, without the applied-move
    /// trace.
    pub telemetry: crate::stats::SolverTelemetry,
}

/// Receives progress and best-solution reports from a running solve.
///
/// Implemented for `()` (reports nothing) and for every
/// `Fn(SolverProgressRef<'_, S>) + Send + Sync` closure.
pub trait ProgressCallback<S: PlanningSolution>: Send + Sync {
    #[doc(hidden)]
    const PUBLISHES_PROGRESS: bool = true;

    /// Handles one report. Runs on the solver thread, so it should return
    /// quickly.
    fn invoke(&self, progress: SolverProgressRef<'_, S>);
}

impl<S: PlanningSolution> ProgressCallback<S> for () {
    const PUBLISHES_PROGRESS: bool = false;

    fn invoke(&self, _progress: SolverProgressRef<'_, S>) {}
}

impl<S, F> ProgressCallback<S> for F
where
    S: PlanningSolution,
    F: for<'a> Fn(SolverProgressRef<'a, S>) + Send + Sync,
{
    fn invoke(&self, progress: SolverProgressRef<'_, S>) {
        self(progress);
    }
}

const PROGRESS_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProgressTick {
    pub elapsed: Duration,
    pub step_delta: u64,
    pub move_delta: u64,
}

#[derive(Debug, Clone, Copy)]
struct ProgressPulse {
    phase_index: usize,
    phase_type: &'static str,
    initial_report_pending: bool,
    next_deadline: Instant,
    last_reported_at: Instant,
    last_step_count: u64,
    last_move_count: u64,
}

impl ProgressPulse {
    fn new(
        now: Instant,
        phase_index: usize,
        phase_type: &'static str,
        step_count: u64,
        move_count: u64,
    ) -> Self {
        Self {
            phase_index,
            phase_type,
            initial_report_pending: true,
            next_deadline: now + PROGRESS_INTERVAL,
            last_reported_at: now,
            last_step_count: step_count,
            last_move_count: move_count,
        }
    }

    fn take_due(
        &mut self,
        now: Instant,
        phase_index: usize,
        phase_type: &'static str,
        step_count: u64,
        move_count: u64,
    ) -> Option<ProgressTick> {
        if self.phase_index != phase_index || self.phase_type != phase_type {
            *self = Self::new(now, phase_index, phase_type, step_count, move_count);
            return None;
        }
        let has_new_work = step_count > self.last_step_count || move_count > self.last_move_count;
        let is_initial_report = self.initial_report_pending && has_new_work;
        if !is_initial_report && now < self.next_deadline {
            return None;
        }
        let tick = ProgressTick {
            elapsed: now.duration_since(self.last_reported_at),
            step_delta: step_count.saturating_sub(self.last_step_count),
            move_delta: move_count.saturating_sub(self.last_move_count),
        };
        self.last_reported_at = now;
        self.last_step_count = step_count;
        self.last_move_count = move_count;
        if is_initial_report {
            self.initial_report_pending = false;
            self.next_deadline = now + PROGRESS_INTERVAL;
        } else {
            while self.next_deadline <= now {
                self.next_deadline += PROGRESS_INTERVAL;
            }
        }
        Some(tick)
    }
}
