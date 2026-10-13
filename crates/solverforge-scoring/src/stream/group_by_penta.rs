use super::collector::{Accumulator, Collector};
use super::filter::PentaFilter;
use super::joiner::plan::{CompileCondition, IndexedPlan};
use super::relational::operator::{FilterNode, Operator, Pair};
use super::relational::{Leaf, TupleCollector};
use super::JoinedGroupedStream;
use solverforge_core::score::Score;

type Row<'a, A, B, C, D, E> =
    Pair<Pair<Pair<Pair<Leaf<'a, A>, Leaf<'a, B>>, Leaf<'a, C>>, Leaf<'a, D>>, Leaf<'a, E>>;

impl<S, A, B, C, D, E, EA, EB, EC, ED, EE, P1, P2, P3, P4, F, Sc>
    super::cross::Penta<S, A, B, C, D, E, EA, EB, EC, ED, EE, P1, P2, P3, P4, F, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    C: Clone + Send + Sync + 'static,
    D: Clone + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
    EA: 'static,
    EB: 'static,
    EC: 'static,
    ED: 'static,
    EE: 'static,
    P1: CompileCondition<Plan: IndexedPlan> + 'static,
    P2: CompileCondition<Plan: IndexedPlan> + 'static,
    P3: CompileCondition<Plan: IndexedPlan> + 'static,
    P4: CompileCondition<Plan: IndexedPlan> + 'static,
    for<'a> super::cross_penta_stream::BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4>:
        Operator<S, View<'a> = Row<'a, A, B, C, D, E>> + Send + Sync,
    F: PentaFilter<S, A, B, C, D, E> + 'static,
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
            super::cross_penta_stream::BuiltPenta<S, EA, EB, EC, ED, EE, P1, P2, P3, P4>,
            impl Fn(&S, &Row<'_, A, B, C, D, E>) -> bool + Send + Sync,
        >,
        impl Fn(&Row<'_, A, B, C, D, E>) -> GK + Send + Sync,
        TupleCollector<Co, V, R, Acc>,
        GK,
        V,
        R,
        Acc,
        Sc,
    >
    where
        GK: Eq + std::hash::Hash + Clone + Send + Sync + 'static,
        GF: Fn(&A, &B, &C, &D, &E) -> GK + Send + Sync + 'static,
        Co: for<'i> Collector<
                (&'i A, &'i B, &'i C, &'i D, &'i E),
                Value = V,
                Result = R,
                Accumulator = Acc,
            > + 'static,
        V: Send + Sync + 'static,
        R: Send + Sync + 'static,
        Acc: Accumulator<V, R> + 'static,
    {
        let filter = self.filter;
        let input = FilterNode::new(self.tree, move |s: &S, row: &Row<'_, A, B, C, D, E>| {
            filter.test(
                s,
                row.left.left.left.left.entity,
                row.left.left.left.right.entity,
                row.left.left.right.entity,
                row.left.right.entity,
                row.right.entity,
                row.left.left.left.left.index,
                row.left.left.left.right.index,
                row.left.left.right.index,
                row.left.right.index,
                row.right.index,
            )
        });
        JoinedGroupedStream::new(
            input,
            move |row: &Row<'_, A, B, C, D, E>| {
                key(
                    row.left.left.left.left.entity,
                    row.left.left.left.right.entity,
                    row.left.left.right.entity,
                    row.left.right.entity,
                    row.right.entity,
                )
            },
            TupleCollector::new(collector),
        )
    }
}
