//! Filter adapters for converting between filter types.

use std::marker::PhantomData;

use super::traits::{BiFilter, UniFilter};

/// Applies a uni-filter to both elements of a pair (for self-joins).
pub struct UniBiFilter<F, A> {
    filter: F,
    _phantom: PhantomData<fn() -> A>,
}

impl<F, A> UniBiFilter<F, A> {
    /// Creates a bi-filter from a uni-filter.
    #[inline]
    pub fn new(filter: F) -> Self {
        Self {
            filter,
            _phantom: PhantomData,
        }
    }
}

impl<S, F, A> BiFilter<S, A, A> for UniBiFilter<F, A>
where
    F: UniFilter<S, A>,
    A: Send + Sync,
{
    #[inline]
    fn test(&self, solution: &S, a: &A, b: &A) -> bool {
        self.filter.test(solution, a) && self.filter.test(solution, b)
    }
}

/// Applies a uni-filter to the left element of a cross-entity pair.
pub struct UniLeftBiFilter<F, B> {
    filter: F,
    _phantom: PhantomData<fn() -> B>,
}

impl<F, B> UniLeftBiFilter<F, B> {
    /// Creates a bi-filter from a uni-filter applied to the left element.
    #[inline]
    pub fn new(filter: F) -> Self {
        Self {
            filter,
            _phantom: PhantomData,
        }
    }
}

impl<S, A, B, F> BiFilter<S, A, B> for UniLeftBiFilter<F, B>
where
    F: UniFilter<S, A>,
    B: Send + Sync,
{
    #[inline]
    fn test(&self, solution: &S, a: &A, _: &B) -> bool {
        self.filter.test(solution, a)
    }
}
