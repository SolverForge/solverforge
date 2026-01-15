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

// ============================================================================
// SOLUTION-AWARE FILTERS
// ============================================================================
//
// These filters receive the full solution alongside entities, enabling
// access to shadow variables and computed solution state during filtering.

// ============================================================================
// SolutionUniFilter - solution-aware filter over single entities
// ============================================================================

/// A solution-aware filter over a single entity type.
///
/// Unlike [`UniFilter`], this trait receives the solution reference,
/// enabling access to shadow variables and computed state.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::filter::SolutionUniFilter;
///
/// struct Solution {
///     entities: Vec<i32>,
///     total: i32, // shadow variable
/// }
///
/// struct ThresholdFilter {
///     threshold: i32,
/// }
///
/// impl SolutionUniFilter<Solution, i32> for ThresholdFilter {
///     fn test(&self, solution: &Solution, entity: &i32) -> bool {
///         // Access shadow variable from solution
///         solution.total + entity > self.threshold
///     }
/// }
///
/// let solution = Solution { entities: vec![1, 2, 3], total: 10 };
/// let filter = ThresholdFilter { threshold: 12 };
/// assert!(filter.test(&solution, &5));
/// assert!(!filter.test(&solution, &1));
/// ```
pub trait SolutionUniFilter<S, A>: Send + Sync {
    /// Returns true if the entity passes the filter given the solution context.
    fn test(&self, solution: &S, a: &A) -> bool;
}

// ============================================================================
// SolutionBiFilter - solution-aware filter over pairs
// ============================================================================

/// A solution-aware filter over pairs of entities.
pub trait SolutionBiFilter<S, A, B>: Send + Sync {
    /// Returns true if the pair passes the filter given the solution context.
    fn test(&self, solution: &S, a: &A, b: &B) -> bool;
}

// ============================================================================
// SolutionTriFilter - solution-aware filter over triples
// ============================================================================

/// A solution-aware filter over triples of entities.
pub trait SolutionTriFilter<S, A, B, C>: Send + Sync {
    /// Returns true if the triple passes the filter given the solution context.
    fn test(&self, solution: &S, a: &A, b: &B, c: &C) -> bool;
}

// ============================================================================
// SolutionQuadFilter - solution-aware filter over quadruples
// ============================================================================

/// A solution-aware filter over quadruples of entities.
pub trait SolutionQuadFilter<S, A, B, C, D>: Send + Sync {
    /// Returns true if the quadruple passes the filter given the solution context.
    fn test(&self, solution: &S, a: &A, b: &B, c: &C, d: &D) -> bool;
}

// ============================================================================
// SolutionPentaFilter - solution-aware filter over quintuples
// ============================================================================

/// A solution-aware filter over quintuples of entities.
pub trait SolutionPentaFilter<S, A, B, C, D, E>: Send + Sync {
    /// Returns true if the quintuple passes the filter given the solution context.
    fn test(&self, solution: &S, a: &A, b: &B, c: &C, d: &D, e: &E) -> bool;
}

// ============================================================================
// TrueFilter implementations for solution-aware traits
// ============================================================================

impl<S, A> SolutionUniFilter<S, A> for TrueFilter {
    #[inline]
    fn test(&self, _: &S, _: &A) -> bool {
        true
    }
}

impl<S, A, B> SolutionBiFilter<S, A, B> for TrueFilter {
    #[inline]
    fn test(&self, _: &S, _: &A, _: &B) -> bool {
        true
    }
}

impl<S, A, B, C> SolutionTriFilter<S, A, B, C> for TrueFilter {
    #[inline]
    fn test(&self, _: &S, _: &A, _: &B, _: &C) -> bool {
        true
    }
}

impl<S, A, B, C, D> SolutionQuadFilter<S, A, B, C, D> for TrueFilter {
    #[inline]
    fn test(&self, _: &S, _: &A, _: &B, _: &C, _: &D) -> bool {
        true
    }
}

impl<S, A, B, C, D, E> SolutionPentaFilter<S, A, B, C, D, E> for TrueFilter {
    #[inline]
    fn test(&self, _: &S, _: &A, _: &B, _: &C, _: &D, _: &E) -> bool {
        true
    }
}

// ============================================================================
// EntityOnlyUniFilter - adapts entity-only filter to solution-aware
// ============================================================================

/// Adapts an entity-only [`UniFilter`] to [`SolutionUniFilter`].
///
/// This adapter ignores the solution parameter, allowing existing
/// entity-only filters to be used where solution-aware filters are expected.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::filter::{
///     FnUniFilter, EntityOnlyUniFilter, SolutionUniFilter
/// };
///
/// let entity_filter = FnUniFilter::new(|x: &i32| *x > 10);
/// let solution_filter = EntityOnlyUniFilter(entity_filter);
///
/// // Now works with solution-aware API
/// let solution = ();
/// assert!(solution_filter.test(&solution, &15));
/// assert!(!solution_filter.test(&solution, &5));
/// ```
pub struct EntityOnlyUniFilter<F>(pub F);

impl<S, A, F> SolutionUniFilter<S, A> for EntityOnlyUniFilter<F>
where
    F: UniFilter<A>,
{
    #[inline]
    fn test(&self, _solution: &S, a: &A) -> bool {
        self.0.test(a)
    }
}

// ============================================================================
// EntityOnlyBiFilter - adapts entity-only filter to solution-aware
// ============================================================================

/// Adapts an entity-only [`BiFilter`] to [`SolutionBiFilter`].
pub struct EntityOnlyBiFilter<F>(pub F);

impl<S, A, B, F> SolutionBiFilter<S, A, B> for EntityOnlyBiFilter<F>
where
    F: BiFilter<A, B>,
{
    #[inline]
    fn test(&self, _solution: &S, a: &A, b: &B) -> bool {
        self.0.test(a, b)
    }
}

// ============================================================================
// EntityOnlyTriFilter - adapts entity-only filter to solution-aware
// ============================================================================

/// Adapts an entity-only [`TriFilter`] to [`SolutionTriFilter`].
pub struct EntityOnlyTriFilter<F>(pub F);

impl<S, A, B, C, F> SolutionTriFilter<S, A, B, C> for EntityOnlyTriFilter<F>
where
    F: TriFilter<A, B, C>,
{
    #[inline]
    fn test(&self, _solution: &S, a: &A, b: &B, c: &C) -> bool {
        self.0.test(a, b, c)
    }
}

// ============================================================================
// EntityOnlyQuadFilter - adapts entity-only filter to solution-aware
// ============================================================================

/// Adapts an entity-only [`QuadFilter`] to [`SolutionQuadFilter`].
pub struct EntityOnlyQuadFilter<F>(pub F);

impl<S, A, B, C, D, F> SolutionQuadFilter<S, A, B, C, D> for EntityOnlyQuadFilter<F>
where
    F: QuadFilter<A, B, C, D>,
{
    #[inline]
    fn test(&self, _solution: &S, a: &A, b: &B, c: &C, d: &D) -> bool {
        self.0.test(a, b, c, d)
    }
}

// ============================================================================
// EntityOnlyPentaFilter - adapts entity-only filter to solution-aware
// ============================================================================

/// Adapts an entity-only [`PentaFilter`] to [`SolutionPentaFilter`].
pub struct EntityOnlyPentaFilter<F>(pub F);

impl<S, A, B, C, D, E, F> SolutionPentaFilter<S, A, B, C, D, E> for EntityOnlyPentaFilter<F>
where
    F: PentaFilter<A, B, C, D, E>,
{
    #[inline]
    fn test(&self, _solution: &S, a: &A, b: &B, c: &C, d: &D, e: &E) -> bool {
        self.0.test(a, b, c, d, e)
    }
}

// ============================================================================
// FnSolutionUniFilter - wraps a solution-aware closure
// ============================================================================

/// A solution-aware uni-filter wrapping a closure.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::filter::{FnSolutionUniFilter, SolutionUniFilter};
///
/// struct Schedule {
///     max_hours: i32,
/// }
///
/// let filter = FnSolutionUniFilter::new(|schedule: &Schedule, hours: &i32| {
///     *hours <= schedule.max_hours
/// });
///
/// let schedule = Schedule { max_hours: 40 };
/// assert!(filter.test(&schedule, &35));
/// assert!(!filter.test(&schedule, &50));
/// ```
pub struct FnSolutionUniFilter<F> {
    f: F,
}

impl<F> FnSolutionUniFilter<F> {
    /// Creates a new solution-aware filter from a closure.
    #[inline]
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<S, A, F> SolutionUniFilter<S, A> for FnSolutionUniFilter<F>
where
    F: Fn(&S, &A) -> bool + Send + Sync,
{
    #[inline]
    fn test(&self, solution: &S, a: &A) -> bool {
        (self.f)(solution, a)
    }
}

// ============================================================================
// FnSolutionBiFilter - wraps a solution-aware closure
// ============================================================================

/// A solution-aware bi-filter wrapping a closure.
pub struct FnSolutionBiFilter<F> {
    f: F,
}

impl<F> FnSolutionBiFilter<F> {
    /// Creates a new solution-aware filter from a closure.
    #[inline]
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<S, A, B, F> SolutionBiFilter<S, A, B> for FnSolutionBiFilter<F>
where
    F: Fn(&S, &A, &B) -> bool + Send + Sync,
{
    #[inline]
    fn test(&self, solution: &S, a: &A, b: &B) -> bool {
        (self.f)(solution, a, b)
    }
}

// ============================================================================
// FnSolutionTriFilter - wraps a solution-aware closure
// ============================================================================

/// A solution-aware tri-filter wrapping a closure.
pub struct FnSolutionTriFilter<F> {
    f: F,
}

impl<F> FnSolutionTriFilter<F> {
    /// Creates a new solution-aware filter from a closure.
    #[inline]
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<S, A, B, C, F> SolutionTriFilter<S, A, B, C> for FnSolutionTriFilter<F>
where
    F: Fn(&S, &A, &B, &C) -> bool + Send + Sync,
{
    #[inline]
    fn test(&self, solution: &S, a: &A, b: &B, c: &C) -> bool {
        (self.f)(solution, a, b, c)
    }
}

// ============================================================================
// FnSolutionQuadFilter - wraps a solution-aware closure
// ============================================================================

/// A solution-aware quad-filter wrapping a closure.
pub struct FnSolutionQuadFilter<F> {
    f: F,
}

impl<F> FnSolutionQuadFilter<F> {
    /// Creates a new solution-aware filter from a closure.
    #[inline]
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<S, A, B, C, D, F> SolutionQuadFilter<S, A, B, C, D> for FnSolutionQuadFilter<F>
where
    F: Fn(&S, &A, &B, &C, &D) -> bool + Send + Sync,
{
    #[inline]
    fn test(&self, solution: &S, a: &A, b: &B, c: &C, d: &D) -> bool {
        (self.f)(solution, a, b, c, d)
    }
}

// ============================================================================
// FnSolutionPentaFilter - wraps a solution-aware closure
// ============================================================================

/// A solution-aware penta-filter wrapping a closure.
pub struct FnSolutionPentaFilter<F> {
    f: F,
}

impl<F> FnSolutionPentaFilter<F> {
    /// Creates a new solution-aware filter from a closure.
    #[inline]
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<S, A, B, C, D, E, F> SolutionPentaFilter<S, A, B, C, D, E> for FnSolutionPentaFilter<F>
where
    F: Fn(&S, &A, &B, &C, &D, &E) -> bool + Send + Sync,
{
    #[inline]
    fn test(&self, solution: &S, a: &A, b: &B, c: &C, d: &D, e: &E) -> bool {
        (self.f)(solution, a, b, c, d, e)
    }
}

// ============================================================================
// AndSolutionUniFilter - combines two solution-aware filters with AND
// ============================================================================

/// Combines two solution-aware uni-filters with AND semantics.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::filter::{
///     FnSolutionUniFilter, AndSolutionUniFilter, SolutionUniFilter
/// };
///
/// let f1 = FnSolutionUniFilter::new(|_: &(), x: &i32| *x > 5);
/// let f2 = FnSolutionUniFilter::new(|_: &(), x: &i32| *x < 15);
/// let combined = AndSolutionUniFilter::new(f1, f2);
///
/// assert!(combined.test(&(), &10));
/// assert!(!combined.test(&(), &3));
/// assert!(!combined.test(&(), &20));
/// ```
pub struct AndSolutionUniFilter<F1, F2> {
    first: F1,
    second: F2,
}

impl<F1, F2> AndSolutionUniFilter<F1, F2> {
    /// Creates a combined solution-aware filter.
    #[inline]
    pub fn new(first: F1, second: F2) -> Self {
        Self { first, second }
    }
}

impl<S, A, F1, F2> SolutionUniFilter<S, A> for AndSolutionUniFilter<F1, F2>
where
    F1: SolutionUniFilter<S, A>,
    F2: SolutionUniFilter<S, A>,
{
    #[inline]
    fn test(&self, solution: &S, a: &A) -> bool {
        self.first.test(solution, a) && self.second.test(solution, a)
    }
}

// ============================================================================
// AndSolutionBiFilter - combines two solution-aware filters with AND
// ============================================================================

/// Combines two solution-aware bi-filters with AND semantics.
pub struct AndSolutionBiFilter<F1, F2> {
    first: F1,
    second: F2,
}

impl<F1, F2> AndSolutionBiFilter<F1, F2> {
    /// Creates a combined solution-aware filter.
    #[inline]
    pub fn new(first: F1, second: F2) -> Self {
        Self { first, second }
    }
}

impl<S, A, B, F1, F2> SolutionBiFilter<S, A, B> for AndSolutionBiFilter<F1, F2>
where
    F1: SolutionBiFilter<S, A, B>,
    F2: SolutionBiFilter<S, A, B>,
{
    #[inline]
    fn test(&self, solution: &S, a: &A, b: &B) -> bool {
        self.first.test(solution, a, b) && self.second.test(solution, a, b)
    }
}

// ============================================================================
// AndSolutionTriFilter - combines two solution-aware filters with AND
// ============================================================================

/// Combines two solution-aware tri-filters with AND semantics.
pub struct AndSolutionTriFilter<F1, F2> {
    first: F1,
    second: F2,
}

impl<F1, F2> AndSolutionTriFilter<F1, F2> {
    /// Creates a combined solution-aware filter.
    #[inline]
    pub fn new(first: F1, second: F2) -> Self {
        Self { first, second }
    }
}

impl<S, A, B, C, F1, F2> SolutionTriFilter<S, A, B, C> for AndSolutionTriFilter<F1, F2>
where
    F1: SolutionTriFilter<S, A, B, C>,
    F2: SolutionTriFilter<S, A, B, C>,
{
    #[inline]
    fn test(&self, solution: &S, a: &A, b: &B, c: &C) -> bool {
        self.first.test(solution, a, b, c) && self.second.test(solution, a, b, c)
    }
}

// ============================================================================
// AndSolutionQuadFilter - combines two solution-aware filters with AND
// ============================================================================

/// Combines two solution-aware quad-filters with AND semantics.
pub struct AndSolutionQuadFilter<F1, F2> {
    first: F1,
    second: F2,
}

impl<F1, F2> AndSolutionQuadFilter<F1, F2> {
    /// Creates a combined solution-aware filter.
    #[inline]
    pub fn new(first: F1, second: F2) -> Self {
        Self { first, second }
    }
}

impl<S, A, B, C, D, F1, F2> SolutionQuadFilter<S, A, B, C, D> for AndSolutionQuadFilter<F1, F2>
where
    F1: SolutionQuadFilter<S, A, B, C, D>,
    F2: SolutionQuadFilter<S, A, B, C, D>,
{
    #[inline]
    fn test(&self, solution: &S, a: &A, b: &B, c: &C, d: &D) -> bool {
        self.first.test(solution, a, b, c, d) && self.second.test(solution, a, b, c, d)
    }
}

// ============================================================================
// AndSolutionPentaFilter - combines two solution-aware filters with AND
// ============================================================================

/// Combines two solution-aware penta-filters with AND semantics.
pub struct AndSolutionPentaFilter<F1, F2> {
    first: F1,
    second: F2,
}

impl<F1, F2> AndSolutionPentaFilter<F1, F2> {
    /// Creates a combined solution-aware filter.
    #[inline]
    pub fn new(first: F1, second: F2) -> Self {
        Self { first, second }
    }
}

impl<S, A, B, C, D, E, F1, F2> SolutionPentaFilter<S, A, B, C, D, E>
    for AndSolutionPentaFilter<F1, F2>
where
    F1: SolutionPentaFilter<S, A, B, C, D, E>,
    F2: SolutionPentaFilter<S, A, B, C, D, E>,
{
    #[inline]
    fn test(&self, solution: &S, a: &A, b: &B, c: &C, d: &D, e: &E) -> bool {
        self.first.test(solution, a, b, c, d, e) && self.second.test(solution, a, b, c, d, e)
    }
}

// ============================================================================
// SolutionUniBiFilter - applies solution-aware uni-filter to both elements
// ============================================================================

/// Applies a solution-aware uni-filter to both elements of a pair (for self-joins).
pub struct SolutionUniBiFilter<F, A> {
    filter: F,
    _phantom: PhantomData<fn() -> A>,
}

impl<F, A> SolutionUniBiFilter<F, A> {
    /// Creates a bi-filter from a uni-filter.
    #[inline]
    pub fn new(filter: F) -> Self {
        Self {
            filter,
            _phantom: PhantomData,
        }
    }
}

impl<S, F, A> SolutionBiFilter<S, A, A> for SolutionUniBiFilter<F, A>
where
    F: SolutionUniFilter<S, A>,
    A: Send + Sync,
{
    #[inline]
    fn test(&self, solution: &S, a: &A, b: &A) -> bool {
        self.filter.test(solution, a) && self.filter.test(solution, b)
    }
}

// ============================================================================
// SolutionUniLeftBiFilter - applies solution-aware uni-filter to left element
// ============================================================================

/// Applies a solution-aware uni-filter to the left element of a cross-entity pair.
pub struct SolutionUniLeftBiFilter<F, B> {
    filter: F,
    _phantom: PhantomData<fn() -> B>,
}

impl<F, B> SolutionUniLeftBiFilter<F, B> {
    /// Creates a bi-filter from a uni-filter applied to the left element.
    #[inline]
    pub fn new(filter: F) -> Self {
        Self {
            filter,
            _phantom: PhantomData,
        }
    }
}

impl<S, A, B, F> SolutionBiFilter<S, A, B> for SolutionUniLeftBiFilter<F, B>
where
    F: SolutionUniFilter<S, A>,
    B: Send + Sync,
{
    #[inline]
    fn test(&self, solution: &S, a: &A, _: &B) -> bool {
        self.filter.test(solution, a)
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
