use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;

use crate::constraint::relational::{
    GroupAdapter, GroupWeight, OperatorScorer, SharedOperatorBuilder, SharedOperatorTerminal,
};
use crate::stream::collection_extract::CollectionExtract;
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::filter::UniFilter;
use crate::stream::relational::operator::{FilterNode, GroupNode};
use crate::stream::weighting_support::ConstraintWeight;

use super::complemented_grouped::ComplementedGrouped;
use super::source::{ProjectedCollector, ProjectedRow, Source, SourceNode};

/// Projected rows grouped by a key `K` and aggregated by a collector.
pub struct Grouped<S, Out, K, Src, F, KF, C, V, R, Acc, Sc>
where
    Sc: Score,
{
    pub(crate) source: Src,
    pub(crate) filter: F,
    pub(crate) key_fn: KF,
    pub(crate) collector: C,
    pub(crate) _phantom: PhantomData<(
        fn() -> S,
        fn() -> Out,
        fn() -> K,
        fn() -> V,
        fn() -> R,
        fn() -> Acc,
        fn() -> Sc,
    )>,
}

impl<S, Out, K, Src, F, KF, C, V, R, Acc, Sc> Grouped<S, Out, K, Src, F, KF, C, V, R, Acc, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    Src: Source<S, Out>,
    F: UniFilter<S, Out>,
    KF: Fn(&Out) -> K + Send + Sync,
    C: for<'i> Collector<&'i Out, Value = V, Result = R, Accumulator = Acc> + Send + Sync + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    Sc: Score + 'static,
{
    fn into_weighted_builder<W>(
        self,
        impact_type: solverforge_core::ImpactType,
        weight_fn: W,
        is_hard: bool,
    ) -> GroupedBuilder<S, Out, K, Src, F, KF, C, V, R, Acc, W, Sc>
    where
        W: Fn(&K, &R) -> Sc + Send + Sync,
    {
        GroupedBuilder {
            source: self.source,
            filter: self.filter,
            key_fn: self.key_fn,
            collector: self.collector,
            impact_type,
            weight_fn,
            is_hard,
            _phantom: PhantomData,
        }
    }

    /// Penalizes each group by `weight_fn`.
    ///
    /// `weight_fn` is a fixed score, or a closure over the matched `(&key, &result)` returning one.
    /// A fixed weight with a non-zero hard level marks the constraint hard; closures
    /// are soft unless wrapped in [`hard_weight`](crate::stream::hard_weight).
    pub fn penalize<W>(
        self,
        weight_fn: W,
    ) -> GroupedBuilder<S, Out, K, Src, F, KF, C, V, R, Acc, impl Fn(&K, &R) -> Sc + Send + Sync, Sc>
    where
        W: for<'w> ConstraintWeight<(&'w K, &'w R), Sc> + Send + Sync,
    {
        let is_hard = weight_fn.is_hard();
        self.into_weighted_builder(
            solverforge_core::ImpactType::Penalty,
            move |key: &K, result: &R| weight_fn.score((key, result)),
            is_hard,
        )
    }

    /// Rewards each group by `weight_fn`.
    ///
    /// `weight_fn` is a fixed score, or a closure over the matched `(&key, &result)` returning one.
    /// A fixed weight with a non-zero hard level marks the constraint hard; closures
    /// are soft unless wrapped in [`hard_weight`](crate::stream::hard_weight).
    pub fn reward<W>(
        self,
        weight_fn: W,
    ) -> GroupedBuilder<S, Out, K, Src, F, KF, C, V, R, Acc, impl Fn(&K, &R) -> Sc + Send + Sync, Sc>
    where
        W: for<'w> ConstraintWeight<(&'w K, &'w R), Sc> + Send + Sync,
    {
        let is_hard = weight_fn.is_hard();
        self.into_weighted_builder(
            solverforge_core::ImpactType::Reward,
            move |key: &K, result: &R| weight_fn.score((key, result)),
            is_hard,
        )
    }

    /// Complements the groups with the entities of `extractor_b`: each `b` yields a
    /// row keyed `key_b(b)` holding the real group's result, or `default_fn(b)` when
    /// that group is empty.
    pub fn complement<B, EB, KB, D>(
        self,
        extractor_b: EB,
        key_b: KB,
        default_fn: D,
    ) -> ComplementedGrouped<
        S,
        Out,
        B,
        K,
        Src,
        EB,
        F,
        impl Fn(&Out) -> Option<K> + Send + Sync,
        KB,
        C,
        V,
        R,
        Acc,
        D,
        Sc,
    >
    where
        B: Send + Sync + 'static,
        EB: CollectionExtract<S, Item = B>,
        KB: Fn(&B) -> K + Send + Sync,
        D: Fn(&B) -> R + Send + Sync,
    {
        let key_fn = self.key_fn;
        let key_a = move |output: &Out| Some((key_fn)(output));
        ComplementedGrouped {
            source: self.source,
            extractor_b,
            filter: self.filter,
            key_a,
            key_b,
            collector: self.collector,
            default_fn,
            _phantom: PhantomData,
        }
    }

    /// Like [`complement`](Self::complement), regrouping rows by `key_a`; rows whose
    /// key is `None` join no target and contribute nothing.
    pub fn complement_with_key<B, EB, KA2, KB, D>(
        self,
        extractor_b: EB,
        key_a: KA2,
        key_b: KB,
        default_fn: D,
    ) -> ComplementedGrouped<S, Out, B, K, Src, EB, F, KA2, KB, C, V, R, Acc, D, Sc>
    where
        B: Send + Sync + 'static,
        EB: CollectionExtract<S, Item = B>,
        KA2: Fn(&Out) -> Option<K> + Send + Sync,
        KB: Fn(&B) -> K + Send + Sync,
        D: Fn(&B) -> R + Send + Sync,
    {
        ComplementedGrouped {
            source: self.source,
            extractor_b,
            filter: self.filter,
            key_a,
            key_b,
            collector: self.collector,
            default_fn,
            _phantom: PhantomData,
        }
    }
}

/// Builder holding a weighted grouped projected constraint until
/// [`named`](Self::named) finalizes it.
pub struct GroupedBuilder<S, Out, K, Src, F, KF, C, V, R, Acc, W, Sc>
where
    Sc: Score,
{
    pub(crate) source: Src,
    pub(crate) filter: F,
    pub(crate) key_fn: KF,
    pub(crate) collector: C,
    pub(crate) impact_type: solverforge_core::ImpactType,
    pub(crate) weight_fn: W,
    pub(crate) is_hard: bool,
    pub(crate) _phantom: PhantomData<(
        fn() -> S,
        fn() -> Out,
        fn() -> K,
        fn() -> V,
        fn() -> R,
        fn() -> Acc,
        fn() -> Sc,
    )>,
}

impl<S, Out, K, Src, F, KF, C, V, R, Acc, W, Sc>
    GroupedBuilder<S, Out, K, Src, F, KF, C, V, R, Acc, W, Sc>
where
    S: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    Src: Source<S, Out> + 'static,
    F: UniFilter<S, Out> + 'static,
    KF: Fn(&Out) -> K + Send + Sync + 'static,
    C: for<'i> Collector<&'i Out, Value = V, Result = R, Accumulator = Acc> + Send + Sync + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
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
        GroupNode<
            FilterNode<
                SourceNode<S, Out, Src>,
                impl Fn(&S, &ProjectedRow<'_, Out>) -> bool + Send + Sync,
            >,
            impl Fn(&ProjectedRow<'_, Out>) -> K + Send + Sync,
            ProjectedCollector<C, V, R, Acc>,
            K,
            Acc,
            V,
            R,
        >,
        GroupAdapter<K, R>,
        OperatorScorer<GroupWeight<W>, Sc>,
        Sc,
    > {
        let filter = self.filter;
        let key = self.key_fn;
        let tree = GroupNode::new(
            FilterNode::new(
                SourceNode::new(self.source),
                move |solution: &S, row: &ProjectedRow<'_, Out>| filter.test(solution, row.value),
            ),
            move |row: &ProjectedRow<'_, Out>| key(row.value),
            ProjectedCollector::new(self.collector),
        );
        SharedOperatorBuilder::first_with_hard(
            tree,
            GroupAdapter::new(),
            self.impact_type,
            self.weight_fn,
            self.is_hard,
        )
        .first_named(name)
    }
}
