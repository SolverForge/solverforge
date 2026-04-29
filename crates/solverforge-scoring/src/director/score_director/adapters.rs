use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::Score;

use super::typed::ScoreDirector;
use crate::api::constraint_set::{ConstraintMetadata, ConstraintSet};
use crate::director::Director;

impl<S, C> std::fmt::Debug for ScoreDirector<S, C>
where
    S: PlanningSolution + std::fmt::Debug,
    S::Score: std::fmt::Debug,
    C: ConstraintSet<S, S::Score>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScoreDirector")
            .field("initialized", &self.is_initialized())
            .field("cached_score", &self.get_score())
            .field("constraint_count", &self.constraint_count())
            .finish()
    }
}

impl<S, C> Director<S> for ScoreDirector<S, C>
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score> + Send,
{
    fn working_solution(&self) -> &S {
        self.working_solution()
    }

    fn working_solution_mut(&mut self) -> &mut S {
        self.working_solution_mut()
    }

    fn calculate_score(&mut self) -> S::Score {
        self.calculate_score_impl()
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        &self.solution_descriptor
    }

    fn clone_working_solution(&self) -> S {
        self.clone_working_solution_impl()
    }

    fn before_variable_changed(&mut self, descriptor_index: usize, entity_index: usize) {
        self.before_variable_changed_impl(descriptor_index, entity_index);
    }

    fn after_variable_changed(&mut self, descriptor_index: usize, entity_index: usize) {
        self.after_variable_changed_impl(descriptor_index, entity_index);
    }

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        Some((self.entity_counter)(
            &self.working_solution,
            descriptor_index,
        ))
    }

    fn total_entity_count(&self) -> Option<usize> {
        let count: usize = (0..self.solution_descriptor.entity_descriptors.len())
            .map(|i| (self.entity_counter)(&self.working_solution, i))
            .sum();
        Some(count)
    }

    fn constraint_metadata(&self) -> &[ConstraintMetadata] {
        self.constraint_metadata()
    }

    fn is_incremental(&self) -> bool {
        true
    }

    fn snapshot_score_state(&self) -> crate::director::DirectorScoreState<S::Score> {
        self.snapshot_score_state_impl()
    }

    fn restore_score_state(&mut self, state: crate::director::DirectorScoreState<S::Score>) {
        self.restore_score_state_impl(state);
    }

    fn reset(&mut self) {
        self.reset_impl();
    }
}
