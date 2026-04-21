// Step-level scope.

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::{Director, RecordingDirector};

use crate::heuristic::r#move::Move;

use super::solver::ProgressCallback;
use super::PhaseScope;

/// Scope for a single step within a phase.
///
/// # Type Parameters
/// * `'t` - Lifetime of the termination flag
/// * `'a` - Lifetime of the phase scope reference
/// * `'b` - Lifetime of the solver scope reference
/// * `S` - The planning solution type
/// * `D` - The score director type
/// * `BestCb` - The best-solution callback type
pub struct StepScope<'t, 'a, 'b, S: PlanningSolution, D: Director<S>, BestCb = ()> {
    // Reference to the parent phase scope.
    phase_scope: &'a mut PhaseScope<'t, 'b, S, D, BestCb>,
    // Index of this step within the phase (0-based).
    step_index: u64,
    // Score after this step.
    step_score: Option<S::Score>,
}

impl<'t, 'a, 'b, S: PlanningSolution, D: Director<S>, BestCb: ProgressCallback<S>>
    StepScope<'t, 'a, 'b, S, D, BestCb>
{
    pub fn new(phase_scope: &'a mut PhaseScope<'t, 'b, S, D, BestCb>) -> Self {
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

    pub fn set_step_score(&mut self, score: S::Score)
    where
        S::Score: Copy,
    {
        self.phase_scope.solver_scope_mut().set_current_score(score);
        self.step_score = Some(score);
    }

    /// Marks this step as complete and increments counters.
    pub fn complete(&mut self) {
        self.phase_scope.increment_step_count();
        self.phase_scope.solver_scope_mut().pause_if_requested();
    }

    pub fn phase_scope(&self) -> &PhaseScope<'t, 'b, S, D, BestCb> {
        self.phase_scope
    }

    pub fn phase_scope_mut(&mut self) -> &mut PhaseScope<'t, 'b, S, D, BestCb> {
        self.phase_scope
    }

    pub fn score_director(&self) -> &D {
        self.phase_scope.score_director()
    }

    pub(crate) fn score_director_mut(&mut self) -> &mut D {
        self.phase_scope.score_director_mut()
    }

    pub fn trial<T, F>(&mut self, trial: F) -> T
    where
        F: FnOnce(&mut RecordingDirector<'_, S, D>) -> T,
    {
        self.phase_scope.trial(trial)
    }

    pub fn mutate<T, F>(&mut self, mutate: F) -> T
    where
        F: FnOnce(&mut D) -> T,
    {
        self.phase_scope.mutate(mutate)
    }

    pub(crate) fn apply_committed_move<M>(&mut self, mov: &M)
    where
        M: Move<S>,
    {
        self.phase_scope
            .solver_scope_mut()
            .apply_committed_move(mov);
    }

    pub(crate) fn apply_committed_change<F>(&mut self, change: F)
    where
        F: FnOnce(&mut D),
    {
        self.phase_scope
            .solver_scope_mut()
            .apply_committed_change(change);
    }

    /// Calculates the current score.
    pub fn calculate_score(&mut self) -> S::Score {
        self.phase_scope.calculate_score()
    }
}
