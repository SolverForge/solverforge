use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;

use super::super::collection_extract::CollectionExtract;
use super::super::collector::UniCollector;
use super::super::complemented_stream::ComplementedConstraintStream;
use super::super::filter::UniFilter;

/* Zero-erasure constraint stream over grouped entities.

`GroupedConstraintStream` is created by `UniConstraintStream::group_by()`
and operates on (key, collector_result) tuples.

All type parameters are concrete - no trait objects, no Arc allocations.
*/
pub struct GroupedConstraintStream<S, A, K, E, Fi, KF, C, Sc>
where
    Sc: Score,
{
    pub(super) extractor: E,
    pub(super) filter: Fi,
    pub(super) key_fn: KF,
    pub(super) collector: C,
    pub(super) _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> K, fn() -> Sc)>,
}

impl<S, A, K, E, Fi, KF, C, Sc> GroupedConstraintStream<S, A, K, E, Fi, KF, C, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    E: CollectionExtract<S, Item = A>,
    Fi: UniFilter<S, A>,
    KF: Fn(&A) -> K + Send + Sync,
    C: UniCollector<A> + Send + Sync + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Clone + Send + Sync,
    Sc: Score + 'static,
{
    fn into_weighted_builder<W>(
        self,
        impact_type: solverforge_core::ImpactType,
        weight_fn: W,
        is_hard: bool,
    ) -> super::weighting::GroupedConstraintBuilder<S, A, K, E, Fi, KF, C, W, Sc>
    where
        W: Fn(&C::Result) -> Sc + Send + Sync,
    {
        super::weighting::GroupedConstraintBuilder {
            extractor: self.extractor,
            filter: self.filter,
            key_fn: self.key_fn,
            collector: self.collector,
            impact_type,
            weight_fn,
            is_hard,
            expected_descriptor: None,
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

    /* Penalizes each group with a weight based on the collector result. */
    pub fn penalize_with<W>(
        self,
        weight_fn: W,
    ) -> super::weighting::GroupedConstraintBuilder<S, A, K, E, Fi, KF, C, W, Sc>
    where
        W: Fn(&C::Result) -> Sc + Send + Sync,
    {
        self.into_weighted_builder(solverforge_core::ImpactType::Penalty, weight_fn, false)
    }

    // Penalizes each group with a weight, explicitly marked as hard constraint.
    pub fn penalize_hard_with<W>(
        self,
        weight_fn: W,
    ) -> super::weighting::GroupedConstraintBuilder<S, A, K, E, Fi, KF, C, W, Sc>
    where
        W: Fn(&C::Result) -> Sc + Send + Sync,
    {
        self.into_weighted_builder(solverforge_core::ImpactType::Penalty, weight_fn, true)
    }

    // Rewards each group with a weight based on the collector result.
    pub fn reward_with<W>(
        self,
        weight_fn: W,
    ) -> super::weighting::GroupedConstraintBuilder<S, A, K, E, Fi, KF, C, W, Sc>
    where
        W: Fn(&C::Result) -> Sc + Send + Sync,
    {
        self.into_weighted_builder(solverforge_core::ImpactType::Reward, weight_fn, false)
    }

    // Rewards each group with a weight, explicitly marked as hard constraint.
    pub fn reward_hard_with<W>(
        self,
        weight_fn: W,
    ) -> super::weighting::GroupedConstraintBuilder<S, A, K, E, Fi, KF, C, W, Sc>
    where
        W: Fn(&C::Result) -> Sc + Send + Sync,
    {
        self.into_weighted_builder(solverforge_core::ImpactType::Reward, weight_fn, true)
    }

    // Penalizes each group with one hard score unit.
    pub fn penalize_hard(
        self,
    ) -> super::weighting::GroupedConstraintBuilder<
        S,
        A,
        K,
        E,
        Fi,
        KF,
        C,
        impl Fn(&C::Result) -> Sc + Send + Sync,
        Sc,
    >
    where
        Sc: Copy,
    {
        let w = Sc::one_hard();
        self.penalize_hard_with(move |_: &C::Result| w)
    }

    // Penalizes each group with one soft score unit.
    pub fn penalize_soft(
        self,
    ) -> super::weighting::GroupedConstraintBuilder<
        S,
        A,
        K,
        E,
        Fi,
        KF,
        C,
        impl Fn(&C::Result) -> Sc + Send + Sync,
        Sc,
    >
    where
        Sc: Copy,
    {
        let w = Sc::one_soft();
        self.penalize_with(move |_: &C::Result| w)
    }

    // Rewards each group with one hard score unit.
    pub fn reward_hard(
        self,
    ) -> super::weighting::GroupedConstraintBuilder<
        S,
        A,
        K,
        E,
        Fi,
        KF,
        C,
        impl Fn(&C::Result) -> Sc + Send + Sync,
        Sc,
    >
    where
        Sc: Copy,
    {
        let w = Sc::one_hard();
        self.reward_hard_with(move |_: &C::Result| w)
    }

    // Rewards each group with one soft score unit.
    pub fn reward_soft(
        self,
    ) -> super::weighting::GroupedConstraintBuilder<
        S,
        A,
        K,
        E,
        Fi,
        KF,
        C,
        impl Fn(&C::Result) -> Sc + Send + Sync,
        Sc,
    >
    where
        Sc: Copy,
    {
        let w = Sc::one_soft();
        self.reward_with(move |_: &C::Result| w)
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
        D,
        Sc,
    >
    where
        B: Clone + Send + Sync + 'static,
        EB: CollectionExtract<S, Item = B>,
        KB: Fn(&B) -> K + Send + Sync,
        D: Fn(&B) -> C::Result + Send + Sync,
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
    ) -> ComplementedConstraintStream<S, A, B, K, E, EB, KA2, KB, C, D, Sc>
    where
        B: Clone + Send + Sync + 'static,
        EB: CollectionExtract<S, Item = B>,
        KA2: Fn(&A) -> Option<K> + Send + Sync,
        KB: Fn(&B) -> K + Send + Sync,
        D: Fn(&B) -> C::Result + Send + Sync,
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

impl<S, A, K, E, Fi, KF, C, Sc: Score> std::fmt::Debug
    for GroupedConstraintStream<S, A, K, E, Fi, KF, C, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupedConstraintStream").finish()
    }
}
