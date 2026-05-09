use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::constraint::cross_bi_incremental::{IncrementalCrossBiConstraint, PairWeight};

use super::super::collection_extract::CollectionExtract;
use super::super::filter::BiFilter;
use super::super::weighting_support::ConstraintWeight;
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

    pub fn penalize<W>(
        self,
        weight: W,
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
        W: for<'w> ConstraintWeight<(&'w A, &'w B), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            ImpactType::Penalty,
            move |a: &A, b: &B| weight.score((a, b)),
            is_hard,
        )
    }

    pub fn reward<W>(
        self,
        weight: W,
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
        W: for<'w> ConstraintWeight<(&'w A, &'w B), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            ImpactType::Reward,
            move |a: &A, b: &B| weight.score((a, b)),
            is_hard,
        )
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
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
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
        PairWeight<W>,
        Sc,
    > {
        let filter = self.filter;
        let combined_filter = move |s: &S, a: &A, b: &B| filter.test(s, a, b, 0, 0);

        IncrementalCrossBiConstraint::new_pair_weight(
            ConstraintRef::new("", name),
            self.impact_type,
            self.extractor_a,
            self.extractor_b,
            self.key_a,
            self.key_b,
            combined_filter,
            self.weight,
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
