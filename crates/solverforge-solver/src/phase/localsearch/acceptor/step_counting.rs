//! Step Counting Hill Climbing acceptor.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;

use super::Acceptor;

/// Step Counting Hill Climbing acceptor - allows limited non-improving moves.
///
/// This acceptor combines hill climbing with a step limit that resets whenever
/// an improving move is made. It accepts:
/// 1. Any improving move (resets the step counter)
/// 2. Non-improving moves if step count since last improvement is below threshold
///
/// This enables exploration while still requiring eventual improvement.
///
/// # Example
///
/// ```
/// use solverforge_solver::phase::localsearch::StepCountingHillClimbingAcceptor;
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
/// // Allow up to 100 non-improving steps before requiring improvement
/// let acceptor = StepCountingHillClimbingAcceptor::<MySolution>::new(100);
/// ```
pub struct StepCountingHillClimbingAcceptor<S: PlanningSolution> {
    /// Maximum steps allowed without improvement.
    step_count_limit: u64,
    /// Current steps since last improvement.
    steps_since_improvement: u64,
    /// Best score seen so far.
    best_score: Option<S::Score>,
}

impl<S: PlanningSolution> Debug for StepCountingHillClimbingAcceptor<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StepCountingHillClimbingAcceptor")
            .field("step_count_limit", &self.step_count_limit)
            .field("steps_since_improvement", &self.steps_since_improvement)
            .finish()
    }
}

impl<S: PlanningSolution> Clone for StepCountingHillClimbingAcceptor<S> {
    fn clone(&self) -> Self {
        Self {
            step_count_limit: self.step_count_limit,
            steps_since_improvement: self.steps_since_improvement,
            best_score: self.best_score,
        }
    }
}

impl<S: PlanningSolution> StepCountingHillClimbingAcceptor<S> {
    /// Creates a new Step Counting Hill Climbing acceptor.
    ///
    /// # Arguments
    /// * `step_count_limit` - Maximum steps allowed without finding improvement
    pub fn new(step_count_limit: u64) -> Self {
        Self {
            step_count_limit,
            steps_since_improvement: 0,
            best_score: None,
        }
    }
}

impl<S: PlanningSolution> Default for StepCountingHillClimbingAcceptor<S> {
    fn default() -> Self {
        Self::new(100)
    }
}

impl<S: PlanningSolution> Acceptor<S> for StepCountingHillClimbingAcceptor<S> {
    fn is_accepted(&self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
        // Always accept improving moves
        if move_score > last_step_score {
            return true;
        }

        // Accept non-improving moves if within step count limit
        self.steps_since_improvement < self.step_count_limit
    }

    fn phase_started(&mut self, initial_score: &S::Score) {
        self.best_score = Some(*initial_score);
        self.steps_since_improvement = 0;
    }

    fn step_ended(&mut self, step_score: &S::Score) {
        // Check if this step improved on the best score
        let improved = match &self.best_score {
            Some(best) => step_score > best,
            None => true,
        };

        if improved {
            self.best_score = Some(*step_score);
            self.steps_since_improvement = 0;
        } else {
            self.steps_since_improvement += 1;
        }
    }

    fn phase_ended(&mut self) {
        self.best_score = None;
        self.steps_since_improvement = 0;
    }
}
