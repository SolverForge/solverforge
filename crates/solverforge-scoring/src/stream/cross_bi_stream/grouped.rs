use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::ImpactType;

use crate::constraint::relational::{
    GroupAdapter, GroupWeight, OperatorScorer, SharedOperatorBuilder, SharedOperatorTerminal,
};
use crate::stream::collection_extract::CollectionExtract;
use crate::stream::collector::{Accumulator, Collector};
use crate::stream::filter::BiFilter;
use crate::stream::relational::operator::{CollectionNode, GroupNode, JoinNode, Pair};
use crate::stream::relational::{Leaf, TupleCollector};
use crate::stream::weighting_support::ConstraintWeight;

use super::complemented_grouped::ComplementedGrouped;
use super::scored::{BiScored, BiUnaryPlan};

type BiInput<S, A, B, EA, EB, JK, KA, KB, F> =
    BiScored<S, A, B, EA, EB, BiUnaryPlan<JK, KA, KB>, F>;

pub struct Grouped<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, Sc>
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
    Grouped<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, Sc>
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
    F: BiFilter<S, A, B> + 'static,
    GF: Fn(&A, &B) -> GK + Send + Sync + 'static,
    C: for<'i> Collector<(&'i A, &'i B), Value = V, Result = R, Accumulator = Acc>
        + Send
        + Sync
        + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    Sc: Score + 'static,
{
    fn into_weighted_builder<W>(
        self,
        impact_type: ImpactType,
        weight_fn: W,
        is_hard: bool,
    ) -> GroupedBuilder<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, W, Sc>
    where
        W: Fn(&GK, &R) -> Sc + Send + Sync,
    {
        GroupedBuilder {
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
    ) -> GroupedBuilder<
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
    ) -> GroupedBuilder<
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
    ) -> ComplementedGrouped<S, A, B, T, JK, GK, EA, EB, ET, KA, KB, F, GF, KT, C, V, R, Acc, D, Sc>
    where
        T: Clone + Send + Sync + 'static,
        ET: CollectionExtract<S, Item = T>,
        KT: Fn(&T) -> GK + Send + Sync,
        D: Fn(&T) -> R + Send + Sync,
    {
        ComplementedGrouped {
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

pub struct GroupedBuilder<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, W, Sc>
where
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
    GroupedBuilder<S, A, B, JK, GK, EA, EB, KA, KB, F, GF, C, V, R, Acc, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    JK: Eq + Hash + Clone + Send + Sync + 'static,
    GK: Eq + Hash + Clone + Send + Sync + 'static,
    EA: CollectionExtract<S, Item = A> + Send + Sync + 'static,
    EB: CollectionExtract<S, Item = B> + Send + Sync + 'static,
    KA: Fn(&A) -> JK + Send + Sync + 'static,
    KB: Fn(&B) -> JK + Send + Sync + 'static,
    F: BiFilter<S, A, B> + 'static,
    GF: Fn(&A, &B) -> GK + Send + Sync + 'static,
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
    ) -> SharedOperatorTerminal<
        S,
        GroupNode<
            BiInput<S, A, B, EA, EB, JK, KA, KB, F>,
            impl Fn(&Pair<Leaf<'_, A>, Leaf<'_, B>>) -> GK + Send + Sync,
            TupleCollector<C, V, R, Acc>,
            GK,
            Acc,
            V,
            R,
        >,
        GroupAdapter<GK, R>,
        OperatorScorer<GroupWeight<W>, Sc>,
        Sc,
    > {
        let input = BiScored::new(
            JoinNode::new(
                CollectionNode::new(self.extractor_a, 0),
                CollectionNode::new(self.extractor_b, 1),
                BiUnaryPlan::new(self.key_a, self.key_b),
            ),
            self.filter,
        );
        let group_key = self.group_key_fn;
        let tree = GroupNode::new(
            input,
            move |row: &Pair<Leaf<'_, A>, Leaf<'_, B>>| {
                group_key(row.left.entity, row.right.entity)
            },
            TupleCollector::new(self.collector),
        );
        SharedOperatorBuilder::first_with_hard(
            tree,
            GroupAdapter::new(),
            self.impact_type,
            self.weight_fn,
            self.is_hard,
        )
        .first_named(name)
    }
}
