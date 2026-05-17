use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::analysis::ConstraintAnalysis;
use crate::api::constraint_set::{ConstraintMetadata, ConstraintResult, ConstraintSet};
use crate::constraint::grouped::{ComplementedGroupedScorerSet, GroupedTerminalScorer};
use crate::stream::collection_extract::CollectionExtract;
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::filter::UniFilter;
use crate::stream::ConstraintWeight;
use crate::stream::ProjectedSource;

use super::state::ProjectedComplementedGroupedNodeState;

pub struct SharedProjectedComplementedGroupedConstraintSet<
    S,
    Out,
    B,
    K,
    Src,
    EB,
    F,
    KA,
    KB,
    C,
    V,
    R,
    Acc,
    D,
    Scorers,
    Sc,
> where
    Src: ProjectedSource<S, Out>,
    Acc: Accumulator<V, R>,
    Sc: Score,
{
    state: ProjectedComplementedGroupedNodeState<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D>,
    scorers: Scorers,
    cached_score: Sc,
    _phantom: PhantomData<fn() -> Sc>,
}

pub struct ProjectedComplementedGroupedConstraintSetBuilder<
    S,
    Out,
    B,
    K,
    Src,
    EB,
    F,
    KA,
    KB,
    C,
    V,
    R,
    Acc,
    D,
    Scorers,
    W,
    Sc,
> where
    Src: ProjectedSource<S, Out>,
    Acc: Accumulator<V, R>,
    Sc: Score,
{
    state: ProjectedComplementedGroupedNodeState<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D>,
    scorers: Scorers,
    cached_score: Sc,
    impact_type: ImpactType,
    weight_fn: W,
    is_hard: bool,
    _phantom: PhantomData<fn() -> Sc>,
}

impl<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D, Scorers, Sc>
    SharedProjectedComplementedGroupedConstraintSet<
        S,
        Out,
        B,
        K,
        Src,
        EB,
        F,
        KA,
        KB,
        C,
        V,
        R,
        Acc,
        D,
        Scorers,
        Sc,
    >
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    B: Send + Sync + 'static,
    K: Eq + Hash + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    EB: CollectionExtract<S, Item = B>,
    F: UniFilter<S, Out>,
    KA: Fn(&Out) -> Option<K> + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    C: for<'i> Collector<&'i Out, Value = V, Result = R, Accumulator = Acc> + Send + Sync,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    D: Fn(&B) -> R + Send + Sync,
    Scorers: ComplementedGroupedScorerSet<K, R, Sc>,
    Sc: Score + 'static,
{
    pub fn new(
        state: ProjectedComplementedGroupedNodeState<
            S,
            Out,
            B,
            K,
            Src,
            EB,
            F,
            KA,
            KB,
            C,
            V,
            R,
            Acc,
            D,
        >,
        scorers: Scorers,
    ) -> Self {
        Self {
            state,
            scorers,
            cached_score: Sc::zero(),
            _phantom: PhantomData,
        }
    }

    pub fn state(
        &self,
    ) -> &ProjectedComplementedGroupedNodeState<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D>
    {
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
    ) -> ProjectedComplementedGroupedConstraintSetBuilder<
        S,
        Out,
        B,
        K,
        Src,
        EB,
        F,
        KA,
        KB,
        C,
        V,
        R,
        Acc,
        D,
        Scorers,
        W,
        Sc,
    >
    where
        W: Fn(&K, &R) -> Sc + Send + Sync,
    {
        ProjectedComplementedGroupedConstraintSetBuilder {
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
    ) -> ProjectedComplementedGroupedConstraintSetBuilder<
        S,
        Out,
        B,
        K,
        Src,
        EB,
        F,
        KA,
        KB,
        C,
        V,
        R,
        Acc,
        D,
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
    ) -> ProjectedComplementedGroupedConstraintSetBuilder<
        S,
        Out,
        B,
        K,
        Src,
        EB,
        F,
        KA,
        KB,
        C,
        V,
        R,
        Acc,
        D,
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

impl<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D, Scorers, W, Sc>
    ProjectedComplementedGroupedConstraintSetBuilder<
        S,
        Out,
        B,
        K,
        Src,
        EB,
        F,
        KA,
        KB,
        C,
        V,
        R,
        Acc,
        D,
        Scorers,
        W,
        Sc,
    >
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    B: Send + Sync + 'static,
    K: Eq + Hash + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    EB: CollectionExtract<S, Item = B>,
    F: UniFilter<S, Out>,
    KA: Fn(&Out) -> Option<K> + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    C: for<'i> Collector<&'i Out, Value = V, Result = R, Accumulator = Acc> + Send + Sync,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    D: Fn(&B) -> R + Send + Sync,
    Scorers: ComplementedGroupedScorerSet<K, R, Sc>,
    W: Fn(&K, &R) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    pub fn named(
        self,
        name: &str,
    ) -> SharedProjectedComplementedGroupedConstraintSet<
        S,
        Out,
        B,
        K,
        Src,
        EB,
        F,
        KA,
        KB,
        C,
        V,
        R,
        Acc,
        D,
        (Scorers, GroupedTerminalScorer<K, R, W, Sc>),
        Sc,
    > {
        let scorer = GroupedTerminalScorer::new(
            ConstraintRef::new("", name),
            self.impact_type,
            self.weight_fn,
            self.is_hard,
        );
        SharedProjectedComplementedGroupedConstraintSet {
            state: self.state,
            scorers: (self.scorers, scorer),
            cached_score: self.cached_score,
            _phantom: PhantomData,
        }
    }
}

impl<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D, Scorers, Sc> ConstraintSet<S, Sc>
    for SharedProjectedComplementedGroupedConstraintSet<
        S,
        Out,
        B,
        K,
        Src,
        EB,
        F,
        KA,
        KB,
        C,
        V,
        R,
        Acc,
        D,
        Scorers,
        Sc,
    >
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    B: Send + Sync + 'static,
    K: Eq + Hash + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    EB: CollectionExtract<S, Item = B>,
    F: UniFilter<S, Out>,
    KA: Fn(&Out) -> Option<K> + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    C: for<'i> Collector<&'i Out, Value = V, Result = R, Accumulator = Acc> + Send + Sync,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    D: Fn(&B) -> R + Send + Sync,
    Scorers: ComplementedGroupedScorerSet<K, R, Sc>,
    Sc: Score + 'static,
{
    fn evaluate_all(&self, solution: &S) -> Sc {
        let state = self.state.evaluation_state(solution);
        self.scorers.evaluate(&state)
    }

    fn constraint_count(&self) -> usize {
        self.scorers.constraint_count()
    }

    fn constraint_metadata_entries(&self) -> Vec<ConstraintMetadata<'_>> {
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
        self.state
            .on_insert(solution, entity_index, descriptor_index);
        self.refresh_from_state()
    }

    fn on_retract_all(&mut self, solution: &S, entity_index: usize, descriptor_index: usize) -> Sc {
        self.state
            .on_retract(solution, entity_index, descriptor_index);
        self.refresh_from_state()
    }

    fn reset_all(&mut self) {
        self.state.reset();
        self.scorers.reset();
        self.cached_score = Sc::zero();
    }
}

impl<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D, Scorers, Sc>
    SharedProjectedComplementedGroupedConstraintSet<
        S,
        Out,
        B,
        K,
        Src,
        EB,
        F,
        KA,
        KB,
        C,
        V,
        R,
        Acc,
        D,
        Scorers,
        Sc,
    >
where
    Src: ProjectedSource<S, Out>,
    Acc: Accumulator<V, R>,
    Scorers: ComplementedGroupedScorerSet<K, R, Sc>,
    K: Eq + Hash,
    Sc: Score,
{
    fn refresh_from_state(&mut self) -> Sc {
        let delta = self.scorers.refresh_changed(&self.state);
        self.cached_score = self.cached_score + delta;
        delta
    }
}
