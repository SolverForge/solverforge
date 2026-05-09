use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::ImpactType;

use super::super::collection_extract::CollectionExtract;
use super::super::filter::BiFilter;
use super::super::weighting_support::ConstraintWeight;
use super::base::FlattenedBiConstraintStream;
use super::builder::FlattenedBiConstraintBuilder;

impl<S, A, B, C, K, CK, EA, EB, KA, KB, Flatten, CKeyFn, ALookup, F, Sc>
    FlattenedBiConstraintStream<S, A, B, C, K, CK, EA, EB, KA, KB, Flatten, CKeyFn, ALookup, F, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    CK: Eq + Hash + Clone + Send + Sync,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    Flatten: Fn(&B) -> &[C] + Send + Sync,
    CKeyFn: Fn(&C) -> CK + Send + Sync,
    ALookup: Fn(&A) -> CK + Send + Sync,
    F: BiFilter<S, A, C>,
    Sc: Score + 'static,
{
    fn into_weighted_builder<W>(
        self,
        impact_type: ImpactType,
        weight: W,
        is_hard: bool,
    ) -> FlattenedBiConstraintBuilder<
        S,
        A,
        B,
        C,
        K,
        CK,
        EA,
        EB,
        KA,
        KB,
        Flatten,
        CKeyFn,
        ALookup,
        F,
        W,
        Sc,
    >
    where
        W: Fn(&A, &C) -> Sc + Send + Sync,
    {
        FlattenedBiConstraintBuilder {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            flatten: self.flatten,
            c_key_fn: self.c_key_fn,
            a_lookup_fn: self.a_lookup_fn,
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
    ) -> FlattenedBiConstraintBuilder<
        S,
        A,
        B,
        C,
        K,
        CK,
        EA,
        EB,
        KA,
        KB,
        Flatten,
        CKeyFn,
        ALookup,
        F,
        impl Fn(&A, &C) -> Sc + Send + Sync,
        Sc,
    >
    where
        W: for<'w> ConstraintWeight<(&'w A, &'w C), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            ImpactType::Penalty,
            move |a: &A, c: &C| weight.score((a, c)),
            is_hard,
        )
    }

    pub fn reward<W>(
        self,
        weight: W,
    ) -> FlattenedBiConstraintBuilder<
        S,
        A,
        B,
        C,
        K,
        CK,
        EA,
        EB,
        KA,
        KB,
        Flatten,
        CKeyFn,
        ALookup,
        F,
        impl Fn(&A, &C) -> Sc + Send + Sync,
        Sc,
    >
    where
        W: for<'w> ConstraintWeight<(&'w A, &'w C), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            ImpactType::Reward,
            move |a: &A, c: &C| weight.score((a, c)),
            is_hard,
        )
    }
}
