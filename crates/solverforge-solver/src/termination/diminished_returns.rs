//! Diminished returns termination.
//!
//! Terminates when the rate of score improvement drops below a threshold.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::{Duration, Instant};

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::ScoreDirector;

use super::Termination;
use crate::scope::SolverScope;

/// Terminates when the rate of score improvement falls below a threshold.
///
/// Tracks score improvements over a sliding time window and terminates
/// when the improvement rate drops below a minimum, indicating diminished
/// returns from continued optimization.
///
/// # Example
///
/// ```
/// use std::time::Duration;
/// use solverforge_solver::termination::DiminishedReturnsTermination;
/// use solverforge_core::score::SimpleScore;
/// use solverforge_core::domain::PlanningSolution;
///
/// #[derive(Clone)]
/// struct MySolution;
/// impl PlanningSolution for MySolution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { None }
///     fn set_score(&mut self, _: Option<Self::Score>) {}
/// }
///
/// // Terminate when improvement rate falls below 0.1 per second over a 10s window
/// let term = DiminishedReturnsTermination::<MySolution>::new(
///     Duration::from_secs(10),
///     0.1,
/// );
/// ```
pub struct DiminishedReturnsTermination<S: PlanningSolution> {
    /// Time window for measuring improvement rate.
    window: Duration,
    /// Minimum improvement rate (score units per second) below which to terminate.
    min_rate: f64,
    /// Internal state tracking improvements.
    state: RefCell<DiminishedState<S::Score>>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S: PlanningSolution> Debug for DiminishedReturnsTermination<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiminishedReturnsTermination")
            .field("window", &self.window)
            .field("min_rate", &self.min_rate)
            .finish()
    }
}

struct DiminishedState<Sc: Score> {
    /// Score samples within the window: (timestamp, score).
    samples: VecDeque<(Instant, Sc)>,
    /// Timestamp of first sample for initial grace period.
    start_time: Option<Instant>,
}

impl<Sc: Score> Default for DiminishedState<Sc> {
    fn default() -> Self {
        Self {
            samples: VecDeque::new(),
            start_time: None,
        }
    }
}

impl<S: PlanningSolution> DiminishedReturnsTermination<S> {
    /// Creates a new diminished returns termination.
    ///
    /// # Arguments
    /// * `window` - Time window for measuring improvement rate
    /// * `min_rate` - Minimum improvement rate (score units per second)
    pub fn new(window: Duration, min_rate: f64) -> Self {
        Self {
            window,
            min_rate,
            state: RefCell::new(DiminishedState::default()),
            _phantom: PhantomData,
        }
    }

    /// Creates with a window in seconds.
    pub fn with_seconds(window_secs: u64, min_rate: f64) -> Self {
        Self::new(Duration::from_secs(window_secs), min_rate)
    }
}

// Safety: The RefCell is only accessed from within is_terminated,
// which is called from a single thread during solving.
unsafe impl<S: PlanningSolution> Send for DiminishedReturnsTermination<S> {}

impl<S: PlanningSolution, D: ScoreDirector<S>> Termination<S, D>
    for DiminishedReturnsTermination<S>
{
    fn is_terminated(&self, solver_scope: &SolverScope<S, D>) -> bool {
        let Some(current_score) = solver_scope.best_score() else {
            return false; // No score yet
        };

        let mut state = self.state.borrow_mut();
        let now = Instant::now();

        // Initialize start time on first call
        if state.start_time.is_none() {
            state.start_time = Some(now);
        }

        // Don't terminate during the initial grace period (first window)
        if now.duration_since(state.start_time.unwrap()) < self.window {
            // Still record the sample
            state.samples.push_back((now, *current_score));
            return false;
        }

        // Remove samples outside the window
        let cutoff = now - self.window;
        while let Some((time, _)) = state.samples.front() {
            if *time < cutoff {
                state.samples.pop_front();
            } else {
                break;
            }
        }

        // Add current sample
        state.samples.push_back((now, *current_score));

        // Need at least 2 samples to calculate rate
        if state.samples.len() < 2 {
            return false;
        }

        // Calculate improvement rate
        let (oldest_time, oldest_score) = state.samples.front().unwrap();
        let elapsed = now.duration_since(*oldest_time).as_secs_f64();

        if elapsed < 0.001 {
            return false; // Avoid division by near-zero
        }

        // Use the last (lowest priority / soft) level for rate calculation
        // This is the optimization objective being improved after feasibility
        let current_levels = current_score.to_level_numbers();
        let oldest_levels = oldest_score.to_level_numbers();

        // Use the last level (soft score) for improvement rate
        // For SimpleScore: the only value
        // For HardSoftScore: the soft value
        let current_value = *current_levels.last().unwrap_or(&0);
        let oldest_value = *oldest_levels.last().unwrap_or(&0);

        // Improvement = current - oldest (positive is better)
        let improvement = (current_value - oldest_value) as f64;
        let rate = improvement / elapsed;

        // Terminate if rate is below threshold
        rate < self.min_rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{create_test_scope, create_test_scope_with_score, TestSolution};
    use solverforge_core::score::SimpleScore;
    use std::thread::sleep;

    #[test]
    fn test_not_terminated_during_grace_period() {
        let termination =
            DiminishedReturnsTermination::<TestSolution>::new(Duration::from_millis(100), 0.0);

        let scope = create_test_scope_with_score(SimpleScore::of(-100));

        // During grace period, should not terminate even with no improvement
        assert!(!termination.is_terminated(&scope));
    }

    #[test]
    fn test_terminates_with_zero_improvement() {
        // Use 200ms window with larger margins for cross-platform reliability
        let termination =
            DiminishedReturnsTermination::<TestSolution>::new(Duration::from_millis(200), 0.1);

        let scope = create_test_scope_with_score(SimpleScore::of(-100));

        // First call starts tracking at T0
        assert!(!termination.is_terminated(&scope));

        // Wait well into grace period but keep first sample in window
        sleep(Duration::from_millis(120));

        // Second call adds sample at T0+120ms (still in window)
        // Both samples have score -100, so rate is 0
        assert!(!termination.is_terminated(&scope));

        // Wait past grace period with margin for timing variance
        sleep(Duration::from_millis(100));

        // Third call: past grace period (220ms > 200ms), 2+ samples, rate ~0
        assert!(termination.is_terminated(&scope));
    }

    #[test]
    fn test_not_terminated_with_sufficient_improvement() {
        let termination =
            DiminishedReturnsTermination::<TestSolution>::new(Duration::from_millis(50), 10.0);

        let mut scope = create_test_scope_with_score(SimpleScore::of(-100));

        // Check once to start tracking
        assert!(!termination.is_terminated(&scope));

        sleep(Duration::from_millis(60));

        // Significant improvement: -100 -> 0 = +100 improvement over ~60ms
        // Rate = 100 / 0.060 = ~1667/s, well above 10/s threshold
        scope.set_best_solution(
            TestSolution {
                score: Some(SimpleScore::of(0)),
            },
            SimpleScore::of(0),
        );
        assert!(!termination.is_terminated(&scope));
    }

    #[test]
    fn test_no_score_does_not_terminate() {
        let termination =
            DiminishedReturnsTermination::<TestSolution>::new(Duration::from_millis(10), 0.0);

        let scope = create_test_scope(); // No best score set

        sleep(Duration::from_millis(20));
        assert!(!termination.is_terminated(&scope));
    }
}
