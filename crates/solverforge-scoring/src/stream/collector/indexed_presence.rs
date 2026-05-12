use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::ops::Range;

use super::runs::{runs_from_counts, Runs};
use super::{Accumulator, UniCollector};

pub fn indexed_presence<A, F>(index_fn: F) -> IndexedPresenceCollector<A, F>
where
    A: Send + Sync,
    F: Fn(&A) -> i64 + Send + Sync,
{
    IndexedPresenceCollector {
        index_fn,
        _phantom: PhantomData,
    }
}

pub struct IndexedPresenceCollector<A, F> {
    index_fn: F,
    _phantom: PhantomData<fn(&A)>,
}

impl<A, F> UniCollector<A> for IndexedPresenceCollector<A, F>
where
    A: Send + Sync,
    F: Fn(&A) -> i64 + Send + Sync,
{
    type Value = i64;
    type Result = IndexedPresence;
    type Accumulator = IndexedPresenceAccumulator;

    #[inline]
    fn extract(&self, entity: &A) -> Self::Value {
        (self.index_fn)(entity)
    }

    fn create_accumulator(&self) -> Self::Accumulator {
        IndexedPresenceAccumulator::new()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IndexedPresence {
    points: BTreeMap<i64, usize>,
    item_count: usize,
}

impl IndexedPresence {
    #[inline]
    pub fn contains(&self, index: i64) -> bool {
        self.points.contains_key(&index)
    }

    #[inline]
    pub fn count(&self) -> usize {
        self.points.len()
    }

    #[inline]
    pub fn item_count(&self) -> usize {
        self.item_count
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn runs(&self) -> Runs {
        runs_from_counts(&self.points)
    }

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

    pub fn count_in(&self, range: Range<i64>) -> usize {
        if range.start >= range.end {
            return 0;
        }
        self.points.range(range).count()
    }

    pub fn any_in(&self, range: Range<i64>) -> bool {
        self.count_in(range) > 0
    }
}

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
