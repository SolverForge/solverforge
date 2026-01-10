//! Step-level scope.

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::PhaseScope;

/// Scope for a single step within a phase.
pub struct StepScope<'a, 'b, S: PlanningSolution, D: ScoreDirector<S>> {
    phase_scope: &'a mut PhaseScope<'b, S, D>,
    step_index: u64,
    step_score: Option<S::Score>,
}

impl<'a, 'b, S: PlanningSolution, D: ScoreDirector<S>> StepScope<'a, 'b, S, D> {
    pub fn new(phase_scope: &'a mut PhaseScope<'b, S, D>) -> Self {
        let step_index = phase_scope.step_count();
        Self {
            phase_scope,
            step_index,
            step_score: None,
        }
    }

    pub fn step_index(&self) -> u64 {
        self.step_index
    }

    pub fn step_score(&self) -> Option<&S::Score> {
        self.step_score.as_ref()
    }

    pub fn set_step_score(&mut self, score: S::Score) {
        self.step_score = Some(score);
    }

    pub fn complete(&mut self) {
        self.phase_scope.increment_step_count();
    }

    pub fn phase_scope(&self) -> &PhaseScope<'b, S, D> {
        self.phase_scope
    }

    pub fn phase_scope_mut(&mut self) -> &mut PhaseScope<'b, S, D> {
        self.phase_scope
    }

    pub fn score_director(&self) -> &D {
        self.phase_scope.score_director()
    }

    pub fn score_director_mut(&mut self) -> &mut D {
        self.phase_scope.score_director_mut()
    }

    pub fn calculate_score(&mut self) -> S::Score {
        self.phase_scope.calculate_score()
    }
}
