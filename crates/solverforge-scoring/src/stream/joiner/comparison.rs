// Comparison joiners for less than / greater than matching.

use std::marker::PhantomData;

use super::Joiner;

/* Creates a joiner that matches when `left(a) < right(b)`.

# Example

```
use solverforge_scoring::stream::joiner::{Joiner, less_than};

#[derive(Clone)]
struct Task { end: i64, start: i64 }

// Task A must end before Task B starts
let sequential = less_than(|t: &Task| t.end, |t: &Task| t.start);

assert!(sequential.matches(
&Task { end: 10, start: 0 },
&Task { end: 20, start: 15 }
));

assert!(!sequential.matches(
&Task { end: 10, start: 0 },
&Task { end: 20, start: 5 }
));
```
*/
pub fn less_than<T, Fa, Fb>(left: Fa, right: Fb) -> LessThanJoiner<Fa, Fb, T>
where
    T: Ord,
    Fa: Send + Sync,
    Fb: Send + Sync,
{
    LessThanJoiner {
        left,
        right,
        _phantom: PhantomData,
    }
}

/* Creates a joiner matching when the left row's key is less than the right's.

Row-aware form of [`less_than`]: the left closure receives the whole
borrowed left row, so a later comparison can inspect any earlier
binding. Extractors stay accessible for ordered index planning.
*/
pub fn less_than_on<T, LK, KC>(left: LK, right: KC) -> LessThanJoiner<LK, KC, T>
where
    T: Ord,
    LK: Send + Sync,
    KC: Send + Sync,
{
    LessThanJoiner {
        left,
        right,
        _phantom: PhantomData,
    }
}

// A joiner that matches when `left(a) < right(b)`.
pub struct LessThanJoiner<Fa, Fb, T> {
    pub(super) left: Fa,
    pub(super) right: Fb,
    _phantom: PhantomData<fn() -> T>,
}

impl<Fa, Fb, T> LessThanJoiner<Fa, Fb, T> {
    /// Compose without prematurely binding a borrowed row lifetime.
    pub fn and<J>(self, other: J) -> super::AndJoiner<Self, J> {
        super::AndJoiner {
            first: self,
            second: other,
        }
    }
    /* Consumes the joiner and returns the key extractors for index planning. */
    #[inline]
    pub fn into_keys(self) -> (Fa, Fb) {
        (self.left, self.right)
    }
}

impl<A, B, T, Fa, Fb> Joiner<A, B> for LessThanJoiner<Fa, Fb, T>
where
    T: Ord,
    Fa: Fn(&A) -> T + Send + Sync,
    Fb: Fn(&B) -> T + Send + Sync,
{
    #[inline]
    fn matches(&self, a: &A, b: &B) -> bool {
        (self.left)(a) < (self.right)(b)
    }
}

/* Creates a joiner that matches when `left(a) <= right(b)`.

# Example

```
use solverforge_scoring::stream::joiner::{Joiner, less_than_or_equal};

let joiner = less_than_or_equal(|x: &i32| *x, |y: &i32| *y);

assert!(joiner.matches(&5, &10));
assert!(joiner.matches(&5, &5));
assert!(!joiner.matches(&10, &5));
```
*/
pub fn less_than_or_equal<T, Fa, Fb>(left: Fa, right: Fb) -> LessThanOrEqualJoiner<Fa, Fb, T>
where
    T: Ord,
    Fa: Send + Sync,
    Fb: Send + Sync,
{
    LessThanOrEqualJoiner {
        left,
        right,
        _phantom: PhantomData,
    }
}

/* Row-aware form of [`less_than_or_equal`]: strict/inclusive direction
preserved, left closure over the whole borrowed row. */
pub fn less_than_or_equal_on<T, LK, KC>(left: LK, right: KC) -> LessThanOrEqualJoiner<LK, KC, T>
where
    T: Ord,
    LK: Send + Sync,
    KC: Send + Sync,
{
    LessThanOrEqualJoiner {
        left,
        right,
        _phantom: PhantomData,
    }
}

// A joiner that matches when `left(a) <= right(b)`.
pub struct LessThanOrEqualJoiner<Fa, Fb, T> {
    pub(super) left: Fa,
    pub(super) right: Fb,
    _phantom: PhantomData<fn() -> T>,
}

impl<Fa, Fb, T> LessThanOrEqualJoiner<Fa, Fb, T> {
    /// Compose without prematurely binding a borrowed row lifetime.
    pub fn and<J>(self, other: J) -> super::AndJoiner<Self, J> {
        super::AndJoiner {
            first: self,
            second: other,
        }
    }
    /* Consumes the joiner and returns the key extractors for index planning. */
    #[inline]
    pub fn into_keys(self) -> (Fa, Fb) {
        (self.left, self.right)
    }
}

impl<A, B, T, Fa, Fb> Joiner<A, B> for LessThanOrEqualJoiner<Fa, Fb, T>
where
    T: Ord,
    Fa: Fn(&A) -> T + Send + Sync,
    Fb: Fn(&B) -> T + Send + Sync,
{
    #[inline]
    fn matches(&self, a: &A, b: &B) -> bool {
        (self.left)(a) <= (self.right)(b)
    }
}

/* Creates a joiner that matches when `left(a) > right(b)`.

# Example

```
use solverforge_scoring::stream::joiner::{Joiner, greater_than};

let joiner = greater_than(|x: &i32| *x, |y: &i32| *y);

assert!(joiner.matches(&10, &5));
assert!(!joiner.matches(&5, &10));
assert!(!joiner.matches(&5, &5));
```
*/
pub fn greater_than<T, Fa, Fb>(left: Fa, right: Fb) -> GreaterThanJoiner<Fa, Fb, T>
where
    T: Ord,
    Fa: Send + Sync,
    Fb: Send + Sync,
{
    GreaterThanJoiner {
        left,
        right,
        _phantom: PhantomData,
    }
}

/* Row-aware form of [`greater_than`]: direction preserved, left closure
over the whole borrowed row. */
pub fn greater_than_on<T, LK, KC>(left: LK, right: KC) -> GreaterThanJoiner<LK, KC, T>
where
    T: Ord,
    LK: Send + Sync,
    KC: Send + Sync,
{
    GreaterThanJoiner {
        left,
        right,
        _phantom: PhantomData,
    }
}

// A joiner that matches when `left(a) > right(b)`.
pub struct GreaterThanJoiner<Fa, Fb, T> {
    pub(super) left: Fa,
    pub(super) right: Fb,
    _phantom: PhantomData<fn() -> T>,
}

impl<Fa, Fb, T> GreaterThanJoiner<Fa, Fb, T> {
    /// Compose without prematurely binding a borrowed row lifetime.
    pub fn and<J>(self, other: J) -> super::AndJoiner<Self, J> {
        super::AndJoiner {
            first: self,
            second: other,
        }
    }
    /* Consumes the joiner and returns the key extractors for index planning. */
    #[inline]
    pub fn into_keys(self) -> (Fa, Fb) {
        (self.left, self.right)
    }
}

impl<A, B, T, Fa, Fb> Joiner<A, B> for GreaterThanJoiner<Fa, Fb, T>
where
    T: Ord,
    Fa: Fn(&A) -> T + Send + Sync,
    Fb: Fn(&B) -> T + Send + Sync,
{
    #[inline]
    fn matches(&self, a: &A, b: &B) -> bool {
        (self.left)(a) > (self.right)(b)
    }
}

/* Creates a joiner that matches when `left(a) >= right(b)`.

# Example

```
use solverforge_scoring::stream::joiner::{Joiner, greater_than_or_equal};

let joiner = greater_than_or_equal(|x: &i32| *x, |y: &i32| *y);

assert!(joiner.matches(&10, &5));
assert!(joiner.matches(&5, &5));
assert!(!joiner.matches(&5, &10));
```
*/
pub fn greater_than_or_equal<T, Fa, Fb>(left: Fa, right: Fb) -> GreaterThanOrEqualJoiner<Fa, Fb, T>
where
    T: Ord,
    Fa: Send + Sync,
    Fb: Send + Sync,
{
    GreaterThanOrEqualJoiner {
        left,
        right,
        _phantom: PhantomData,
    }
}

/* Row-aware form of [`greater_than_or_equal`]: direction preserved, left
closure over the whole borrowed row. */
pub fn greater_than_or_equal_on<T, LK, KC>(
    left: LK,
    right: KC,
) -> GreaterThanOrEqualJoiner<LK, KC, T>
where
    T: Ord,
    LK: Send + Sync,
    KC: Send + Sync,
{
    GreaterThanOrEqualJoiner {
        left,
        right,
        _phantom: PhantomData,
    }
}

// A joiner that matches when `left(a) >= right(b)`.
pub struct GreaterThanOrEqualJoiner<Fa, Fb, T> {
    pub(super) left: Fa,
    pub(super) right: Fb,
    _phantom: PhantomData<fn() -> T>,
}

impl<Fa, Fb, T> GreaterThanOrEqualJoiner<Fa, Fb, T> {
    /// Compose without prematurely binding a borrowed row lifetime.
    pub fn and<J>(self, other: J) -> super::AndJoiner<Self, J> {
        super::AndJoiner {
            first: self,
            second: other,
        }
    }
    /* Consumes the joiner and returns the key extractors for index planning. */
    #[inline]
    pub fn into_keys(self) -> (Fa, Fb) {
        (self.left, self.right)
    }
}

impl<A, B, T, Fa, Fb> Joiner<A, B> for GreaterThanOrEqualJoiner<Fa, Fb, T>
where
    T: Ord,
    Fa: Fn(&A) -> T + Send + Sync,
    Fb: Fn(&B) -> T + Send + Sync,
{
    #[inline]
    fn matches(&self, a: &A, b: &B) -> bool {
        (self.left)(a) >= (self.right)(b)
    }
}
