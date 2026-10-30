//! Keyed equality plans: keyed probes and key readback for retained joins.
//!
//! `JoinNode` probes by rows. A complement's restore pass needs the same probe
//! by KEY: when a result row is removed, the targets that may regain their
//! default are the live left rows still indexed under the removed row's key.
//! Capturing row handles at index time instead would replay stale handles after
//! a target retracts and re-inserts (new generation on the same slot), skipping
//! their defaults — traced as a retained-vs-full-evaluation divergence. The key
//! is stable where handles are not, so the node reads the removed row's key back
//! from its index and re-derives current target handles from the left index.

use crate::stream::joiner::plan::{EqualityPlan, IndexedPlan};
use crate::stream::relational::index::HashIndex;
use crate::stream::relational::RowHandle;
use std::borrow::Cow;

pub trait KeyOf: IndexedPlan {
    type Key: Eq + std::hash::Hash + Clone;

    /// Live left rows whose key matches `key`, straight from the left index.
    fn left_candidates_by_key<'i>(
        &self,
        indexes: &'i Self::Indexes,
        key: &Self::Key,
    ) -> Cow<'i, [RowHandle]>;
}

pub trait RightKeyOf: IndexedPlan {
    type Key: Eq + std::hash::Hash + Clone;

    /// The retained key of one live right row, read back from its index.
    fn right_row_key<'i>(
        &self,
        indexes: &'i Self::Indexes,
        handle: RowHandle,
    ) -> Option<&'i Self::Key>;
}

/// Blanket over hash-indexed equality plans: both indexes are keyed by the
/// plan's own `EqualityPlan::Key`, so both probes are member lookups on the
/// index pair. Implemented per side through the shape traits below.
impl<P> KeyOf for P
where
    P: EqualityPlan + IndexedPlan,
    P::Key: 'static,
    P::Indexes: LeftHashIndex<P::Key>,
{
    type Key = P::Key;

    #[inline]
    fn left_candidates_by_key<'i>(
        &self,
        indexes: &'i Self::Indexes,
        key: &P::Key,
    ) -> Cow<'i, [RowHandle]> {
        Cow::Borrowed(indexes.left_hash().lookup(key))
    }
}

impl<P> RightKeyOf for P
where
    P: EqualityPlan + IndexedPlan,
    P::Key: 'static,
    P::Indexes: RightHashIndex<P::Key>,
{
    type Key = P::Key;

    #[inline]
    fn right_row_key<'i>(
        &self,
        indexes: &'i Self::Indexes,
        handle: RowHandle,
    ) -> Option<&'i P::Key> {
        indexes.right_hash().key_of(handle)
    }
}

/// The left/right index of a plan's `Indexes` pair is a `HashIndex` keyed by
/// K. Implemented only where the shape actually holds, so no structural
/// guessing.
pub trait LeftHashIndex<K> {
    fn left_hash(&self) -> &HashIndex<K>;
}

pub trait RightHashIndex<K> {
    fn right_hash(&self) -> &HashIndex<K>;
}

impl<K: Eq + std::hash::Hash + Clone + 'static> LeftHashIndex<K> for (HashIndex<K>, HashIndex<K>) {
    #[inline]
    fn left_hash(&self) -> &HashIndex<K> {
        &self.0
    }
}

impl<K: Eq + std::hash::Hash + Clone + 'static> RightHashIndex<K> for (HashIndex<K>, HashIndex<K>) {
    #[inline]
    fn right_hash(&self) -> &HashIndex<K> {
        &self.1
    }
}
