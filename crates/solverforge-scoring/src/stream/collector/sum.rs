/* Zero-erasure sum collector for summing values.

All type information is preserved at compile time - no Arc, no dyn, no Clone.
*/

use std::marker::PhantomData;
use std::ops::{AddAssign, SubAssign};

use super::{Accumulator, Collector};

/* Creates a zero-erasure collector that sums values extracted from stream matches.

# Example

```
use solverforge_scoring::stream::collector::{sum, Accumulator, Collector};

struct Item { value: i64 }

let collector = sum(|item: &Item| item.value);
let mut acc = collector.create_accumulator();

acc.accumulate(collector.extract(&Item { value: 5 }));
let middle = acc.accumulate(collector.extract(&Item { value: 3 }));
acc.accumulate(collector.extract(&Item { value: 7 }));
assert_eq!(acc.finish(), 15);

acc.retract(middle);
assert_eq!(acc.finish(), 12);
```
*/
pub fn sum<T, F>(mapper: F) -> SumCollector<T, F>
where
    T: Default + Copy + AddAssign + SubAssign + Send + Sync + 'static,
    F: Send + Sync + 'static,
{
    SumCollector {
        mapper,
        _phantom: PhantomData,
    }
}

/* Zero-erasure collector that sums values extracted from stream matches.

Created by the [`sum()`] function.
The mapper function is stored once in the collector, not cloned into accumulators.
*/
pub struct SumCollector<T, F> {
    mapper: F,
    _phantom: PhantomData<fn() -> T>,
}

impl<Input, T, F> Collector<Input> for SumCollector<T, F>
where
    Input: Send + Sync,
    T: Default + Copy + AddAssign + SubAssign + Send + Sync + 'static,
    F: Fn(Input) -> T + Send + Sync + 'static,
{
    type Value = T;
    type Result = T;
    type Accumulator = SumAccumulator<T>;

    #[inline]
    fn extract(&self, input: Input) -> T {
        (self.mapper)(input)
    }

    fn create_accumulator(&self) -> Self::Accumulator {
        SumAccumulator { sum: T::default() }
    }
}

/* Zero-erasure accumulator for summing values.

Works with pre-extracted values, not entities directly.
*/
pub struct SumAccumulator<T> {
    sum: T,
}

impl<T> Accumulator<T, T> for SumAccumulator<T>
where
    T: Default + Copy + AddAssign + SubAssign + Send + Sync,
{
    type Retraction = T;

    #[inline]
    fn accumulate(&mut self, value: T) -> Self::Retraction {
        self.sum += value;
        value
    }

    #[inline]
    fn retract(&mut self, value: Self::Retraction) {
        self.sum -= value;
    }

    #[inline]
    fn with_result<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&self.sum)
    }

    #[inline]
    fn reset(&mut self) {
        self.sum = T::default();
    }
}
