//! Filter composition types for combining filters with AND semantics.

use super::traits::{BiFilter, PentaFilter, QuadFilter, TriFilter, UniFilter};

/// Combines two uni-filters with AND semantics.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::filter::{FnUniFilter, AndUniFilter, UniFilter};
///
/// let f1 = FnUniFilter::new(|_: &(), x: &i32| *x > 5);
/// let f2 = FnUniFilter::new(|_: &(), x: &i32| *x < 15);
/// let combined = AndUniFilter::new(f1, f2);
///
/// assert!(combined.test(&(), &10));
/// assert!(!combined.test(&(), &3));
/// assert!(!combined.test(&(), &20));
/// ```
pub struct AndUniFilter<F1, F2> {
    first: F1,
    second: F2,
}

impl<F1, F2> AndUniFilter<F1, F2> {
    /// Creates a combined filter.
    #[inline]
    pub fn new(first: F1, second: F2) -> Self {
        Self { first, second }
    }
}

impl<S, A, F1, F2> UniFilter<S, A> for AndUniFilter<F1, F2>
where
    F1: UniFilter<S, A>,
    F2: UniFilter<S, A>,
{
    #[inline]
    fn test(&self, solution: &S, a: &A) -> bool {
        self.first.test(solution, a) && self.second.test(solution, a)
    }
}

/// Combines two bi-filters with AND semantics.
pub struct AndBiFilter<F1, F2> {
    first: F1,
    second: F2,
}

impl<F1, F2> AndBiFilter<F1, F2> {
    /// Creates a combined filter.
    #[inline]
    pub fn new(first: F1, second: F2) -> Self {
        Self { first, second }
    }
}

impl<S, A, B, F1, F2> BiFilter<S, A, B> for AndBiFilter<F1, F2>
where
    F1: BiFilter<S, A, B>,
    F2: BiFilter<S, A, B>,
{
    #[inline]
    fn test(&self, solution: &S, a: &A, b: &B) -> bool {
        self.first.test(solution, a, b) && self.second.test(solution, a, b)
    }
}

/// Combines two tri-filters with AND semantics.
pub struct AndTriFilter<F1, F2> {
    first: F1,
    second: F2,
}

impl<F1, F2> AndTriFilter<F1, F2> {
    /// Creates a combined filter.
    #[inline]
    pub fn new(first: F1, second: F2) -> Self {
        Self { first, second }
    }
}

impl<S, A, B, C, F1, F2> TriFilter<S, A, B, C> for AndTriFilter<F1, F2>
where
    F1: TriFilter<S, A, B, C>,
    F2: TriFilter<S, A, B, C>,
{
    #[inline]
    fn test(&self, solution: &S, a: &A, b: &B, c: &C) -> bool {
        self.first.test(solution, a, b, c) && self.second.test(solution, a, b, c)
    }
}

/// Combines two quad-filters with AND semantics.
pub struct AndQuadFilter<F1, F2> {
    first: F1,
    second: F2,
}

impl<F1, F2> AndQuadFilter<F1, F2> {
    /// Creates a combined filter.
    #[inline]
    pub fn new(first: F1, second: F2) -> Self {
        Self { first, second }
    }
}

impl<S, A, B, C, D, F1, F2> QuadFilter<S, A, B, C, D> for AndQuadFilter<F1, F2>
where
    F1: QuadFilter<S, A, B, C, D>,
    F2: QuadFilter<S, A, B, C, D>,
{
    #[inline]
    fn test(&self, solution: &S, a: &A, b: &B, c: &C, d: &D) -> bool {
        self.first.test(solution, a, b, c, d) && self.second.test(solution, a, b, c, d)
    }
}

/// Combines two penta-filters with AND semantics.
pub struct AndPentaFilter<F1, F2> {
    first: F1,
    second: F2,
}

impl<F1, F2> AndPentaFilter<F1, F2> {
    /// Creates a combined filter.
    #[inline]
    pub fn new(first: F1, second: F2) -> Self {
        Self { first, second }
    }
}

impl<S, A, B, C, D, E, F1, F2> PentaFilter<S, A, B, C, D, E> for AndPentaFilter<F1, F2>
where
    F1: PentaFilter<S, A, B, C, D, E>,
    F2: PentaFilter<S, A, B, C, D, E>,
{
    #[inline]
    fn test(&self, solution: &S, a: &A, b: &B, c: &C, d: &D, e: &E) -> bool {
        self.first.test(solution, a, b, c, d, e) && self.second.test(solution, a, b, c, d, e)
    }
}
