use super::collector::{Accumulator, Collector};
use super::filter::QuadFilter;
use super::joiner::plan::{CompileCondition, IndexedPlan};
use super::relational::operator::{FilterNode, Operator, Pair};
use super::relational::{Leaf, TupleCollector};
use super::JoinedGroupedStream;
use solverforge_core::score::Score;

type Row<'a, A, B, C, D> = Pair<Pair<Pair<Leaf<'a, A>, Leaf<'a, B>>, Leaf<'a, C>>, Leaf<'a, D>>;

impl<S, A, B, C, D, EA, EB, EC, ED, P1, P2, P3, F, Sc>
    super::cross::Quad<S, A, B, C, D, EA, EB, EC, ED, P1, P2, P3, F, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    D: Clone + Send + Sync + 'static,
    EA: 'static,
    EB: 'static,
    EC: 'static,
    ED: 'static,
    P1: CompileCondition<Plan: IndexedPlan> + 'static,
    P2: CompileCondition<Plan: IndexedPlan> + 'static,
    P3: CompileCondition<Plan: IndexedPlan> + 'static,
    for<'a> super::cross_quad_stream::BuiltQuad<S, EA, EB, EC, ED, P1, P2, P3>:
        Operator<S, View<'a> = Row<'a, A, B, C, D>> + Send + Sync,
    F: QuadFilter<S, A, B, C, D> + 'static,
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
            super::cross_quad_stream::BuiltQuad<S, EA, EB, EC, ED, P1, P2, P3>,
            impl Fn(&S, &Row<'_, A, B, C, D>) -> bool + Send + Sync,
        >,
        impl Fn(&Row<'_, A, B, C, D>) -> GK + Send + Sync,
        TupleCollector<Co, V, R, Acc>,
        GK,
        V,
        R,
        Acc,
        Sc,
    >
    where
        GK: Eq + std::hash::Hash + Clone + Send + Sync + 'static,
        GF: Fn(&A, &B, &C, &D) -> GK + Send + Sync + 'static,
        Co: for<'i> Collector<
                (&'i A, &'i B, &'i C, &'i D),
                Value = V,
                Result = R,
                Accumulator = Acc,
            > + 'static,
        V: Send + Sync + 'static,
        R: Send + Sync + 'static,
        Acc: Accumulator<V, R> + 'static,
    {
        let filter = self.filter;
        let input = FilterNode::new(self.tree, move |s: &S, row: &Row<'_, A, B, C, D>| {
            filter.test(
                s,
                row.left.left.left.entity,
                row.left.left.right.entity,
                row.left.right.entity,
                row.right.entity,
                row.left.left.left.index,
                row.left.left.right.index,
                row.left.right.index,
                row.right.index,
            )
        });
        JoinedGroupedStream::new(
            input,
            move |row: &Row<'_, A, B, C, D>| {
                key(
                    row.left.left.left.entity,
                    row.left.left.right.entity,
                    row.left.right.entity,
                    row.right.entity,
                )
            },
            TupleCollector::new(collector),
        )
    }
}
