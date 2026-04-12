use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::ImpactType;

use super::super::collection_extract::CollectionExtract;
use super::super::filter::BiFilter;
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
    // Penalizes each matching (A, C) pair with a fixed weight.
    pub fn penalize(
        self,
        weight: Sc,
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
        Sc: Copy,
    {
        let is_hard = weight
            .to_level_numbers()
            .first()
            .map(|&h| h != 0)
            .unwrap_or(false);
        FlattenedBiConstraintBuilder {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            flatten: self.flatten,
            c_key_fn: self.c_key_fn,
            a_lookup_fn: self.a_lookup_fn,
            filter: self.filter,
            impact_type: ImpactType::Penalty,
            weight: move |_: &A, _: &C| weight,
            is_hard,
            _phantom: PhantomData,
        }
    }

    // Penalizes each matching (A, C) pair with a dynamic weight.
    pub fn penalize_with<W>(
        self,
        weight_fn: W,
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
            impact_type: ImpactType::Penalty,
            weight: weight_fn,
            is_hard: false,
            _phantom: PhantomData,
        }
    }

    // Penalizes each matching (A, C) pair with a dynamic weight, explicitly marked as hard.
    pub fn penalize_hard_with<W>(
        self,
        weight_fn: W,
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
            impact_type: ImpactType::Penalty,
            weight: weight_fn,
            is_hard: true,
            _phantom: PhantomData,
        }
    }

    // Rewards each matching (A, C) pair with a fixed weight.
    pub fn reward(
        self,
        weight: Sc,
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
        Sc: Copy,
    {
        let is_hard = weight
            .to_level_numbers()
            .first()
            .map(|&h| h != 0)
            .unwrap_or(false);
        FlattenedBiConstraintBuilder {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            flatten: self.flatten,
            c_key_fn: self.c_key_fn,
            a_lookup_fn: self.a_lookup_fn,
            filter: self.filter,
            impact_type: ImpactType::Reward,
            weight: move |_: &A, _: &C| weight,
            is_hard,
            _phantom: PhantomData,
        }
    }

    // Penalizes each matching (A, C) pair with one hard score unit.
    pub fn penalize_hard(
        self,
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
        Sc: Copy,
    {
        self.penalize(Sc::one_hard())
    }

    // Penalizes each matching (A, C) pair with one soft score unit.
    pub fn penalize_soft(
        self,
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
        Sc: Copy,
    {
        self.penalize(Sc::one_soft())
    }

    // Rewards each matching (A, C) pair with one hard score unit.
    pub fn reward_hard(
        self,
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
        Sc: Copy,
    {
        self.reward(Sc::one_hard())
    }

    // Rewards each matching (A, C) pair with one soft score unit.
    pub fn reward_soft(
        self,
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
        Sc: Copy,
    {
        self.reward(Sc::one_soft())
    }

    // Rewards each matching (A, C) pair with a dynamic weight.
    pub fn reward_with<W>(
        self,
        weight_fn: W,
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
            impact_type: ImpactType::Reward,
            weight: weight_fn,
            is_hard: false,
            _phantom: PhantomData,
        }
    }
}
