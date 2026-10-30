use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::ImpactType;

use super::super::collection_extract::CollectionExtract;
use super::super::collector::{Accumulator, Collector};
use super::super::filter::UniFilter;
use super::super::relational::leaf_collector::{leaf_key, LeafCollector, LeafFilter};
use super::super::relational::operator::{CollectionNode, FilterNode, GroupNode};
use crate::constraint::relational::{
    GroupAdapter, GroupWeight, OperatorScorer, SharedOperatorBuilder, SharedOperatorTerminal,
};

type SourceInput<S, A, E, Fi> = FilterNode<CollectionNode<S, E>, LeafFilter<S, A, Fi>>;

/// Zero-erasure builder for finalizing a grouped constraint.
pub struct GroupedConstraintBuilder<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc>
where
    Sc: Score,
{
    pub(super) extractor: E,
    pub(super) filter: Fi,
    pub(super) key_fn: KF,
    pub(super) collector: C,
    pub(super) impact_type: ImpactType,
    pub(super) weight_fn: W,
    pub(super) is_hard: bool,
    pub(super) _phantom: PhantomData<(
        fn() -> S,
        fn() -> A,
        fn() -> K,
        fn() -> V,
        fn() -> R,
        fn() -> Acc,
        fn() -> Sc,
    )>,
}

impl<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc>
    GroupedConstraintBuilder<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    K: Clone + Eq + Hash + Send + Sync + 'static,
    E: CollectionExtract<S, Item = A> + 'static,
    Fi: UniFilter<S, A> + 'static,
    KF: Fn(&A) -> K + Send + Sync + 'static,
    C: for<'i> Collector<&'i A, Value = V, Result = R, Accumulator = Acc> + Send + Sync + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + Send + Sync + 'static,
    W: Fn(&K, &R) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    /// Names the constraint and builds it as a
    /// [`SharedOperatorTerminal`](crate::constraint::relational::SharedOperatorTerminal),
    /// to which further terminals over the same grouped node can be appended.
    pub fn named(
        self,
        name: &str,
    ) -> SharedOperatorTerminal<
        S,
        GroupNode<
            SourceInput<S, A, E, Fi>,
            impl Fn(&super::super::relational::Leaf<'_, A>) -> K + Send + Sync,
            LeafCollector<C, V, R, Acc>,
            K,
            Acc,
            V,
            R,
        >,
        GroupAdapter<K, R>,
        OperatorScorer<GroupWeight<W>, Sc>,
        Sc,
    > {
        let tree = GroupNode::new(
            FilterNode::new(
                CollectionNode::new(self.extractor, 0),
                LeafFilter::new(self.filter),
            ),
            leaf_key(self.key_fn),
            LeafCollector::new(self.collector),
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

impl<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc: Score> std::fmt::Debug
    for GroupedConstraintBuilder<S, A, K, E, Fi, KF, C, V, R, Acc, W, Sc>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GroupedConstraintBuilder")
            .field("impact_type", &self.impact_type)
            .finish()
    }
}
