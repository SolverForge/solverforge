//! Late acceptance acceptor.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;

use super::Acceptor;

/// Late acceptance acceptor - accepts moves that improve on a historical score.
///
/// Maintains a circular buffer of recent scores and accepts moves that
/// are better than a score from N steps ago.
///
/// # Example
///
/// ```
/// use solverforge_solver::phase::localsearch::LateAcceptanceAcceptor;
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
/// let acceptor = LateAcceptanceAcceptor::<MySolution>::new(400);
/// ```
pub struct LateAcceptanceAcceptor<S: PlanningSolution> {
    /// Size of the late acceptance list.
    late_acceptance_size: usize,
    /// Circular buffer of historical scores.
    score_history: Vec<Option<S::Score>>,
    /// Current index in the buffer.
    current_index: usize,
}

impl<S: PlanningSolution> Debug for LateAcceptanceAcceptor<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LateAcceptanceAcceptor")
            .field("late_acceptance_size", &self.late_acceptance_size)
            .field("current_index", &self.current_index)
            .finish()
    }
}

impl<S: PlanningSolution> Clone for LateAcceptanceAcceptor<S> {
    fn clone(&self) -> Self {
        Self {
            late_acceptance_size: self.late_acceptance_size,
            score_history: self.score_history.clone(),
            current_index: self.current_index,
        }
    }
}

impl<S: PlanningSolution> LateAcceptanceAcceptor<S> {
    /// Creates a new late acceptance acceptor.
    ///
    /// # Arguments
    /// * `late_acceptance_size` - Number of historical scores to keep
    pub fn new(late_acceptance_size: usize) -> Self {
        Self {
            late_acceptance_size,
            score_history: vec![None; late_acceptance_size],
            current_index: 0,
        }
    }
}

impl<S: PlanningSolution> Default for LateAcceptanceAcceptor<S> {
    fn default() -> Self {
        Self::new(400)
    }
}

impl<S: PlanningSolution> Acceptor<S> for LateAcceptanceAcceptor<S> {
    fn is_accepted(&self, last_step_score: &S::Score, move_score: &S::Score) -> bool {
        // Always accept improving moves
        if move_score > last_step_score {
            return true;
        }

        // Accept if better than the late score
        if let Some(late_score) = &self.score_history[self.current_index] {
            move_score >= late_score
        } else {
            // No history yet, accept
            true
        }
    }

    fn phase_started(&mut self, initial_score: &S::Score) {
        // Initialize history with the initial score
        for slot in &mut self.score_history {
            *slot = Some(initial_score.clone());
        }
        self.current_index = 0;
    }

    fn step_ended(&mut self, step_score: &S::Score) {
        // Record the step score in the history
        self.score_history[self.current_index] = Some(step_score.clone());
        self.current_index = (self.current_index + 1) % self.late_acceptance_size;
    }
}
