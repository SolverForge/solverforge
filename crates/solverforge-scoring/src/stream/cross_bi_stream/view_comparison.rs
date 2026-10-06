/* Named view-keyed plans for comparison and overlap conditions.

A comparison joiner is authored as `Fn(&A) -> T` over entities, but the
operator tree hands join plans leaf *views*. Equality solves this with
`BiUnaryPlan`, a hand-written plan that stores the entity closures and reads
`.entity` off each leaf. These are the same idea for the ordered and interval
strategies, so all four comparison directions and overlap compose at every
fluent arity instead of only the two-argument raw form.
*/

use std::borrow::Cow;
use std::marker::PhantomData;

use super::super::relational::index::{IntervalIndex, OrderedIndex};
use super::super::relational::{Leaf, RowHandle};
use crate::stream::joiner::plan::{CompileCondition, ExecutablePlan, IndexedPlan, ResidualKind};
use crate::stream::joiner::Joiner;

/* Ordered comparison plan. `LESS` true means the relationship is
`left(a) < right(b)`; false means `left(a) > right(b)`. `INCLUSIVE` widens the
strict operator to `<=` / `>=`. */
pub struct ViewComparisonPlan<K, KA, KB, const LESS: bool, const INCLUSIVE: bool> {
    key_a: KA,
    key_b: KB,
    marker: PhantomData<fn() -> K>,
}

impl<K, KA, KB, const LESS: bool, const INCLUSIVE: bool>
    ViewComparisonPlan<K, KA, KB, LESS, INCLUSIVE>
{
    pub fn new(key_a: KA, key_b: KB) -> Self {
        Self {
            key_a,
            key_b,
            marker: PhantomData,
        }
    }
}

impl<K, KA, KB, const LESS: bool, const INCLUSIVE: bool> CompileCondition
    for ViewComparisonPlan<K, KA, KB, LESS, INCLUSIVE>
{
    type Plan = Self;
    fn compile(self) -> Self {
        self
    }
}

impl<K: Ord + Clone, KA, KB, const LESS: bool, const INCLUSIVE: bool> IndexedPlan
    for ViewComparisonPlan<K, KA, KB, LESS, INCLUSIVE>
where
    KA: Send + Sync,
    KB: Send + Sync,
{
    type Kind = ResidualKind;
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

impl<'x, A, B, K, KA, KB, const LESS: bool, const INCLUSIVE: bool> Joiner<Leaf<'x, A>, Leaf<'x, B>>
    for ViewComparisonPlan<K, KA, KB, LESS, INCLUSIVE>
where
    K: Ord,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
{
    fn matches(&self, left: &Leaf<'x, A>, right: &Leaf<'x, B>) -> bool {
        let a = (self.key_a)(left.entity);
        let b = (self.key_b)(right.entity);
        if LESS {
            if INCLUSIVE {
                a <= b
            } else {
                a < b
            }
        } else if INCLUSIVE {
            a >= b
        } else {
            a > b
        }
    }
}

impl<'x, A, B, K, KA, KB, const LESS: bool, const INCLUSIVE: bool>
    ExecutablePlan<Leaf<'x, A>, Leaf<'x, B>> for ViewComparisonPlan<K, KA, KB, LESS, INCLUSIVE>
where
    K: Ord + Clone,
    KA: Fn(&A) -> K + Send + Sync,
    KB: Fn(&B) -> K + Send + Sync,
{
    fn insert_left(&self, i: &mut Self::Indexes, h: RowHandle, row: &Leaf<'x, A>) {
        i.0.insert(h, (self.key_a)(row.entity));
    }
    fn insert_right(&self, i: &mut Self::Indexes, h: RowHandle, row: &Leaf<'x, B>) {
        i.1.insert(h, (self.key_b)(row.entity));
    }
    fn insert_right_transient(&self, i: &mut Self::Indexes, h: RowHandle, row: &Leaf<'x, B>) {
        i.1.insert(h, (self.key_b)(row.entity));
    }
    /* Candidates in the right index satisfying the direction against the left
    key. A left value `l` pairs with right values `r` where `l < r` (LESS) or
    `l > r` (not LESS), so the right probe is the inverse direction. */
    fn right_candidates<'i>(
        &self,
        i: &'i Self::Indexes,
        row: &Leaf<'x, A>,
    ) -> Cow<'i, [RowHandle]> {
        let left = (self.key_a)(row.entity);
        if LESS {
            Cow::Owned(i.1.greater_than(&left, INCLUSIVE))
        } else {
            Cow::Owned(i.1.less_than(&left, INCLUSIVE))
        }
    }
    fn left_candidates<'i>(&self, i: &'i Self::Indexes, row: &Leaf<'x, B>) -> Cow<'i, [RowHandle]> {
        let right = (self.key_b)(row.entity);
        if LESS {
            Cow::Owned(i.0.less_than(&right, INCLUSIVE))
        } else {
            Cow::Owned(i.0.greater_than(&right, INCLUSIVE))
        }
    }
}

/* Interval overlap plan: `start_a < end_b && start_b < end_a` (half-open),
the same predicate the raw overlap joiner evaluates. */
pub struct ViewOverlapPlan<K, SA, EA, SB, EB> {
    start_a: SA,
    end_a: EA,
    start_b: SB,
    end_b: EB,
    marker: PhantomData<fn() -> K>,
}

impl<K, SA, EA, SB, EB> ViewOverlapPlan<K, SA, EA, SB, EB> {
    pub fn new(start_a: SA, end_a: EA, start_b: SB, end_b: EB) -> Self {
        Self {
            start_a,
            end_a,
            start_b,
            end_b,
            marker: PhantomData,
        }
    }
}

impl<K, SA, EA, SB, EB> CompileCondition for ViewOverlapPlan<K, SA, EA, SB, EB> {
    type Plan = Self;
    fn compile(self) -> Self {
        self
    }
}

impl<K: Ord + Clone, SA, EA, SB, EB> IndexedPlan for ViewOverlapPlan<K, SA, EA, SB, EB>
where
    SA: Send + Sync,
    EA: Send + Sync,
    SB: Send + Sync,
    EB: Send + Sync,
{
    type Kind = ResidualKind;
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

impl<'x, A, B, K, SA, EA, SB, EB> Joiner<Leaf<'x, A>, Leaf<'x, B>>
    for ViewOverlapPlan<K, SA, EA, SB, EB>
where
    K: Ord,
    SA: Fn(&A) -> K + Send + Sync,
    EA: Fn(&A) -> K + Send + Sync,
    SB: Fn(&B) -> K + Send + Sync,
    EB: Fn(&B) -> K + Send + Sync,
{
    fn matches(&self, left: &Leaf<'x, A>, right: &Leaf<'x, B>) -> bool {
        let sa = (self.start_a)(left.entity);
        let ea = (self.end_a)(left.entity);
        let sb = (self.start_b)(right.entity);
        let eb = (self.end_b)(right.entity);
        sa < eb && sb < ea
    }
}

impl<'x, A, B, K, SA, EA, SB, EB> ExecutablePlan<Leaf<'x, A>, Leaf<'x, B>>
    for ViewOverlapPlan<K, SA, EA, SB, EB>
where
    K: Ord + Clone,
    SA: Fn(&A) -> K + Send + Sync,
    EA: Fn(&A) -> K + Send + Sync,
    SB: Fn(&B) -> K + Send + Sync,
    EB: Fn(&B) -> K + Send + Sync,
{
    fn insert_left(&self, i: &mut Self::Indexes, h: RowHandle, row: &Leaf<'x, A>) {
        i.0.insert(h, (self.start_a)(row.entity), (self.end_a)(row.entity));
    }
    fn insert_right(&self, i: &mut Self::Indexes, h: RowHandle, row: &Leaf<'x, B>) {
        i.1.insert(h, (self.start_b)(row.entity), (self.end_b)(row.entity));
    }
    fn insert_right_transient(&self, i: &mut Self::Indexes, h: RowHandle, row: &Leaf<'x, B>) {
        i.1.insert(h, (self.start_b)(row.entity), (self.end_b)(row.entity));
    }
    fn right_candidates<'i>(
        &self,
        i: &'i Self::Indexes,
        row: &Leaf<'x, A>,
    ) -> Cow<'i, [RowHandle]> {
        Cow::Owned(i.1.overlapping((self.start_a)(row.entity), (self.end_a)(row.entity)))
    }
    fn left_candidates<'i>(&self, i: &'i Self::Indexes, row: &Leaf<'x, B>) -> Cow<'i, [RowHandle]> {
        Cow::Owned(i.0.overlapping((self.start_b)(row.entity), (self.end_b)(row.entity)))
    }
}
