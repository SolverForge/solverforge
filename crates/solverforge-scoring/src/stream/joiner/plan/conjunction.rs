use super::{CompileCondition, ExecutablePlan, IndexedPlan, PlannedCondition, Strategy};
use crate::stream::joiner::AndJoiner;
use crate::stream::relational::RowHandle;
use std::borrow::Cow;

impl<A, B> CompileCondition for AndJoiner<A, B>
where
    A: CompileCondition,
    B: CompileCondition,
    A::Plan: IndexedPlan,
    B::Plan: IndexedPlan,
    (
        <A::Plan as IndexedPlan>::Kind,
        <B::Plan as IndexedPlan>::Kind,
    ): super::ComposePlans<A::Plan, B::Plan>,
{
    type Plan = <(
        <A::Plan as IndexedPlan>::Kind,
        <B::Plan as IndexedPlan>::Kind,
    ) as super::ComposePlans<A::Plan, B::Plan>>::Plan;
    fn compile(self) -> Self::Plan {
        <(
            <A::Plan as IndexedPlan>::Kind,
            <B::Plan as IndexedPlan>::Kind,
        ) as super::ComposePlans<A::Plan, B::Plan>>::compose(
            self.first.compile(),
            self.second.compile(),
        )
    }
}
impl<A: IndexedPlan, B: IndexedPlan> IndexedPlan for AndJoiner<A, B> {
    type Kind = super::ResidualKind;
    type Indexes = (A::Indexes, B::Indexes);
    fn new_indexes(&self) -> Self::Indexes {
        (self.first.new_indexes(), self.second.new_indexes())
    }
    fn remove_left(&self, i: &mut Self::Indexes, h: RowHandle) {
        self.first.remove_left(&mut i.0, h);
        self.second.remove_left(&mut i.1, h);
    }
    fn remove_right(&self, i: &mut Self::Indexes, h: RowHandle) {
        self.first.remove_right(&mut i.0, h);
        self.second.remove_right(&mut i.1, h);
    }
}
impl<L, R, A, B> ExecutablePlan<L, R> for AndJoiner<A, B>
where
    A: ExecutablePlan<L, R> + PlannedCondition<L, R>,
    B: ExecutablePlan<L, R> + PlannedCondition<L, R>,
{
    fn insert_left(&self, i: &mut Self::Indexes, h: RowHandle, row: &L) {
        self.first.insert_left(&mut i.0, h, row);
        self.second.insert_left(&mut i.1, h, row);
    }
    fn insert_right(&self, i: &mut Self::Indexes, h: RowHandle, row: &R) {
        self.first.insert_right(&mut i.0, h, row);
        self.second.insert_right(&mut i.1, h, row);
    }
    fn insert_right_transient(&self, i: &mut Self::Indexes, h: RowHandle, row: &R) {
        self.first.insert_right_transient(&mut i.0, h, row);
        self.second.insert_right_transient(&mut i.1, h, row);
    }
    fn right_candidates<'i>(&self, i: &'i Self::Indexes, row: &L) -> Cow<'i, [RowHandle]> {
        if A::STRATEGY == Strategy::Scan && B::STRATEGY != Strategy::Scan {
            return self.second.right_candidates(&i.1, row);
        }
        let first = self.first.right_candidates(&i.0, row);
        if B::STRATEGY == Strategy::Scan {
            return first;
        }
        Cow::Owned(intersect(
            first.into_owned(),
            self.second.right_candidates(&i.1, row).into_owned(),
        ))
    }
    fn left_candidates<'i>(&self, i: &'i Self::Indexes, row: &R) -> Cow<'i, [RowHandle]> {
        if A::STRATEGY == Strategy::Scan && B::STRATEGY != Strategy::Scan {
            return self.second.left_candidates(&i.1, row);
        }
        let first = self.first.left_candidates(&i.0, row);
        if B::STRATEGY == Strategy::Scan {
            return first;
        }
        Cow::Owned(intersect(
            first.into_owned(),
            self.second.left_candidates(&i.1, row).into_owned(),
        ))
    }
}
fn intersect(mut left: Vec<RowHandle>, mut right: Vec<RowHandle>) -> Vec<RowHandle> {
    right.sort_unstable();
    left.retain(|h| right.binary_search(h).is_ok());
    left
}
