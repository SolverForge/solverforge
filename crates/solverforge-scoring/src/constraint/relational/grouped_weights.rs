use std::marker::PhantomData;

use solverforge_core::score::Score;

use crate::stream::{
    collector::{Accumulator, Collector},
    relational::{
        operator::{
            CollectionNode, ComplementNode, ComplementView, GroupNode, GroupView, Operator, Pair,
        },
        Leaf,
    },
    ConstraintWeight,
};

use super::{AdaptWeight, RowWeight};

pub struct GroupAdapter<K, R>(PhantomData<fn() -> (K, R)>);

impl<K, R> GroupAdapter<K, R> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<K, R> Default for GroupAdapter<K, R> {
    fn default() -> Self {
        Self::new()
    }
}

#[doc(hidden)]
pub struct GroupWeight<W>(W);

impl<S, I, F, C, K, Acc, V, R, W, Sc> RowWeight<S, GroupNode<I, F, C, K, Acc, V, R>, Sc>
    for GroupWeight<W>
where
    S: 'static,
    I: Operator<S>,
    F: for<'a> Fn(&I::View<'a>) -> K + Send + Sync + 'static,
    C: for<'a> Collector<I::View<'a>, Value = V, Result = R, Accumulator = Acc> + 'static,
    K: Eq + std::hash::Hash + Clone + Send + Sync + 'static,
    Acc: Accumulator<V, R> + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    W: for<'a> ConstraintWeight<(&'a K, &'a R), Sc> + Send + Sync,
    Sc: Score,
{
    fn score(&self, _solution: &S, row: &GroupView<'_, S, I, K, Acc, V, R>) -> Sc {
        row.with_result(|result| self.0.score((row.key, result)))
    }
}

impl<S, I, F, C, K, Acc, V, R, W, Sc> AdaptWeight<S, GroupNode<I, F, C, K, Acc, V, R>, W, Sc>
    for GroupAdapter<K, R>
where
    S: 'static,
    I: Operator<S>,
    F: for<'a> Fn(&I::View<'a>) -> K + Send + Sync + 'static,
    C: for<'a> Collector<I::View<'a>, Value = V, Result = R, Accumulator = Acc> + 'static,
    K: Eq + std::hash::Hash + Clone + Send + Sync + 'static,
    Acc: Accumulator<V, R> + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    W: for<'a> ConstraintWeight<(&'a K, &'a R), Sc> + Send + Sync,
    Sc: Score,
{
    type RowWeight = GroupWeight<W>;
    fn adapt(&self, weight: W) -> Self::RowWeight {
        GroupWeight(weight)
    }
    fn is_hard(&self, weight: &W) -> bool {
        weight.is_hard()
    }
}

pub struct GroupedJoinAdapter<K, R, B>(PhantomData<fn() -> (K, R, B)>);

impl<K, R, B> GroupedJoinAdapter<K, R, B> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<K, R, B> Default for GroupedJoinAdapter<K, R, B> {
    fn default() -> Self {
        Self::new()
    }
}

#[doc(hidden)]
pub struct GroupedJoinWeight<W>(W);

impl<S, I, F, C, K, Acc, V, R, EB, P, B, W, Sc>
    RowWeight<
        S,
        crate::stream::relational::operator::JoinNode<
            S,
            GroupNode<I, F, C, K, Acc, V, R>,
            CollectionNode<S, EB>,
            P,
        >,
        Sc,
    > for GroupedJoinWeight<W>
where
    S: 'static,
    I: Operator<S>,
    F: for<'a> Fn(&I::View<'a>) -> K + Send + Sync + 'static,
    C: for<'a> Collector<I::View<'a>, Value = V, Result = R, Accumulator = Acc> + 'static,
    K: Eq + std::hash::Hash + Clone + Send + Sync + 'static,
    Acc: Accumulator<V, R> + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    EB: crate::stream::collection_extract::CollectionExtract<S, Item = B> + 'static,
    P: crate::stream::joiner::plan::IndexedPlan + 'static,
    for<'a> P:
        crate::stream::joiner::plan::ExecutablePlan<GroupView<'a, S, I, K, Acc, V, R>, Leaf<'a, B>>,
    W: for<'a> ConstraintWeight<(&'a K, &'a R, &'a B), Sc> + Send + Sync,
    Sc: Score,
{
    fn score(
        &self,
        _solution: &S,
        row: &Pair<GroupView<'_, S, I, K, Acc, V, R>, Leaf<'_, B>>,
    ) -> Sc {
        row.left
            .with_result(|result| self.0.score((row.left.key, result, row.right.entity)))
    }
}

impl<S, I, F, C, K, Acc, V, R, EB, P, B, W, Sc>
    AdaptWeight<
        S,
        crate::stream::relational::operator::JoinNode<
            S,
            GroupNode<I, F, C, K, Acc, V, R>,
            CollectionNode<S, EB>,
            P,
        >,
        W,
        Sc,
    > for GroupedJoinAdapter<K, R, B>
where
    S: 'static,
    I: Operator<S>,
    F: for<'a> Fn(&I::View<'a>) -> K + Send + Sync + 'static,
    C: for<'a> Collector<I::View<'a>, Value = V, Result = R, Accumulator = Acc> + 'static,
    K: Eq + std::hash::Hash + Clone + Send + Sync + 'static,
    Acc: Accumulator<V, R> + 'static,
    V: Send + Sync + 'static,
    R: Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    EB: crate::stream::collection_extract::CollectionExtract<S, Item = B> + 'static,
    P: crate::stream::joiner::plan::IndexedPlan + 'static,
    for<'a> P:
        crate::stream::joiner::plan::ExecutablePlan<GroupView<'a, S, I, K, Acc, V, R>, Leaf<'a, B>>,
    W: for<'a> ConstraintWeight<(&'a K, &'a R, &'a B), Sc> + Send + Sync,
    Sc: Score,
{
    type RowWeight = GroupedJoinWeight<W>;
    fn adapt(&self, weight: W) -> Self::RowWeight {
        GroupedJoinWeight(weight)
    }
    fn is_hard(&self, weight: &W) -> bool {
        weight.is_hard()
    }
}

pub struct DirectComplementAdapter<KB, K, R> {
    key: KB,
    marker: PhantomData<fn() -> (K, R)>,
}

pub struct OptionalComplementAdapter<KB, K, R> {
    key: KB,
    marker: PhantomData<fn() -> (K, R)>,
}

impl<KB, K, R> DirectComplementAdapter<KB, K, R> {
    pub fn new(key: KB) -> Self {
        Self {
            key,
            marker: PhantomData,
        }
    }
}
impl<KB, K, R> OptionalComplementAdapter<KB, K, R> {
    pub fn new(key: KB) -> Self {
        Self {
            key,
            marker: PhantomData,
        }
    }
}

#[doc(hidden)]
pub struct DirectComplementWeight<W, KB>(W, KB);
#[doc(hidden)]
pub struct OptionalComplementWeight<W, KB>(W, KB);

macro_rules! impl_complement_weight {
    ($adapter:ident, $row_weight:ident, $group_key:ty) => {
        impl<S, I, F, C, K, Acc, V, R, EB, P, D, B, W, KB, Sc>
            RowWeight<
                S,
                ComplementNode<
                    S,
                    CollectionNode<S, EB>,
                    GroupNode<I, F, C, $group_key, Acc, V, R>,
                    P,
                    D,
                    R,
                >,
                Sc,
            > for $row_weight<W, KB>
        where
            S: 'static,
            I: Operator<S>,
            F: for<'a> Fn(&I::View<'a>) -> $group_key + Send + Sync + 'static,
            C: for<'a> Collector<I::View<'a>, Value = V, Result = R, Accumulator = Acc> + 'static,
            K: Eq + std::hash::Hash + Clone + Send + Sync + 'static,
            Acc: Accumulator<V, R> + 'static,
            V: Send + Sync + 'static,
            R: Send + Sync + 'static,
            B: Send + Sync + 'static,
            EB: crate::stream::collection_extract::CollectionExtract<S, Item = B> + 'static,
            P: crate::stream::joiner::plan::IndexedPlan
                + crate::stream::relational::keyed_plan::KeyOf
                + crate::stream::relational::keyed_plan::RightKeyOf<
                    Key = <P as crate::stream::relational::keyed_plan::KeyOf>::Key,
                > + 'static,
            for<'a> P: crate::stream::joiner::plan::ExecutablePlan<
                Leaf<'a, B>,
                GroupView<'a, S, I, $group_key, Acc, V, R>,
            >,
            D: for<'a> Fn(&S, &Leaf<'a, B>) -> R + Send + Sync + 'static,
            W: for<'a> ConstraintWeight<(&'a K, &'a R), Sc> + Send + Sync,
            KB: Fn(&B) -> K + Send + Sync,
            Sc: Score,
        {
            fn score(
                &self,
                _solution: &S,
                row: &ComplementView<
                    '_,
                    Leaf<'_, B>,
                    GroupView<'_, S, I, $group_key, Acc, V, R>,
                    R,
                >,
            ) -> Sc {
                match row {
                    ComplementView::Real(pair) => {
                        let key = (self.1)(pair.left.entity);
                        pair.right
                            .with_result(|result| self.0.score((&key, result)))
                    }
                    ComplementView::Default(projected) => {
                        let key = (self.1)(projected.input.entity);
                        self.0.score((&key, projected.value))
                    }
                }
            }
        }

        impl<S, I, F, C, K, Acc, V, R, EB, P, D, B, W, KB, Sc>
            AdaptWeight<
                S,
                ComplementNode<
                    S,
                    CollectionNode<S, EB>,
                    GroupNode<I, F, C, $group_key, Acc, V, R>,
                    P,
                    D,
                    R,
                >,
                W,
                Sc,
            > for $adapter<KB, K, R>
        where
            S: 'static,
            I: Operator<S>,
            F: for<'a> Fn(&I::View<'a>) -> $group_key + Send + Sync + 'static,
            C: for<'a> Collector<I::View<'a>, Value = V, Result = R, Accumulator = Acc> + 'static,
            K: Eq + std::hash::Hash + Clone + Send + Sync + 'static,
            Acc: Accumulator<V, R> + 'static,
            V: Send + Sync + 'static,
            R: Send + Sync + 'static,
            B: Send + Sync + 'static,
            EB: crate::stream::collection_extract::CollectionExtract<S, Item = B> + 'static,
            P: crate::stream::joiner::plan::IndexedPlan
                + crate::stream::relational::keyed_plan::KeyOf
                + crate::stream::relational::keyed_plan::RightKeyOf<
                    Key = <P as crate::stream::relational::keyed_plan::KeyOf>::Key,
                > + 'static,
            for<'a> P: crate::stream::joiner::plan::ExecutablePlan<
                Leaf<'a, B>,
                GroupView<'a, S, I, $group_key, Acc, V, R>,
            >,
            D: for<'a> Fn(&S, &Leaf<'a, B>) -> R + Send + Sync + 'static,
            W: for<'a> ConstraintWeight<(&'a K, &'a R), Sc> + Send + Sync,
            KB: Fn(&B) -> K + Send + Sync + Clone,
            Sc: Score,
        {
            type RowWeight = $row_weight<W, KB>;
            fn adapt(&self, weight: W) -> Self::RowWeight {
                $row_weight(weight, self.key.clone())
            }
            fn is_hard(&self, weight: &W) -> bool {
                weight.is_hard()
            }
        }
    };
}

impl_complement_weight!(DirectComplementAdapter, DirectComplementWeight, K);
impl_complement_weight!(
    OptionalComplementAdapter,
    OptionalComplementWeight,
    Option<K>
);
