//! Step-level scope.

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::PhaseScope;

/// Scope for a single step within a phase.
pub struct StepScope<'a, 'b, S: PlanningSolution> {
    /// Reference to the parent phase scope.
    phase_scope: &'a mut PhaseScope<'b, S>,
    /// Index of this step within the phase (0-based).
    step_index: u64,
    /// Score after this step.
    step_score: Option<S::Score>,
}

impl<'a, 'b, S: PlanningSolution> StepScope<'a, 'b, S> {
    /// Creates a new step scope.
    pub fn new(phase_scope: &'a mut PhaseScope<'b, S>) -> Self {
        let step_index = phase_scope.step_count();
        Self {
            phase_scope,
            step_index,
            step_score: None,
        }
    }

    /// Returns the step index within the phase.
    pub fn step_index(&self) -> u64 {
        self.step_index
    }

    /// Returns the step score.
    pub fn step_score(&self) -> Option<&S::Score> {
        self.step_score.as_ref()
    }

    /// Sets the step score.
    pub fn set_step_score(&mut self, score: S::Score) {
        self.step_score = Some(score);
    }

    /// Marks this step as complete and increments counters.
    pub fn complete(&mut self) {
        self.phase_scope.increment_step_count();
    }

    /// Returns a reference to the phase scope.
    pub fn phase_scope(&self) -> &PhaseScope<'b, S> {
        self.phase_scope
    }

    /// Returns a mutable reference to the phase scope.
    pub fn phase_scope_mut(&mut self) -> &mut PhaseScope<'b, S> {
        self.phase_scope
    }

    /// Convenience: returns the score director.
    pub fn score_director(&self) -> &dyn ScoreDirector<S> {
        self.phase_scope.score_director()
    }

    /// Convenience: returns a mutable score director.
    pub fn score_director_mut(&mut self) -> &mut dyn ScoreDirector<S> {
        self.phase_scope.score_director_mut()
    }

    /// Convenience: calculates the current score.
    pub fn calculate_score(&mut self) -> S::Score {
        self.phase_scope.calculate_score()
    }
}
