//! The `move_thread_count = "auto"` policy: per step, sequential evaluation
//! or parallel windows, whichever the measured costs say is faster.
//!
//! Sequential and parallel evaluation reach identical trajectories, so the
//! policy changes how fast a step runs, never which move it selects.

use std::hint::black_box;
use std::time::{Duration, Instant};

use rayon::prelude::*;

use super::pool::MAX_PARALLEL_EVALUATION_WINDOW;

/// Consecutive steps that must favour the other mode before the policy
/// switches, so one noisy step never triggers a replica rebuild.
pub(crate) const AUTO_SWITCH_STREAK: u32 = 8;

/// Fork-join round trips timed when the policy calibrates; the fastest one is
/// the window overhead.
const CALIBRATION_ROUNDS: usize = 16;

/// The cost model and switching state of one `auto` local-search phase.
#[derive(Debug)]
pub(crate) struct AutoPolicy {
    evaluators: usize,
    window_overhead: Duration,
    parallel: bool,
    streak: u32,
    streak_saving_nanos: i128,
    replica_build: Duration,
}

impl AutoPolicy {
    /// A sequential policy for `evaluators` evaluators (main director
    /// included) whose windows cost `window_overhead` to fork and join.
    pub(crate) fn new(evaluators: usize, window_overhead: Duration) -> Self {
        Self {
            evaluators: evaluators.clamp(1, MAX_PARALLEL_EVALUATION_WINDOW),
            window_overhead,
            parallel: false,
            streak: 0,
            streak_saving_nanos: 0,
            replica_build: Duration::ZERO,
        }
    }

    /// Times an empty fork-join across `evaluators` on the current Rayon pool.
    pub(crate) fn calibrate(evaluators: usize) -> Self {
        let mut fastest = Duration::MAX;
        for _ in 0..CALIBRATION_ROUNDS {
            let started = Instant::now();
            rayon::join(
                || black_box(0usize),
                || {
                    (1..evaluators.max(1))
                        .into_par_iter()
                        .for_each(|evaluator| {
                            black_box(evaluator);
                        })
                },
            );
            fastest = fastest.min(started.elapsed());
        }
        Self::new(evaluators, fastest)
    }

    /// Whether the next step runs parallel windows.
    pub(crate) fn is_parallel(&self) -> bool {
        self.parallel
    }

    /// Records what building the replicas cost; the next switch to parallel
    /// must be expected to win that back.
    pub(crate) fn record_replica_build(&mut self, elapsed: Duration) {
        self.replica_build = elapsed;
    }

    /// Feeds one finished step: `consumed` candidates whose trials took
    /// `trial_time` in total. Returns whether the next step runs parallel.
    pub(crate) fn observe_step(&mut self, consumed: u64, trial_time: Duration) -> bool {
        if self.evaluators <= 1 || consumed == 0 {
            return self.parallel;
        }
        let saving = self.parallel_saving_nanos(consumed, trial_time);
        let favours_parallel = saving > 0;
        if favours_parallel == self.parallel {
            self.streak = 0;
            self.streak_saving_nanos = 0;
            return self.parallel;
        }
        self.streak += 1;
        self.streak_saving_nanos += saving;
        let switching_to_parallel_pays =
            self.streak_saving_nanos >= self.replica_build.as_nanos() as i128;
        if self.streak >= AUTO_SWITCH_STREAK && (self.parallel || switching_to_parallel_pays) {
            self.parallel = !self.parallel;
            self.streak = 0;
            self.streak_saving_nanos = 0;
        }
        self.parallel
    }

    /// Expected nanoseconds one step like the observed one saves in parallel
    /// windows; negative when sequential evaluation is faster.
    ///
    /// Sequential cost is the trial time. Parallel cost is one fork-join per
    /// window plus the trials spread over the evaluators, including up to one
    /// speculative trial per extra evaluator past the step's decision point.
    pub(crate) fn parallel_saving_nanos(&self, consumed: u64, trial_time: Duration) -> i128 {
        let consumed = i128::from(consumed.max(1));
        let evaluators = self.evaluators as i128;
        let trial_nanos = trial_time.as_nanos() as i128;
        let window = MAX_PARALLEL_EVALUATION_WINDOW as i128;
        let windows = (consumed + window - 1) / window;
        let parallel_trials = trial_nanos * (consumed + evaluators - 1) / (consumed * evaluators);
        let parallel = windows * self.window_overhead.as_nanos() as i128 + parallel_trials;
        trial_nanos - parallel
    }
}
