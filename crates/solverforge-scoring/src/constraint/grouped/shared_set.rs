use std::marker::PhantomData;

use solverforge_core::score::Score;

use crate::api::analysis::ConstraintAnalysis;
use crate::api::constraint_set::{ConstraintMetadata, ConstraintResult, ConstraintSet};

use super::scorer_set::GroupedScorerSet;
use super::state::GroupedNodeState;
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::filter::UniFilter;

pub struct SharedGroupedConstraintSet<S, A, K, E, Fi, KF, C, V, R, Acc, Scorers, Sc>
where
    Acc: Accumulator<V, R>,
    Sc: Score,
{
    node_name: String,
    state: GroupedNodeState<S, A, K, E, Fi, KF, C, V, R, Acc>,
    scorers: Scorers,
    _phantom: PhantomData<fn() -> Sc>,
}

impl<S, A, K, E, Fi, KF, C, V, R, Acc, Scorers, Sc>
    SharedGroupedConstraintSet<S, A, K, E, Fi, KF, C, V, R, Acc, Scorers, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    K: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
    E: crate::stream::collection_extract::CollectionExtract<S, Item = A>,
    Fi: UniFilter<S, A>,
    KF: Fn(&A) -> K + Send + Sync,
    C: for<'i> Collector<&'i A, Value = V, Result = R, Accumulator = Acc> + Send + Sync + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    Scorers: GroupedScorerSet<K, R, Sc>,
    Sc: Score + 'static,
{
    pub fn new(
        node_name: impl Into<String>,
        state: GroupedNodeState<S, A, K, E, Fi, KF, C, V, R, Acc>,
        scorers: Scorers,
    ) -> Self {
        Self {
            node_name: node_name.into(),
            state,
            scorers,
            _phantom: PhantomData,
        }
    }

    pub fn state(&self) -> &GroupedNodeState<S, A, K, E, Fi, KF, C, V, R, Acc> {
        &self.state
    }
}

impl<S, A, K, E, Fi, KF, C, V, R, Acc, Scorers, Sc> ConstraintSet<S, Sc>
    for SharedGroupedConstraintSet<S, A, K, E, Fi, KF, C, V, R, Acc, Scorers, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    K: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
    E: crate::stream::collection_extract::CollectionExtract<S, Item = A>,
    Fi: UniFilter<S, A>,
    KF: Fn(&A) -> K + Send + Sync,
    C: for<'i> Collector<&'i A, Value = V, Result = R, Accumulator = Acc> + Send + Sync + 'static,
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
            .on_insert(solution, entity_index, descriptor_index, &self.node_name);
        let changed_keys = self.state.take_changed_keys();
        self.scorers
            .refresh_changed_keys(&self.state, &changed_keys)
    }

    fn on_retract_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        self.state
            .on_retract(solution, entity_index, descriptor_index, &self.node_name);
        let changed_keys = self.state.take_changed_keys();
        self.scorers
            .refresh_changed_keys(&self.state, &changed_keys)
    }

    fn reset_all(&mut self) {
        self.state.reset();
        self.scorers.reset();
    }
}
