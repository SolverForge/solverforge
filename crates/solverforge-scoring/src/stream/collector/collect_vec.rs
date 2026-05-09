use std::marker::PhantomData;

use super::{Accumulator, UniCollector};

/// Creates a collector that gathers mapped values into a vector.
///
/// The returned vector preserves the accumulator's current insertion order, but that ordering is
/// not a semantic guarantee. Consumers that depend on ordering must sort the collected values
/// before scoring.
pub fn collect_vec<A, T, F>(mapper: F) -> CollectVecCollector<A, T, F>
where
    A: Send + Sync + 'static,
    T: Copy + PartialEq + Send + Sync + 'static,
    F: Fn(&A) -> T + Send + Sync + 'static,
{
    CollectVecCollector {
        mapper,
        _phantom: PhantomData,
    }
}

pub struct CollectVecCollector<A, T, F> {
    mapper: F,
    _phantom: PhantomData<fn(&A) -> T>,
}

impl<A, T, F> UniCollector<A> for CollectVecCollector<A, T, F>
where
    A: Send + Sync + 'static,
    T: Copy + PartialEq + Send + Sync + 'static,
    F: Fn(&A) -> T + Send + Sync + 'static,
{
    type Value = T;
    type Result = Vec<T>;
    type Accumulator = CollectVecAccumulator<T>;

    #[inline]
    fn extract(&self, entity: &A) -> T {
        (self.mapper)(entity)
    }

    fn create_accumulator(&self) -> Self::Accumulator {
        CollectVecAccumulator { values: Vec::new() }
    }
}

pub struct CollectVecAccumulator<T> {
    values: Vec<T>,
}

impl<T> Accumulator<T, Vec<T>> for CollectVecAccumulator<T>
where
    T: Copy + PartialEq + Send + Sync,
{
    #[inline]
    fn accumulate(&mut self, value: &T) {
        self.values.push(*value);
    }

    #[inline]
    fn retract(&mut self, value: &T) {
        if let Some(index) = self.values.iter().position(|current| current == value) {
            self.values.remove(index);
        }
    }

    #[inline]
    fn finish(&self) -> Vec<T> {
        self.values.iter().copied().collect()
    }

    #[inline]
    fn reset(&mut self) {
        self.values.clear();
    }
}
