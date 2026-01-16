//! Zero-erasure cross-bi-collector for grouping pairs of different entity types.
//!
//! Provides collectors that work with cross-entity pairs (A, B) from CrossBiConstraintStream.

use std::marker::PhantomData;

use super::{Accumulator, UniCollector};

/// A collector that aggregates cross-entity pairs (A, B) into a result.
///
/// CrossBiCollectors are used in `CrossBiConstraintStream::group_by()` operations.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::collector::{cross_bi_count, CrossBiCollector, Accumulator};
///
/// #[derive(Clone)]
/// struct Shift { day: u32 }
/// #[derive(Clone)]
/// struct Employee { id: usize }
///
/// let collector = cross_bi_count::<Shift, Employee>();
/// let mut acc = collector.create_accumulator();
///
/// let shift = Shift { day: 1 };
/// let emp = Employee { id: 0 };
///
/// acc.accumulate(&collector.extract(&shift, &emp));
/// acc.accumulate(&collector.extract(&shift, &emp));
/// assert_eq!(acc.finish(), 2);
/// ```
pub trait CrossBiCollector<A, B>: Send + Sync {
    type Value;
    type Result: Clone + Send + Sync;
    type Accumulator: Accumulator<Self::Value, Self::Result>;

    fn extract(&self, a: &A, b: &B) -> Self::Value;
    fn create_accumulator(&self) -> Self::Accumulator;
}

/// Adapts a UniCollector to work with cross-entity pairs using a mapping function.
pub struct MappedCrossBiCollector<A, B, T, M, C> {
    mapper: M,
    collector: C,
    _phantom: PhantomData<fn(&A, &B) -> T>,
}

impl<A, B, T, M, C> MappedCrossBiCollector<A, B, T, M, C>
where
    A: Send + Sync + 'static,
    B: Send + Sync + 'static,
    T: Send + Sync + 'static,
    M: Fn(&A, &B) -> T + Send + Sync + 'static,
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

impl<A, B, T, M, C> CrossBiCollector<A, B> for MappedCrossBiCollector<A, B, T, M, C>
where
    A: Send + Sync + 'static,
    B: Send + Sync + 'static,
    T: Send + Sync + 'static,
    M: Fn(&A, &B) -> T + Send + Sync + 'static,
    C: UniCollector<T> + 'static,
    C::Accumulator: Send + Sync,
    C::Result: Clone + Send + Sync,
{
    type Value = C::Value;
    type Result = C::Result;
    type Accumulator = C::Accumulator;

    #[inline]
    fn extract(&self, a: &A, b: &B) -> Self::Value {
        let mapped = (self.mapper)(a, b);
        self.collector.extract(&mapped)
    }

    fn create_accumulator(&self) -> Self::Accumulator {
        self.collector.create_accumulator()
    }
}

/// Creates a collector that counts cross-entity pairs.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::collector::{cross_bi_count, CrossBiCollector, Accumulator};
///
/// #[derive(Clone)]
/// struct Shift { day: u32 }
/// #[derive(Clone)]
/// struct Employee { id: usize }
///
/// let collector = cross_bi_count::<Shift, Employee>();
/// let mut acc = collector.create_accumulator();
///
/// let shift = Shift { day: 1 };
/// let emp = Employee { id: 0 };
///
/// acc.accumulate(&collector.extract(&shift, &emp));
/// assert_eq!(acc.finish(), 1);
/// ```
pub fn cross_bi_count<A, B>() -> MappedCrossBiCollector<A, B, (), impl Fn(&A, &B) + Send + Sync, super::CountCollector<()>>
where
    A: Send + Sync + 'static,
    B: Send + Sync + 'static,
{
    MappedCrossBiCollector::new(|_: &A, _: &B| (), super::count::<()>())
}

/// Creates a collector that sums values extracted from cross-entity pairs.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::collector::{cross_bi_sum, CrossBiCollector, Accumulator};
///
/// #[derive(Clone)]
/// struct Shift { hours: i64 }
/// #[derive(Clone)]
/// struct Employee { rate: i64 }
///
/// let collector = cross_bi_sum(|s: &Shift, e: &Employee| s.hours * e.rate);
/// let mut acc = collector.create_accumulator();
///
/// let shift = Shift { hours: 8 };
/// let emp = Employee { rate: 25 };
///
/// acc.accumulate(&collector.extract(&shift, &emp));
/// assert_eq!(acc.finish(), 200);
/// ```
pub fn cross_bi_sum<A, B, T, M>(
    mapper: M,
) -> MappedCrossBiCollector<A, B, T, M, super::SumCollector<T, T, impl Fn(&T) -> T + Send + Sync>>
where
    A: Send + Sync + 'static,
    B: Send + Sync + 'static,
    T: Default + Copy + std::ops::AddAssign + std::ops::SubAssign + Send + Sync + 'static,
    M: Fn(&A, &B) -> T + Send + Sync + 'static,
{
    MappedCrossBiCollector::new(mapper, super::sum(|v: &T| *v))
}
