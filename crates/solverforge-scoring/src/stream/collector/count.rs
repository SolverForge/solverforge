// Count collector for counting stream matches.

use super::{Accumulator, Collector};

/* Creates a collector that counts stream matches.

# Example

```
use solverforge_scoring::stream::collector::{count, Accumulator, Collector};

let collector = count();
let mut acc = collector.create_accumulator();

acc.accumulate(collector.extract(&1));
let second = acc.accumulate(collector.extract(&2));
acc.accumulate(collector.extract(&3));
assert_eq!(acc.get(), 3);

acc.retract(second);
assert_eq!(acc.get(), 2);
```
*/
pub const fn count() -> CountCollector {
    CountCollector
}

/* A collector that counts stream matches.

Created by the [`count()`] function.
*/
pub struct CountCollector;

impl<Input> Collector<Input> for CountCollector
where
    Input: Send + Sync,
{
    type Value = ();
    type Result = usize;
    type Accumulator = CountAccumulator;

    #[inline]
    fn extract(&self, _input: Input) {}

    fn create_accumulator(&self) -> Self::Accumulator {
        CountAccumulator { count: 0 }
    }
}

// Accumulator for counting entities.
pub struct CountAccumulator {
    count: usize,
}

impl CountAccumulator {
    // Returns the current count.
    #[inline]
    pub fn get(&self) -> usize {
        self.count
    }
}

impl Accumulator<(), usize> for CountAccumulator {
    type Retraction = ();

    #[inline]
    fn accumulate(&mut self, _: ()) -> Self::Retraction {
        self.count += 1;
    }

    #[inline]
    fn retract(&mut self, _: Self::Retraction) {
        self.count = self.count.saturating_sub(1);
    }

    #[inline]
    fn with_result<T>(&self, f: impl FnOnce(&usize) -> T) -> T {
        f(&self.count)
    }

    #[inline]
    fn reset(&mut self) {
        self.count = 0;
    }
}
