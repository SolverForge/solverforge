/* Semi/anti existence streams over the composable operator tree.

An existence constraint is a semi/anti join: it retains only left rows whose
matching-right count is nonzero (or zero, for `NotExists`). There is no third
retention engine — the relationship is a compiled `EqualJoiner` over the two
operators' views, the left is a collection over the left stream (whose
membership is the stream's own filter), the right is a collection over the
right stream or a `FlattenNode` over it, and scoring is the generic
`OperatorTerminal`.
*/

use std::marker::PhantomData;

use solverforge_core::score::Score;
use solverforge_core::{ConstraintRef, ImpactType};

use super::relational::index::HashIndex;
use super::relational::operator::{ExistenceNode, FlattenView, Operator};
use super::relational::{Leaf, RowHandle};
use super::weighting_support::ConstraintWeight;
use crate::constraint::relational::OperatorTerminal;
use crate::stream::joiner::plan::{CompileCondition, EqualityKind, ExecutablePlan, IndexedPlan};
use crate::stream::joiner::Joiner;
use std::borrow::Cow;
use std::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExistenceMode {
    Exists,
    NotExists,
}

/* Reads a join key out of an operator view. Named so an existence stream's
condition type is spellable: `impl Fn` cannot appear in an associated type. */
pub trait ViewKey<V> {
    type Key;

    fn key(&self, view: &V) -> Self::Key;
}

/* Wraps an entity key extractor onto a leaf view. */
pub struct EntityKey<F>(F);

impl<F> EntityKey<F> {
    pub(super) fn new(inner: F) -> Self {
        Self(inner)
    }
}

impl<A, F, K> ViewKey<Leaf<'_, A>> for EntityKey<F>
where
    F: Fn(&A) -> K,
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
    pub(super) fn new(inner: F) -> Self {
        Self(inner)
    }
}

impl<V, B, F, K> ViewKey<FlattenView<'_, V, B>> for ChildKey<F>
where
    F: Fn(&B) -> K,
{
    type Key = K;

    #[inline]
    fn key(&self, view: &FlattenView<'_, V, B>) -> K {
        (self.0)(view.value)
    }
}

/* Equality relationship between two operator views.

Each side owns a named `ViewKey`, so the plan type is fully spellable: a
direct right input uses `EntityKey<KB>` over leaves, a flattened one uses
`ChildKey<KB>` over child views. Typed hash indexes on both sides, exact
equality on every candidate.
*/
pub struct ExistsEqualPlan<K, KeyA, KeyB> {
    key_a: KeyA,
    key_b: KeyB,
    marker: PhantomData<fn() -> K>,
}

impl<K, KeyA, KeyB> ExistsEqualPlan<K, KeyA, KeyB> {
    pub(super) fn new(key_a: KeyA, key_b: KeyB) -> Self {
        Self {
            key_a,
            key_b,
            marker: PhantomData,
        }
    }
}

impl<K, KeyA, KeyB> CompileCondition for ExistsEqualPlan<K, KeyA, KeyB> {
    type Plan = Self;
    fn compile(self) -> Self {
        self
    }
}

impl<K: Eq + Hash + Clone, KeyA, KeyB> IndexedPlan for ExistsEqualPlan<K, KeyA, KeyB> {
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

impl<L, R, K, KeyA, KeyB> Joiner<L, R> for ExistsEqualPlan<K, KeyA, KeyB>
where
    K: PartialEq,
    KeyA: ViewKey<L, Key = K> + Send + Sync,
    KeyB: ViewKey<R, Key = K> + Send + Sync,
{
    fn matches(&self, left: &L, right: &R) -> bool {
        self.key_a.key(left) == self.key_b.key(right)
    }
}

impl<L, R, K, KeyA, KeyB> ExecutablePlan<L, R> for ExistsEqualPlan<K, KeyA, KeyB>
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

/* Zero-erasure existence stream: a left operator, a right operator, and the
compiled relationship between them. `named()` builds the `ExistenceNode` and
its generic terminal. */
pub struct ExistsConstraintStream<S, A, L, R, C, Sc>
where
    C: CompileCondition,
    Sc: Score,
{
    pub(super) mode: ExistenceMode,
    pub(super) left: L,
    pub(super) right: R,
    pub(super) condition: C,
    pub(super) _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> Sc)>,
}

impl<S, A, L, R, C, Sc> ExistsConstraintStream<S, A, L, R, C, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    L: Operator<S>,
    for<'a> L: Operator<S, View<'a> = Leaf<'a, A>>,
    C: CompileCondition,
    Sc: Score + 'static,
{
    pub(super) fn new(mode: ExistenceMode, left: L, right: R, condition: C) -> Self {
        Self {
            mode,
            left,
            right,
            condition,
            _phantom: PhantomData,
        }
    }

    fn into_weighted_builder<W>(
        self,
        impact_type: ImpactType,
        weight: W,
        is_hard: bool,
    ) -> ExistsConstraintBuilder<S, A, L, R, C, W, Sc>
    where
        W: for<'a> Fn(&S, &Leaf<'a, A>) -> Sc,
    {
        ExistsConstraintBuilder {
            mode: self.mode,
            left: self.left,
            right: self.right,
            condition: self.condition,
            impact_type,
            weight,
            is_hard,
            _phantom: PhantomData,
        }
    }

    pub fn penalize<W>(
        self,
        weight: W,
    ) -> ExistsConstraintBuilder<
        S,
        A,
        L,
        R,
        C,
        impl for<'a> Fn(&S, &Leaf<'a, A>) -> Sc + Send + Sync,
        Sc,
    >
    where
        W: for<'w> ConstraintWeight<(&'w A,), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            ImpactType::Penalty,
            move |_s: &S, row: &Leaf<'_, A>| weight.score((row.entity,)),
            is_hard,
        )
    }

    pub fn reward<W>(
        self,
        weight: W,
    ) -> ExistsConstraintBuilder<
        S,
        A,
        L,
        R,
        C,
        impl for<'a> Fn(&S, &Leaf<'a, A>) -> Sc + Send + Sync,
        Sc,
    >
    where
        W: for<'w> ConstraintWeight<(&'w A,), Sc> + Send + Sync,
    {
        let is_hard = weight.is_hard();
        self.into_weighted_builder(
            ImpactType::Reward,
            move |_s: &S, row: &Leaf<'_, A>| weight.score((row.entity,)),
            is_hard,
        )
    }
}

pub struct ExistsConstraintBuilder<S, A, L, R, C, W, Sc>
where
    C: CompileCondition,
    Sc: Score,
{
    mode: ExistenceMode,
    left: L,
    right: R,
    condition: C,
    impact_type: ImpactType,
    weight: W,
    is_hard: bool,
    _phantom: PhantomData<(fn() -> S, fn() -> A, fn() -> Sc)>,
}

impl<S, A, L, R, C, W, Sc> ExistsConstraintBuilder<S, A, L, R, C, W, Sc>
where
    S: Send + Sync + 'static,
    A: 'static,
    L: Operator<S>,
    for<'a> L: Operator<S, View<'a> = Leaf<'a, A>>,
    R: Operator<S>,
    C: CompileCondition + 'static,
    C::Plan: IndexedPlan + 'static,
    for<'a> C::Plan: ExecutablePlan<L::View<'a>, R::View<'a>>,
    W: for<'a> Fn(&S, &Leaf<'a, A>) -> Sc + Send + Sync,
    Sc: Score + 'static,
{
    pub fn named(
        self,
        name: &str,
    ) -> OperatorTerminal<
        S,
        ExistenceNode<S, L, R, C::Plan>,
        impl for<'a> Fn(&S, &L::View<'a>) -> Sc + Send + Sync,
        Sc,
    > {
        let exists = matches!(self.mode, ExistenceMode::Exists);
        let node = ExistenceNode::new(self.left, self.right, self.condition, exists);
        OperatorTerminal::new(
            ConstraintRef::new("", name),
            self.impact_type,
            node,
            self.weight,
            self.is_hard,
        )
    }
}

impl<S, A, L, R, C, Sc: Score> std::fmt::Debug for ExistsConstraintStream<S, A, L, R, C, Sc>
where
    C: CompileCondition,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExistsConstraintStream")
            .field("mode", &self.mode)
            .finish()
    }
}

impl<S, A, L, R, C, W, Sc: Score> std::fmt::Debug for ExistsConstraintBuilder<S, A, L, R, C, W, Sc>
where
    C: CompileCondition,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExistsConstraintBuilder")
            .field("mode", &self.mode)
            .field("impact_type", &self.impact_type)
            .finish()
    }
}
