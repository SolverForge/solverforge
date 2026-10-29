#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverProgressKind {
    Progress,
    BestSolution,
}

#[derive(Debug, Clone)]
pub struct SolverProgressRef<'a, S: PlanningSolution> {
    pub kind: SolverProgressKind,
    pub status: SolverLifecycleState,
    pub solution: Option<&'a S>,
    pub current_score: Option<&'a S::Score>,
    pub best_score: Option<&'a S::Score>,
    pub telemetry: crate::stats::SolverTelemetry,
}

pub trait ProgressCallback<S: PlanningSolution>: Send + Sync {
    #[doc(hidden)]
    const PUBLISHES_PROGRESS: bool = true;

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
