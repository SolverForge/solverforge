// Diversified Late Acceptance Search with exact worst-score occurrence tracking.
// Acceptance and ring updates advance for each evaluated candidate.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::Score;

use super::Acceptor;
use crate::heuristic::r#move::MoveTabuSignature;

pub struct DiversifiedLateAcceptanceAcceptor<S: PlanningSolution> {
    late_acceptance_size: usize,
    previous_scores: Vec<S::Score>,
    late_score_index: usize,
    late_worse_score: S::Score,
    late_worse_occurrences: usize,
}

impl<S: PlanningSolution> Debug for DiversifiedLateAcceptanceAcceptor<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiversifiedLateAcceptanceAcceptor")
            .field("late_acceptance_size", &self.late_acceptance_size)
            .field("late_score_index", &self.late_score_index)
            .field("late_worse_occurrences", &self.late_worse_occurrences)
            .finish()
    }
}

impl<S: PlanningSolution> Clone for DiversifiedLateAcceptanceAcceptor<S> {
    fn clone(&self) -> Self {
        Self {
            late_acceptance_size: self.late_acceptance_size,
            previous_scores: self.previous_scores.clone(),
            late_score_index: self.late_score_index,
            late_worse_score: self.late_worse_score,
            late_worse_occurrences: self.late_worse_occurrences,
        }
    }
}

impl<S: PlanningSolution> DiversifiedLateAcceptanceAcceptor<S> {
    pub fn new(late_acceptance_size: usize) -> Self {
        assert!(
            late_acceptance_size > 0,
            "late_acceptance_size must be > 0, got 0"
        );
        Self {
            late_acceptance_size,
            previous_scores: Vec::new(),
            late_score_index: 0,
            late_worse_score: S::Score::zero(),
            late_worse_occurrences: late_acceptance_size,
        }
    }
}

impl<S: PlanningSolution> Default for DiversifiedLateAcceptanceAcceptor<S> {
    fn default() -> Self {
        Self::new(400)
    }
}

impl<S: PlanningSolution> Acceptor<S> for DiversifiedLateAcceptanceAcceptor<S> {
    fn is_accepted(
        &mut self,
        last_step_score: &S::Score,
        move_score: &S::Score,
        _move_signature: Option<&MoveTabuSignature>,
    ) -> bool {
        if self.previous_scores.is_empty() {
            // phase_started has not run; defer to the plain late-acceptance
            // behavior so uninitialized use cannot panic.
            return *move_score >= *last_step_score;
        }

        // accept = (move == current) || (move > lateWorse)
        let accepted = move_score == last_step_score || *move_score > self.late_worse_score;
        let current = if accepted {
            *move_score
        } else {
            *last_step_score
        };

        let late_score = self.previous_scores[self.late_score_index];
        let current_score_worse = current < late_score;
        let current_score_better = current > late_score && current > *last_step_score;
        if current_score_worse || current_score_better {
            self.update_late_score(current);
        }
        self.late_score_index = (self.late_score_index + 1) % self.late_acceptance_size;
        accepted
    }

    fn phase_started(&mut self, initial_score: &S::Score) {
        self.previous_scores = vec![*initial_score; self.late_acceptance_size];
        self.late_score_index = 0;
        self.late_worse_occurrences = self.late_acceptance_size;
        self.late_worse_score = *initial_score;
    }

    fn step_ended(
        &mut self,
        _step_score: &S::Score,
        _accepted_move_signature: Option<&MoveTabuSignature>,
    ) {
    }
}

impl<S: PlanningSolution> DiversifiedLateAcceptanceAcceptor<S> {
    fn update_late_score(&mut self, new_score: S::Score) {
        let slot_score = self.previous_scores[self.late_score_index];
        let slot_equal_to_worse = slot_score == self.late_worse_score;

        match new_score.cmp(&self.late_worse_score) {
            std::cmp::Ordering::Less => {
                self.late_worse_score = new_score;
                self.late_worse_occurrences = 1;
            }
            std::cmp::Ordering::Equal => {
                if !slot_equal_to_worse {
                    self.late_worse_occurrences += 1;
                }
            }
            std::cmp::Ordering::Greater => {
                if slot_equal_to_worse {
                    self.late_worse_occurrences -= 1;
                }
            }
        }
        self.previous_scores[self.late_score_index] = new_score;

        if self.late_worse_occurrences == 0 {
            let mut worse = self.previous_scores[0];
            let mut occurrences = 1;
            for score in self.previous_scores.iter().skip(1) {
                match (*score).cmp(&worse) {
                    std::cmp::Ordering::Less => {
                        worse = *score;
                        occurrences = 1;
                    }
                    std::cmp::Ordering::Equal => occurrences += 1,
                    std::cmp::Ordering::Greater => {}
                }
            }
            self.late_worse_score = worse;
            self.late_worse_occurrences = occurrences;
        }
    }
}

#[cfg(test)]
mod tests;
