//! Filter wrappers for closures and constant filters.

use super::traits::{BiFilter, PentaFilter, QuadFilter, TriFilter, UniFilter};

/// A filter that always returns true.
#[derive(Debug, Clone, Copy, Default)]
pub struct TrueFilter;

impl<S, A> UniFilter<S, A> for TrueFilter {
    #[inline]
    fn test(&self, _: &S, _: &A) -> bool {
        true
    }
}

impl<S, A, B> BiFilter<S, A, B> for TrueFilter {
    #[inline]
    fn test(&self, _: &S, _: &A, _: &B) -> bool {
        true
    }
}

impl<S, A, B, C> TriFilter<S, A, B, C> for TrueFilter {
    #[inline]
    fn test(&self, _: &S, _: &A, _: &B, _: &C) -> bool {
        true
    }
}

impl<S, A, B, C, D> QuadFilter<S, A, B, C, D> for TrueFilter {
    #[inline]
    fn test(&self, _: &S, _: &A, _: &B, _: &C, _: &D) -> bool {
        true
    }
}

impl<S, A, B, C, D, E> PentaFilter<S, A, B, C, D, E> for TrueFilter {
    #[inline]
    fn test(&self, _: &S, _: &A, _: &B, _: &C, _: &D, _: &E) -> bool {
        true
    }
}

/// A uni-filter wrapping a closure.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::filter::{FnUniFilter, UniFilter};
///
/// struct Schedule {
///     max_hours: i32,
/// }
///
/// let filter = FnUniFilter::new(|schedule: &Schedule, hours: &i32| {
///     *hours <= schedule.max_hours
/// });
///
/// let schedule = Schedule { max_hours: 40 };
/// assert!(filter.test(&schedule, &35));
/// assert!(!filter.test(&schedule, &50));
/// ```
pub struct FnUniFilter<F> {
    f: F,
}

impl<F> FnUniFilter<F> {
    /// Creates a new filter from a closure.
    #[inline]
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<S, A, F> UniFilter<S, A> for FnUniFilter<F>
where
    F: Fn(&S, &A) -> bool + Send + Sync,
{
    #[inline]
    fn test(&self, solution: &S, a: &A) -> bool {
        (self.f)(solution, a)
    }
}

/// A bi-filter wrapping a closure.
pub struct FnBiFilter<F> {
    f: F,
}

impl<F> FnBiFilter<F> {
    /// Creates a new filter from a closure.
    #[inline]
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<S, A, B, F> BiFilter<S, A, B> for FnBiFilter<F>
where
    F: Fn(&S, &A, &B) -> bool + Send + Sync,
{
    #[inline]
    fn test(&self, solution: &S, a: &A, b: &B) -> bool {
        (self.f)(solution, a, b)
    }
}

/// A tri-filter wrapping a closure.
pub struct FnTriFilter<F> {
    f: F,
}

impl<F> FnTriFilter<F> {
    /// Creates a new filter from a closure.
    #[inline]
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<S, A, B, C, F> TriFilter<S, A, B, C> for FnTriFilter<F>
where
    F: Fn(&S, &A, &B, &C) -> bool + Send + Sync,
{
    #[inline]
    fn test(&self, solution: &S, a: &A, b: &B, c: &C) -> bool {
        (self.f)(solution, a, b, c)
    }
}

/// A quad-filter wrapping a closure.
pub struct FnQuadFilter<F> {
    f: F,
}

impl<F> FnQuadFilter<F> {
    /// Creates a new filter from a closure.
    #[inline]
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<S, A, B, C, D, F> QuadFilter<S, A, B, C, D> for FnQuadFilter<F>
where
    F: Fn(&S, &A, &B, &C, &D) -> bool + Send + Sync,
{
    #[inline]
    fn test(&self, solution: &S, a: &A, b: &B, c: &C, d: &D) -> bool {
        (self.f)(solution, a, b, c, d)
    }
}

/// A penta-filter wrapping a closure.
pub struct FnPentaFilter<F> {
    f: F,
}

impl<F> FnPentaFilter<F> {
    /// Creates a new filter from a closure.
    #[inline]
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<S, A, B, C, D, E, F> PentaFilter<S, A, B, C, D, E> for FnPentaFilter<F>
where
    F: Fn(&S, &A, &B, &C, &D, &E) -> bool + Send + Sync,
{
    #[inline]
    fn test(&self, solution: &S, a: &A, b: &B, c: &C, d: &D, e: &E) -> bool {
        (self.f)(solution, a, b, c, d, e)
    }
}
