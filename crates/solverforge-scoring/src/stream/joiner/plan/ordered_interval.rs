use super::{CompileCondition, ExecutablePlan, IndexedPlan};
use crate::stream::joiner::*;
use crate::stream::relational::{
    index::{IntervalIndex, OrderedIndex},
    RowHandle,
};
use std::borrow::Cow;

macro_rules! ordered_plan {
    ($ty:ident, $right_query:ident, $left_query:ident, $inclusive:expr) => {
        impl<FA, FB, K> CompileCondition for $ty<FA, FB, K> {
            type Plan = Self;
            fn compile(self) -> Self {
                self
            }
        }
        impl<FA, FB, K: Ord + Clone> IndexedPlan for $ty<FA, FB, K> {
            type Kind = super::ResidualKind;
            type Indexes = (OrderedIndex<K>, OrderedIndex<K>);
            fn new_indexes(&self) -> Self::Indexes {
                (OrderedIndex::new(), OrderedIndex::new())
            }
            fn remove_left(&self, i: &mut Self::Indexes, h: RowHandle) {
                i.0.remove(h);
            }
            fn remove_right(&self, i: &mut Self::Indexes, h: RowHandle) {
                i.1.remove(h);
            }
        }
        impl<L, R, FA, FB, K> ExecutablePlan<L, R> for $ty<FA, FB, K>
        where
            K: Ord + Clone,
            FA: Fn(&L) -> K + Send + Sync,
            FB: Fn(&R) -> K + Send + Sync,
        {
            fn insert_left(&self, i: &mut Self::Indexes, h: RowHandle, row: &L) {
                i.0.insert(h, (self.left)(row));
            }
            fn insert_right(&self, i: &mut Self::Indexes, h: RowHandle, row: &R) {
                i.1.insert(h, (self.right)(row));
            }
            fn right_candidates<'i>(&self, i: &'i Self::Indexes, row: &L) -> Cow<'i, [RowHandle]> {
                Cow::Owned(i.1.$right_query(&(self.left)(row), $inclusive))
            }
            fn left_candidates<'i>(&self, i: &'i Self::Indexes, row: &R) -> Cow<'i, [RowHandle]> {
                Cow::Owned(i.0.$left_query(&(self.right)(row), $inclusive))
            }
        }
    };
}
ordered_plan!(LessThanJoiner, greater_than, less_than, false);
ordered_plan!(LessThanOrEqualJoiner, greater_than, less_than, true);
ordered_plan!(GreaterThanJoiner, less_than, greater_than, false);
ordered_plan!(GreaterThanOrEqualJoiner, less_than, greater_than, true);

impl<LS, LE, RS, RE, K> CompileCondition for OverlappingJoiner<LS, LE, RS, RE, K> {
    type Plan = Self;
    fn compile(self) -> Self {
        self
    }
}
impl<LS, LE, RS, RE, K: Ord + Clone> IndexedPlan for OverlappingJoiner<LS, LE, RS, RE, K> {
    type Kind = super::ResidualKind;
    type Indexes = (IntervalIndex<K>, IntervalIndex<K>);
    fn new_indexes(&self) -> Self::Indexes {
        (IntervalIndex::new(), IntervalIndex::new())
    }
    fn remove_left(&self, i: &mut Self::Indexes, h: RowHandle) {
        i.0.remove(h);
    }
    fn remove_right(&self, i: &mut Self::Indexes, h: RowHandle) {
        i.1.remove(h);
    }
}
impl<L, R, LS, LE, RS, RE, K> ExecutablePlan<L, R> for OverlappingJoiner<LS, LE, RS, RE, K>
where
    K: Ord + Clone,
    LS: Fn(&L) -> K + Send + Sync,
    LE: Fn(&L) -> K + Send + Sync,
    RS: Fn(&R) -> K + Send + Sync,
    RE: Fn(&R) -> K + Send + Sync,
{
    fn insert_left(&self, i: &mut Self::Indexes, h: RowHandle, row: &L) {
        i.0.insert(h, (self.start_a)(row), (self.end_a)(row));
    }
    fn insert_right(&self, i: &mut Self::Indexes, h: RowHandle, row: &R) {
        i.1.insert(h, (self.start_b)(row), (self.end_b)(row));
    }
    fn right_candidates<'i>(&self, i: &'i Self::Indexes, row: &L) -> Cow<'i, [RowHandle]> {
        Cow::Owned(i.1.overlapping((self.start_a)(row), (self.end_a)(row)))
    }
    fn left_candidates<'i>(&self, i: &'i Self::Indexes, row: &R) -> Cow<'i, [RowHandle]> {
        Cow::Owned(i.0.overlapping((self.start_b)(row), (self.end_b)(row)))
    }
}
