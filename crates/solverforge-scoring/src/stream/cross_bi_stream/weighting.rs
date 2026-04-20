use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::constraint::cross_bi_incremental::IncrementalCrossBiConstraint;

use super::super::collection_extract::CollectionExtract;
use super::super::filter::BiFilter;
use super::super::weighting_support::fixed_weight_is_hard;
use super::base::CrossBiConstraintStream;

impl<S, A, B, K, EA, EB, KA, KB, F, Sc> CrossBiConstraintStream<S, A, B, K, EA, EB, KA, KB, F, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    F: BiFilter<S, A, B>,
    Sc: Score + 'static,
{
    fn into_weighted_builder<W>(
        self,
        impact_type: ImpactType,
        weight: W,
        is_hard: bool,
    ) -> CrossBiConstraintBuilder<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
    where
        W: Fn(&A, &B) -> Sc + Send + Sync,
    {
        CrossBiConstraintBuilder {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            filter: self.filter,
            impact_type,
            weight,
            is_hard,
            _phantom: PhantomData,
        }
    }

    // Penalizes each matching pair with a fixed weight.
    pub fn penalize(
        self,
        weight: Sc,
    ) -> CrossBiConstraintBuilder<
        S,
        A,
        B,
        K,
        EA,
        EB,
        KA,
        KB,
        F,
        impl Fn(&A, &B) -> Sc + Send + Sync,
        Sc,
    >
    where
        Sc: Copy,
    {
        self.into_weighted_builder(
            ImpactType::Penalty,
            move |_: &A, _: &B| weight,
            fixed_weight_is_hard(weight),
        )
    }

    // Penalizes each matching pair with a dynamic weight.
    pub fn penalize_with<W>(
        self,
        weight_fn: W,
    ) -> CrossBiConstraintBuilder<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
    where
        W: Fn(&A, &B) -> Sc + Send + Sync,
    {
        self.into_weighted_builder(ImpactType::Penalty, weight_fn, false)
    }

    // Penalizes each matching pair with a dynamic weight, explicitly marked as hard.
    pub fn penalize_hard_with<W>(
        self,
        weight_fn: W,
    ) -> CrossBiConstraintBuilder<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
    where
        W: Fn(&A, &B) -> Sc + Send + Sync,
    {
        self.into_weighted_builder(ImpactType::Penalty, weight_fn, true)
    }

    // Rewards each matching pair with a fixed weight.
    pub fn reward(
        self,
        weight: Sc,
    ) -> CrossBiConstraintBuilder<
        S,
        A,
        B,
        K,
        EA,
        EB,
        KA,
        KB,
        F,
        impl Fn(&A, &B) -> Sc + Send + Sync,
        Sc,
    >
    where
        Sc: Copy,
    {
        self.into_weighted_builder(
            ImpactType::Reward,
            move |_: &A, _: &B| weight,
            fixed_weight_is_hard(weight),
        )
    }

    // Rewards each matching pair with a dynamic weight.
    pub fn reward_with<W>(
        self,
        weight_fn: W,
    ) -> CrossBiConstraintBuilder<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
    where
        W: Fn(&A, &B) -> Sc + Send + Sync,
    {
        self.into_weighted_builder(ImpactType::Reward, weight_fn, false)
    }

    // Rewards each matching pair with a dynamic weight, explicitly marked as hard.
    pub fn reward_hard_with<W>(
        self,
        weight_fn: W,
    ) -> CrossBiConstraintBuilder<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
    where
        W: Fn(&A, &B) -> Sc + Send + Sync,
    {
        self.into_weighted_builder(ImpactType::Reward, weight_fn, true)
    }

    // Penalizes each matching pair with one hard score unit.
    pub fn penalize_hard(
        self,
    ) -> CrossBiConstraintBuilder<
        S,
        A,
        B,
        K,
        EA,
        EB,
        KA,
        KB,
        F,
        impl Fn(&A, &B) -> Sc + Send + Sync,
        Sc,
    >
    where
        Sc: Copy,
    {
        self.penalize(Sc::one_hard())
    }

    // Penalizes each matching pair with one soft score unit.
    pub fn penalize_soft(
        self,
    ) -> CrossBiConstraintBuilder<
        S,
        A,
        B,
        K,
        EA,
        EB,
        KA,
        KB,
        F,
        impl Fn(&A, &B) -> Sc + Send + Sync,
        Sc,
    >
    where
        Sc: Copy,
    {
        self.penalize(Sc::one_soft())
    }

    // Rewards each matching pair with one hard score unit.
    pub fn reward_hard(
        self,
    ) -> CrossBiConstraintBuilder<
        S,
        A,
        B,
        K,
        EA,
        EB,
        KA,
        KB,
        F,
        impl Fn(&A, &B) -> Sc + Send + Sync,
        Sc,
    >
    where
        Sc: Copy,
    {
        self.reward(Sc::one_hard())
    }

    // Rewards each matching pair with one soft score unit.
    pub fn reward_soft(
        self,
    ) -> CrossBiConstraintBuilder<
        S,
        A,
        B,
        K,
        EA,
        EB,
        KA,
        KB,
        F,
        impl Fn(&A, &B) -> Sc + Send + Sync,
        Sc,
    >
    where
        Sc: Copy,
    {
        self.reward(Sc::one_soft())
    }
}

// Zero-erasure builder for finalizing a cross-bi constraint.
pub struct CrossBiConstraintBuilder<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
where
    Sc: Score,
{
    extractor_a: EA,
    extractor_b: EB,
    key_a: KA,
    key_b: KB,
    filter: F,
    impact_type: ImpactType,
    weight: W,
    is_hard: bool,
    _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> B, fn() -> K, fn() -> Sc)>,
}

impl<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
    CrossBiConstraintBuilder<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    EA: CollectionExtract<S, Item = A> + Clone,
    EB: CollectionExtract<S, Item = B> + Clone,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    F: BiFilter<S, A, B>,
    W: Fn(&A, &B) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    pub fn named(
        self,
        name: &str,
    ) -> IncrementalCrossBiConstraint<
        S,
        A,
        B,
        K,
        EA,
        EB,
        KA,
        KB,
        impl Fn(&S, &A, &B) -> bool + Send + Sync,
        impl Fn(&S, usize, usize) -> Sc + Send + Sync,
        Sc,
    > {
        let filter = self.filter;
        let combined_filter = move |s: &S, a: &A, b: &B| filter.test(s, a, b, 0, 0);

        let extractor_a = self.extractor_a.clone();
        let extractor_b = self.extractor_b.clone();
        let weight = self.weight;
        let adapted_weight = move |s: &S, a_idx: usize, b_idx: usize| {
            let entities_a = extractor_a.extract(s);
            let entities_b = extractor_b.extract(s);
            let a = &entities_a[a_idx];
            let b = &entities_b[b_idx];
            weight(a, b)
        };

        IncrementalCrossBiConstraint::new(
            ConstraintRef::new("", name),
            self.impact_type,
            self.extractor_a,
            self.extractor_b,
            self.key_a,
            self.key_b,
            combined_filter,
            adapted_weight,
            self.is_hard,
        )
    }
}

impl<S, A, B, K, EA, EB, KA, KB, F, W, Sc: Score> std::fmt::Debug
    for CrossBiConstraintBuilder<S, A, B, K, EA, EB, KA, KB, F, W, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CrossBiConstraintBuilder")
            .field("impact_type", &self.impact_type)
            .finish()
    }
}
