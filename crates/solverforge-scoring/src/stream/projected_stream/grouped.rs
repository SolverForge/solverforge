use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;

use crate::stream::collection_extract::CollectionExtract;
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::filter::UniFilter;
use crate::stream::weighting_support::ConstraintWeight;

use super::complemented_grouped::ProjectedComplementedGroupedConstraintStream;
use super::source::ProjectedSource;

pub struct ProjectedGroupedConstraintStream<S, Out, K, Src, F, KF, C, V, R, Acc, Sc>
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

impl<S, Out, K, Src, F, KF, C, V, R, Acc, Sc>
    ProjectedGroupedConstraintStream<S, Out, K, Src, F, KF, C, V, R, Acc, Sc>
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
    Sc: Score + 'static,
{
    fn into_weighted_builder<W>(
        self,
        impact_type: solverforge_core::ImpactType,
        weight_fn: W,
        is_hard: bool,
    ) -> ProjectedGroupedConstraintBuilder<S, Out, K, Src, F, KF, C, V, R, Acc, W, Sc>
    where
        W: Fn(&K, &R) -> Sc + Send + Sync,
    {
        ProjectedGroupedConstraintBuilder {
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

    pub fn penalize<W>(
        self,
        weight_fn: W,
    ) -> ProjectedGroupedConstraintBuilder<
        S,
        Out,
        K,
        Src,
        F,
        KF,
        C,
        V,
        R,
        Acc,
        impl Fn(&K, &R) -> Sc + Send + Sync,
        Sc,
    >
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

    pub fn reward<W>(
        self,
        weight_fn: W,
    ) -> ProjectedGroupedConstraintBuilder<
        S,
        Out,
        K,
        Src,
        F,
        KF,
        C,
        V,
        R,
        Acc,
        impl Fn(&K, &R) -> Sc + Send + Sync,
        Sc,
    >
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

    pub fn complement<B, EB, KB, D>(
        self,
        extractor_b: EB,
        key_b: KB,
        default_fn: D,
    ) -> ProjectedComplementedGroupedConstraintStream<
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
        B: Clone + Send + Sync + 'static,
        EB: CollectionExtract<S, Item = B>,
        KB: Fn(&B) -> K + Send + Sync,
        D: Fn(&B) -> R + Send + Sync,
    {
        let key_fn = self.key_fn;
        let key_a = move |output: &Out| Some((key_fn)(output));
        ProjectedComplementedGroupedConstraintStream {
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

    pub fn complement_with_key<B, EB, KA2, KB, D>(
        self,
        extractor_b: EB,
        key_a: KA2,
        key_b: KB,
        default_fn: D,
    ) -> ProjectedComplementedGroupedConstraintStream<
        S,
        Out,
        B,
        K,
        Src,
        EB,
        F,
        KA2,
        KB,
        C,
        V,
        R,
        Acc,
        D,
        Sc,
    >
    where
        B: Clone + Send + Sync + 'static,
        EB: CollectionExtract<S, Item = B>,
        KA2: Fn(&Out) -> Option<K> + Send + Sync,
        KB: Fn(&B) -> K + Send + Sync,
        D: Fn(&B) -> R + Send + Sync,
    {
        ProjectedComplementedGroupedConstraintStream {
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

pub struct ProjectedGroupedConstraintBuilder<S, Out, K, Src, F, KF, C, V, R, Acc, W, Sc>
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
    ProjectedGroupedConstraintBuilder<S, Out, K, Src, F, KF, C, V, R, Acc, W, Sc>
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
    W: Fn(&K, &R) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    pub fn named(
        self,
        name: &str,
    ) -> crate::constraint::projected::ProjectedGroupedConstraint<
        S,
        Out,
        K,
        Src,
        F,
        KF,
        C,
        V,
        R,
        Acc,
        W,
        Sc,
    > {
        crate::constraint::projected::ProjectedGroupedConstraint::new(
            solverforge_core::ConstraintRef::new("", name),
            self.impact_type,
            self.source,
            self.filter,
            self.key_fn,
            self.collector,
            self.weight_fn,
            self.is_hard,
        )
    }
}
