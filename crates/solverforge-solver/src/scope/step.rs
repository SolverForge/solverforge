//! Step-level scope.

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::PhaseScope;

/// Scope for a single step within a phase.
///
/// # Type Parameters
/// * `'t` - Lifetime of the termination flag
/// * `'a` - Lifetime of the phase scope reference
/// * `'b` - Lifetime of the solver scope reference
/// * `S` - The planning solution type
/// * `D` - The score director type
pub struct StepScope<'t, 'a, 'b, S: PlanningSolution, D: ScoreDirector<S>> {
    /// Reference to the parent phase scope.
    phase_scope: &'a mut PhaseScope<'t, 'b, S, D>,
    /// Index of this step within the phase (0-based).
    step_index: u64,
    /// Score after this step.
    step_score: Option<S::Score>,
}

impl<'t, 'a, 'b, S: PlanningSolution, D: ScoreDirector<S>> StepScope<'t, 'a, 'b, S, D> {
    /// Creates a new step scope.
    pub fn new(phase_scope: &'a mut PhaseScope<'t, 'b, S, D>) -> Self {
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
    pub fn phase_scope(&self) -> &PhaseScope<'t, 'b, S, D> {
        self.phase_scope
    }

    /// Returns a mutable reference to the phase scope.
    pub fn phase_scope_mut(&mut self) -> &mut PhaseScope<'t, 'b, S, D> {
        self.phase_scope
    }

    /// Returns a reference to the score director.
    pub fn score_director(&self) -> &D {
        self.phase_scope.score_director()
    }

    /// Returns a mutable reference to the score director.
    pub fn score_director_mut(&mut self) -> &mut D {
        self.phase_scope.score_director_mut()
    }

    /// Calculates the current score.
    pub fn calculate_score(&mut self) -> S::Score {
        self.phase_scope.calculate_score()
    }
}
