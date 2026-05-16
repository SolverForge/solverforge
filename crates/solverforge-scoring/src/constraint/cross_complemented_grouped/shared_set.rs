use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::analysis::ConstraintAnalysis;
use crate::api::constraint_set::{ConstraintMetadata, ConstraintResult, ConstraintSet};
use crate::constraint::grouped::{ComplementedGroupedScorerSet, GroupedTerminalScorer};
use crate::stream::collection_extract::CollectionExtract;
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::ConstraintWeight;

use super::state::CrossComplementedGroupedNodeState;

pub struct SharedCrossComplementedGroupedConstraintSet<
    S,
    A,
    B,
    T,
    JK,
    GK,
    EA,
    EB,
    ET,
    KA,
    KB,
    F,
    GF,
    KT,
    C,
    V,
    R,
    Acc,
    D,
    Scorers,
    Sc,
> where
    Acc: Accumulator<V, R>,
    Sc: Score,
{
    node_name: String,
    state: CrossComplementedGroupedNodeState<
        S,
        A,
        B,
        T,
        JK,
        GK,
        EA,
        EB,
        ET,
        KA,
        KB,
        F,
        GF,
        KT,
        C,
        V,
        R,
        Acc,
        D,
    >,
    scorers: Scorers,
    _phantom: PhantomData<fn() -> Sc>,
}

pub struct CrossComplementedGroupedConstraintSetBuilder<
    S,
    A,
    B,
    T,
    JK,
    GK,
    EA,
    EB,
    ET,
    KA,
    KB,
    F,
    GF,
    KT,
    C,
    V,
    R,
    Acc,
    D,
    Scorers,
    W,
    Sc,
> where
    Acc: Accumulator<V, R>,
    Sc: Score,
{
    node_name: String,
    state: CrossComplementedGroupedNodeState<
        S,
        A,
        B,
        T,
        JK,
        GK,
        EA,
        EB,
        ET,
        KA,
        KB,
        F,
        GF,
        KT,
        C,
        V,
        R,
        Acc,
        D,
    >,
    scorers: Scorers,
    impact_type: ImpactType,
    weight_fn: W,
    is_hard: bool,
    _phantom: PhantomData<fn() -> Sc>,
}

impl<S, A, B, T, JK, GK, EA, EB, ET, KA, KB, F, GF, KT, C, V, R, Acc, D, Scorers, Sc>
    SharedCrossComplementedGroupedConstraintSet<
        S,
        A,
        B,
        T,
        JK,
        GK,
        EA,
        EB,
        ET,
        KA,
        KB,
        F,
        GF,
        KT,
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
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    GK: Eq + Hash + Clone + Send + Sync + 'static,
    EA: CollectionExtract<S, Item = A> + Send + Sync,
    EB: CollectionExtract<S, Item = B> + Send + Sync,
    ET: CollectionExtract<S, Item = T> + Send + Sync,
    KA: Fn(&A) -> JK + Send + Sync,
    KB: Fn(&B) -> JK + Send + Sync,
    F: Fn(&S, &A, &B, usize, usize) -> bool + Send + Sync,
    GF: Fn(&A, &B) -> GK + Send + Sync,
    KT: Fn(&T) -> GK + Send + Sync,
    C: for<'i> Collector<(&'i A, &'i B), Value = V, Result = R, Accumulator = Acc> + Send + Sync,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    D: Fn(&T) -> R + Send + Sync,
    Scorers: ComplementedGroupedScorerSet<GK, R, Sc>,
    Sc: Score + 'static,
{
    pub fn new(
        node_name: impl Into<String>,
        state: CrossComplementedGroupedNodeState<
            S,
            A,
            B,
            T,
            JK,
            GK,
            EA,
            EB,
            ET,
            KA,
            KB,
            F,
            GF,
            KT,
            C,
            V,
            R,
            Acc,
            D,
        >,
        scorers: Scorers,
    ) -> Self {
        Self {
            node_name: node_name.into(),
            state,
            scorers,
            _phantom: PhantomData,
        }
    }

    pub fn state(
        &self,
    ) -> &CrossComplementedGroupedNodeState<
        S,
        A,
        B,
        T,
        JK,
        GK,
        EA,
        EB,
        ET,
        KA,
        KB,
        F,
        GF,
        KT,
        C,
        V,
        R,
        Acc,
        D,
    > {
        &self.state
    }

    fn into_weighted_builder<W>(
        self,
        impact_type: ImpactType,
        weight_fn: W,
        is_hard: bool,
    ) -> CrossComplementedGroupedConstraintSetBuilder<
        S,
        A,
        B,
        T,
        JK,
        GK,
        EA,
        EB,
        ET,
        KA,
        KB,
        F,
        GF,
        KT,
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
        W: Fn(&GK, &R) -> Sc + Send + Sync,
    {
        CrossComplementedGroupedConstraintSetBuilder {
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
    ) -> CrossComplementedGroupedConstraintSetBuilder<
        S,
        A,
        B,
        T,
        JK,
        GK,
        EA,
        EB,
        ET,
        KA,
        KB,
        F,
        GF,
        KT,
        C,
        V,
        R,
        Acc,
        D,
        Scorers,
        impl Fn(&GK, &R) -> Sc + Send + Sync,
        Sc,
    >
    where
        W: for<'w> ConstraintWeight<(&'w GK, &'w R), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            ImpactType::Penalty,
            move |key: &GK, result: &R| weight.score((key, result)),
            is_hard,
        )
    }

    pub fn reward<W>(
        self,
        weight: W,
    ) -> CrossComplementedGroupedConstraintSetBuilder<
        S,
        A,
        B,
        T,
        JK,
        GK,
        EA,
        EB,
        ET,
        KA,
        KB,
        F,
        GF,
        KT,
        C,
        V,
        R,
        Acc,
        D,
        Scorers,
        impl Fn(&GK, &R) -> Sc + Send + Sync,
        Sc,
    >
    where
        W: for<'w> ConstraintWeight<(&'w GK, &'w R), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            ImpactType::Reward,
            move |key: &GK, result: &R| weight.score((key, result)),
            is_hard,
        )
    }
}

impl<S, A, B, T, JK, GK, EA, EB, ET, KA, KB, F, GF, KT, C, V, R, Acc, D, Scorers, W, Sc>
    CrossComplementedGroupedConstraintSetBuilder<
        S,
        A,
        B,
        T,
        JK,
        GK,
        EA,
        EB,
        ET,
        KA,
        KB,
        F,
        GF,
        KT,
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
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    GK: Eq + Hash + Clone + Send + Sync + 'static,
    EA: CollectionExtract<S, Item = A> + Send + Sync,
    EB: CollectionExtract<S, Item = B> + Send + Sync,
    ET: CollectionExtract<S, Item = T> + Send + Sync,
    KA: Fn(&A) -> JK + Send + Sync,
    KB: Fn(&B) -> JK + Send + Sync,
    F: Fn(&S, &A, &B, usize, usize) -> bool + Send + Sync,
    GF: Fn(&A, &B) -> GK + Send + Sync,
    KT: Fn(&T) -> GK + Send + Sync,
    C: for<'i> Collector<(&'i A, &'i B), Value = V, Result = R, Accumulator = Acc> + Send + Sync,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    D: Fn(&T) -> R + Send + Sync,
    Scorers: ComplementedGroupedScorerSet<GK, R, Sc>,
    W: Fn(&GK, &R) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    pub fn named(
        self,
        name: &str,
    ) -> SharedCrossComplementedGroupedConstraintSet<
        S,
        A,
        B,
        T,
        JK,
        GK,
        EA,
        EB,
        ET,
        KA,
        KB,
        F,
        GF,
        KT,
        C,
        V,
        R,
        Acc,
        D,
        (Scorers, GroupedTerminalScorer<GK, R, W, Sc>),
        Sc,
    > {
        let scorer = GroupedTerminalScorer::new(
            ConstraintRef::new("", name),
            self.impact_type,
            self.weight_fn,
            self.is_hard,
        );
        SharedCrossComplementedGroupedConstraintSet::new(
            self.node_name,
            self.state,
            (self.scorers, scorer),
        )
    }
}

impl<S, A, B, T, JK, GK, EA, EB, ET, KA, KB, F, GF, KT, C, V, R, Acc, D, Scorers, Sc>
    ConstraintSet<S, Sc>
    for SharedCrossComplementedGroupedConstraintSet<
        S,
        A,
        B,
        T,
        JK,
        GK,
        EA,
        EB,
        ET,
        KA,
        KB,
        F,
        GF,
        KT,
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
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    GK: Eq + Hash + Clone + Send + Sync + 'static,
    EA: CollectionExtract<S, Item = A> + Send + Sync,
    EB: CollectionExtract<S, Item = B> + Send + Sync,
    ET: CollectionExtract<S, Item = T> + Send + Sync,
    KA: Fn(&A) -> JK + Send + Sync,
    KB: Fn(&B) -> JK + Send + Sync,
    F: Fn(&S, &A, &B, usize, usize) -> bool + Send + Sync,
    GF: Fn(&A, &B) -> GK + Send + Sync,
    KT: Fn(&T) -> GK + Send + Sync,
    C: for<'i> Collector<(&'i A, &'i B), Value = V, Result = R, Accumulator = Acc> + Send + Sync,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    D: Fn(&T) -> R + Send + Sync,
    Scorers: ComplementedGroupedScorerSet<GK, R, Sc>,
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
