use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::constraint::cross_complemented_grouped::CrossComplementedGroupedConstraint;
use crate::stream::collection_extract::CollectionExtract;
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::filter::BiFilter;
use crate::stream::weighting_support::ConstraintWeight;

pub struct CrossComplementedGroupedConstraintStream<
    S,
    A,
    B,
    T,
    JK,
    GK,
    EA,
    EB,
    ET,
    KA,
    KB,
    F,
    GF,
    KT,
    C,
    V,
    R,
    Acc,
    D,
    Sc,
> where
    Sc: Score,
{
    pub(super) extractor_a: EA,
    pub(super) extractor_b: EB,
    pub(super) extractor_t: ET,
    pub(super) key_a: KA,
    pub(super) key_b: KB,
    pub(super) filter: F,
    pub(super) group_key_fn: GF,
    pub(super) key_t: KT,
    pub(super) collector: C,
    pub(super) default_fn: D,
    pub(super) _phantom: PhantomData<(
        fn() -> S,
        fn() -> A,
        fn() -> B,
        fn() -> T,
        fn() -> JK,
        fn() -> GK,
        fn() -> V,
        fn() -> R,
        fn() -> Acc,
        fn() -> Sc,
    )>,
}

impl<S, A, B, T, JK, GK, EA, EB, ET, KA, KB, F, GF, KT, C, V, R, Acc, D, Sc>
    CrossComplementedGroupedConstraintStream<
        S,
        A,
        B,
        T,
        JK,
        GK,
        EA,
        EB,
        ET,
        KA,
        KB,
        F,
        GF,
        KT,
        C,
        V,
        R,
        Acc,
        D,
        Sc,
    >
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    GK: Eq + Hash + Clone + Send + Sync + 'static,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    ET: CollectionExtract<S, Item = T>,
    KA: Fn(&A) -> JK + Send + Sync,
    KB: Fn(&B) -> JK + Send + Sync,
    F: BiFilter<S, A, B>,
    GF: Fn(&A, &B) -> GK + Send + Sync,
    KT: Fn(&T) -> GK + Send + Sync,
    C: for<'i> Collector<(&'i A, &'i B), Value = V, Result = R, Accumulator = Acc>
        + Send
        + Sync
        + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    D: Fn(&T) -> R + Send + Sync,
    Sc: Score + 'static,
{
    fn into_weighted_builder<W>(
        self,
        impact_type: ImpactType,
        weight_fn: W,
        is_hard: bool,
    ) -> CrossComplementedGroupedConstraintBuilder<
        S,
        A,
        B,
        T,
        JK,
        GK,
        EA,
        EB,
        ET,
        KA,
        KB,
        F,
        GF,
        KT,
        C,
        V,
        R,
        Acc,
        D,
        W,
        Sc,
    >
    where
        W: Fn(&GK, &R) -> Sc + Send + Sync,
    {
        CrossComplementedGroupedConstraintBuilder {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            extractor_t: self.extractor_t,
            key_a: self.key_a,
            key_b: self.key_b,
            filter: self.filter,
            group_key_fn: self.group_key_fn,
            key_t: self.key_t,
            collector: self.collector,
            default_fn: self.default_fn,
            impact_type,
            weight_fn,
            is_hard,
            _phantom: PhantomData,
        }
    }

    pub fn penalize<W>(
        self,
        weight: W,
    ) -> CrossComplementedGroupedConstraintBuilder<
        S,
        A,
        B,
        T,
        JK,
        GK,
        EA,
        EB,
        ET,
        KA,
        KB,
        F,
        GF,
        KT,
        C,
        V,
        R,
        Acc,
        D,
        impl Fn(&GK, &R) -> Sc + Send + Sync,
        Sc,
    >
    where
        W: for<'w> ConstraintWeight<(&'w GK, &'w R), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            ImpactType::Penalty,
            move |key: &GK, result: &R| weight.score((key, result)),
            is_hard,
        )
    }

    pub fn reward<W>(
        self,
        weight: W,
    ) -> CrossComplementedGroupedConstraintBuilder<
        S,
        A,
        B,
        T,
        JK,
        GK,
        EA,
        EB,
        ET,
        KA,
        KB,
        F,
        GF,
        KT,
        C,
        V,
        R,
        Acc,
        D,
        impl Fn(&GK, &R) -> Sc + Send + Sync,
        Sc,
    >
    where
        W: for<'w> ConstraintWeight<(&'w GK, &'w R), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            ImpactType::Reward,
            move |key: &GK, result: &R| weight.score((key, result)),
            is_hard,
        )
    }
}

pub struct CrossComplementedGroupedConstraintBuilder<
    S,
    A,
    B,
    T,
    JK,
    GK,
    EA,
    EB,
    ET,
    KA,
    KB,
    F,
    GF,
    KT,
    C,
    V,
    R,
    Acc,
    D,
    W,
    Sc,
> where
    Sc: Score,
{
    extractor_a: EA,
    extractor_b: EB,
    extractor_t: ET,
    key_a: KA,
    key_b: KB,
    filter: F,
    group_key_fn: GF,
    key_t: KT,
    collector: C,
    default_fn: D,
    impact_type: ImpactType,
    weight_fn: W,
    is_hard: bool,
    _phantom: PhantomData<(
        fn() -> S,
        fn() -> A,
        fn() -> B,
        fn() -> T,
        fn() -> JK,
        fn() -> GK,
        fn() -> V,
        fn() -> R,
        fn() -> Acc,
        fn() -> Sc,
    )>,
}

impl<S, A, B, T, JK, GK, EA, EB, ET, KA, KB, F, GF, KT, C, V, R, Acc, D, W, Sc>
    CrossComplementedGroupedConstraintBuilder<
        S,
        A,
        B,
        T,
        JK,
        GK,
        EA,
        EB,
        ET,
        KA,
        KB,
        F,
        GF,
        KT,
        C,
        V,
        R,
        Acc,
        D,
        W,
        Sc,
    >
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    GK: Eq + Hash + Clone + Send + Sync + 'static,
    EA: CollectionExtract<S, Item = A> + Send + Sync,
    EB: CollectionExtract<S, Item = B> + Send + Sync,
    ET: CollectionExtract<S, Item = T> + Send + Sync,
    KA: Fn(&A) -> JK + Send + Sync,
    KB: Fn(&B) -> JK + Send + Sync,
    F: BiFilter<S, A, B>,
    GF: Fn(&A, &B) -> GK + Send + Sync,
    KT: Fn(&T) -> GK + Send + Sync,
    C: for<'i> Collector<(&'i A, &'i B), Value = V, Result = R, Accumulator = Acc>
        + Send
        + Sync
        + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    D: Fn(&T) -> R + Send + Sync,
    W: Fn(&GK, &R) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    pub fn named(
        self,
        name: &str,
    ) -> CrossComplementedGroupedConstraint<
        S,
        A,
        B,
        T,
        JK,
        GK,
        EA,
        EB,
        ET,
        KA,
        KB,
        impl Fn(&S, &A, &B, usize, usize) -> bool + Send + Sync,
        GF,
        KT,
        C,
        V,
        R,
        Acc,
        D,
        W,
        Sc,
    > {
        let filter = self.filter;
        let combined_filter = move |s: &S, a: &A, b: &B, a_idx: usize, b_idx: usize| {
            filter.test(s, a, b, a_idx, b_idx)
        };
        CrossComplementedGroupedConstraint::new(
            ConstraintRef::new("", name),
            self.impact_type,
            self.extractor_a,
            self.extractor_b,
            self.extractor_t,
            self.key_a,
            self.key_b,
            combined_filter,
            self.group_key_fn,
            self.key_t,
            self.collector,
            self.default_fn,
            self.weight_fn,
            self.is_hard,
        )
    }
}
