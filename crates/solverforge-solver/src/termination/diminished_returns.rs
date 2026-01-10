//! Diminished returns termination.

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
/// let term = DiminishedReturnsTermination::<MySolution>::new(
///     Duration::from_secs(10),
///     0.1,
/// );
/// ```
pub struct DiminishedReturnsTermination<S: PlanningSolution> {
    window: Duration,
    min_rate: f64,
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
    samples: VecDeque<(Instant, Sc)>,
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
    pub fn new(window: Duration, min_rate: f64) -> Self {
        Self {
            window,
            min_rate,
            state: RefCell::new(DiminishedState::default()),
            _phantom: PhantomData,
        }
    }

    pub fn with_seconds(window_secs: u64, min_rate: f64) -> Self {
        Self::new(Duration::from_secs(window_secs), min_rate)
    }
}

unsafe impl<S: PlanningSolution> Send for DiminishedReturnsTermination<S> {}

impl<S: PlanningSolution, D: ScoreDirector<S>> Termination<S, D>
    for DiminishedReturnsTermination<S>
{
    fn is_terminated(&self, solver_scope: &SolverScope<S, D>) -> bool {
        let Some(current_score) = solver_scope.best_score() else {
            return false;
        };

        let mut state = self.state.borrow_mut();
        let now = Instant::now();

        if state.start_time.is_none() {
            state.start_time = Some(now);
        }

        if now.duration_since(state.start_time.unwrap()) < self.window {
            state.samples.push_back((now, current_score.clone()));
            return false;
        }

        let cutoff = now - self.window;
        while let Some((time, _)) = state.samples.front() {
            if *time < cutoff {
                state.samples.pop_front();
            } else {
                break;
            }
        }

        state.samples.push_back((now, current_score.clone()));

        if state.samples.len() < 2 {
            return false;
        }

        let (oldest_time, oldest_score) = state.samples.front().unwrap();
        let elapsed = now.duration_since(*oldest_time).as_secs_f64();

        if elapsed < 0.001 {
            return false;
        }

        let current_levels = current_score.to_level_numbers();
        let oldest_levels = oldest_score.to_level_numbers();

        let current_value = *current_levels.last().unwrap_or(&0);
        let oldest_value = *oldest_levels.last().unwrap_or(&0);

        let improvement = (current_value - oldest_value) as f64;
        let rate = improvement / elapsed;

        rate < self.min_rate
    }
}
