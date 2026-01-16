//! Zero-erasure quad-collector for grouping quadruples of entities.

use std::marker::PhantomData;

use super::{Accumulator, UniCollector};

/// A collector that aggregates quadruples of entities (A, A, A, A) into a result.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::collector::{quad_count, QuadCollector, Accumulator};
///
/// #[derive(Clone)]
/// struct Task { team: u32 }
///
/// let collector = quad_count::<Task>();
/// let mut acc = collector.create_accumulator();
///
/// let t1 = Task { team: 1 };
/// let t2 = Task { team: 1 };
/// let t3 = Task { team: 1 };
/// let t4 = Task { team: 1 };
///
/// acc.accumulate(&collector.extract(&t1, &t2, &t3, &t4));
/// assert_eq!(acc.finish(), 1);
/// ```
pub trait QuadCollector<A>: Send + Sync {
    type Value;
    type Result: Clone + Send + Sync;
    type Accumulator: Accumulator<Self::Value, Self::Result>;

    fn extract(&self, a: &A, b: &A, c: &A, d: &A) -> Self::Value;
    fn create_accumulator(&self) -> Self::Accumulator;
}

/// Adapts a UniCollector to work with quadruples using a mapping function.
pub struct MappedQuadCollector<A, T, M, C> {
    mapper: M,
    collector: C,
    _phantom: PhantomData<fn(&A, &A, &A, &A) -> T>,
}

impl<A, T, M, C> MappedQuadCollector<A, T, M, C>
where
    A: Send + Sync + 'static,
    T: Send + Sync + 'static,
    M: Fn(&A, &A, &A, &A) -> T + Send + Sync + 'static,
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

impl<A, T, M, C> QuadCollector<A> for MappedQuadCollector<A, T, M, C>
where
    A: Send + Sync + 'static,
    T: Send + Sync + 'static,
    M: Fn(&A, &A, &A, &A) -> T + Send + Sync + 'static,
    C: UniCollector<T> + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Clone + Send + Sync,
{
    type Value = C::Value;
    type Result = C::Result;
    type Accumulator = C::Accumulator;

    #[inline]
    fn extract(&self, a: &A, b: &A, c: &A, d: &A) -> Self::Value {
        let mapped = (self.mapper)(a, b, c, d);
        self.collector.extract(&mapped)
    }

    fn create_accumulator(&self) -> Self::Accumulator {
        self.collector.create_accumulator()
    }
}

/// Creates a collector that counts quadruples.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::collector::{quad_count, QuadCollector, Accumulator};
///
/// #[derive(Clone)]
/// struct Task { team: u32 }
///
/// let collector = quad_count::<Task>();
/// let mut acc = collector.create_accumulator();
///
/// let t = Task { team: 1 };
/// acc.accumulate(&collector.extract(&t, &t, &t, &t));
/// acc.accumulate(&collector.extract(&t, &t, &t, &t));
/// assert_eq!(acc.finish(), 2);
/// ```
pub fn quad_count<A>() -> MappedQuadCollector<A, (), impl Fn(&A, &A, &A, &A) -> () + Send + Sync, super::CountCollector<()>>
where
    A: Send + Sync + 'static,
{
    MappedQuadCollector::new(|_: &A, _: &A, _: &A, _: &A| (), super::count::<()>())
}

/// Creates a collector that sums values extracted from quadruples.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::collector::{quad_sum, QuadCollector, Accumulator};
///
/// #[derive(Clone)]
/// struct Task { cost: i64 }
///
/// let collector = quad_sum(|a: &Task, b: &Task, c: &Task, d: &Task| a.cost + b.cost + c.cost + d.cost);
/// let mut acc = collector.create_accumulator();
///
/// let t1 = Task { cost: 1 };
/// let t2 = Task { cost: 2 };
/// let t3 = Task { cost: 3 };
/// let t4 = Task { cost: 4 };
///
/// acc.accumulate(&collector.extract(&t1, &t2, &t3, &t4));
/// assert_eq!(acc.finish(), 10);
/// ```
pub fn quad_sum<A, T, M>(
    mapper: M,
) -> MappedQuadCollector<A, T, M, super::SumCollector<T, T, impl Fn(&T) -> T + Send + Sync>>
where
    A: Send + Sync + 'static,
    T: Default + Copy + std::ops::AddAssign + std::ops::SubAssign + Send + Sync + 'static,
    M: Fn(&A, &A, &A, &A) -> T + Send + Sync + 'static,
{
    MappedQuadCollector::new(mapper, super::sum(|v: &T| *v))
}
