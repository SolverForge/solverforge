//! Termination conditions based on lack of improvement.

use std::cell::RefCell;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::{Duration, Instant};

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::ScoreDirector;

use super::Termination;
use crate::scope::SolverScope;

/// Terminates if no improvement occurs for a specified number of steps.
///
/// # Example
///
/// ```
/// use solverforge_solver::termination::UnimprovedStepCountTermination;
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
/// let term = UnimprovedStepCountTermination::<MySolution>::new(100);
/// ```
pub struct UnimprovedStepCountTermination<S: PlanningSolution> {
    limit: u64,
    state: RefCell<UnimprovedState<S::Score>>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S: PlanningSolution> Debug for UnimprovedStepCountTermination<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = self.state.borrow();
        f.debug_struct("UnimprovedStepCountTermination")
            .field("limit", &self.limit)
            .field("steps_since_improvement", &state.steps_since_improvement)
            .finish()
    }
}

#[derive(Clone)]
struct UnimprovedState<Sc: Score> {
    last_best_score: Option<Sc>,
    steps_since_improvement: u64,
    last_checked_step: Option<u64>,
}

impl<Sc: Score> Default for UnimprovedState<Sc> {
    fn default() -> Self {
        Self {
            last_best_score: None,
            steps_since_improvement: 0,
            last_checked_step: None,
        }
    }
}

impl<S: PlanningSolution> UnimprovedStepCountTermination<S> {
    pub fn new(limit: u64) -> Self {
        Self {
            limit,
            state: RefCell::new(UnimprovedState::default()),
            _phantom: PhantomData,
        }
    }
}

unsafe impl<S: PlanningSolution> Send for UnimprovedStepCountTermination<S> {}

impl<S: PlanningSolution, D: ScoreDirector<S>> Termination<S, D>
    for UnimprovedStepCountTermination<S>
{
    fn is_terminated(&self, solver_scope: &SolverScope<S, D>) -> bool {
        let mut state = self.state.borrow_mut();
        let current_step = solver_scope.total_step_count();

        if state.last_checked_step == Some(current_step) {
            return state.steps_since_improvement >= self.limit;
        }
        state.last_checked_step = Some(current_step);

        let current_best = solver_scope.best_score();

        match (&state.last_best_score, current_best) {
            (None, Some(score)) => {
                state.last_best_score = Some(score.clone());
                state.steps_since_improvement = 0;
            }
            (Some(last), Some(current)) => {
                if *current > *last {
                    state.last_best_score = Some(current.clone());
                    state.steps_since_improvement = 0;
                } else {
                    state.steps_since_improvement += 1;
                }
            }
            (Some(_), None) => {
                state.steps_since_improvement += 1;
            }
            (None, None) => {}
        }

        state.steps_since_improvement >= self.limit
    }
}

/// Terminates if no improvement occurs for a specified duration.
///
/// # Example
///
/// ```
/// use std::time::Duration;
/// use solverforge_solver::termination::UnimprovedTimeTermination;
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
/// let term = UnimprovedTimeTermination::<MySolution>::seconds(5);
/// ```
pub struct UnimprovedTimeTermination<S: PlanningSolution> {
    limit: Duration,
    state: RefCell<UnimprovedTimeState<S::Score>>,
    _phantom: PhantomData<fn() -> S>,
}

impl<S: PlanningSolution> Debug for UnimprovedTimeTermination<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnimprovedTimeTermination")
            .field("limit", &self.limit)
            .finish()
    }
}

struct UnimprovedTimeState<Sc: Score> {
    last_best_score: Option<Sc>,
    last_improvement_time: Option<Instant>,
}

impl<Sc: Score> Default for UnimprovedTimeState<Sc> {
    fn default() -> Self {
        Self {
            last_best_score: None,
            last_improvement_time: None,
        }
    }
}

impl<S: PlanningSolution> UnimprovedTimeTermination<S> {
    pub fn new(limit: Duration) -> Self {
        Self {
            limit,
            state: RefCell::new(UnimprovedTimeState::default()),
            _phantom: PhantomData,
        }
    }

    pub fn millis(ms: u64) -> Self {
        Self::new(Duration::from_millis(ms))
    }

    pub fn seconds(secs: u64) -> Self {
        Self::new(Duration::from_secs(secs))
    }
}

unsafe impl<S: PlanningSolution> Send for UnimprovedTimeTermination<S> {}

impl<S: PlanningSolution, D: ScoreDirector<S>> Termination<S, D>
    for UnimprovedTimeTermination<S>
{
    fn is_terminated(&self, solver_scope: &SolverScope<S, D>) -> bool {
        let mut state = self.state.borrow_mut();
        let current_best = solver_scope.best_score();
        let now = Instant::now();

        match (&state.last_best_score, current_best) {
            (None, Some(score)) => {
                state.last_best_score = Some(score.clone());
                state.last_improvement_time = Some(now);
                false
            }
            (Some(last), Some(current)) => {
                if *current > *last {
                    state.last_best_score = Some(current.clone());
                    state.last_improvement_time = Some(now);
                    false
                } else {
                    state
                        .last_improvement_time
                        .map(|t| now.duration_since(t) >= self.limit)
                        .unwrap_or(false)
                }
            }
            (Some(_), None) => state
                .last_improvement_time
                .map(|t| now.duration_since(t) >= self.limit)
                .unwrap_or(false),
            (None, None) => false,
        }
    }
}
