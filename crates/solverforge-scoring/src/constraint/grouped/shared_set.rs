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
    state: GroupedNodeState<S, A, K, E, Fi, KF, C, V, R, Acc>,
    scorers: Scorers,
    cached_score: Sc,
    _phantom: PhantomData<fn() -> Sc>,
}

pub struct GroupedConstraintSetBuilder<S, A, K, E, Fi, KF, C, V, R, Acc, Scorers, W, Sc>
where
    Acc: Accumulator<V, R>,
    Sc: Score,
{
    state: GroupedNodeState<S, A, K, E, Fi, KF, C, V, R, Acc>,
    scorers: Scorers,
    cached_score: Sc,
    impact_type: ImpactType,
    weight_fn: W,
    is_hard: bool,
    _phantom: PhantomData<fn() -> Sc>,
}

impl<S, A, K, E, Fi, KF, C, V, R, Acc, Scorers, Sc>
    SharedGroupedConstraintSet<S, A, K, E, Fi, KF, C, V, R, Acc, Scorers, Sc>
where
    S: Send + Sync + 'static,
    A: Send + Sync + 'static,
    K: Eq + std::hash::Hash + Send + Sync + 'static,
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
        state: GroupedNodeState<S, A, K, E, Fi, KF, C, V, R, Acc>,
        scorers: Scorers,
    ) -> Self {
        Self {
            state,
            scorers,
            cached_score: Sc::zero(),
            _phantom: PhantomData,
        }
    }

    pub fn state(&self) -> &GroupedNodeState<S, A, K, E, Fi, KF, C, V, R, Acc> {
        &self.state
    }

    pub(crate) fn primary_constraint_ref(&self) -> &ConstraintRef {
        self.scorers.primary_constraint_ref()
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
            state: self.state,
            scorers: self.scorers,
            cached_score: self.cached_score,
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
    A: Send + Sync + 'static,
    K: Eq + std::hash::Hash + Send + Sync + 'static,
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
        SharedGroupedConstraintSet {
            state: self.state,
            scorers: (self.scorers, scorer),
            cached_score: self.cached_score,
            _phantom: PhantomData,
        }
    }
}

impl<S, A, K, E, Fi, KF, C, V, R, Acc, Scorers, Sc> ConstraintSet<S, Sc>
    for SharedGroupedConstraintSet<S, A, K, E, Fi, KF, C, V, R, Acc, Scorers, Sc>
where
    S: Send + Sync + 'static,
    A: Send + Sync + 'static,
    K: Eq + std::hash::Hash + Send + Sync + 'static,
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
        self.cached_score = self.scorers.initialize(&self.state);
        self.cached_score
    }

    fn on_insert_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let node_name = &self.scorers.primary_constraint_ref().name;
        self.state
            .on_insert(solution, entity_index, descriptor_index, node_name);
        self.refresh_from_state()
    }

    fn on_retract_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        let node_name = &self.scorers.primary_constraint_ref().name;
        self.state
            .on_retract(solution, entity_index, descriptor_index, node_name);
        self.refresh_from_state()
    }

    fn reset_all(&mut self) {
        self.state.reset();
        self.scorers.reset();
        self.cached_score = Sc::zero();
    }
}

impl<S, A, K, E, Fi, KF, C, V, R, Acc, Scorers, Sc>
    SharedGroupedConstraintSet<S, A, K, E, Fi, KF, C, V, R, Acc, Scorers, Sc>
where
    Acc: Accumulator<V, R>,
    Scorers: GroupedScorerSet<K, R, Sc>,
    K: Eq + std::hash::Hash,
    Sc: Score,
{
    fn refresh_from_state(&mut self) -> Sc {
        let delta = self.scorers.refresh_changed(&self.state);
        self.cached_score = self.cached_score + delta;
        delta
    }
}
