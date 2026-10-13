use std::hash::Hash;
use std::marker::PhantomData;

use super::collector::{Accumulator, Collector};
use super::relational::operator::{ExplainRow, GroupNode, GroupView, Operator};
use super::weighting_support::ConstraintWeight;
use crate::constraint::relational::OperatorTerminal;
use solverforge_core::{score::Score, ConstraintRef, ImpactType};

/// Grouped joined rows, backed by the common grouping operator.
pub struct JoinedGroupedStream<S, O, F, C, K, V, R, Acc: Accumulator<V, R>, Sc> {
    node: GroupNode<O, F, C, K, Acc, V, R>,
    marker: PhantomData<fn() -> (S, Sc)>,
}

impl<S, O, F, C, K, V, R, Acc: Accumulator<V, R>, Sc>
    JoinedGroupedStream<S, O, F, C, K, V, R, Acc, Sc>
{
    pub(crate) fn new(input: O, key: F, collector: C) -> Self {
        Self {
            node: GroupNode::new(input, key, collector),
            marker: PhantomData,
        }
    }
}

impl<S, O, F, C, K, V, R, Acc, Sc> JoinedGroupedStream<S, O, F, C, K, V, R, Acc, Sc>
where
    S: Send + Sync + 'static,
    O: Operator<S> + Send + Sync,
    F: for<'a> Fn(&O::View<'a>) -> K + Send + Sync + 'static,
    C: for<'a> Collector<O::View<'a>, Value = V, Result = R, Accumulator = Acc> + 'static,
    K: Eq + Hash + Clone + Send + Sync + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + 'static,
    Sc: Score + 'static,
{
    /// Penalizes each group using its key and borrowed collector result.
    pub fn penalize<W>(
        self,
        weight: W,
    ) -> JoinedGroupedBuilder<S, O, F, C, K, V, R, Acc, impl Fn(&K, &R) -> Sc + Send + Sync, Sc>
    where
        W: for<'a> ConstraintWeight<(&'a K, &'a R), Sc> + Send + Sync,
    {
        let hard = weight.is_hard();
        JoinedGroupedBuilder {
            node: self.node,
            weight: move |k: &K, r: &R| weight.score((k, r)),
            impact: ImpactType::Penalty,
            hard,
            marker: PhantomData,
        }
    }

    /// Rewards each group using its key and borrowed collector result.
    pub fn reward<W>(
        self,
        weight: W,
    ) -> JoinedGroupedBuilder<S, O, F, C, K, V, R, Acc, impl Fn(&K, &R) -> Sc + Send + Sync, Sc>
    where
        W: for<'a> ConstraintWeight<(&'a K, &'a R), Sc> + Send + Sync,
    {
        let hard = weight.is_hard();
        JoinedGroupedBuilder {
            node: self.node,
            weight: move |k: &K, r: &R| weight.score((k, r)),
            impact: ImpactType::Reward,
            hard,
            marker: PhantomData,
        }
    }
}

/// Finalizer for grouped joined rows.
pub struct JoinedGroupedBuilder<S, O, F, C, K, V, R, Acc: Accumulator<V, R>, W, Sc> {
    node: GroupNode<O, F, C, K, Acc, V, R>,
    weight: W,
    impact: ImpactType,
    hard: bool,
    marker: PhantomData<fn() -> (S, Sc)>,
}

impl<S, O, F, C, K, V, R, Acc, W, Sc> JoinedGroupedBuilder<S, O, F, C, K, V, R, Acc, W, Sc>
where
    S: Send + Sync + 'static,
    O: Operator<S> + Send + Sync,
    for<'a> O::View<'a>: ExplainRow,
    F: for<'a> Fn(&O::View<'a>) -> K + Send + Sync + 'static,
    C: for<'a> Collector<O::View<'a>, Value = V, Result = R, Accumulator = Acc> + 'static,
    K: Eq + Hash + Clone + Send + Sync + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    Acc: Accumulator<V, R> + 'static,
    W: Fn(&K, &R) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    /// Finalizes the grouped constraint without copying collector results.
    pub fn named(
        self,
        name: &str,
    ) -> OperatorTerminal<
        S,
        GroupNode<O, F, C, K, Acc, V, R>,
        impl Fn(&S, &GroupView<'_, S, O, K, Acc, V, R>) -> Sc + Send + Sync,
        Sc,
    > {
        let weight = self.weight;
        OperatorTerminal::new(
            ConstraintRef::new("", name),
            self.impact,
            self.node,
            move |_: &S, row: &GroupView<'_, S, O, K, Acc, V, R>| {
                row.with_result(|r| weight(row.key, r))
            },
            self.hard,
        )
    }
}
