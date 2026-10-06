#![allow(dead_code)] // PairKey/FlattenedPairKey used by the flattened_bi port
                     /* Named key adapters and a generic equality plan for relational joins.

                     A join's condition must be a nameable type so a stream can store it. Key
                     extractors implement `ViewKey`, spelled per view shape: `EntityKey` reads one
                     entity off a leaf, `ChildKey` reads a flattened child, `PairKey` composes two
                     keys over one view, and `FlattenedPairKey` composes the owning entity's key
                     with the flattened child's key. `ViewEqualPlan` is the typed-hash-equality
                     plan over any two such keys.
                     */

use std::borrow::Cow;
use std::hash::Hash;
use std::marker::PhantomData;

use super::index::HashIndex;
use super::operator::FlattenView;
use super::{Leaf, RowHandle};
use crate::stream::joiner::plan::{CompileCondition, EqualityKind, ExecutablePlan, IndexedPlan};
use crate::stream::joiner::Joiner;

/* Reads a join key out of an operator view. */
pub trait ViewKey<V> {
    type Key;

    fn key(&self, view: &V) -> Self::Key;
}

/* Wraps an entity key extractor onto a leaf view. */
pub struct EntityKey<F>(F);

impl<F> EntityKey<F> {
    pub fn new(inner: F) -> Self {
        Self(inner)
    }
}

impl<A, F, K> ViewKey<Leaf<'_, A>> for EntityKey<F>
where
    F: Fn(&A) -> K + Send + Sync,
{
    type Key = K;

    #[inline]
    fn key(&self, view: &Leaf<'_, A>) -> K {
        (self.0)(view.entity)
    }
}

/* Wraps an entity key extractor onto a flattened child view. */
pub struct ChildKey<F>(F);

impl<F> ChildKey<F> {
    pub fn new(inner: F) -> Self {
        Self(inner)
    }
}

impl<V, B, F, K> ViewKey<FlattenView<'_, V, B>> for ChildKey<F>
where
    F: Fn(&B) -> K + Send + Sync,
{
    type Key = K;

    #[inline]
    fn key(&self, view: &FlattenView<'_, V, B>) -> K {
        (self.0)(view.value)
    }
}

/* Composes two keys over the same view into a tuple, e.g. (join key, lookup key). */
pub struct PairKey<A, B>(A, B);

impl<A, B> PairKey<A, B> {
    pub fn new(first: A, second: B) -> Self {
        Self(first, second)
    }
}

impl<V, A, B> ViewKey<V> for PairKey<A, B>
where
    A: ViewKey<V>,
    B: ViewKey<V>,
{
    type Key = (A::Key, B::Key);

    #[inline]
    fn key(&self, view: &V) -> Self::Key {
        (self.0.key(view), self.1.key(view))
    }
}

/* Composes the owning entity's key with the flattened child's key:
`(key_b(owner), c_key(child))`. The right input is a `FlattenNode` over a
collection of `B`, so the owner view is `Leaf<B>`. */
pub struct FlattenedPairKey<KB, CK>(KB, CK);

impl<KB, CK> FlattenedPairKey<KB, CK> {
    pub fn new(owner_key: KB, child_key: CK) -> Self {
        Self(owner_key, child_key)
    }
}

impl<B, C, KB, CK, K, K2> ViewKey<FlattenView<'_, Leaf<'_, B>, C>> for FlattenedPairKey<KB, CK>
where
    KB: Fn(&B) -> K + Send + Sync,
    CK: Fn(&C) -> K2 + Send + Sync,
{
    type Key = (K, K2);

    #[inline]
    fn key(&self, view: &FlattenView<'_, Leaf<'_, B>, C>) -> Self::Key {
        ((self.0)(view.input.entity), (self.1)(view.value))
    }
}

/* Reads a grouped row's own key, for matching against a target domain. */
pub struct GroupKey;

impl<S, O, K, A, V, R> ViewKey<super::operator::GroupView<'_, S, O, K, A, V, R>> for GroupKey
where
    S: 'static,
    O: super::operator::Operator<S>,
    K: Clone,
    A: super::super::collector::Accumulator<V, R>,
{
    type Key = K;

    #[inline]
    fn key(&self, view: &super::operator::GroupView<'_, S, O, K, A, V, R>) -> K {
        view.key.clone()
    }
}

/* Typed hash-equality plan over two named view keys. */
pub struct ViewEqualPlan<K, KeyA, KeyB> {
    key_a: KeyA,
    key_b: KeyB,
    marker: PhantomData<fn() -> K>,
}

impl<K, KeyA, KeyB> ViewEqualPlan<K, KeyA, KeyB> {
    pub fn new(key_a: KeyA, key_b: KeyB) -> Self {
        Self {
            key_a,
            key_b,
            marker: PhantomData,
        }
    }
}

impl<K, KeyA, KeyB> CompileCondition for ViewEqualPlan<K, KeyA, KeyB> {
    type Plan = Self;
    fn compile(self) -> Self {
        self
    }
}

impl<K: Eq + Hash + Clone, KeyA, KeyB> IndexedPlan for ViewEqualPlan<K, KeyA, KeyB> {
    type Kind = EqualityKind;
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

impl<L, R, K, KeyA, KeyB> Joiner<L, R> for ViewEqualPlan<K, KeyA, KeyB>
where
    K: PartialEq,
    KeyA: ViewKey<L, Key = K> + Send + Sync,
    KeyB: ViewKey<R, Key = K> + Send + Sync,
{
    fn matches(&self, left: &L, right: &R) -> bool {
        self.key_a.key(left) == self.key_b.key(right)
    }
}

impl<L, R, K, KeyA, KeyB> ExecutablePlan<L, R> for ViewEqualPlan<K, KeyA, KeyB>
where
    K: Eq + Hash + Clone,
    KeyA: ViewKey<L, Key = K> + Send + Sync,
    KeyB: ViewKey<R, Key = K> + Send + Sync,
{
    fn insert_left(&self, i: &mut Self::Indexes, h: RowHandle, row: &L) {
        i.0.insert(h, self.key_a.key(row));
    }
    fn insert_right(&self, i: &mut Self::Indexes, h: RowHandle, row: &R) {
        i.1.insert(h, self.key_b.key(row));
    }
    fn insert_right_transient(&self, i: &mut Self::Indexes, h: RowHandle, row: &R) {
        i.1.insert_transient(h, self.key_b.key(row));
    }
    fn right_candidates<'i>(&self, i: &'i Self::Indexes, row: &L) -> Cow<'i, [RowHandle]> {
        Cow::Borrowed(i.1.lookup(&self.key_a.key(row)))
    }
    fn left_candidates<'i>(&self, i: &'i Self::Indexes, row: &R) -> Cow<'i, [RowHandle]> {
        Cow::Borrowed(i.0.lookup(&self.key_b.key(row)))
    }
}
