use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::ImpactType;

use crate::constraint::relational::{
    OperatorScorer, OptionalComplementAdapter, OptionalComplementWeight, SharedOperatorBuilder,
    SharedOperatorTerminal,
};
use crate::stream::collection_extract::CollectionExtract;
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::filter::UniFilter;
use crate::stream::relational::leaf_collector::OptionalEntityKey;
use crate::stream::relational::operator::{CollectionNode, ComplementNode, FilterNode, GroupNode};
use crate::stream::relational::view_plan::{GroupKey, ViewEqualPlan};
use crate::stream::relational::Leaf;
use crate::stream::weighting_support::ConstraintWeight;

use super::source::{ProjectedCollector, ProjectedRow, Source, SourceNode};

/// Grouped projected stream complemented with `B` entities: every `B` yields one
/// row keyed `key_b(b)`, holding the real group's result or `default_fn(b)`.
pub struct ComplementedGrouped<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D, Sc>
where
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
    ComplementedGrouped<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    B: Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    Src: Source<S, Out>,
    EB: CollectionExtract<S, Item = B> + 'static,
    F: UniFilter<S, Out>,
    KA: Fn(&Out) -> Option<K> + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync + Clone,
    C: for<'i> Collector<&'i Out, Value = V, Result = R, Accumulator = Acc> + Send + Sync + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    D: Fn(&B) -> R + Send + Sync,
    Sc: Score + 'static,
{
    fn into_weighted_builder<W>(
        self,
        impact_type: ImpactType,
        weight_fn: W,
        is_hard: bool,
    ) -> ComplementedGroupedBuilder<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D, W, Sc>
    where
        W: Fn(&K, &R) -> Sc + Send + Sync,
    {
        ComplementedGroupedBuilder {
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

    /// Penalizes each row by `weight`.
    ///
    /// `weight` is a fixed score, or a closure over the matched `(&key, &result)` returning one.
    /// A fixed weight with a non-zero hard level marks the constraint hard; closures
    /// are soft unless wrapped in [`hard_weight`](crate::stream::hard_weight).
    pub fn penalize<W>(
        self,
        weight: W,
    ) -> ComplementedGroupedBuilder<
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

    /// Rewards each row by `weight`.
    ///
    /// `weight` is a fixed score, or a closure over the matched `(&key, &result)` returning one.
    /// A fixed weight with a non-zero hard level marks the constraint hard; closures
    /// are soft unless wrapped in [`hard_weight`](crate::stream::hard_weight).
    pub fn reward<W>(
        self,
        weight: W,
    ) -> ComplementedGroupedBuilder<
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

/// Builder holding a weighted complemented grouped projected constraint until
/// [`named`](Self::named) finalizes it.
pub struct ComplementedGroupedBuilder<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D, W, Sc>
where
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
    ComplementedGroupedBuilder<S, Out, B, K, Src, EB, F, KA, KB, C, V, R, Acc, D, W, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    B: Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    Src: Source<S, Out> + 'static,
    EB: CollectionExtract<S, Item = B> + 'static,
    F: UniFilter<S, Out> + 'static,
    KA: Fn(&Out) -> Option<K> + Send + Sync + 'static,
    KB: Fn(&B) -> K + Send + Sync + Clone + 'static,
    C: for<'i> Collector<&'i Out, Value = V, Result = R, Accumulator = Acc> + Send + Sync + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    D: Fn(&B) -> R + Send + Sync + 'static,
    W: Fn(&K, &R) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    /// Names the constraint and builds it as a
    /// [`SharedOperatorTerminal`](crate::constraint::relational::SharedOperatorTerminal),
    /// to which further terminals over the same grouped node can be appended.
    pub fn named(
        self,
        name: &str,
    ) -> SharedOperatorTerminal<
        S,
        ComplementNode<
            S,
            CollectionNode<S, EB>,
            GroupNode<
                FilterNode<
                    SourceNode<S, Out, Src>,
                    impl Fn(&S, &ProjectedRow<'_, Out>) -> bool + Send + Sync,
                >,
                impl Fn(&ProjectedRow<'_, Out>) -> Option<K> + Send + Sync,
                ProjectedCollector<C, V, R, Acc>,
                Option<K>,
                Acc,
                V,
                R,
            >,
            ViewEqualPlan<Option<K>, OptionalEntityKey<KB>, GroupKey>,
            impl Fn(&S, &Leaf<'_, B>) -> R + Send + Sync,
            R,
        >,
        OptionalComplementAdapter<KB, K, R>,
        OperatorScorer<OptionalComplementWeight<W, KB>, Sc>,
        Sc,
    > {
        let target_binding =
            u32::try_from(self.source.source_count()).expect("source binding count");
        let filter = self.filter;
        let key_a = self.key_a;
        let group = GroupNode::new(
            FilterNode::new(
                SourceNode::new(self.source),
                move |solution: &S, row: &ProjectedRow<'_, Out>| filter.test(solution, row.value),
            ),
            move |row: &ProjectedRow<'_, Out>| key_a(row.value),
            ProjectedCollector::new(self.collector),
        );
        let key_b = self.key_b;
        let default_fn = self.default_fn;
        let tree = ComplementNode::new(
            CollectionNode::new(self.extractor_b, target_binding),
            group,
            ViewEqualPlan::new(OptionalEntityKey::new(key_b.clone()), GroupKey),
            move |_solution: &S, row: &Leaf<'_, B>| default_fn(row.entity),
        );
        SharedOperatorBuilder::first_with_hard(
            tree,
            OptionalComplementAdapter::new(key_b),
            self.impact_type,
            self.weight_fn,
            self.is_hard,
        )
        .first_named(name)
    }
}
