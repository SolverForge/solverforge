// Tests for the move module.

use super::*;
use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::{Director, ScoreDirector};
use std::any::TypeId;
use std::ptr::NonNull;

struct SnapshotDirector<S, D>
where
    S: PlanningSolution,
    D: Director<S>,
{
    inner: NonNull<D>,
    snapshot: S,
    score_state: solverforge_scoring::DirectorScoreState<S::Score>,
}

unsafe impl<S, D> Send for SnapshotDirector<S, D>
where
    S: PlanningSolution,
    D: Director<S>,
{
}

impl<S, D> SnapshotDirector<S, D>
where
    S: PlanningSolution,
    D: Director<S>,
{
    fn new(inner: &mut D) -> Self {
        Self {
            snapshot: inner.clone_working_solution(),
            score_state: inner.snapshot_score_state(),
            inner: NonNull::from(inner),
        }
    }

    fn undo_changes(&mut self) {
        *self.inner_mut().working_solution_mut() = self.snapshot.clone();
        let current_score_state = self.inner_ref().snapshot_score_state();
        let score_state = std::mem::replace(&mut self.score_state, current_score_state);
        self.inner_mut().restore_score_state(score_state);
    }

    fn inner_ref(&self) -> &D {
        unsafe { self.inner.as_ref() }
    }

    fn inner_mut(&mut self) -> &mut D {
        unsafe { self.inner.as_mut() }
    }
}

impl<S, D> Director<S> for SnapshotDirector<S, D>
where
    S: PlanningSolution,
    D: Director<S>,
{
    fn working_solution(&self) -> &S {
        self.inner_ref().working_solution()
    }

    fn working_solution_mut(&mut self) -> &mut S {
        self.inner_mut().working_solution_mut()
    }

    fn calculate_score(&mut self) -> S::Score {
        self.inner_mut().calculate_score()
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        self.inner_ref().solution_descriptor()
    }

    fn clone_working_solution(&self) -> S {
        self.inner_ref().clone_working_solution()
    }

    fn before_variable_changed(&mut self, descriptor_index: usize, entity_index: usize) {
        self.inner_mut()
            .before_variable_changed(descriptor_index, entity_index);
    }

    fn after_variable_changed(&mut self, descriptor_index: usize, entity_index: usize) {
        self.inner_mut()
            .after_variable_changed(descriptor_index, entity_index);
    }

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        self.inner_ref().entity_count(descriptor_index)
    }

    fn total_entity_count(&self) -> Option<usize> {
        self.inner_ref().total_entity_count()
    }

    fn constraint_metadata(&self) -> Vec<solverforge_scoring::ConstraintMetadata<'_>> {
        self.inner_ref().constraint_metadata()
    }

    fn is_incremental(&self) -> bool {
        self.inner_ref().is_incremental()
    }

    fn snapshot_score_state(&self) -> solverforge_scoring::DirectorScoreState<S::Score> {
        self.inner_ref().snapshot_score_state()
    }

    fn restore_score_state(&mut self, state: solverforge_scoring::DirectorScoreState<S::Score>) {
        self.inner_mut().restore_score_state(state);
    }
}

mod arena;
mod change;
mod compound_scalar;
mod conflict_repair;
mod k_opt;
mod list_change;
mod list_reverse;
mod list_ruin;
mod list_swap;
mod pillar_change;
mod pillar_swap;
mod ruin;
mod sublist_change;
mod sublist_swap;
mod swap;
