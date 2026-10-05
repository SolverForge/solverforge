use super::composite::{EqualityKeys, EqualityKind, EqualityPlan};
use super::{ExecutablePlan, IndexedPlan, PlannedCondition, Strategy};
use crate::stream::joiner::Joiner;
use crate::stream::relational::{index::HashIndex, RowHandle};
use std::borrow::Cow;

/// Extract indexed equality keys while preserving authored residual order.
pub struct EqualityWithResidual<E, P, const EQUALITY_FIRST: bool> {
    pub(super) equality: E,
    pub(super) residual: P,
}
impl<E: EqualityPlan, P, const FIRST: bool> EqualityPlan for EqualityWithResidual<E, P, FIRST> {
    type Key = E::Key;
}
impl<L, R, E: EqualityKeys<L, R>, P: Joiner<L, R>, const FIRST: bool> EqualityKeys<L, R>
    for EqualityWithResidual<E, P, FIRST>
{
    fn left_key(&self, row: &L) -> Self::Key {
        self.equality.left_key(row)
    }
    fn right_key(&self, row: &R) -> Self::Key {
        self.equality.right_key(row)
    }
    // Preserve authored short-circuit invocation order.
    #[allow(clippy::if_same_then_else)]
    fn residual_matches(&self, left: &L, right: &R) -> bool {
        if FIRST {
            self.equality.residual_matches(left, right) && self.residual.matches(left, right)
        } else {
            self.residual.matches(left, right) && self.equality.residual_matches(left, right)
        }
    }
}
impl<L, R, E: Joiner<L, R>, P: Joiner<L, R>, const FIRST: bool> Joiner<L, R>
    for EqualityWithResidual<E, P, FIRST>
{
    // Preserve authored short-circuit invocation order.
    #[allow(clippy::if_same_then_else)]
    fn matches(&self, left: &L, right: &R) -> bool {
        if FIRST {
            self.equality.matches(left, right) && self.residual.matches(left, right)
        } else {
            self.residual.matches(left, right) && self.equality.matches(left, right)
        }
    }
}
impl<L, R, E: Joiner<L, R>, P: Joiner<L, R>, const FIRST: bool> PlannedCondition<L, R>
    for EqualityWithResidual<E, P, FIRST>
{
    const STRATEGY: Strategy = Strategy::EquiHash;
    fn check(&self, left: &L, right: &R) -> bool {
        self.matches(left, right)
    }
}
impl<E: EqualityPlan, P, const FIRST: bool> IndexedPlan for EqualityWithResidual<E, P, FIRST> {
    type Kind = EqualityKind;
    type Indexes = (HashIndex<E::Key>, HashIndex<E::Key>);
    fn new_indexes(&self) -> Self::Indexes {
        (HashIndex::new(), HashIndex::new())
    }
    fn remove_left(&self, i: &mut Self::Indexes, h: RowHandle) {
        i.0.remove(h);
    }
    fn remove_right(&self, i: &mut Self::Indexes, h: RowHandle) {
        i.1.remove(h);
    }
}
impl<L, R, E, P, const FIRST: bool> ExecutablePlan<L, R> for EqualityWithResidual<E, P, FIRST>
where
    E: EqualityKeys<L, R> + Joiner<L, R>,
    P: Joiner<L, R>,
{
    fn candidate_matches(&self, left: &L, right: &R) -> bool {
        self.residual_matches(left, right)
    }
    fn insert_left(&self, i: &mut Self::Indexes, h: RowHandle, row: &L) {
        i.0.insert(h, self.left_key(row));
    }
    fn insert_right(&self, i: &mut Self::Indexes, h: RowHandle, row: &R) {
        i.1.insert(h, self.right_key(row));
    }
    fn insert_right_transient(&self, i: &mut Self::Indexes, h: RowHandle, row: &R) {
        i.1.insert_transient(h, self.right_key(row));
    }
    fn right_candidates<'i>(&self, i: &'i Self::Indexes, row: &L) -> Cow<'i, [RowHandle]> {
        Cow::Borrowed(i.1.lookup(&self.left_key(row)))
    }
    fn left_candidates<'i>(&self, i: &'i Self::Indexes, row: &R) -> Cow<'i, [RowHandle]> {
        Cow::Borrowed(i.0.lookup(&self.right_key(row)))
    }
}
