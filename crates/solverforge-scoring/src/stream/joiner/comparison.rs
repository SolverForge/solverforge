//! Comparison joiners for less than / greater than matching.

use std::marker::PhantomData;

use super::Joiner;

/// Creates a joiner that matches when `left(a) < right(b)`.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::joiner::{Joiner, less_than};
///
/// #[derive(Clone)]
/// struct Task { end: i64, start: i64 }
///
/// // Task A must end before Task B starts
/// let sequential = less_than(|t: &Task| t.end, |t: &Task| t.start);
///
/// assert!(sequential.matches(
///     &Task { end: 10, start: 0 },
///     &Task { end: 20, start: 15 }
/// ));
///
/// assert!(!sequential.matches(
///     &Task { end: 10, start: 0 },
///     &Task { end: 20, start: 5 }
/// ));
/// ```
pub fn less_than<A, B, T, Fa, Fb>(left: Fa, right: Fb) -> LessThanJoiner<Fa, Fb, T>
where
    T: Ord,
    Fa: Fn(&A) -> T + Send + Sync,
    Fb: Fn(&B) -> T + Send + Sync,
{
    LessThanJoiner {
        left,
        right,
        _phantom: PhantomData,
    }
}

/// A joiner that matches when `left(a) < right(b)`.
pub struct LessThanJoiner<Fa, Fb, T> {
    left: Fa,
    right: Fb,
    _phantom: PhantomData<fn() -> T>,
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

/// Creates a joiner that matches when `left(a) <= right(b)`.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::joiner::{Joiner, less_than_or_equal};
///
/// let joiner = less_than_or_equal(|x: &i32| *x, |y: &i32| *y);
///
/// assert!(joiner.matches(&5, &10));
/// assert!(joiner.matches(&5, &5));
/// assert!(!joiner.matches(&10, &5));
/// ```
pub fn less_than_or_equal<A, B, T, Fa, Fb>(left: Fa, right: Fb) -> LessThanOrEqualJoiner<Fa, Fb, T>
where
    T: Ord,
    Fa: Fn(&A) -> T + Send + Sync,
    Fb: Fn(&B) -> T + Send + Sync,
{
    LessThanOrEqualJoiner {
        left,
        right,
        _phantom: PhantomData,
    }
}

/// A joiner that matches when `left(a) <= right(b)`.
pub struct LessThanOrEqualJoiner<Fa, Fb, T> {
    left: Fa,
    right: Fb,
    _phantom: PhantomData<fn() -> T>,
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

/// Creates a joiner that matches when `left(a) > right(b)`.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::joiner::{Joiner, greater_than};
///
/// let joiner = greater_than(|x: &i32| *x, |y: &i32| *y);
///
/// assert!(joiner.matches(&10, &5));
/// assert!(!joiner.matches(&5, &10));
/// assert!(!joiner.matches(&5, &5));
/// ```
pub fn greater_than<A, B, T, Fa, Fb>(left: Fa, right: Fb) -> GreaterThanJoiner<Fa, Fb, T>
where
    T: Ord,
    Fa: Fn(&A) -> T + Send + Sync,
    Fb: Fn(&B) -> T + Send + Sync,
{
    GreaterThanJoiner {
        left,
        right,
        _phantom: PhantomData,
    }
}

/// A joiner that matches when `left(a) > right(b)`.
pub struct GreaterThanJoiner<Fa, Fb, T> {
    left: Fa,
    right: Fb,
    _phantom: PhantomData<fn() -> T>,
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

/// Creates a joiner that matches when `left(a) >= right(b)`.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::joiner::{Joiner, greater_than_or_equal};
///
/// let joiner = greater_than_or_equal(|x: &i32| *x, |y: &i32| *y);
///
/// assert!(joiner.matches(&10, &5));
/// assert!(joiner.matches(&5, &5));
/// assert!(!joiner.matches(&5, &10));
/// ```
pub fn greater_than_or_equal<A, B, T, Fa, Fb>(
    left: Fa,
    right: Fb,
) -> GreaterThanOrEqualJoiner<Fa, Fb, T>
where
    T: Ord,
    Fa: Fn(&A) -> T + Send + Sync,
    Fb: Fn(&B) -> T + Send + Sync,
{
    GreaterThanOrEqualJoiner {
        left,
        right,
        _phantom: PhantomData,
    }
}

/// A joiner that matches when `left(a) >= right(b)`.
pub struct GreaterThanOrEqualJoiner<Fa, Fb, T> {
    left: Fa,
    right: Fb,
    _phantom: PhantomData<fn() -> T>,
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
