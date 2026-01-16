//! Zero-erasure tri-collector for grouping triples of entities.
//!
//! Provides collectors that work with triples (A, A, A) from TriConstraintStream,
//! enabling `group_by` operations on self-join results.

use std::marker::PhantomData;

use super::{Accumulator, UniCollector};

/// A collector that aggregates triples of entities (A, A, A) into a result of type R.
///
/// TriCollectors are used in `TriConstraintStream::group_by()` operations to reduce
/// triples of entities into summary values.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::collector::{tri_sum, TriCollector, Accumulator};
///
/// #[derive(Clone)]
/// struct Task { team: u32, cost: i64 }
///
/// // Sum costs for each triple
/// let collector = tri_sum(|a: &Task, b: &Task, c: &Task| a.cost + b.cost + c.cost);
/// let mut acc = collector.create_accumulator();
///
/// let t1 = Task { team: 1, cost: 5 };
/// let t2 = Task { team: 1, cost: 3 };
/// let t3 = Task { team: 1, cost: 2 };
///
/// acc.accumulate(&collector.extract(&t1, &t2, &t3));
/// assert_eq!(acc.finish(), 10); // 5+3+2
/// ```
pub trait TriCollector<A>: Send + Sync {
    /// The value type extracted from entity triples and passed to the accumulator.
    type Value;

    /// The result type produced by this collector.
    type Result: Clone + Send + Sync;

    /// The accumulator type used during collection.
    type Accumulator: Accumulator<Self::Value, Self::Result>;

    /// Extracts the value to accumulate from an entity triple.
    fn extract(&self, a: &A, b: &A, c: &A) -> Self::Value;

    /// Creates a fresh accumulator.
    fn create_accumulator(&self) -> Self::Accumulator;
}

/// Adapts a UniCollector to work with triples using a mapping function.
pub struct MappedTriCollector<A, T, M, C> {
    mapper: M,
    collector: C,
    _phantom: PhantomData<fn(&A, &A, &A) -> T>,
}

impl<A, T, M, C> MappedTriCollector<A, T, M, C>
where
    A: Send + Sync + 'static,
    T: Send + Sync + 'static,
    M: Fn(&A, &A, &A) -> T + Send + Sync + 'static,
    C: UniCollector<T> + 'static,
{
    /// Creates a new mapped tri-collector.
    pub fn new(mapper: M, collector: C) -> Self {
        Self {
            mapper,
            collector,
            _phantom: PhantomData,
        }
    }
}

impl<A, T, M, C> TriCollector<A> for MappedTriCollector<A, T, M, C>
where
    A: Send + Sync + 'static,
    T: Send + Sync + 'static,
    M: Fn(&A, &A, &A) -> T + Send + Sync + 'static,
    C: UniCollector<T> + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Clone + Send + Sync,
{
    type Value = C::Value;
    type Result = C::Result;
    type Accumulator = C::Accumulator;

    #[inline]
    fn extract(&self, a: &A, b: &A, c: &A) -> Self::Value {
        let mapped = (self.mapper)(a, b, c);
        self.collector.extract(&mapped)
    }

    fn create_accumulator(&self) -> Self::Accumulator {
        self.collector.create_accumulator()
    }
}

/// Creates a collector that counts triples.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::collector::{tri_count, TriCollector, Accumulator};
///
/// #[derive(Clone)]
/// struct Task { team: u32 }
///
/// let collector = tri_count::<Task>();
/// let mut acc = collector.create_accumulator();
///
/// let t1 = Task { team: 1 };
/// let t2 = Task { team: 1 };
/// let t3 = Task { team: 1 };
///
/// acc.accumulate(&collector.extract(&t1, &t2, &t3));
/// acc.accumulate(&collector.extract(&t1, &t2, &t3));
/// assert_eq!(acc.finish(), 2);
/// ```
pub fn tri_count<A>() -> MappedTriCollector<A, (), impl Fn(&A, &A, &A) + Send + Sync, super::CountCollector<()>>
where
    A: Send + Sync + 'static,
{
    MappedTriCollector::new(|_: &A, _: &A, _: &A| (), super::count::<()>())
}

/// Creates a collector that sums values extracted from triples.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::collector::{tri_sum, TriCollector, Accumulator};
///
/// #[derive(Clone)]
/// struct Task { cost: i64 }
///
/// let collector = tri_sum(|a: &Task, b: &Task, c: &Task| a.cost + b.cost + c.cost);
/// let mut acc = collector.create_accumulator();
///
/// let t1 = Task { cost: 5 };
/// let t2 = Task { cost: 3 };
/// let t3 = Task { cost: 7 };
///
/// acc.accumulate(&collector.extract(&t1, &t2, &t3));
/// assert_eq!(acc.finish(), 15);
/// ```
pub fn tri_sum<A, T, M>(
    mapper: M,
) -> MappedTriCollector<A, T, M, super::SumCollector<T, T, impl Fn(&T) -> T + Send + Sync>>
where
    A: Send + Sync + 'static,
    T: Default + Copy + std::ops::AddAssign + std::ops::SubAssign + Send + Sync + 'static,
    M: Fn(&A, &A, &A) -> T + Send + Sync + 'static,
{
    MappedTriCollector::new(mapper, super::sum(|v: &T| *v))
}
