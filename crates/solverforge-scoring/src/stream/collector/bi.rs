//! Zero-erasure bi-collector for grouping pairs of entities.
//!
//! Provides collectors that work with pairs (A, A) from BiConstraintStream,
//! enabling `group_by` operations on self-join results.

use std::marker::PhantomData;

use super::{Accumulator, UniCollector};

/// A collector that aggregates pairs of entities (A, A) into a result of type R.
///
/// BiCollectors are used in `BiConstraintStream::group_by()` operations to reduce
/// pairs of entities into summary values.
///
/// # Zero-Erasure Design
///
/// The collector owns any mapping functions and provides `extract()` to convert
/// entity pairs to values. The accumulator only works with extracted values.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::collector::{bi_sum, BiCollector, Accumulator};
///
/// #[derive(Clone)]
/// struct Task { team: u32, cost: i64 }
///
/// // Sum costs for each pair
/// let collector = bi_sum(|a: &Task, b: &Task| a.cost + b.cost);
/// let mut acc = collector.create_accumulator();
///
/// let t1 = Task { team: 1, cost: 5 };
/// let t2 = Task { team: 1, cost: 3 };
/// let t3 = Task { team: 1, cost: 2 };
///
/// acc.accumulate(&collector.extract(&t1, &t2));
/// acc.accumulate(&collector.extract(&t2, &t3));
/// assert_eq!(acc.finish(), 13); // (5+3) + (3+2)
/// ```
pub trait BiCollector<A>: Send + Sync {
    /// The value type extracted from entity pairs and passed to the accumulator.
    type Value;

    /// The result type produced by this collector.
    type Result: Clone + Send + Sync;

    /// The accumulator type used during collection.
    type Accumulator: Accumulator<Self::Value, Self::Result>;

    /// Extracts the value to accumulate from an entity pair.
    fn extract(&self, a: &A, b: &A) -> Self::Value;

    /// Creates a fresh accumulator.
    fn create_accumulator(&self) -> Self::Accumulator;
}

/// Adapts a UniCollector to work with pairs using a mapping function.
///
/// This enables reusing existing collectors (count, sum, etc.) with pairs
/// by providing a mapper that converts (A, A) → T.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::collector::{bi_sum, BiCollector, Accumulator};
///
/// #[derive(Clone)]
/// struct Item { value: i64 }
///
/// let collector = bi_sum(|a: &Item, b: &Item| a.value * b.value);
/// let mut acc = collector.create_accumulator();
///
/// let i1 = Item { value: 2 };
/// let i2 = Item { value: 3 };
/// let i3 = Item { value: 4 };
///
/// acc.accumulate(&collector.extract(&i1, &i2)); // 2*3 = 6
/// acc.accumulate(&collector.extract(&i2, &i3)); // 3*4 = 12
/// assert_eq!(acc.finish(), 18);
/// ```
pub struct MappedBiCollector<A, T, M, C> {
    mapper: M,
    collector: C,
    _phantom: PhantomData<fn(&A, &A) -> T>,
}

impl<A, T, M, C> MappedBiCollector<A, T, M, C>
where
    A: Send + Sync + 'static,
    T: Send + Sync + 'static,
    M: Fn(&A, &A) -> T + Send + Sync + 'static,
    C: UniCollector<T> + 'static,
{
    /// Creates a new mapped bi-collector.
    pub fn new(mapper: M, collector: C) -> Self {
        Self {
            mapper,
            collector,
            _phantom: PhantomData,
        }
    }
}

impl<A, T, M, C> BiCollector<A> for MappedBiCollector<A, T, M, C>
where
    A: Send + Sync + 'static,
    T: Send + Sync + 'static,
    M: Fn(&A, &A) -> T + Send + Sync + 'static,
    C: UniCollector<T> + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Clone + Send + Sync,
{
    type Value = C::Value;
    type Result = C::Result;
    type Accumulator = C::Accumulator;

    #[inline]
    fn extract(&self, a: &A, b: &A) -> Self::Value {
        let mapped = (self.mapper)(a, b);
        self.collector.extract(&mapped)
    }

    fn create_accumulator(&self) -> Self::Accumulator {
        self.collector.create_accumulator()
    }
}

/// Creates a collector that counts pairs.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::collector::{bi_count, BiCollector, Accumulator};
///
/// #[derive(Clone)]
/// struct Task { team: u32 }
///
/// let collector = bi_count::<Task>();
/// let mut acc = collector.create_accumulator();
///
/// let t1 = Task { team: 1 };
/// let t2 = Task { team: 1 };
///
/// acc.accumulate(&collector.extract(&t1, &t2));
/// acc.accumulate(&collector.extract(&t1, &t2));
/// assert_eq!(acc.finish(), 2);
/// ```
pub fn bi_count<A>() -> MappedBiCollector<A, (), impl Fn(&A, &A) -> () + Send + Sync, super::CountCollector<()>>
where
    A: Send + Sync + 'static,
{
    MappedBiCollector::new(|_: &A, _: &A| (), super::count::<()>())
}

/// Creates a collector that sums values extracted from pairs.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::collector::{bi_sum, BiCollector, Accumulator};
///
/// #[derive(Clone)]
/// struct Task { cost: i64 }
///
/// let collector = bi_sum(|a: &Task, b: &Task| a.cost + b.cost);
/// let mut acc = collector.create_accumulator();
///
/// let t1 = Task { cost: 5 };
/// let t2 = Task { cost: 3 };
/// let t3 = Task { cost: 7 };
///
/// acc.accumulate(&collector.extract(&t1, &t2)); // 5+3=8
/// acc.accumulate(&collector.extract(&t2, &t3)); // 3+7=10
/// assert_eq!(acc.finish(), 18);
/// ```
pub fn bi_sum<A, T, M>(
    mapper: M,
) -> MappedBiCollector<A, T, M, super::SumCollector<T, T, impl Fn(&T) -> T + Send + Sync>>
where
    A: Send + Sync + 'static,
    T: Default + Copy + std::ops::AddAssign + std::ops::SubAssign + Send + Sync + 'static,
    M: Fn(&A, &A) -> T + Send + Sync + 'static,
{
    MappedBiCollector::new(mapper, super::sum(|v: &T| *v))
}
