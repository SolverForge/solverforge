// Zero-erasure sum collector for summing values.
//
// All type information is preserved at compile time - no Arc, no dyn, no Clone.

use std::marker::PhantomData;
use std::ops::{AddAssign, SubAssign};

use super::{Accumulator, UniCollector};

// Creates a zero-erasure collector that sums values extracted from entities.
//
// # Example
//
// ```
// use solverforge_scoring::stream::collector::{sum, UniCollector, Accumulator};
//
// struct Item { value: i64 }
//
// let collector = sum(|item: &Item| item.value);
// let mut acc = collector.create_accumulator();
//
// acc.accumulate(&collector.extract(&Item { value: 5 }));
// acc.accumulate(&collector.extract(&Item { value: 3 }));
// acc.accumulate(&collector.extract(&Item { value: 7 }));
// assert_eq!(acc.finish(), 15);
//
// acc.retract(&collector.extract(&Item { value: 3 }));
// assert_eq!(acc.finish(), 12);
// ```
pub fn sum<A, T, F>(mapper: F) -> SumCollector<A, T, F>
where
    A: Send + Sync + 'static,
    T: Default + Copy + AddAssign + SubAssign + Send + Sync + 'static,
    F: Fn(&A) -> T + Send + Sync + 'static,
{
    SumCollector {
        mapper,
        _phantom: PhantomData,
    }
}

// Zero-erasure collector that sums values extracted from entities.
//
// Created by the [`sum()`] function.
// The mapper function is stored once in the collector, not cloned into accumulators.
pub struct SumCollector<A, T, F> {
    mapper: F,
    _phantom: PhantomData<fn(&A) -> T>,
}

impl<A, T, F> UniCollector<A> for SumCollector<A, T, F>
where
    A: Send + Sync + 'static,
    T: Default + Copy + AddAssign + SubAssign + Send + Sync + 'static,
    F: Fn(&A) -> T + Send + Sync + 'static,
{
    type Value = T;
    type Result = T;
    type Accumulator = SumAccumulator<T>;

    #[inline]
    fn extract(&self, entity: &A) -> T {
        (self.mapper)(entity)
    }

    fn create_accumulator(&self) -> Self::Accumulator {
        SumAccumulator { sum: T::default() }
    }
}

// Zero-erasure accumulator for summing values.
//
// Works with pre-extracted values, not entities directly.
pub struct SumAccumulator<T> {
    sum: T,
}

impl<T> Accumulator<T, T> for SumAccumulator<T>
where
    T: Default + Copy + AddAssign + SubAssign + Send + Sync,
{
    #[inline]
    fn accumulate(&mut self, value: &T) {
        self.sum += *value;
    }

    #[inline]
    fn retract(&mut self, value: &T) {
        self.sum -= *value;
    }

    #[inline]
    fn finish(&self) -> T {
        self.sum
    }

    #[inline]
    fn reset(&mut self) {
        self.sum = T::default();
    }
}
