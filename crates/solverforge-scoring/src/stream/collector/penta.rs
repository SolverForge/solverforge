//! Zero-erasure penta-collector for grouping quintuples of entities.

use std::marker::PhantomData;

use super::{Accumulator, UniCollector};

/// A collector that aggregates quintuples of entities (A, A, A, A, A) into a result.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::collector::{penta_count, PentaCollector, Accumulator};
///
/// #[derive(Clone)]
/// struct Task { team: u32 }
///
/// let collector = penta_count::<Task>();
/// let mut acc = collector.create_accumulator();
///
/// let t = Task { team: 1 };
/// acc.accumulate(&collector.extract(&t, &t, &t, &t, &t));
/// assert_eq!(acc.finish(), 1);
/// ```
pub trait PentaCollector<A>: Send + Sync {
    type Value;
    type Result: Clone + Send + Sync;
    type Accumulator: Accumulator<Self::Value, Self::Result>;

    fn extract(&self, a: &A, b: &A, c: &A, d: &A, e: &A) -> Self::Value;
    fn create_accumulator(&self) -> Self::Accumulator;
}

/// Adapts a UniCollector to work with quintuples using a mapping function.
pub struct MappedPentaCollector<A, T, M, C> {
    mapper: M,
    collector: C,
    _phantom: PhantomData<fn(&A, &A, &A, &A, &A) -> T>,
}

impl<A, T, M, C> MappedPentaCollector<A, T, M, C>
where
    A: Send + Sync + 'static,
    T: Send + Sync + 'static,
    M: Fn(&A, &A, &A, &A, &A) -> T + Send + Sync + 'static,
    C: UniCollector<T> + 'static,
{
    pub fn new(mapper: M, collector: C) -> Self {
        Self {
            mapper,
            collector,
            _phantom: PhantomData,
        }
    }
}

impl<A, T, M, C> PentaCollector<A> for MappedPentaCollector<A, T, M, C>
where
    A: Send + Sync + 'static,
    T: Send + Sync + 'static,
    M: Fn(&A, &A, &A, &A, &A) -> T + Send + Sync + 'static,
    C: UniCollector<T> + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Clone + Send + Sync,
{
    type Value = C::Value;
    type Result = C::Result;
    type Accumulator = C::Accumulator;

    #[inline]
    fn extract(&self, a: &A, b: &A, c: &A, d: &A, e: &A) -> Self::Value {
        let mapped = (self.mapper)(a, b, c, d, e);
        self.collector.extract(&mapped)
    }

    fn create_accumulator(&self) -> Self::Accumulator {
        self.collector.create_accumulator()
    }
}

/// Creates a collector that counts quintuples.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::collector::{penta_count, PentaCollector, Accumulator};
///
/// #[derive(Clone)]
/// struct Task { team: u32 }
///
/// let collector = penta_count::<Task>();
/// let mut acc = collector.create_accumulator();
///
/// let t = Task { team: 1 };
/// acc.accumulate(&collector.extract(&t, &t, &t, &t, &t));
/// acc.accumulate(&collector.extract(&t, &t, &t, &t, &t));
/// assert_eq!(acc.finish(), 2);
/// ```
pub fn penta_count<A>() -> MappedPentaCollector<A, (), impl Fn(&A, &A, &A, &A, &A) + Send + Sync, super::CountCollector<()>>
where
    A: Send + Sync + 'static,
{
    MappedPentaCollector::new(|_: &A, _: &A, _: &A, _: &A, _: &A| (), super::count::<()>())
}

/// Creates a collector that sums values extracted from quintuples.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::collector::{penta_sum, PentaCollector, Accumulator};
///
/// #[derive(Clone)]
/// struct Task { cost: i64 }
///
/// let collector = penta_sum(|a: &Task, b: &Task, c: &Task, d: &Task, e: &Task| {
///     a.cost + b.cost + c.cost + d.cost + e.cost
/// });
/// let mut acc = collector.create_accumulator();
///
/// let t1 = Task { cost: 1 };
/// let t2 = Task { cost: 2 };
/// let t3 = Task { cost: 3 };
/// let t4 = Task { cost: 4 };
/// let t5 = Task { cost: 5 };
///
/// acc.accumulate(&collector.extract(&t1, &t2, &t3, &t4, &t5));
/// assert_eq!(acc.finish(), 15);
/// ```
pub fn penta_sum<A, T, M>(
    mapper: M,
) -> MappedPentaCollector<A, T, M, super::SumCollector<T, T, impl Fn(&T) -> T + Send + Sync>>
where
    A: Send + Sync + 'static,
    T: Default + Copy + std::ops::AddAssign + std::ops::SubAssign + Send + Sync + 'static,
    M: Fn(&A, &A, &A, &A, &A) -> T + Send + Sync + 'static,
{
    MappedPentaCollector::new(mapper, super::sum(|v: &T| *v))
}
