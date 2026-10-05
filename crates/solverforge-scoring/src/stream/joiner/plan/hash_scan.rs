use super::{CompileCondition, ExecutablePlan, IndexedPlan};
use crate::stream::joiner::{EqualJoiner, FilteringJoiner};
use crate::stream::relational::{index::HashIndex, RowHandle};
use std::borrow::Cow;
use std::collections::BTreeSet;
use std::hash::Hash;

impl<FA, FB, K, Mode> CompileCondition for EqualJoiner<FA, FB, K, Mode> {
    type Plan = Self;
    fn compile(self) -> Self {
        self
    }
}
impl<FA, FB, K: Eq + Hash + Clone, Mode> IndexedPlan for EqualJoiner<FA, FB, K, Mode> {
    type Kind = super::EqualityKind;
    type Indexes = (HashIndex<K>, HashIndex<K>);
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
impl<L, R, FA, FB, K, Mode> ExecutablePlan<L, R> for EqualJoiner<FA, FB, K, Mode>
where
    K: Eq + Hash + Clone,
    FA: Fn(&L) -> K + Send + Sync,
    FB: Fn(&R) -> K + Send + Sync,
{
    fn candidate_matches(&self, _: &L, _: &R) -> bool {
        // HashMap resolves collisions with Eq: every bucket candidate is exact.
        true
    }
    fn insert_left(&self, i: &mut Self::Indexes, h: RowHandle, row: &L) {
        i.0.insert(h, self.key_a(row));
    }
    fn insert_right(&self, i: &mut Self::Indexes, h: RowHandle, row: &R) {
        i.1.insert(h, self.key_b(row));
    }
    fn insert_right_transient(&self, i: &mut Self::Indexes, h: RowHandle, row: &R) {
        i.1.insert_transient(h, self.key_b(row));
    }
    fn right_candidates<'i>(&self, i: &'i Self::Indexes, row: &L) -> Cow<'i, [RowHandle]> {
        Cow::Borrowed(i.1.lookup(&self.key_a(row)))
    }
    fn left_candidates<'i>(&self, i: &'i Self::Indexes, row: &R) -> Cow<'i, [RowHandle]> {
        Cow::Borrowed(i.0.lookup(&self.key_b(row)))
    }
}

impl<F> CompileCondition for FilteringJoiner<F> {
    type Plan = Self;
    fn compile(self) -> Self {
        self
    }
}
impl<F> IndexedPlan for FilteringJoiner<F> {
    type Kind = super::ResidualKind;
    type Indexes = (BTreeSet<RowHandle>, BTreeSet<RowHandle>);
    fn new_indexes(&self) -> Self::Indexes {
        (BTreeSet::new(), BTreeSet::new())
    }
    fn remove_left(&self, i: &mut Self::Indexes, h: RowHandle) {
        i.0.remove(&h);
    }
    fn remove_right(&self, i: &mut Self::Indexes, h: RowHandle) {
        i.1.remove(&h);
    }
}
impl<L, R, F: Fn(&L, &R) -> bool + Send + Sync> ExecutablePlan<L, R> for FilteringJoiner<F> {
    fn insert_left(&self, i: &mut Self::Indexes, h: RowHandle, _: &L) {
        i.0.insert(h);
    }
    fn insert_right(&self, i: &mut Self::Indexes, h: RowHandle, _: &R) {
        i.1.insert(h);
    }
    fn right_candidates<'i>(&self, i: &'i Self::Indexes, _: &L) -> Cow<'i, [RowHandle]> {
        Cow::Owned(i.1.iter().copied().collect())
    }
    fn left_candidates<'i>(&self, i: &'i Self::Indexes, _: &R) -> Cow<'i, [RowHandle]> {
        Cow::Owned(i.0.iter().copied().collect())
    }
}
