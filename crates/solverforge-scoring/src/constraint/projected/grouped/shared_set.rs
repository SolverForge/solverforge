use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;

use crate::api::analysis::ConstraintAnalysis;
use crate::api::constraint_set::{ConstraintMetadata, ConstraintResult, ConstraintSet};
use crate::constraint::grouped::GroupedScorerSet;
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::filter::UniFilter;
use crate::stream::ProjectedSource;

use super::state::ProjectedGroupedNodeState;

pub struct SharedProjectedGroupedConstraintSet<S, Out, K, Src, F, KF, C, V, R, Acc, Scorers, Sc>
where
    Src: ProjectedSource<S, Out>,
    Acc: Accumulator<V, R>,
    Sc: Score,
{
    state: ProjectedGroupedNodeState<S, Out, K, Src, F, KF, C, V, R, Acc>,
    scorers: Scorers,
    _phantom: PhantomData<fn() -> Sc>,
}

impl<S, Out, K, Src, F, KF, C, V, R, Acc, Scorers, Sc>
    SharedProjectedGroupedConstraintSet<S, Out, K, Src, F, KF, C, V, R, Acc, Scorers, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    F: UniFilter<S, Out>,
    KF: Fn(&Out) -> K + Send + Sync,
    C: for<'i> Collector<&'i Out, Value = V, Result = R, Accumulator = Acc> + Send + Sync + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    Scorers: GroupedScorerSet<K, R, Sc>,
    Sc: Score + 'static,
{
    pub fn new(
        state: ProjectedGroupedNodeState<S, Out, K, Src, F, KF, C, V, R, Acc>,
        scorers: Scorers,
    ) -> Self {
        Self {
            state,
            scorers,
            _phantom: PhantomData,
        }
    }

    pub fn state(&self) -> &ProjectedGroupedNodeState<S, Out, K, Src, F, KF, C, V, R, Acc> {
        &self.state
    }
}

impl<S, Out, K, Src, F, KF, C, V, R, Acc, Scorers, Sc> ConstraintSet<S, Sc>
    for SharedProjectedGroupedConstraintSet<S, Out, K, Src, F, KF, C, V, R, Acc, Scorers, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    F: UniFilter<S, Out>,
    KF: Fn(&Out) -> K + Send + Sync,
    C: for<'i> Collector<&'i Out, Value = V, Result = R, Accumulator = Acc> + Send + Sync + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    Scorers: GroupedScorerSet<K, R, Sc>,
    Sc: Score + 'static,
{
    fn evaluate_all(&self, solution: &S) -> Sc {
        let state = self.state.evaluation_state(solution);
        self.scorers.evaluate(&state)
    }

    fn constraint_count(&self) -> usize {
        self.scorers.constraint_count()
    }

    fn constraint_metadata(&self) -> Vec<ConstraintMetadata<'_>> {
        self.scorers.constraint_metadata()
    }

    fn evaluate_each<'a>(&'a self, solution: &S) -> Vec<ConstraintResult<'a, Sc>> {
        let state = self.state.evaluation_state(solution);
        self.scorers.evaluate_each(&state)
    }

    fn evaluate_detailed<'a>(&'a self, solution: &S) -> Vec<ConstraintAnalysis<'a, Sc>> {
        let state = self.state.evaluation_state(solution);
        self.scorers.evaluate_detailed(&state)
    }

    fn initialize_all(&mut self, solution: &S) -> Sc {
        self.state.initialize(solution);
        self.scorers.initialize(&self.state)
    }

    fn on_insert_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        self.state
            .on_insert(solution, entity_index, descriptor_index);
        let changed_keys = self.state.take_changed_keys();
        self.scorers
            .refresh_changed_keys(&self.state, &changed_keys)
    }

    fn on_retract_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        self.state
            .on_retract(solution, entity_index, descriptor_index);
        let changed_keys = self.state.take_changed_keys();
        self.scorers
            .refresh_changed_keys(&self.state, &changed_keys)
    }

    fn reset_all(&mut self) {
        self.state.reset();
        self.scorers.reset();
    }
}
