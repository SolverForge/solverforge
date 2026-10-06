use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use super::super::collection_extract::CollectionExtract;
use super::super::collector::{Accumulator, Collector};
use super::super::filter::UniFilter;
use super::super::joiner::plan::{CompileCondition, ExecutablePlan, IndexedPlan};
use super::super::relational::leaf_collector::{leaf_key, LeafCollector, LeafFilter};
use super::super::relational::operator::{
    CollectionNode, FilterNode, GroupNode, GroupView, JoinNode, Pair,
};
use super::super::relational::view_plan::{EntityKey, GroupKey, ViewEqualPlan};
use super::super::relational::Leaf;
use super::super::weighting_support::ConstraintWeight;
use crate::constraint::relational::OperatorTerminal;

/* The grouped source input: the leaf collection with its authored source
filter applied, then grouped. Naming this keeps the `GroupView`'s upstream
operator concrete in bounds without a closure type. */
type SourceInput<S, A, E, Fi> = FilterNode<CollectionNode<S, E>, LeafFilter<S, A, Fi>>;

/* A grouped result related to a second collection by the group key.

`GroupedJoinedStream` is produced by `GroupedConstraintStream::join`. The left
tree filters the source, groups it, and joins the group key to the second
collection. The terminal weight sees the group key, the aggregated result, and
the right entity, so a grouped aggregate is scored together with the entities
it relates to without becoming a terminal first.
*/
pub struct GroupedJoinedStream<S, A, B, K, E, EB, Fi, KF, C, V, R, Acc, KB, Sc>
where
    Sc: Score,
{
    extractor_a: E,
    filter_a: Fi,
    key_fn: KF,
    collector: C,
    extractor_b: EB,
    right_key: KB,
    _phantom: PhantomData<(
        fn() -> S,
        fn() -> A,
        fn() -> B,
        fn() -> K,
        fn() -> V,
        fn() -> R,
        fn() -> Acc,
        fn() -> Sc,
    )>,
}

impl<S, A, B, K, E, EB, Fi, KF, C, V, R, Acc, KB, Sc>
    GroupedJoinedStream<S, A, B, K, E, EB, Fi, KF, C, V, R, Acc, KB, Sc>
where
    Sc: Score,
{
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        extractor_a: E,
        filter_a: Fi,
        key_fn: KF,
        collector: C,
        extractor_b: EB,
        right_key: KB,
    ) -> Self {
        Self {
            extractor_a,
            filter_a,
            key_fn,
            collector,
            extractor_b,
            right_key,
            _phantom: PhantomData,
        }
    }
}

impl<S, A, B, K, E, EB, Fi, KF, C, V, R, Acc, KB, Sc>
    GroupedJoinedStream<S, A, B, K, E, EB, Fi, KF, C, V, R, Acc, KB, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    E: CollectionExtract<S, Item = A> + 'static,
    EB: CollectionExtract<S, Item = B> + 'static,
    Fi: UniFilter<S, A> + 'static,
    KF: Fn(&A) -> K + Send + Sync + 'static,
    C: for<'i> Collector<&'i A, Value = V, Result = R, Accumulator = Acc> + Send + Sync + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    KB: Fn(&B) -> K + Send + Sync + 'static,
    Sc: Score + 'static,
    for<'x> ViewEqualPlan<K, GroupKey, EntityKey<KB>>:
        ExecutablePlan<GroupView<'x, S, SourceInput<S, A, E, Fi>, K, Acc, V, R>, Leaf<'x, B>>,
    ViewEqualPlan<K, GroupKey, EntityKey<KB>>: IndexedPlan<Indexes: Send + Sync>
        + CompileCondition<Plan = ViewEqualPlan<K, GroupKey, EntityKey<KB>>>,
{
    fn into_weighted_builder<W>(
        self,
        impact_type: ImpactType,
        weight: W,
        is_hard: bool,
    ) -> GroupedJoinedBuilder<S, A, B, K, E, EB, Fi, KF, C, V, R, Acc, KB, W, Sc>
    where
        W: Fn(&K, &R, &B) -> Sc + Send + Sync,
    {
        GroupedJoinedBuilder {
            extractor_a: self.extractor_a,
            filter_a: self.filter_a,
            key_fn: self.key_fn,
            collector: self.collector,
            extractor_b: self.extractor_b,
            right_key: self.right_key,
            impact_type,
            weight,
            is_hard,
            _phantom: PhantomData,
        }
    }

    pub fn penalize<W>(
        self,
        weight: W,
    ) -> GroupedJoinedBuilder<
        S,
        A,
        B,
        K,
        E,
        EB,
        Fi,
        KF,
        C,
        V,
        R,
        Acc,
        KB,
        impl Fn(&K, &R, &B) -> Sc + Send + Sync,
        Sc,
    >
    where
        W: for<'w> ConstraintWeight<(&'w K, &'w R, &'w B), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            ImpactType::Penalty,
            move |k: &K, r: &R, b: &B| weight.score((k, r, b)),
            is_hard,
        )
    }

    pub fn reward<W>(
        self,
        weight: W,
    ) -> GroupedJoinedBuilder<
        S,
        A,
        B,
        K,
        E,
        EB,
        Fi,
        KF,
        C,
        V,
        R,
        Acc,
        KB,
        impl Fn(&K, &R, &B) -> Sc + Send + Sync,
        Sc,
    >
    where
        W: for<'w> ConstraintWeight<(&'w K, &'w R, &'w B), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            ImpactType::Reward,
            move |k: &K, r: &R, b: &B| weight.score((k, r, b)),
            is_hard,
        )
    }
}

pub struct GroupedJoinedBuilder<S, A, B, K, E, EB, Fi, KF, C, V, R, Acc, KB, W, Sc>
where
    Sc: Score,
{
    extractor_a: E,
    filter_a: Fi,
    key_fn: KF,
    collector: C,
    extractor_b: EB,
    right_key: KB,
    impact_type: ImpactType,
    weight: W,
    is_hard: bool,
    _phantom: PhantomData<(
        fn() -> S,
        fn() -> A,
        fn() -> B,
        fn() -> K,
        fn() -> V,
        fn() -> R,
        fn() -> Acc,
        fn() -> Sc,
    )>,
}

impl<S, A, B, K, E, EB, Fi, KF, C, V, R, Acc, KB, W, Sc>
    GroupedJoinedBuilder<S, A, B, K, E, EB, Fi, KF, C, V, R, Acc, KB, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    E: CollectionExtract<S, Item = A> + 'static,
    EB: CollectionExtract<S, Item = B> + 'static,
    Fi: UniFilter<S, A> + 'static,
    KF: Fn(&A) -> K + Send + Sync + 'static,
    C: for<'i> Collector<&'i A, Value = V, Result = R, Accumulator = Acc> + Send + Sync + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    KB: Fn(&B) -> K + Send + Sync + 'static,
    W: Fn(&K, &R, &B) -> Sc + Send + Sync,
    Sc: Score + 'static,
    for<'x> ViewEqualPlan<K, GroupKey, EntityKey<KB>>:
        ExecutablePlan<GroupView<'x, S, SourceInput<S, A, E, Fi>, K, Acc, V, R>, Leaf<'x, B>>,
    ViewEqualPlan<K, GroupKey, EntityKey<KB>>: IndexedPlan<Indexes: Send + Sync>
        + CompileCondition<Plan = ViewEqualPlan<K, GroupKey, EntityKey<KB>>>,
{
    pub fn named(
        self,
        name: &str,
    ) -> OperatorTerminal<
        S,
        JoinNode<
            S,
            GroupNode<
                SourceInput<S, A, E, Fi>,
                impl Fn(&Leaf<'_, A>) -> K + Send + Sync,
                LeafCollector<C, V, R, Acc>,
                K,
                Acc,
                V,
                R,
            >,
            CollectionNode<S, EB>,
            ViewEqualPlan<K, GroupKey, EntityKey<KB>>,
        >,
        impl for<'x> Fn(
                &S,
                &Pair<GroupView<'x, S, SourceInput<S, A, E, Fi>, K, Acc, V, R>, Leaf<'x, B>>,
            ) -> Sc
            + Send
            + Sync,
        Sc,
    > {
        let right_key = self.right_key;
        let weight = self.weight;
        let tree = JoinNode::new(
            GroupNode::new(
                FilterNode::new(
                    CollectionNode::new(self.extractor_a, 0),
                    LeafFilter::new(self.filter_a),
                ),
                leaf_key(self.key_fn),
                LeafCollector::new(self.collector),
            ),
            CollectionNode::new(self.extractor_b, 1),
            ViewEqualPlan::new(GroupKey, EntityKey::new(right_key)),
        );
        let weight_fn = move |_s: &S,
                              row: &Pair<
            GroupView<'_, S, SourceInput<S, A, E, Fi>, K, Acc, V, R>,
            Leaf<'_, B>,
        >| {
            row.left
                .with_result(|r| weight(row.left.key, r, row.right.entity))
        };
        OperatorTerminal::new(
            ConstraintRef::new("", name),
            self.impact_type,
            tree,
            weight_fn,
            self.is_hard,
        )
    }
}
