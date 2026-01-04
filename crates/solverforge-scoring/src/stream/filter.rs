//! Zero-erasure filter composition for constraint streams.
//!
//! Filters are composed at compile time using nested generic types,
//! avoiding dynamic dispatch and Arc allocations.

use std::marker::PhantomData;

/// A filter over a single entity type.
pub trait UniFilter<A>: Send + Sync {
    /// Returns true if the entity passes the filter.
    fn test(&self, a: &A) -> bool;
}

/// A filter over pairs of entities.
pub trait BiFilter<A, B>: Send + Sync {
    /// Returns true if the pair passes the filter.
    fn test(&self, a: &A, b: &B) -> bool;
}

// ============================================================================
// TrueFilter - always passes
// ============================================================================

/// A filter that always returns true.
#[derive(Debug, Clone, Copy, Default)]
pub struct TrueFilter;

impl<A> UniFilter<A> for TrueFilter {
    #[inline]
    fn test(&self, _: &A) -> bool {
        true
    }
}

impl<A, B> BiFilter<A, B> for TrueFilter {
    #[inline]
    fn test(&self, _: &A, _: &B) -> bool {
        true
    }
}

// ============================================================================
// FnUniFilter - wraps a closure
// ============================================================================

/// A uni-filter wrapping a closure.
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

impl<A, F> UniFilter<A> for FnUniFilter<F>
where
    F: Fn(&A) -> bool + Send + Sync,
{
    #[inline]
    fn test(&self, a: &A) -> bool {
        (self.f)(a)
    }
}

// ============================================================================
// FnBiFilter - wraps a closure
// ============================================================================

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

impl<A, B, F> BiFilter<A, B> for FnBiFilter<F>
where
    F: Fn(&A, &B) -> bool + Send + Sync,
{
    #[inline]
    fn test(&self, a: &A, b: &B) -> bool {
        (self.f)(a, b)
    }
}

// ============================================================================
// AndUniFilter - combines two filters with AND semantics
// ============================================================================

/// Combines two uni-filters with AND semantics.
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

impl<A, F1, F2> UniFilter<A> for AndUniFilter<F1, F2>
where
    F1: UniFilter<A>,
    F2: UniFilter<A>,
{
    #[inline]
    fn test(&self, a: &A) -> bool {
        self.first.test(a) && self.second.test(a)
    }
}

// ============================================================================
// AndBiFilter - combines two filters with AND semantics
// ============================================================================

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

impl<A, B, F1, F2> BiFilter<A, B> for AndBiFilter<F1, F2>
where
    F1: BiFilter<A, B>,
    F2: BiFilter<A, B>,
{
    #[inline]
    fn test(&self, a: &A, b: &B) -> bool {
        self.first.test(a, b) && self.second.test(a, b)
    }
}

// ============================================================================
// UniBiFilter - applies a uni-filter to both elements of a pair
// ============================================================================

/// Applies a uni-filter to both elements of a pair (for self-joins).
pub struct UniBiFilter<F, A> {
    filter: F,
    _phantom: PhantomData<A>,
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

impl<F, A> BiFilter<A, A> for UniBiFilter<F, A>
where
    F: UniFilter<A>,
    A: Send + Sync,
{
    #[inline]
    fn test(&self, a: &A, b: &A) -> bool {
        self.filter.test(a) && self.filter.test(b)
    }
}

// ============================================================================
// UniLeftBiFilter - applies a uni-filter to the left element only
// ============================================================================

/// Applies a uni-filter to the left element of a cross-entity pair.
pub struct UniLeftBiFilter<F, B> {
    filter: F,
    _phantom: PhantomData<B>,
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

impl<A, B, F> BiFilter<A, B> for UniLeftBiFilter<F, B>
where
    F: UniFilter<A>,
    B: Send + Sync,
{
    #[inline]
    fn test(&self, a: &A, _: &B) -> bool {
        self.filter.test(a)
    }
}

// ============================================================================
// TriFilter - filter over triples
// ============================================================================

/// A filter over triples of entities.
pub trait TriFilter<A, B, C>: Send + Sync {
    /// Returns true if the triple passes the filter.
    fn test(&self, a: &A, b: &B, c: &C) -> bool;
}

impl<A, B, C> TriFilter<A, B, C> for TrueFilter {
    #[inline]
    fn test(&self, _: &A, _: &B, _: &C) -> bool {
        true
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

impl<A, B, C, F> TriFilter<A, B, C> for FnTriFilter<F>
where
    F: Fn(&A, &B, &C) -> bool + Send + Sync,
{
    #[inline]
    fn test(&self, a: &A, b: &B, c: &C) -> bool {
        (self.f)(a, b, c)
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

impl<A, B, C, F1, F2> TriFilter<A, B, C> for AndTriFilter<F1, F2>
where
    F1: TriFilter<A, B, C>,
    F2: TriFilter<A, B, C>,
{
    #[inline]
    fn test(&self, a: &A, b: &B, c: &C) -> bool {
        self.first.test(a, b, c) && self.second.test(a, b, c)
    }
}

// ============================================================================
// QuadFilter - filter over quadruples
// ============================================================================

/// A filter over quadruples of entities.
pub trait QuadFilter<A, B, C, D>: Send + Sync {
    /// Returns true if the quadruple passes the filter.
    fn test(&self, a: &A, b: &B, c: &C, d: &D) -> bool;
}

impl<A, B, C, D> QuadFilter<A, B, C, D> for TrueFilter {
    #[inline]
    fn test(&self, _: &A, _: &B, _: &C, _: &D) -> bool {
        true
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

impl<A, B, C, D, F> QuadFilter<A, B, C, D> for FnQuadFilter<F>
where
    F: Fn(&A, &B, &C, &D) -> bool + Send + Sync,
{
    #[inline]
    fn test(&self, a: &A, b: &B, c: &C, d: &D) -> bool {
        (self.f)(a, b, c, d)
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

impl<A, B, C, D, F1, F2> QuadFilter<A, B, C, D> for AndQuadFilter<F1, F2>
where
    F1: QuadFilter<A, B, C, D>,
    F2: QuadFilter<A, B, C, D>,
{
    #[inline]
    fn test(&self, a: &A, b: &B, c: &C, d: &D) -> bool {
        self.first.test(a, b, c, d) && self.second.test(a, b, c, d)
    }
}

// ============================================================================
// PentaFilter - filter over quintuples
// ============================================================================

/// A filter over quintuples of entities.
pub trait PentaFilter<A, B, C, D, E>: Send + Sync {
    /// Returns true if the quintuple passes the filter.
    fn test(&self, a: &A, b: &B, c: &C, d: &D, e: &E) -> bool;
}

impl<A, B, C, D, E> PentaFilter<A, B, C, D, E> for TrueFilter {
    #[inline]
    fn test(&self, _: &A, _: &B, _: &C, _: &D, _: &E) -> bool {
        true
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

impl<A, B, C, D, E, F> PentaFilter<A, B, C, D, E> for FnPentaFilter<F>
where
    F: Fn(&A, &B, &C, &D, &E) -> bool + Send + Sync,
{
    #[inline]
    fn test(&self, a: &A, b: &B, c: &C, d: &D, e: &E) -> bool {
        (self.f)(a, b, c, d, e)
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

impl<A, B, C, D, E, F1, F2> PentaFilter<A, B, C, D, E> for AndPentaFilter<F1, F2>
where
    F1: PentaFilter<A, B, C, D, E>,
    F2: PentaFilter<A, B, C, D, E>,
{
    #[inline]
    fn test(&self, a: &A, b: &B, c: &C, d: &D, e: &E) -> bool {
        self.first.test(a, b, c, d, e) && self.second.test(a, b, c, d, e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_true_filter() {
        let f = TrueFilter;
        assert!(UniFilter::<i32>::test(&f, &42));
        assert!(BiFilter::<i32, i32>::test(&f, &1, &2));
    }

    #[test]
    fn test_fn_uni_filter() {
        let f = FnUniFilter::new(|x: &i32| *x > 10);
        assert!(f.test(&15));
        assert!(!f.test(&5));
    }

    #[test]
    fn test_fn_bi_filter() {
        let f = FnBiFilter::new(|a: &i32, b: &i32| a + b > 10);
        assert!(f.test(&7, &8));
        assert!(!f.test(&3, &4));
    }

    #[test]
    fn test_and_uni_filter() {
        let f1 = FnUniFilter::new(|x: &i32| *x > 5);
        let f2 = FnUniFilter::new(|x: &i32| *x < 15);
        let combined = AndUniFilter::new(f1, f2);
        assert!(combined.test(&10));
        assert!(!combined.test(&3));
        assert!(!combined.test(&20));
    }

    #[test]
    fn test_and_bi_filter() {
        let f1 = FnBiFilter::new(|a: &i32, _b: &i32| *a > 0);
        let f2 = FnBiFilter::new(|_a: &i32, b: &i32| *b > 0);
        let combined = AndBiFilter::new(f1, f2);
        assert!(combined.test(&1, &2));
        assert!(!combined.test(&-1, &2));
        assert!(!combined.test(&1, &-2));
    }
}
