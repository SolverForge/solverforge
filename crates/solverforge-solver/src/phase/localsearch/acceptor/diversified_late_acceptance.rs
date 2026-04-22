// Diversified late acceptance acceptor.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::Score;

use super::Acceptor;
use crate::heuristic::r#move::MoveTabuSignature;

/// Diversified late acceptance acceptor - combines late acceptance with best score tracking.
///
/// Extends [`LateAcceptanceAcceptor`] by also tracking the best score found.
/// Accepts a move if it:
/// 1. Improves the last step score (always accepted), OR
/// 2. Is at least as good as the score from N steps ago, OR
/// 3. Is within a tolerance of the best score found so far
///
/// The third condition allows escaping from local optima by accepting
/// moves that don't regress too far from the best known solution.
///
/// # Example
///
/// ```
/// use solverforge_solver::phase::localsearch::DiversifiedLateAcceptanceAcceptor;
/// use solverforge_core::score::SoftScore;
/// use solverforge_core::domain::PlanningSolution;
///
/// #[derive(Clone)]
/// struct MySolution;
/// impl PlanningSolution for MySolution {
///     type Score = SoftScore;
///     fn score(&self) -> Option<Self::Score> { None }
///     fn set_score(&mut self, _: Option<Self::Score>) {}
/// }
///
/// // Accept if better than 400-step-old score OR within 5% of best
/// let acceptor = DiversifiedLateAcceptanceAcceptor::<MySolution>::new(400, 0.05);
/// ```
pub struct DiversifiedLateAcceptanceAcceptor<S: PlanningSolution> {
    // Size of the late acceptance list.
    late_acceptance_size: usize,
    // Circular buffer of historical scores.
    score_history: Vec<Option<S::Score>>,
    // Current index in the buffer.
    current_index: usize,
    // Best score found so far in this phase.
    best_score: Option<S::Score>,
    // Tolerance as a fraction (0.05 = 5% worse than best is acceptable).
    tolerance: f64,
}

impl<S: PlanningSolution> Debug for DiversifiedLateAcceptanceAcceptor<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiversifiedLateAcceptanceAcceptor")
            .field("late_acceptance_size", &self.late_acceptance_size)
            .field("current_index", &self.current_index)
            .field("tolerance", &self.tolerance)
            .finish()
    }
}

impl<S: PlanningSolution> Clone for DiversifiedLateAcceptanceAcceptor<S> {
    fn clone(&self) -> Self {
        Self {
            late_acceptance_size: self.late_acceptance_size,
            score_history: self.score_history.clone(),
            current_index: self.current_index,
            best_score: self.best_score,
            tolerance: self.tolerance,
        }
    }
}

impl<S: PlanningSolution> DiversifiedLateAcceptanceAcceptor<S> {
    /// Creates a new diversified late acceptance acceptor.
    ///
    /// # Arguments
    /// * `late_acceptance_size` - Number of historical scores to keep. Must be > 0.
    /// * `tolerance` - Fraction of best score to accept (0.05 = 5% tolerance)
    ///
    /// # Panics
    ///
    /// Panics if `late_acceptance_size` is 0.
    pub fn new(late_acceptance_size: usize, tolerance: f64) -> Self {
        assert!(
            late_acceptance_size > 0,
            "late_acceptance_size must be > 0, got 0"
        );
        Self {
            late_acceptance_size,
            score_history: vec![None; late_acceptance_size],
            current_index: 0,
            best_score: None,
            tolerance,
        }
    }

    /// Creates with default tolerance of 0.01 (1%).
    pub fn with_default_tolerance(late_acceptance_size: usize) -> Self {
        Self::new(late_acceptance_size, 0.01)
    }
}

impl<S: PlanningSolution> Default for DiversifiedLateAcceptanceAcceptor<S> {
    fn default() -> Self {
        Self::new(400, 0.01)
    }
}

impl<S: PlanningSolution> Acceptor<S> for DiversifiedLateAcceptanceAcceptor<S> {
    fn is_accepted(
        &mut self,
        last_step_score: &S::Score,
        move_score: &S::Score,
        _move_signature: Option<&MoveTabuSignature>,
    ) -> bool {
        // Accept non-worsening moves (consistent with LateAcceptanceAcceptor which uses >=)
        if move_score >= last_step_score {
            return true;
        }

        // Accept if better than or equal to the late score
        if let Some(late_score) = &self.score_history[self.current_index] {
            if move_score >= late_score {
                return true;
            }
        } else {
            // No history yet, accept
            return true;
        }

        // Diversification: accept if within tolerance of best score
        if let Some(best) = &self.best_score {
            // Calculate threshold: best - tolerance * |best|
            let abs_best = best.abs();
            let threshold = *best - abs_best.multiply(self.tolerance);
            if move_score >= &threshold {
                return true;
            }
        }

        false
    }

    fn phase_started(&mut self, initial_score: &S::Score) {
        // Initialize history with the initial score
        for slot in &mut self.score_history {
            *slot = Some(*initial_score);
        }
        self.current_index = 0;
        self.best_score = Some(*initial_score);
    }

    fn step_ended(
        &mut self,
        step_score: &S::Score,
        _accepted_move_signature: Option<&MoveTabuSignature>,
    ) {
        // Update best score if improved
        if let Some(best) = &self.best_score {
            if step_score > best {
                self.best_score = Some(*step_score);
            }
        } else {
            self.best_score = Some(*step_score);
        }

        // Record the step score in the history
        self.score_history[self.current_index] = Some(*step_score);
        self.current_index = (self.current_index + 1) % self.late_acceptance_size;
    }
}

#[cfg(test)]
#[path = "diversified_late_acceptance_tests.rs"]
mod tests;
