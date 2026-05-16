use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::constraint::projected::ProjectedComplementedGroupedConstraint;
use crate::stream::collection_extract::CollectionExtract;
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::filter::UniFilter;
use crate::stream::weighting_support::ConstraintWeight;

use super::source::ProjectedSource;

pub struct ProjectedComplementedGroupedConstraintStream<
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
    Sc,
> where
    Sc: Score,
{
    pub(crate) source: Src,
    pub(crate) extractor_b: EB,
    pub(crate) filter: F,
    pub(crate) key_a: KA,
    pub(crate) key_b: KB,
    pub(crate) collector: C,
    pub(crate) default_fn: D,
    pub(crate) _phantom: PhantomData<(
        fn() -> S,
        fn() -> Out,
        fn() -> B,
        fn() -> K,
        fn() -> V,
        fn() -> R,
        fn() -> Acc,
        fn() -> Sc,
    )>,
}

impl<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D, Sc>
    ProjectedComplementedGroupedConstraintStream<
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
        Sc,
    >
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    EB: CollectionExtract<S, Item = B>,
    F: UniFilter<S, Out>,
    KA: Fn(&Out) -> Option<K> + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    C: for<'i> Collector<&'i Out, Value = V, Result = R, Accumulator = Acc> + Send + Sync + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    D: Fn(&B) -> R + Send + Sync,
    Sc: Score + 'static,
{
    #[doc(hidden)]
    pub fn into_shared_node_state(
        self,
    ) -> crate::constraint::projected::ProjectedComplementedGroupedNodeState<
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
    > {
        crate::constraint::projected::ProjectedComplementedGroupedNodeState::new(
            self.source,
            self.extractor_b,
            self.filter,
            self.key_a,
            self.key_b,
            self.collector,
            self.default_fn,
        )
    }

    #[doc(hidden)]
    pub fn into_shared_constraint_set<Scorers>(
        self,
        _node_name: impl Into<String>,
        scorers: Scorers,
    ) -> crate::constraint::projected::SharedProjectedComplementedGroupedConstraintSet<
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
        Scorers: crate::constraint::grouped::ComplementedGroupedScorerSet<K, R, Sc>,
    {
        crate::constraint::projected::SharedProjectedComplementedGroupedConstraintSet::new(
            self.into_shared_node_state(),
            scorers,
        )
    }

    fn into_weighted_builder<W>(
        self,
        impact_type: ImpactType,
        weight_fn: W,
        is_hard: bool,
    ) -> ProjectedComplementedGroupedConstraintBuilder<
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
        W,
        Sc,
    >
    where
        W: Fn(&K, &R) -> Sc + Send + Sync,
    {
        ProjectedComplementedGroupedConstraintBuilder {
            source: self.source,
            extractor_b: self.extractor_b,
            filter: self.filter,
            key_a: self.key_a,
            key_b: self.key_b,
            collector: self.collector,
            default_fn: self.default_fn,
            impact_type,
            weight_fn,
            is_hard,
            _phantom: PhantomData,
        }
    }

    pub fn penalize<W>(
        self,
        weight: W,
    ) -> ProjectedComplementedGroupedConstraintBuilder<
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
    ) -> ProjectedComplementedGroupedConstraintBuilder<
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

pub struct ProjectedComplementedGroupedConstraintBuilder<
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
    W,
    Sc,
> where
    Sc: Score,
{
    source: Src,
    extractor_b: EB,
    filter: F,
    key_a: KA,
    key_b: KB,
    collector: C,
    default_fn: D,
    impact_type: ImpactType,
    weight_fn: W,
    is_hard: bool,
    _phantom: PhantomData<(
        fn() -> S,
        fn() -> Out,
        fn() -> B,
        fn() -> K,
        fn() -> V,
        fn() -> R,
        fn() -> Acc,
        fn() -> Sc,
    )>,
}

impl<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D, W, Sc>
    ProjectedComplementedGroupedConstraintBuilder<
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
        W,
        Sc,
    >
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    Src: ProjectedSource<S, Out>,
    EB: CollectionExtract<S, Item = B>,
    F: UniFilter<S, Out>,
    KA: Fn(&Out) -> Option<K> + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    C: for<'i> Collector<&'i Out, Value = V, Result = R, Accumulator = Acc> + Send + Sync + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    D: Fn(&B) -> R + Send + Sync,
    W: Fn(&K, &R) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    pub fn named(
        self,
        name: &str,
    ) -> ProjectedComplementedGroupedConstraint<
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
        W,
        Sc,
    > {
        ProjectedComplementedGroupedConstraint::new(
            ConstraintRef::new("", name),
            self.impact_type,
            self.source,
            self.extractor_b,
            self.filter,
            self.key_a,
            self.key_b,
            self.collector,
            self.default_fn,
            self.weight_fn,
            self.is_hard,
        )
    }
}
