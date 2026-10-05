/* Generic terminal path for the fluent cross-bi stream.

A `Bi` finalizes onto the same `OperatorTerminal` used by the canonical
tri chain: the first relationship is the stream's own unary key pair,
adapted onto borrowed `Leaf` views by `BiUnaryPlan`, and the authored
`BiFilter` runs inside the scored operator so evaluate, match_count,
initialize, and mutations honor it exactly once. No shared-key retention
engine is involved.
*/

use std::borrow::Cow;
use std::hash::Hash;
use std::marker::PhantomData;

use super::super::collection_extract::CollectionExtract;
use super::super::filter::BiFilter;
use super::super::joiner::plan::{CompileCondition, EqualityKind, ExecutablePlan, IndexedPlan};
use super::super::joiner::Joiner;
use super::super::relational::index::HashIndex;
use super::super::relational::operator::{CollectionNode, JoinNode, Operator, Pair, RowChanges};
use super::super::relational::{HandleMap, Leaf, RowHandle};

/* The first-join plan as compiled from a Bi stream's unary key pair.

Keys take entities, not rows; the adapter lifts them onto the borrowed
`Leaf` views the join operator probes with. Both closure types stay
concrete and monomorphized.
*/
pub struct BiUnaryPlan<K, KA, KB> {
    key_a: KA,
    key_b: KB,
    marker: PhantomData<fn() -> K>,
}

impl<K, KA, KB> BiUnaryPlan<K, KA, KB> {
    pub(super) fn new(key_a: KA, key_b: KB) -> Self {
        Self {
            key_a,
            key_b,
            marker: PhantomData,
        }
    }
}

impl<K, KA, KB> CompileCondition for BiUnaryPlan<K, KA, KB> {
    type Plan = Self;
    fn compile(self) -> Self {
        self
    }
}

impl<K: Eq + Hash + Clone, KA, KB> IndexedPlan for BiUnaryPlan<K, KA, KB> {
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

impl<'x, A, B, K, KA, KB> Joiner<Leaf<'x, A>, Leaf<'x, B>> for BiUnaryPlan<K, KA, KB>
where
    K: PartialEq,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
{
    fn matches(&self, left: &Leaf<'x, A>, right: &Leaf<'x, B>) -> bool {
        (self.key_a)(left.entity) == (self.key_b)(right.entity)
    }
}

impl<'x, A, B, K, KA, KB> ExecutablePlan<Leaf<'x, A>, Leaf<'x, B>> for BiUnaryPlan<K, KA, KB>
where
    K: Eq + Hash + Clone,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
{
    fn candidate_matches(&self, left: &Leaf<'x, A>, right: &Leaf<'x, B>) -> bool {
        self.matches(left, right)
    }
    fn insert_left(&self, i: &mut Self::Indexes, h: RowHandle, row: &Leaf<'x, A>) {
        i.0.insert(h, (self.key_a)(row.entity));
    }
    fn insert_right(&self, i: &mut Self::Indexes, h: RowHandle, row: &Leaf<'x, B>) {
        i.1.insert(h, (self.key_b)(row.entity));
    }
    fn insert_right_transient(&self, i: &mut Self::Indexes, h: RowHandle, row: &Leaf<'x, B>) {
        i.1.insert_transient(h, (self.key_b)(row.entity));
    }
    fn right_candidates<'i>(
        &self,
        i: &'i Self::Indexes,
        row: &Leaf<'x, A>,
    ) -> Cow<'i, [RowHandle]> {
        Cow::Borrowed(i.1.lookup(&(self.key_a)(row.entity)))
    }
    fn left_candidates<'i>(&self, i: &'i Self::Indexes, row: &Leaf<'x, B>) -> Cow<'i, [RowHandle]> {
        Cow::Borrowed(i.0.lookup(&(self.key_b)(row.entity)))
    }
}

/* Borrowed (A, B) entity pair off a bi row view. */
pub trait BiEntities<'a, A, B> {
    fn entities(&self) -> (&'a A, &'a B);
    fn indexes(&self) -> (usize, usize);
}

impl<'a, A, B> BiEntities<'a, A, B> for Pair<Leaf<'a, A>, Leaf<'a, B>> {
    #[inline]
    fn entities(&self) -> (&'a A, &'a B) {
        (self.left.entity, self.right.entity)
    }
    #[inline]
    fn indexes(&self) -> (usize, usize) {
        (self.left.index, self.right.index)
    }
}

/* Scored bi operator: the join tree plus the authored filter as one row producer.

The terminal's weight closure receives filtered rows only. Filters never run
twice: stateless evaluation applies them here, and retained mutations publish
post-filter deltas through the same predicate.
*/
pub struct BiScored<S, A, B, EA, EB, P, F>
where
    P: IndexedPlan,
    JoinNode<S, CollectionNode<S, EA>, CollectionNode<S, EB>, P>: Operator<S>,
{
    inner: JoinNode<S, CollectionNode<S, EA>, CollectionNode<S, EB>, P>,
    filter: F,
    accepted: HandleMap<()>,
    marker: PhantomData<fn() -> (A, B)>,
}

impl<S, A, B, EA, EB, P, F> BiScored<S, A, B, EA, EB, P, F>
where
    P: IndexedPlan,
    JoinNode<S, CollectionNode<S, EA>, CollectionNode<S, EB>, P>: Operator<S>,
{
    pub(super) fn new(
        inner: JoinNode<S, CollectionNode<S, EA>, CollectionNode<S, EB>, P>,
        filter: F,
    ) -> Self {
        Self {
            inner,
            filter,
            accepted: HandleMap::new(),
            marker: PhantomData,
        }
    }
}

impl<S, A, B, EA, EB, P, F> Operator<S> for BiScored<S, A, B, EA, EB, P, F>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    P: IndexedPlan + 'static,
    EA: CollectionExtract<S, Item = A>,
    EB: CollectionExtract<S, Item = B>,
    for<'a> P: ExecutablePlan<Leaf<'a, A>, Leaf<'a, B>>,
    for<'a> JoinNode<S, CollectionNode<S, EA>, CollectionNode<S, EB>, P>:
        Operator<S, View<'a> = Pair<Leaf<'a, A>, Leaf<'a, B>>>,
    F: BiFilter<S, A, B> + 'static,
{
    type View<'a> = Pair<Leaf<'a, A>, Leaf<'a, B>>;
    type Evaluation =
        <JoinNode<S, CollectionNode<S, EA>, CollectionNode<S, EB>, P> as Operator<S>>::Evaluation;

    #[inline]
    fn prepare_evaluation(&self, solution: &S) -> Self::Evaluation {
        self.inner.prepare_evaluation(solution)
    }

    #[inline]
    fn visit_evaluation<'a>(
        &'a self,
        solution: &'a S,
        evaluation: &'a Self::Evaluation,
        visitor: &mut impl FnMut(Self::View<'a>),
    ) {
        self.inner
            .visit_evaluation(solution, evaluation, &mut |row| {
                let (a, b) = row.entities();
                let (ai, bi) = row.indexes();
                if self.filter.test(solution, a, b, ai, bi) {
                    visitor(row);
                }
            });
    }

    fn clear(&mut self) {
        self.inner.clear();
        self.accepted.clear();
    }

    fn initialize(&mut self, solution: &S) {
        self.inner.initialize(solution);
        self.accepted.clear();
        for handle in self.inner.handles() {
            let row = self.inner.resolve(solution, handle).expect("live bi row");
            if self.test(solution, row) {
                self.accepted.insert(handle, ());
            }
        }
    }

    fn handles(&self) -> Vec<RowHandle> {
        self.inner
            .handles()
            .into_iter()
            .filter(|h| self.accepted.get(*h).is_some())
            .collect()
    }

    fn resolve<'a>(&'a self, solution: &'a S, handle: RowHandle) -> Option<Self::View<'a>> {
        self.accepted.get(handle)?;
        self.inner.resolve(solution, handle)
    }

    fn visit_provenance(&self, handle: RowHandle, visitor: &mut impl FnMut(u32, usize, usize)) {
        if self.accepted.get(handle).is_some() {
            self.inner.visit_provenance(handle, visitor);
        }
    }

    fn retract(&mut self, solution: &S, descriptor: usize, index: usize) -> RowChanges {
        let mut changes = self.inner.retract(solution, descriptor, index);
        changes
            .removed
            .retain(|h| self.accepted.remove(*h).is_some());
        changes
    }

    fn insert(&mut self, solution: &S, descriptor: usize, index: usize) -> RowChanges {
        let mut changes = self.inner.insert(solution, descriptor, index);
        let mut accepted = Vec::with_capacity(changes.inserted.len());
        for handle in changes.inserted.drain(..) {
            let row = self.inner.resolve(solution, handle).expect("live bi row");
            if self.test(solution, row) {
                self.accepted.insert(handle, ());
                accepted.push(handle);
            }
        }
        changes.inserted = accepted;
        changes
    }
}

impl<S, A, B, EA, EB, P, F> BiScored<S, A, B, EA, EB, P, F>
where
    S: Send + Sync + 'static,
    A: Clone + Send + Sync + 'static,
    B: Clone + Send + Sync + 'static,
    P: IndexedPlan,
    F: BiFilter<S, A, B>,
    JoinNode<S, CollectionNode<S, EA>, CollectionNode<S, EB>, P>: Operator<S>,
{
    #[inline]
    fn test<'a>(&self, solution: &S, row: Pair<Leaf<'a, A>, Leaf<'a, B>>) -> bool
    where
        A: 'a,
        B: 'a,
    {
        let (a, b) = row.entities();
        let (ai, bi) = row.indexes();
        self.filter.test(solution, a, b, ai, bi)
    }
}

impl<S, A, B, EA, EB, P, F> std::fmt::Debug for BiScored<S, A, B, EA, EB, P, F>
where
    P: IndexedPlan,
    JoinNode<S, CollectionNode<S, EA>, CollectionNode<S, EB>, P>: Operator<S>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BiScored").finish()
    }
}
