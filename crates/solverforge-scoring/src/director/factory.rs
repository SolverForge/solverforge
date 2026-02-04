//! Score director factory for creating score directors.

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};

use super::simple::SimpleScoreDirector;

/// Factory for creating score directors (zero-erasure).
///
/// The calculator function is stored as a concrete generic type parameter,
/// not as `Arc<dyn Fn>`.
pub struct ScoreDirectorFactory<S: PlanningSolution, C> {
    solution_descriptor: SolutionDescriptor,
    score_calculator: C,
    _phantom: std::marker::PhantomData<S>,
}

impl<S, C> ScoreDirectorFactory<S, C>
where
    S: PlanningSolution,
    C: Fn(&S) -> S::Score + Send + Sync,
{
    /// Creates a new ScoreDirectorFactory.
    pub fn new(solution_descriptor: SolutionDescriptor, score_calculator: C) -> Self {
        Self {
            solution_descriptor,
            score_calculator,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Creates a new score director for the given solution.
    pub fn build_score_director(&self, solution: S) -> SimpleScoreDirector<S, &C> {
        SimpleScoreDirector::new(
            solution,
            self.solution_descriptor.clone(),
            &self.score_calculator,
        )
    }

    /// Returns a reference to the solution descriptor.
    pub fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.solution_descriptor
    }
}

impl<S: PlanningSolution, C: Clone> Clone for ScoreDirectorFactory<S, C> {
    fn clone(&self) -> Self {
        Self {
            solution_descriptor: self.solution_descriptor.clone(),
            score_calculator: self.score_calculator.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}
