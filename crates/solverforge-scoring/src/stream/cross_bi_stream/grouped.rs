use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::constraint::cross_grouped::CrossGroupedConstraint;
use crate::stream::collection_extract::CollectionExtract;
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::filter::BiFilter;
use crate::stream::weighting_support::ConstraintWeight;

use super::complemented_grouped::CrossComplementedGroupedConstraintStream;

pub struct CrossGroupedConstraintStream<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, Sc>
where
    Sc: Score,
{
    pub(super) extractor_a: EA,
    pub(super) extractor_b: EB,
    pub(super) key_a: KA,
    pub(super) key_b: KB,
    pub(super) filter: F,
    pub(super) group_key_fn: GF,
    pub(super) collector: C,
    pub(super) _phantom: PhantomData<(
        fn() -> S,
        fn() -> A,
        fn() -> B,
        fn() -> JK,
        fn() -> GK,
        fn() -> V,
        fn() -> R,
        fn() -> Acc,
        fn() -> Sc,
    )>,
}

impl<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, Sc>
    CrossGroupedConstraintStream<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    GK: Eq + Hash + Clone + Send + Sync + 'static,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    KA: Fn(&A) -> JK + Send + Sync,
    KB: Fn(&B) -> JK + Send + Sync,
    F: BiFilter<S, A, B>,
    GF: Fn(&A, &B) -> GK + Send + Sync,
    C: for<'i> Collector<(&'i A, &'i B), Value = V, Result = R, Accumulator = Acc>
        + Send
        + Sync
        + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    Sc: Score + 'static,
{
    #[doc(hidden)]
    pub fn into_shared_node_state(
        self,
    ) -> crate::constraint::cross_grouped::CrossGroupedNodeState<
        S,
        A,
        B,
        JK,
        GK,
        EA,
        EB,
        KA,
        KB,
        impl Fn(&S, &A, &B, usize, usize) -> bool + Send + Sync,
        GF,
        C,
        V,
        R,
        Acc,
    > {
        let filter = self.filter;
        let combined_filter = move |s: &S, a: &A, b: &B, a_idx: usize, b_idx: usize| {
            filter.test(s, a, b, a_idx, b_idx)
        };
        crate::constraint::cross_grouped::CrossGroupedNodeState::new(
            self.extractor_a,
            self.extractor_b,
            self.key_a,
            self.key_b,
            combined_filter,
            self.group_key_fn,
            self.collector,
        )
    }

    #[doc(hidden)]
    pub fn into_shared_constraint_set<Scorers>(
        self,
        node_name: impl Into<String>,
        scorers: Scorers,
    ) -> crate::constraint::cross_grouped::SharedCrossGroupedConstraintSet<
        S,
        A,
        B,
        JK,
        GK,
        EA,
        EB,
        KA,
        KB,
        impl Fn(&S, &A, &B, usize, usize) -> bool + Send + Sync,
        GF,
        C,
        V,
        R,
        Acc,
        Scorers,
        Sc,
    >
    where
        Scorers: crate::constraint::grouped::GroupedScorerSet<GK, R, Sc>,
    {
        crate::constraint::cross_grouped::SharedCrossGroupedConstraintSet::new(
            node_name,
            self.into_shared_node_state(),
            scorers,
        )
    }

    fn into_weighted_builder<W>(
        self,
        impact_type: ImpactType,
        weight_fn: W,
        is_hard: bool,
    ) -> CrossGroupedConstraintBuilder<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, W, Sc>
    where
        W: Fn(&GK, &R) -> Sc + Send + Sync,
    {
        CrossGroupedConstraintBuilder {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            key_a: self.key_a,
            key_b: self.key_b,
            filter: self.filter,
            group_key_fn: self.group_key_fn,
            collector: self.collector,
            impact_type,
            weight_fn,
            is_hard,
            _phantom: PhantomData,
        }
    }

    pub fn penalize<W>(
        self,
        weight: W,
    ) -> CrossGroupedConstraintBuilder<
        S,
        A,
        B,
        JK,
        GK,
        EA,
        EB,
        KA,
        KB,
        F,
        GF,
        C,
        V,
        R,
        Acc,
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
    ) -> CrossGroupedConstraintBuilder<
        S,
        A,
        B,
        JK,
        GK,
        EA,
        EB,
        KA,
        KB,
        F,
        GF,
        C,
        V,
        R,
        Acc,
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

    pub fn complement<T, ET, KT, D>(
        self,
        extractor_t: ET,
        key_t: KT,
        default_fn: D,
    ) -> CrossComplementedGroupedConstraintStream<
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
        T: Clone + Send + Sync + 'static,
        ET: CollectionExtract<S, Item = T>,
        KT: Fn(&T) -> GK + Send + Sync,
        D: Fn(&T) -> R + Send + Sync,
    {
        CrossComplementedGroupedConstraintStream {
            extractor_a: self.extractor_a,
            extractor_b: self.extractor_b,
            extractor_t,
            key_a: self.key_a,
            key_b: self.key_b,
            filter: self.filter,
            group_key_fn: self.group_key_fn,
            key_t,
            collector: self.collector,
            default_fn,
            _phantom: PhantomData,
        }
    }
}

pub struct CrossGroupedConstraintBuilder<
    S,
    A,
    B,
    JK,
    GK,
    EA,
    EB,
    KA,
    KB,
    F,
    GF,
    C,
    V,
    R,
    Acc,
    W,
    Sc,
> where
    Sc: Score,
{
    extractor_a: EA,
    extractor_b: EB,
    key_a: KA,
    key_b: KB,
    filter: F,
    group_key_fn: GF,
    collector: C,
    impact_type: ImpactType,
    weight_fn: W,
    is_hard: bool,
    _phantom: PhantomData<(
        fn() -> S,
        fn() -> A,
        fn() -> B,
        fn() -> JK,
        fn() -> GK,
        fn() -> V,
        fn() -> R,
        fn() -> Acc,
        fn() -> Sc,
    )>,
}

impl<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, W, Sc>
    CrossGroupedConstraintBuilder<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    GK: Eq + Hash + Clone + Send + Sync + 'static,
    EA: CollectionExtract<S, Item = A> + Send + Sync,
    EB: CollectionExtract<S, Item = B> + Send + Sync,
    KA: Fn(&A) -> JK + Send + Sync,
    KB: Fn(&B) -> JK + Send + Sync,
    F: BiFilter<S, A, B>,
    GF: Fn(&A, &B) -> GK + Send + Sync,
    C: for<'i> Collector<(&'i A, &'i B), Value = V, Result = R, Accumulator = Acc>
        + Send
        + Sync
        + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    W: Fn(&GK, &R) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    pub fn named(
        self,
        name: &str,
    ) -> CrossGroupedConstraint<
        S,
        A,
        B,
        JK,
        GK,
        EA,
        EB,
        KA,
        KB,
        impl Fn(&S, &A, &B, usize, usize) -> bool + Send + Sync,
        GF,
        C,
        V,
        R,
        Acc,
        W,
        Sc,
    > {
        let filter = self.filter;
        let combined_filter = move |s: &S, a: &A, b: &B, a_idx: usize, b_idx: usize| {
            filter.test(s, a, b, a_idx, b_idx)
        };
        CrossGroupedConstraint::new(
            ConstraintRef::new("", name),
            self.impact_type,
            self.extractor_a,
            self.extractor_b,
            self.key_a,
            self.key_b,
            combined_filter,
            self.group_key_fn,
            self.collector,
            self.weight_fn,
            self.is_hard,
        )
    }
}
