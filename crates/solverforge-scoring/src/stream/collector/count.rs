//! Count collector for counting entities.

use std::marker::PhantomData;

use super::{Accumulator, UniCollector};

/// Creates a collector that counts entities.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::collector::{count, UniCollector, Accumulator};
///
/// let collector = count::<i32>();
/// let mut acc = collector.create_accumulator();
///
/// acc.accumulate(&collector.extract(&1));
/// acc.accumulate(&collector.extract(&2));
/// acc.accumulate(&collector.extract(&3));
/// assert_eq!(acc.get(), 3);
///
/// acc.retract(&collector.extract(&2));
/// assert_eq!(acc.get(), 2);
/// ```
pub fn count<A>() -> CountCollector<A> {
    CountCollector {
        _phantom: PhantomData,
    }
}

/// A collector that counts entities.
///
/// Created by the [`count()`] function.
pub struct CountCollector<A> {
    _phantom: PhantomData<fn(&A)>,
}

impl<A> UniCollector<A> for CountCollector<A>
where
    A: Send + Sync,
{
    type Value = ();
    type Result = usize;
    type Accumulator = CountAccumulator;

    #[inline]
    fn extract(&self, _entity: &A) {}

    fn create_accumulator(&self) -> Self::Accumulator {
        CountAccumulator { count: 0 }
    }
}

/// Accumulator for counting entities.
pub struct CountAccumulator {
    count: usize,
}

impl CountAccumulator {
    /// Returns the current count.
    #[inline]
    pub fn get(&self) -> usize {
        self.count
    }
}

impl Accumulator<(), usize> for CountAccumulator {
    #[inline]
    fn accumulate(&mut self, _: &()) {
        self.count += 1;
    }

    #[inline]
    fn retract(&mut self, _: &()) {
        self.count = self.count.saturating_sub(1);
    }

    #[inline]
    fn finish(&self) -> usize {
        self.count
    }

    #[inline]
    fn reset(&mut self) {
        self.count = 0;
    }
}
