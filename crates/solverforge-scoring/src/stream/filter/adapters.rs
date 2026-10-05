// Filter adapters for converting between filter types.

use std::marker::PhantomData;

use super::traits::{BiFilter, TriFilter, UniFilter};

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

// Lifts a bi-filter over (A, B) into the tri chain over (A, B, C),
// preserving the A and B source slice indexes and ignoring C.
#[doc(hidden)]
pub struct TriAsBiFilter<F, A, B> {
    filter: F,
    _phantom: PhantomData<(fn() -> A, fn() -> B)>,
}

impl<F, A, B> TriAsBiFilter<F, A, B> {
    // Creates a tri-filter from a bi-filter over the first two sources.
    #[inline]
    pub fn new(filter: F) -> Self {
        Self {
            filter,
            _phantom: PhantomData,
        }
    }
}

impl<S, A, B, C, F> TriFilter<S, A, B, C> for TriAsBiFilter<F, A, B>
where
    F: BiFilter<S, A, B>,
    C: Send + Sync,
{
    #[inline]
    fn test(
        &self,
        solution: &S,
        a: &A,
        b: &B,
        _c: &C,
        a_idx: usize,
        b_idx: usize,
        _c_idx: usize,
    ) -> bool {
        self.filter.test(solution, a, b, a_idx, b_idx)
    }
}

#[doc(hidden)]
pub struct PairFilter<L, R, P> {
    left_filter: L,
    right_filter: R,
    predicate: P,
}

impl<L, R, P> PairFilter<L, R, P> {
    #[inline]
    pub fn new(left_filter: L, right_filter: R, predicate: P) -> Self {
        Self {
            left_filter,
            right_filter,
            predicate,
        }
    }
}

// Applies the two stream membership filters to a joined pair (no predicate).
// Used when the join condition already carries the relationship predicate.
pub struct UniPairFilter<L, R> {
    left_filter: L,
    right_filter: R,
}

impl<L, R> UniPairFilter<L, R> {
    #[inline]
    pub fn new(left_filter: L, right_filter: R) -> Self {
        Self {
            left_filter,
            right_filter,
        }
    }
}

impl<S, A, B, L, R> BiFilter<S, A, B> for UniPairFilter<L, R>
where
    L: UniFilter<S, A>,
    R: UniFilter<S, B>,
{
    #[inline]
    fn test(&self, solution: &S, a: &A, b: &B, _a_idx: usize, _b_idx: usize) -> bool {
        self.left_filter.test(solution, a) && self.right_filter.test(solution, b)
    }
}

impl<S, A, B, L, R, P> BiFilter<S, A, B> for PairFilter<L, R, P>
where
    L: UniFilter<S, A>,
    R: UniFilter<S, B>,
    P: Fn(&A, &B) -> bool + Send + Sync,
{
    #[inline]
    fn test(&self, solution: &S, a: &A, b: &B, _a_idx: usize, _b_idx: usize) -> bool {
        self.left_filter.test(solution, a)
            && self.right_filter.test(solution, b)
            && (self.predicate)(a, b)
    }
}
