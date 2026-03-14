// Filter adapters for converting between filter types.

use std::marker::PhantomData;

use super::traits::{BiFilter, UniFilter};

// Applies a uni-filter to both elements of a pair (for self-joins).
pub struct UniBiFilter<F, A> {
    filter: F,
    _phantom: PhantomData<fn() -> A>,
}

impl<F, A> UniBiFilter<F, A> {
    // Creates a bi-filter from a uni-filter.
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
    fn test(&self, solution: &S, a: &A, b: &A, _a_idx: usize, _b_idx: usize) -> bool {
        self.filter.test(solution, a) && self.filter.test(solution, b)
    }
}

// Applies a uni-filter to the left element of a cross-entity pair.
pub struct UniLeftBiFilter<F, B> {
    filter: F,
    _phantom: PhantomData<fn() -> B>,
}

impl<F, B> UniLeftBiFilter<F, B> {
    // Creates a bi-filter from a uni-filter applied to the left element.
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
    fn test(&self, solution: &S, a: &A, _: &B, _a_idx: usize, _b_idx: usize) -> bool {
        self.filter.test(solution, a)
    }
}

/* Combines a left uni-filter with a cross-entity predicate `Fn(&A, &B) -> bool`.

Used by the predicate cross-join in `JoinTarget` to produce a named concrete type
that avoids `impl Trait` in associated type position.
*/
pub struct UniLeftPredBiFilter<F, P, A> {
    left_filter: F,
    predicate: P,
    _phantom: PhantomData<fn() -> A>,
}

impl<F, P, A> UniLeftPredBiFilter<F, P, A> {
    // Creates a combined filter from a left uni-filter and a cross-entity predicate.
    #[inline]
    pub fn new(left_filter: F, predicate: P) -> Self {
        Self {
            left_filter,
            predicate,
            _phantom: PhantomData,
        }
    }
}

impl<S, A, B, F, P> BiFilter<S, A, B> for UniLeftPredBiFilter<F, P, A>
where
    F: UniFilter<S, A>,
    P: Fn(&A, &B) -> bool + Send + Sync,
    B: Send + Sync,
{
    #[inline]
    fn test(&self, solution: &S, a: &A, b: &B, _a_idx: usize, _b_idx: usize) -> bool {
        self.left_filter.test(solution, a) && (self.predicate)(a, b)
    }
}
