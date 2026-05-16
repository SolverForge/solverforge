use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;

use super::super::collection_extract::CollectionExtract;
use super::super::collector::{Accumulator, Collector};
use super::super::complemented_stream::ComplementedConstraintStream;
use super::super::filter::UniFilter;
use super::super::weighting_support::ConstraintWeight;

/* Zero-erasure constraint stream over grouped entities.

`GroupedConstraintStream` is created by `UniConstraintStream::group_by()`
and operates on (key, collector_result) tuples.

All type parameters are concrete - no trait objects, no Arc allocations.
*/
pub struct GroupedConstraintStream<S, A, K, E, Fi, KF, C, V, R, Acc, Sc>
where
    Sc: Score,
{
    pub(super) extractor: E,
    pub(super) filter: Fi,
    pub(super) key_fn: KF,
    pub(super) collector: C,
    pub(super) _phantom: PhantomData<(
        fn() -> S,
        fn() -> A,
        fn() -> K,
        fn() -> V,
        fn() -> R,
        fn() -> Acc,
        fn() -> Sc,
    )>,
}

impl<S, A, K, E, Fi, KF, C, V, R, Acc, Sc>
    GroupedConstraintStream<S, A, K, E, Fi, KF, C, V, R, Acc, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    E: CollectionExtract<S, Item = A>,
    Fi: UniFilter<S, A>,
    KF: Fn(&A) -> K + Send + Sync,
    C: for<'i> Collector<&'i A, Value = V, Result = R, Accumulator = Acc> + Send + Sync + 'static,
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
    ) -> super::weighting::GroupedConstraintBuilder<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc>
    where
        W: Fn(&K, &R) -> Sc + Send + Sync,
    {
        super::weighting::GroupedConstraintBuilder {
            extractor: self.extractor,
            filter: self.filter,
            key_fn: self.key_fn,
            collector: self.collector,
            impact_type,
            weight_fn,
            is_hard,
            _phantom: PhantomData,
        }
    }

    // Creates a new zero-erasure grouped constraint stream.
    pub(crate) fn new(extractor: E, filter: Fi, key_fn: KF, collector: C) -> Self {
        Self {
            extractor,
            filter,
            key_fn,
            collector,
            _phantom: PhantomData,
        }
    }

    pub fn penalize<W>(
        self,
        weight: W,
    ) -> super::weighting::GroupedConstraintBuilder<
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
        impl Fn(&K, &R) -> Sc + Send + Sync,
        Sc,
    >
    where
        W: for<'w> ConstraintWeight<(&'w K, &'w R), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            solverforge_core::ImpactType::Penalty,
            move |key: &K, result: &R| weight.score((key, result)),
            is_hard,
        )
    }

    pub fn reward<W>(
        self,
        weight: W,
    ) -> super::weighting::GroupedConstraintBuilder<
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
        impl Fn(&K, &R) -> Sc + Send + Sync,
        Sc,
    >
    where
        W: for<'w> ConstraintWeight<(&'w K, &'w R), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            solverforge_core::ImpactType::Reward,
            move |key: &K, result: &R| weight.score((key, result)),
            is_hard,
        )
    }

    /* Adds complement entities with default values for missing keys. */
    pub fn complement<B, EB, KB, D>(
        self,
        extractor_b: EB,
        key_b: KB,
        default_fn: D,
    ) -> ComplementedConstraintStream<
        S,
        A,
        B,
        K,
        E,
        EB,
        impl Fn(&A) -> Option<K> + Send + Sync,
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
        let wrapped_key_fn = move |a: &A| Some((key_fn)(a));
        ComplementedConstraintStream::new(
            self.extractor,
            extractor_b,
            wrapped_key_fn,
            key_b,
            self.collector,
            default_fn,
        )
    }

    /* Adds complement entities with a custom key function for filtering. */
    pub fn complement_with_key<B, EB, KA2, KB, D>(
        self,
        extractor_b: EB,
        key_a: KA2,
        key_b: KB,
        default_fn: D,
    ) -> ComplementedConstraintStream<S, A, B, K, E, EB, KA2, KB, C, V, R, Acc, D, Sc>
    where
        B: Clone + Send + Sync + 'static,
        EB: CollectionExtract<S, Item = B>,
        KA2: Fn(&A) -> Option<K> + Send + Sync,
        KB: Fn(&B) -> K + Send + Sync,
        D: Fn(&B) -> R + Send + Sync,
    {
        ComplementedConstraintStream::new(
            self.extractor,
            extractor_b,
            key_a,
            key_b,
            self.collector,
            default_fn,
        )
    }
}

impl<S, A, K, E, Fi, KF, C, V, R, Acc, Sc: Score> std::fmt::Debug
    for GroupedConstraintStream<S, A, K, E, Fi, KF, C, V, R, Acc, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupedConstraintStream").finish()
    }
}
