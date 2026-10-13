use super::collector::{Accumulator, Collector};
use super::filter::TriFilter;
use super::joiner::plan::{CompileCondition, IndexedPlan};
use super::relational::operator::{FilterNode, Operator, Pair};
use super::relational::{Leaf, TupleCollector};
use super::JoinedGroupedStream;
use solverforge_core::score::Score;

type Row<'a, A, B, C> = Pair<Pair<Leaf<'a, A>, Leaf<'a, B>>, Leaf<'a, C>>;

impl<S, A, B, C, EA, EB, EC, P1, P2, F, Sc> super::cross::Tri<S, A, B, C, EA, EB, EC, P1, P2, F, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    EA: 'static,
    EB: 'static,
    EC: 'static,
    P1: CompileCondition<Plan: IndexedPlan> + 'static,
    P2: CompileCondition<Plan: IndexedPlan> + 'static,
    for<'a> super::cross_tri_stream::base::BuiltTri<S, EA, EB, EC, P1, P2>:
        Operator<S, View<'a> = Row<'a, A, B, C>> + Send + Sync,
    F: TriFilter<S, A, B, C> + 'static,
    Sc: Score + 'static,
{
    /// Groups joined tuples by a key and an owned-value collector.
    /// Upstream filters remain binding on full and incremental evaluation.
    pub fn group_by<GK, GF, Co, V, R, Acc>(
        self,
        key: GF,
        collector: Co,
    ) -> JoinedGroupedStream<
        S,
        FilterNode<
            super::cross_tri_stream::base::BuiltTri<S, EA, EB, EC, P1, P2>,
            impl Fn(&S, &Row<'_, A, B, C>) -> bool + Send + Sync,
        >,
        impl Fn(&Row<'_, A, B, C>) -> GK + Send + Sync,
        TupleCollector<Co, V, R, Acc>,
        GK,
        V,
        R,
        Acc,
        Sc,
    >
    where
        GK: Eq + std::hash::Hash + Clone + Send + Sync + 'static,
        GF: Fn(&A, &B, &C) -> GK + Send + Sync + 'static,
        Co: for<'i> Collector<(&'i A, &'i B, &'i C), Value = V, Result = R, Accumulator = Acc>
            + 'static,
        V: Send + Sync + 'static,
        R: Send + Sync + 'static,
        Acc: Accumulator<V, R> + 'static,
    {
        let filter = self.filter;
        let input = FilterNode::new(self.tree, move |s: &S, row: &Row<'_, A, B, C>| {
            filter.test(
                s,
                row.left.left.entity,
                row.left.right.entity,
                row.right.entity,
                row.left.left.index,
                row.left.right.index,
                row.right.index,
            )
        });
        JoinedGroupedStream::new(
            input,
            move |row: &Row<'_, A, B, C>| {
                key(
                    row.left.left.entity,
                    row.left.right.entity,
                    row.right.entity,
                )
            },
            TupleCollector::new(collector),
        )
    }
}
