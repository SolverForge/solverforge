//! Simple score director with full recalculation.

use std::any::Any;

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};

use super::traits::ScoreDirector;

/// A simple score director that recalculates the full score each time (zero-erasure).
///
/// The calculator is stored as a concrete generic type parameter, not as `Arc<dyn Fn>`.
/// This is inefficient but correct - used for testing and simple problems.
pub struct SimpleScoreDirector<S: PlanningSolution, C> {
    working_solution: S,
    solution_descriptor: SolutionDescriptor,
    score_calculator: C,
    score_dirty: bool,
    cached_score: Option<S::Score>,
}

impl<S, C> SimpleScoreDirector<S, C>
where
    S: PlanningSolution,
    C: Fn(&S) -> S::Score + Send + Sync,
{
    /// Creates a new SimpleScoreDirector.
    pub fn new(solution: S, solution_descriptor: SolutionDescriptor, score_calculator: C) -> Self {
        SimpleScoreDirector {
            working_solution: solution,
            solution_descriptor,
            score_calculator,
            score_dirty: true,
            cached_score: None,
        }
    }

    /// Creates a SimpleScoreDirector with a simple closure.
    ///
    /// This is an alias for `new()` for backward compatibility.
    pub fn with_calculator(
        solution: S,
        solution_descriptor: SolutionDescriptor,
        calculator: C,
    ) -> Self {
        Self::new(solution, solution_descriptor, calculator)
    }

    fn mark_dirty(&mut self) {
        self.score_dirty = true;
    }
}

impl<S, C> ScoreDirector<S> for SimpleScoreDirector<S, C>
where
    S: PlanningSolution,
    C: Fn(&S) -> S::Score + Send + Sync,
{
    fn working_solution(&self) -> &S {
        &self.working_solution
    }

    fn working_solution_mut(&mut self) -> &mut S {
        self.mark_dirty();
        &mut self.working_solution
    }

    fn calculate_score(&mut self) -> S::Score {
        if !self.score_dirty {
            if let Some(ref score) = self.cached_score {
                return *score;
            }
        }

        let score = (self.score_calculator)(&self.working_solution);
        self.working_solution.set_score(Some(score));
        self.cached_score = Some(score);
        self.score_dirty = false;
        score
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.solution_descriptor
    }

    fn clone_working_solution(&self) -> S {
        self.working_solution.clone()
    }

    fn before_variable_changed(
        &mut self,
        _descriptor_index: usize,
        _entity_index: usize,
        _variable_name: &str,
    ) {
        self.mark_dirty();
    }

    fn after_variable_changed(
        &mut self,
        _descriptor_index: usize,
        _entity_index: usize,
        _variable_name: &str,
    ) {
        // Already marked dirty in before_variable_changed
    }

    fn trigger_variable_listeners(&mut self) {
        // No shadow variables in simple score director
    }

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        self.solution_descriptor
            .entity_descriptors
            .get(descriptor_index)?
            .entity_count(&self.working_solution as &dyn Any)
    }

    fn total_entity_count(&self) -> Option<usize> {
        self.solution_descriptor
            .total_entity_count(&self.working_solution as &dyn Any)
    }

    fn get_entity(&self, descriptor_index: usize, entity_index: usize) -> Option<&dyn Any> {
        self.solution_descriptor.get_entity(
            &self.working_solution as &dyn Any,
            descriptor_index,
            entity_index,
        )
    }

    fn is_incremental(&self) -> bool {
        false
    }

    fn reset(&mut self) {
        self.mark_dirty();
        self.cached_score = None;
    }
}
