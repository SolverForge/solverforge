//! Late acceptance acceptor.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;

use super::Acceptor;
use crate::heuristic::r#move::MoveTabuSignature;

/// Late acceptance acceptor - accepts moves that improve on a historical score.
///
/// Maintains a circular buffer of recent scores and accepts moves that
/// are better than a score from N steps ago.
///
pub(crate) struct LateAcceptanceAcceptor<S: PlanningSolution> {
    // Size of the late acceptance list.
    late_acceptance_size: usize,
    // Circular buffer of historical scores.
    score_history: Vec<Option<S::Score>>,
    // Current index in the buffer.
    current_index: usize,
    // Best score seen so far in this phase; the buffer resets to it whenever a
    // new best is committed, re-anchoring the acceptance window.
    best_score: Option<S::Score>,
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
            best_score: self.best_score,
        }
    }
}

impl<S: PlanningSolution> LateAcceptanceAcceptor<S> {
    /// Creates a new late acceptance acceptor.
    ///
    /// # Arguments
    /// * `late_acceptance_size` - Number of historical scores to keep. Must be > 0.
    ///
    /// # Panics
    ///
    /// Panics if `late_acceptance_size` is 0.
    pub(crate) fn new(late_acceptance_size: usize) -> Self {
        assert!(
            late_acceptance_size > 0,
            "late_acceptance_size must be > 0, got 0"
        );
        Self {
            late_acceptance_size,
            score_history: vec![None; late_acceptance_size],
            current_index: 0,
            best_score: None,
        }
    }
}

impl<S: PlanningSolution> Default for LateAcceptanceAcceptor<S> {
    fn default() -> Self {
        Self::new(400)
    }
}

impl<S: PlanningSolution> Acceptor<S> for LateAcceptanceAcceptor<S> {
    fn is_accepted(
        &mut self,
        last_step_score: &S::Score,
        move_score: &S::Score,
        _move_signature: Option<&MoveTabuSignature>,
    ) -> bool {
        // Always accept improving or equal moves
        if move_score >= last_step_score {
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
        self.score_history.fill(Some(*initial_score));
        self.current_index = 0;
        self.best_score = Some(*initial_score);
    }

    fn step_ended(
        &mut self,
        step_score: &S::Score,
        _accepted_move_signature: Option<MoveTabuSignature>,
    ) {
        // Record the step score in the history
        self.score_history[self.current_index] = Some(*step_score);
        self.current_index = (self.current_index + 1) % self.late_acceptance_size;

        // Re-anchor the whole window on a new best so the acceptance bar does
        // not still quote pre-improvement scores.
        let new_best = self.best_score.is_none_or(|best| step_score > &best);
        if new_best {
            self.best_score = Some(*step_score);
            self.score_history.fill(Some(*step_score));
        }
    }
}
