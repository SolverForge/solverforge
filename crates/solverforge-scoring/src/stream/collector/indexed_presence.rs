use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::ops::Range;

use super::runs::{runs_from_counts, Runs};
use super::{Accumulator, Collector};

/// Creates a collector recording which `i64` indexes the group's matches map to.
///
/// `index_fn` maps each match to an index; several matches may share one.
pub fn indexed_presence<F>(index_fn: F) -> IndexedPresenceCollector<F>
where
    F: Send + Sync,
{
    IndexedPresenceCollector {
        index_fn,
        _phantom: PhantomData,
    }
}

/// Collector created by [`indexed_presence`].
pub struct IndexedPresenceCollector<F> {
    index_fn: F,
    _phantom: PhantomData<fn()>,
}

impl<Input, F> Collector<Input> for IndexedPresenceCollector<F>
where
    Input: Send + Sync,
    F: Fn(Input) -> i64 + Send + Sync,
{
    type Value = i64;
    type Result = IndexedPresence;
    type Accumulator = IndexedPresenceAccumulator;

    #[inline]
    fn extract(&self, input: Input) -> Self::Value {
        (self.index_fn)(input)
    }

    fn create_accumulator(&self) -> Self::Accumulator {
        IndexedPresenceAccumulator::new()
    }
}

/// The set of indexes present in a group, with the number of matches mapped to
/// them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IndexedPresence {
    points: BTreeMap<i64, usize>,
    item_count: usize,
}

impl IndexedPresence {
    /// Returns `true` if some match maps to `index`.
    #[inline]
    pub fn contains(&self, index: i64) -> bool {
        self.points.contains_key(&index)
    }

    /// Returns the number of distinct indexes present.
    #[inline]
    pub fn count(&self) -> usize {
        self.points.len()
    }

    /// Returns the number of matches, counting duplicates.
    #[inline]
    pub fn item_count(&self) -> usize {
        self.item_count
    }

    /// Returns `true` when no index is present.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Returns the maximal runs of consecutive present indexes.
    pub fn runs(&self) -> Runs {
        runs_from_counts(&self.points)
    }

    /// Returns the maximal runs of consecutive indexes in `horizon` that are absent.
    ///
    /// Walks every index in `horizon`, so the cost grows with its length. An empty
    /// horizon yields no runs.
    pub fn complement_runs(&self, horizon: Range<i64>) -> Runs {
        if horizon.start >= horizon.end {
            return Runs::default();
        }

        let mut complement = BTreeMap::new();
        let mut index = horizon.start;
        while index < horizon.end {
            if !self.points.contains_key(&index) {
                complement.insert(index, 1);
            }
            let Some(next) = index.checked_add(1) else {
                break;
            };
            index = next;
        }
        runs_from_counts(&complement)
    }

    /// Returns the number of distinct present indexes in `range`.
    pub fn count_in(&self, range: Range<i64>) -> usize {
        if range.start >= range.end {
            return 0;
        }
        self.points.range(range).count()
    }

    /// Returns `true` if any index in `range` is present.
    pub fn any_in(&self, range: Range<i64>) -> bool {
        self.count_in(range) > 0
    }
}

/// Accumulator of [`IndexedPresenceCollector`]; its retraction token is the index.
pub struct IndexedPresenceAccumulator {
    presence: IndexedPresence,
}

impl IndexedPresenceAccumulator {
    fn new() -> Self {
        Self {
            presence: IndexedPresence::default(),
        }
    }
}

impl Accumulator<i64, IndexedPresence> for IndexedPresenceAccumulator {
    type Retraction = i64;

    #[inline]
    fn accumulate(&mut self, value: i64) -> Self::Retraction {
        *self.presence.points.entry(value).or_insert(0) += 1;
        self.presence.item_count += 1;
        value
    }

    #[inline]
    fn retract(&mut self, value: Self::Retraction) {
        let Some(count) = self.presence.points.get_mut(&value) else {
            return;
        };
        *count = count.saturating_sub(1);
        self.presence.item_count = self.presence.item_count.saturating_sub(1);
        if *count == 0 {
            self.presence.points.remove(&value);
        }
    }

    fn with_result<T>(&self, f: impl FnOnce(&IndexedPresence) -> T) -> T {
        f(&self.presence)
    }

    #[inline]
    fn reset(&mut self) {
        self.presence.points.clear();
        self.presence.item_count = 0;
    }
}
