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

// Lifts a tri-filter over (A, B, C) into the quad chain over (A, B, C, D),
// preserving the A, B and C source slice indexes and ignoring D.
#[doc(hidden)]
pub struct TriAsQuadFilter<F, A, B, C> {
    filter: F,
    _phantom: PhantomData<(fn() -> A, fn() -> B, fn() -> C)>,
}

impl<F, A, B, C> TriAsQuadFilter<F, A, B, C> {
    // Creates a quad-filter from a tri-filter over the first three sources.
    #[inline]
    pub fn new(filter: F) -> Self {
        Self {
            filter,
            _phantom: PhantomData,
        }
    }
}

impl<S, A, B, C, D, F> super::traits::QuadFilter<S, A, B, C, D> for TriAsQuadFilter<F, A, B, C>
where
    F: TriFilter<S, A, B, C>,
    D: Send + Sync,
{
    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn test(
        &self,
        solution: &S,
        a: &A,
        b: &B,
        c: &C,
        _d: &D,
        a_idx: usize,
        b_idx: usize,
        c_idx: usize,
        _d_idx: usize,
    ) -> bool {
        self.filter.test(solution, a, b, c, a_idx, b_idx, c_idx)
    }
}

// Lifts a quad-filter over (A, B, C, D) into the penta chain over (A, B, C, D, E),
// preserving source slice indexes and ignoring E.
#[doc(hidden)]
pub struct QuadAsPentaFilter<F, A, B, C, D> {
    filter: F,
    _phantom: PhantomData<(fn() -> A, fn() -> B, fn() -> C, fn() -> D)>,
}

impl<F, A, B, C, D> QuadAsPentaFilter<F, A, B, C, D> {
    #[inline]
    pub fn new(filter: F) -> Self {
        Self {
            filter,
            _phantom: PhantomData,
        }
    }
}

impl<S, A, B, C, D, E, F> super::traits::PentaFilter<S, A, B, C, D, E>
    for QuadAsPentaFilter<F, A, B, C, D>
where
    F: super::traits::QuadFilter<S, A, B, C, D>,
    E: Send + Sync,
{
    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn test(
        &self,
        solution: &S,
        a: &A,
        b: &B,
        c: &C,
        d: &D,
        _e: &E,
        a_idx: usize,
        b_idx: usize,
        c_idx: usize,
        d_idx: usize,
        _e_idx: usize,
    ) -> bool {
        self.filter
            .test(solution, a, b, c, d, a_idx, b_idx, c_idx, d_idx)
    }
}
