// Overlapping joiner for interval overlap detection.

use std::marker::PhantomData;

use super::Joiner;

/* Creates a joiner that matches when two intervals overlap.

Two intervals [start_a, end_a) and [start_b, end_b) overlap if:
- start_a < end_b AND start_b < end_a

This uses half-open intervals: the start is inclusive, the end is exclusive.

# Example

```
use solverforge_scoring::stream::joiner::{Joiner, overlapping};

#[derive(Clone)]
struct Shift { start: i64, end: i64 }

let overlap = overlapping(
|s: &Shift| s.start,
|s: &Shift| s.end,
|s: &Shift| s.start,
|s: &Shift| s.end
);

// Overlapping shifts: [0, 10) and [5, 15) overlap at [5, 10)
assert!(overlap.matches(
&Shift { start: 0, end: 10 },
&Shift { start: 5, end: 15 }
));

// Non-overlapping shifts: [0, 10) and [10, 20) touch but don't overlap
assert!(!overlap.matches(
&Shift { start: 0, end: 10 },
&Shift { start: 10, end: 20 }
));

// Non-overlapping shifts: [0, 5) and [10, 15) are disjoint
assert!(!overlap.matches(
&Shift { start: 0, end: 5 },
&Shift { start: 10, end: 15 }
));
```
*/
pub fn overlapping<T, Fsa, Fea, Fsb, Feb>(
    start_a: Fsa,
    end_a: Fea,
    start_b: Fsb,
    end_b: Feb,
) -> OverlappingJoiner<Fsa, Fea, Fsb, Feb, T>
where
    T: Ord,
    Fsa: Send + Sync,
    Fea: Send + Sync,
    Fsb: Send + Sync,
    Feb: Send + Sync,
{
    OverlappingJoiner {
        start_a,
        end_a,
        start_b,
        end_b,
        _phantom: PhantomData,
    }
}

/* Creates a joiner matching when the left row's interval overlaps the right's.

Row-aware form of [`overlapping`]: start/end closures for the left side
receive the whole borrowed row; the right side sees only the new entity.
Half-open endpoint semantics are unchanged: `[s, e)` overlaps iff
`start_row < end_c AND start_c < end_row`. Bound extractors stay
accessible for interval candidate planning.
*/
pub fn overlap_on<T, LS, LE, CS, CE>(
    start_row: LS,
    end_row: LE,
    start_c: CS,
    end_c: CE,
) -> OverlappingJoiner<LS, LE, CS, CE, T>
where
    T: Ord,
    LS: Send + Sync,
    LE: Send + Sync,
    CS: Send + Sync,
    CE: Send + Sync,
{
    OverlappingJoiner {
        start_a: start_row,
        end_a: end_row,
        start_b: start_c,
        end_b: end_c,
        _phantom: PhantomData,
    }
}

/* A joiner that matches when two intervals overlap.

Created by the [`overlapping()`] function.
*/
pub struct OverlappingJoiner<Fsa, Fea, Fsb, Feb, T> {
    pub(super) start_a: Fsa,
    pub(super) end_a: Fea,
    pub(super) start_b: Fsb,
    pub(super) end_b: Feb,
    _phantom: PhantomData<fn() -> T>,
}

impl<Fsa, Fea, Fsb, Feb, T> OverlappingJoiner<Fsa, Fea, Fsb, Feb, T> {
    /// Compose without prematurely binding a borrowed row lifetime.
    pub fn and<J>(self, other: J) -> super::AndJoiner<Self, J> {
        super::AndJoiner {
            first: self,
            second: other,
        }
    }
    /* Consumes the joiner and returns the bound extractors for index planning. */
    #[inline]
    pub fn into_bounds(self) -> (Fsa, Fea, Fsb, Feb) {
        (self.start_a, self.end_a, self.start_b, self.end_b)
    }
}

impl<A, B, T, Fsa, Fea, Fsb, Feb> Joiner<A, B> for OverlappingJoiner<Fsa, Fea, Fsb, Feb, T>
where
    T: Ord,
    Fsa: Fn(&A) -> T + Send + Sync,
    Fea: Fn(&A) -> T + Send + Sync,
    Fsb: Fn(&B) -> T + Send + Sync,
    Feb: Fn(&B) -> T + Send + Sync,
{
    #[inline]
    fn matches(&self, a: &A, b: &B) -> bool {
        let start_a = (self.start_a)(a);
        let end_a = (self.end_a)(a);
        let start_b = (self.start_b)(b);
        let end_b = (self.end_b)(b);

        // Half-open interval overlap: [start_a, end_a) ∩ [start_b, end_b) ≠ ∅
        // iff start_a < end_b AND start_b < end_a
        start_a < end_b && start_b < end_a
    }
}
