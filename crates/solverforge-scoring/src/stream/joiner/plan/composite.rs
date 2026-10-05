use super::EqualityWithResidual;
use super::{ExecutablePlan, IndexedPlan, PlannedCondition, Strategy};
use crate::stream::joiner::{AndJoiner, EqualJoiner, Joiner};
use crate::stream::relational::{index::HashIndex, RowHandle};
use std::{borrow::Cow, hash::Hash};

pub struct EqualityKind;
pub struct ResidualKind;

/// Static dispatch on plan structure, without specialization or erased conditions.
pub trait ComposePlans<A, B> {
    type Plan;
    fn compose(first: A, second: B) -> Self::Plan;
}
impl<A: EqualityPlan, B: EqualityPlan> ComposePlans<A, B> for (EqualityKind, EqualityKind) {
    type Plan = CompositeEquality<A, B>;
    fn compose(first: A, second: B) -> Self::Plan {
        CompositeEquality { first, second }
    }
}
impl<A: EqualityPlan, B> ComposePlans<A, B> for (EqualityKind, ResidualKind) {
    type Plan = EqualityWithResidual<A, B, true>;
    fn compose(first: A, second: B) -> Self::Plan {
        EqualityWithResidual {
            equality: first,
            residual: second,
        }
    }
}
impl<A, B: EqualityPlan> ComposePlans<A, B> for (ResidualKind, EqualityKind) {
    type Plan = EqualityWithResidual<B, A, false>;
    fn compose(first: A, second: B) -> Self::Plan {
        EqualityWithResidual {
            equality: second,
            residual: first,
        }
    }
}
impl<A, B> ComposePlans<A, B> for (ResidualKind, ResidualKind) {
    type Plan = AndJoiner<A, B>;
    fn compose(first: A, second: B) -> Self::Plan {
        AndJoiner { first, second }
    }
}

pub trait EqualityPlan {
    type Key: Eq + Hash + Clone;
}
pub trait EqualityKeys<L, R>: EqualityPlan {
    fn left_key(&self, row: &L) -> Self::Key;
    fn right_key(&self, row: &R) -> Self::Key;
    fn residual_matches(&self, _: &L, _: &R) -> bool {
        true
    }
}
impl<FA, FB, K: Eq + Hash + Clone, Mode> EqualityPlan for EqualJoiner<FA, FB, K, Mode> {
    type Key = K;
}
impl<L, R, FA, FB, K, Mode> EqualityKeys<L, R> for EqualJoiner<FA, FB, K, Mode>
where
    K: Eq + Hash + Clone,
    FA: Fn(&L) -> K,
    FB: Fn(&R) -> K,
{
    fn left_key(&self, row: &L) -> K {
        self.key_a(row)
    }
    fn right_key(&self, row: &R) -> K {
        self.key_b(row)
    }
}

/// A single heterogeneous tuple key for an equality conjunction of any depth.
pub struct CompositeEquality<A, B> {
    first: A,
    second: B,
}
impl<A: EqualityPlan, B: EqualityPlan> EqualityPlan for CompositeEquality<A, B> {
    type Key = (A::Key, B::Key);
}
impl<L, R, A: EqualityKeys<L, R>, B: EqualityKeys<L, R>> EqualityKeys<L, R>
    for CompositeEquality<A, B>
{
    fn left_key(&self, row: &L) -> Self::Key {
        (self.first.left_key(row), self.second.left_key(row))
    }
    fn right_key(&self, row: &R) -> Self::Key {
        (self.first.right_key(row), self.second.right_key(row))
    }
    fn residual_matches(&self, left: &L, right: &R) -> bool {
        self.first.residual_matches(left, right) && self.second.residual_matches(left, right)
    }
}
impl<L, R, A: Joiner<L, R>, B: Joiner<L, R>> Joiner<L, R> for CompositeEquality<A, B> {
    fn matches(&self, left: &L, right: &R) -> bool {
        self.first.matches(left, right) && self.second.matches(left, right)
    }
}
impl<L, R, A: Joiner<L, R>, B: Joiner<L, R>> PlannedCondition<L, R> for CompositeEquality<A, B> {
    const STRATEGY: Strategy = Strategy::EquiHash;
    fn check(&self, left: &L, right: &R) -> bool {
        self.matches(left, right)
    }
}
impl<A: EqualityPlan, B: EqualityPlan> IndexedPlan for CompositeEquality<A, B> {
    type Kind = EqualityKind;
    type Indexes = (HashIndex<(A::Key, B::Key)>, HashIndex<(A::Key, B::Key)>);
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
impl<L, R, A, B> ExecutablePlan<L, R> for CompositeEquality<A, B>
where
    A: EqualityKeys<L, R> + Joiner<L, R>,
    B: EqualityKeys<L, R> + Joiner<L, R>,
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
