use std::borrow::Cow;
// Compile conditions into concrete indexed plans. Index ownership is separate
// from borrowed row lifetimes, allowing the same plan to consume GAT views.
use super::super::Joiner;
use crate::stream::relational::RowHandle;

pub trait CompileCondition {
    type Plan;
    fn compile(self) -> Self::Plan;
}

pub trait IndexedPlan {
    type Kind;
    type Indexes;
    fn new_indexes(&self) -> Self::Indexes;
    fn remove_left(&self, indexes: &mut Self::Indexes, handle: RowHandle);
    fn remove_right(&self, indexes: &mut Self::Indexes, handle: RowHandle);
}

pub trait ExecutablePlan<L, R>: IndexedPlan + Joiner<L, R> {
    /// Check only semantics not already guaranteed by candidate selection.
    fn candidate_matches(&self, left: &L, right: &R) -> bool {
        self.matches(left, right)
    }
    fn insert_left(&self, indexes: &mut Self::Indexes, handle: RowHandle, row: &L);
    fn insert_right(&self, indexes: &mut Self::Indexes, handle: RowHandle, row: &R);
    fn insert_right_transient(&self, indexes: &mut Self::Indexes, handle: RowHandle, row: &R) {
        self.insert_right(indexes, handle, row);
    }
    fn right_candidates<'i>(&self, indexes: &'i Self::Indexes, left: &L) -> Cow<'i, [RowHandle]>;
    fn left_candidates<'i>(&self, indexes: &'i Self::Indexes, right: &R) -> Cow<'i, [RowHandle]>;
}
