use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::constraint::exists::{IncrementalExistsConstraint, SelfFlatten};
use crate::stream::collection_extract::{FlattenExtract, TrackedCollectionExtract};
use crate::stream::filter::UniFilter;
use crate::stream::weighting_support::fixed_weight_is_hard;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExistenceMode {
    Exists,
    NotExists,
}

pub struct ExistsConstraintStream<S, A, P, B, K, EA, EP, KA, KB, FA, FP, Flatten, Sc>
where
    Sc: Score,
{
    pub(super) mode: ExistenceMode,
    pub(super) extractor_a: EA,
    pub(super) extractor_parent: EP,
    pub(super) key_a: KA,
    pub(super) key_b: KB,
    pub(super) filter_a: FA,
    pub(super) filter_parent: FP,
    pub(super) flatten: Flatten,
    pub(super) _phantom: PhantomData<(
        fn() -> S,
        fn() -> A,
        fn() -> P,
        fn() -> B,
        fn() -> K,
        fn() -> Sc,
    )>,
}

impl<S, A, P, B, K, EA, EP, KA, KB, FA, FP, Flatten, Sc>
    ExistsConstraintStream<S, A, P, B, K, EA, EP, KA, KB, FA, FP, Flatten, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    P: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    EA: TrackedCollectionExtract<S, Item = A>,
    EP: TrackedCollectionExtract<S, Item = P>,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    FA: UniFilter<S, A>,
    FP: UniFilter<S, P>,
    Flatten: FlattenExtract<P, Item = B>,
    Sc: Score + 'static,
{
    fn into_weighted_builder<W>(
        self,
        impact_type: ImpactType,
        weight: W,
        is_hard: bool,
    ) -> ExistsConstraintBuilder<S, A, P, B, K, EA, EP, KA, KB, FA, FP, Flatten, W, Sc>
    where
        W: Fn(&A) -> Sc + Send + Sync,
    {
        ExistsConstraintBuilder {
            mode: self.mode,
            extractor_a: self.extractor_a,
            extractor_parent: self.extractor_parent,
            key_a: self.key_a,
            key_b: self.key_b,
            filter_a: self.filter_a,
            filter_parent: self.filter_parent,
            flatten: self.flatten,
            impact_type,
            weight,
            is_hard,
            _phantom: PhantomData,
        }
    }

    pub fn new(
        mode: ExistenceMode,
        extractor_a: EA,
        extractor_parent: EP,
        keys: (KA, KB),
        filter_a: FA,
        filter_parent: FP,
        flatten: Flatten,
    ) -> Self {
        let (key_a, key_b) = keys;
        Self {
            mode,
            extractor_a,
            extractor_parent,
            key_a,
            key_b,
            filter_a,
            filter_parent,
            flatten,
            _phantom: PhantomData,
        }
    }

    pub fn penalize(
        self,
        weight: Sc,
    ) -> ExistsConstraintBuilder<
        S,
        A,
        P,
        B,
        K,
        EA,
        EP,
        KA,
        KB,
        FA,
        FP,
        Flatten,
        impl Fn(&A) -> Sc + Send + Sync,
        Sc,
    >
    where
        Sc: Copy,
    {
        self.into_weighted_builder(
            ImpactType::Penalty,
            move |_: &A| weight,
            fixed_weight_is_hard(weight),
        )
    }

    pub fn penalize_with<W>(
        self,
        weight_fn: W,
    ) -> ExistsConstraintBuilder<S, A, P, B, K, EA, EP, KA, KB, FA, FP, Flatten, W, Sc>
    where
        W: Fn(&A) -> Sc + Send + Sync,
    {
        self.into_weighted_builder(ImpactType::Penalty, weight_fn, false)
    }

    pub fn penalize_hard_with<W>(
        self,
        weight_fn: W,
    ) -> ExistsConstraintBuilder<S, A, P, B, K, EA, EP, KA, KB, FA, FP, Flatten, W, Sc>
    where
        W: Fn(&A) -> Sc + Send + Sync,
    {
        self.into_weighted_builder(ImpactType::Penalty, weight_fn, true)
    }

    pub fn penalize_hard(
        self,
    ) -> ExistsConstraintBuilder<
        S,
        A,
        P,
        B,
        K,
        EA,
        EP,
        KA,
        KB,
        FA,
        FP,
        Flatten,
        impl Fn(&A) -> Sc + Send + Sync,
        Sc,
    >
    where
        Sc: Copy,
    {
        self.penalize(Sc::one_hard())
    }

    pub fn penalize_soft(
        self,
    ) -> ExistsConstraintBuilder<
        S,
        A,
        P,
        B,
        K,
        EA,
        EP,
        KA,
        KB,
        FA,
        FP,
        Flatten,
        impl Fn(&A) -> Sc + Send + Sync,
        Sc,
    >
    where
        Sc: Copy,
    {
        self.penalize(Sc::one_soft())
    }

    pub fn reward(
        self,
        weight: Sc,
    ) -> ExistsConstraintBuilder<
        S,
        A,
        P,
        B,
        K,
        EA,
        EP,
        KA,
        KB,
        FA,
        FP,
        Flatten,
        impl Fn(&A) -> Sc + Send + Sync,
        Sc,
    >
    where
        Sc: Copy,
    {
        self.into_weighted_builder(
            ImpactType::Reward,
            move |_: &A| weight,
            fixed_weight_is_hard(weight),
        )
    }

    pub fn reward_with<W>(
        self,
        weight_fn: W,
    ) -> ExistsConstraintBuilder<S, A, P, B, K, EA, EP, KA, KB, FA, FP, Flatten, W, Sc>
    where
        W: Fn(&A) -> Sc + Send + Sync,
    {
        self.into_weighted_builder(ImpactType::Reward, weight_fn, false)
    }

    pub fn reward_hard_with<W>(
        self,
        weight_fn: W,
    ) -> ExistsConstraintBuilder<S, A, P, B, K, EA, EP, KA, KB, FA, FP, Flatten, W, Sc>
    where
        W: Fn(&A) -> Sc + Send + Sync,
    {
        self.into_weighted_builder(ImpactType::Reward, weight_fn, true)
    }

    pub fn reward_hard(
        self,
    ) -> ExistsConstraintBuilder<
        S,
        A,
        P,
        B,
        K,
        EA,
        EP,
        KA,
        KB,
        FA,
        FP,
        Flatten,
        impl Fn(&A) -> Sc + Send + Sync,
        Sc,
    >
    where
        Sc: Copy,
    {
        self.reward(Sc::one_hard())
    }

    pub fn reward_soft(
        self,
    ) -> ExistsConstraintBuilder<
        S,
        A,
        P,
        B,
        K,
        EA,
        EP,
        KA,
        KB,
        FA,
        FP,
        Flatten,
        impl Fn(&A) -> Sc + Send + Sync,
        Sc,
    >
    where
        Sc: Copy,
    {
        self.reward(Sc::one_soft())
    }
}

impl<S, A, P, B, K, EA, EP, KA, KB, FA, FP, Flatten, Sc: Score> std::fmt::Debug
    for ExistsConstraintStream<S, A, P, B, K, EA, EP, KA, KB, FA, FP, Flatten, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExistsConstraintStream")
            .field("mode", &self.mode)
            .finish()
    }
}

pub struct ExistsConstraintBuilder<S, A, P, B, K, EA, EP, KA, KB, FA, FP, Flatten, W, Sc>
where
    Sc: Score,
{
    mode: ExistenceMode,
    extractor_a: EA,
    extractor_parent: EP,
    key_a: KA,
    key_b: KB,
    filter_a: FA,
    filter_parent: FP,
    flatten: Flatten,
    impact_type: ImpactType,
    weight: W,
    is_hard: bool,
    _phantom: PhantomData<(
        fn() -> S,
        fn() -> A,
        fn() -> P,
        fn() -> B,
        fn() -> K,
        fn() -> Sc,
    )>,
}

impl<S, A, P, B, K, EA, EP, KA, KB, FA, FP, Flatten, W, Sc>
    ExistsConstraintBuilder<S, A, P, B, K, EA, EP, KA, KB, FA, FP, Flatten, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    P: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    EA: TrackedCollectionExtract<S, Item = A>,
    EP: TrackedCollectionExtract<S, Item = P>,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
    FA: UniFilter<S, A> + Send + Sync,
    FP: UniFilter<S, P> + Send + Sync,
    Flatten: FlattenExtract<P, Item = B> + Send + Sync,
    W: Fn(&A) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    pub fn named(
        self,
        name: &str,
    ) -> IncrementalExistsConstraint<S, A, P, B, K, EA, EP, KA, KB, FA, FP, Flatten, W, Sc> {
        IncrementalExistsConstraint::new(
            ConstraintRef::new("", name),
            self.impact_type,
            self.mode,
            self.extractor_a,
            self.extractor_parent,
            self.key_a,
            self.key_b,
            self.filter_a,
            self.filter_parent,
            self.flatten,
            self.weight,
            self.is_hard,
        )
    }
}

impl<S, A, P, B, K, EA, EP, KA, KB, FA, FP, Flatten, W, Sc: Score> std::fmt::Debug
    for ExistsConstraintBuilder<S, A, P, B, K, EA, EP, KA, KB, FA, FP, Flatten, W, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExistsConstraintBuilder")
            .field("mode", &self.mode)
            .field("impact_type", &self.impact_type)
            .finish()
    }
}

pub(crate) type DirectExistenceStream<S, A, B, K, EA, EP, KA, KB, FA, FP, Sc> =
    ExistsConstraintStream<S, A, B, B, K, EA, EP, KA, KB, FA, FP, SelfFlatten, Sc>;
