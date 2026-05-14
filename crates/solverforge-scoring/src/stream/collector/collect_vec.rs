use std::marker::PhantomData;

use super::{Accumulator, Collector};

#[derive(Debug)]
pub struct CollectedVec<T> {
    slots: Vec<Option<T>>,
    order: Vec<usize>,
    len: usize,
}

impl<T> CollectedVec<T> {
    fn new() -> Self {
        Self {
            slots: Vec::new(),
            order: Vec::new(),
            len: 0,
        }
    }

    pub fn iter(&self) -> CollectedVecIter<'_, T> {
        CollectedVecIter {
            values: self,
            order_index: 0,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn to_vec(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.iter().cloned().collect()
    }

    fn push(&mut self, value: T) -> usize {
        let slot = self.slots.len();
        self.slots.push(Some(value));
        self.order.push(slot);
        self.len += 1;
        slot
    }

    fn remove_slot(&mut self, slot: usize) {
        let Some(value) = self.slots.get_mut(slot) else {
            return;
        };
        if value.take().is_some() {
            self.len -= 1;
            self.order.retain(|current| *current != slot);
        }
    }

    fn clear(&mut self) {
        self.slots.clear();
        self.order.clear();
        self.len = 0;
    }
}

pub struct CollectedVecIter<'a, T> {
    values: &'a CollectedVec<T>,
    order_index: usize,
}

impl<'a, T> Iterator for CollectedVecIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(&slot) = self.values.order.get(self.order_index) {
            self.order_index += 1;
            if let Some(value) = self.values.slots.get(slot).and_then(Option::as_ref) {
                return Some(value);
            }
        }
        None
    }
}

impl<'a, T> IntoIterator for &'a CollectedVec<T> {
    type Item = &'a T;
    type IntoIter = CollectedVecIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Creates a collector that gathers mapped values into an insertion-order view.
///
/// The returned view preserves the accumulator's current insertion order, but that ordering is
/// not a semantic guarantee. Consumers that depend on ordering must sort the collected values
/// before scoring.
pub fn collect_vec<T, F>(mapper: F) -> CollectVecCollector<T, F>
where
    T: Send + Sync + 'static,
    F: Send + Sync + 'static,
{
    CollectVecCollector {
        mapper,
        _phantom: PhantomData,
    }
}

pub struct CollectVecCollector<T, F> {
    mapper: F,
    _phantom: PhantomData<fn() -> T>,
}

impl<Input, T, F> Collector<Input> for CollectVecCollector<T, F>
where
    Input: Send + Sync,
    T: Send + Sync + 'static,
    F: Fn(Input) -> T + Send + Sync + 'static,
{
    type Value = T;
    type Result = CollectedVec<T>;
    type Accumulator = CollectVecAccumulator<T>;

    #[inline]
    fn extract(&self, input: Input) -> T {
        (self.mapper)(input)
    }

    fn create_accumulator(&self) -> Self::Accumulator {
        CollectVecAccumulator {
            values: CollectedVec::new(),
        }
    }
}

pub struct CollectVecAccumulator<T> {
    values: CollectedVec<T>,
}

impl<T> Accumulator<T, CollectedVec<T>> for CollectVecAccumulator<T>
where
    T: Send + Sync,
{
    type Retraction = usize;

    #[inline]
    fn accumulate(&mut self, value: T) -> Self::Retraction {
        self.values.push(value)
    }

    #[inline]
    fn retract(&mut self, retraction: Self::Retraction) {
        self.values.remove_slot(retraction);
    }

    #[inline]
    fn with_result<R>(&self, f: impl FnOnce(&CollectedVec<T>) -> R) -> R {
        f(&self.values)
    }

    #[inline]
    fn reset(&mut self) {
        self.values.clear();
    }
}
