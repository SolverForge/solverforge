use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::analysis::ConstraintAnalysis;
use crate::api::constraint_set::{ConstraintMetadata, ConstraintResult, ConstraintSet};
use crate::stream::ConstraintWeight;

use super::scorer::GroupedTerminalScorer;
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

pub struct GroupedConstraintSetBuilder<S, A, K, E, Fi, KF, C, V, R, Acc, Scorers, W, Sc>
where
    Acc: Accumulator<V, R>,
    Sc: Score,
{
    node_name: String,
    state: GroupedNodeState<S, A, K, E, Fi, KF, C, V, R, Acc>,
    scorers: Scorers,
    impact_type: ImpactType,
    weight_fn: W,
    is_hard: bool,
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

    fn into_weighted_builder<W>(
        self,
        impact_type: ImpactType,
        weight_fn: W,
        is_hard: bool,
    ) -> GroupedConstraintSetBuilder<S, A, K, E, Fi, KF, C, V, R, Acc, Scorers, W, Sc>
    where
        W: Fn(&K, &R) -> Sc + Send + Sync,
    {
        GroupedConstraintSetBuilder {
            node_name: self.node_name,
            state: self.state,
            scorers: self.scorers,
            impact_type,
            weight_fn,
            is_hard,
            _phantom: PhantomData,
        }
    }

    pub fn penalize<W>(
        self,
        weight: W,
    ) -> GroupedConstraintSetBuilder<
        S,
        A,
        K,
        E,
        Fi,
        KF,
        C,
        V,
        R,
        Acc,
        Scorers,
        impl Fn(&K, &R) -> Sc + Send + Sync,
        Sc,
    >
    where
        W: for<'w> ConstraintWeight<(&'w K, &'w R), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            ImpactType::Penalty,
            move |key: &K, result: &R| weight.score((key, result)),
            is_hard,
        )
    }

    pub fn reward<W>(
        self,
        weight: W,
    ) -> GroupedConstraintSetBuilder<
        S,
        A,
        K,
        E,
        Fi,
        KF,
        C,
        V,
        R,
        Acc,
        Scorers,
        impl Fn(&K, &R) -> Sc + Send + Sync,
        Sc,
    >
    where
        W: for<'w> ConstraintWeight<(&'w K, &'w R), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            ImpactType::Reward,
            move |key: &K, result: &R| weight.score((key, result)),
            is_hard,
        )
    }
}

impl<S, A, K, E, Fi, KF, C, V, R, Acc, Scorers, W, Sc>
    GroupedConstraintSetBuilder<S, A, K, E, Fi, KF, C, V, R, Acc, Scorers, W, Sc>
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
    W: Fn(&K, &R) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    pub fn named(
        self,
        name: &str,
    ) -> SharedGroupedConstraintSet<
        S,
        A,
        K,
        E,
        Fi,
        KF,
        C,
        V,
        R,
        Acc,
        (Scorers, GroupedTerminalScorer<K, R, W, Sc>),
        Sc,
    > {
        let scorer = GroupedTerminalScorer::new(
            ConstraintRef::new("", name),
            self.impact_type,
            self.weight_fn,
            self.is_hard,
        );
        SharedGroupedConstraintSet::new(self.node_name, self.state, (self.scorers, scorer))
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
