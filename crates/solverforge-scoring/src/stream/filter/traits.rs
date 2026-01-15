//! Filter traits for different arities.

/// A filter over a single entity type with access to the solution.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::filter::UniFilter;
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
/// impl UniFilter<Solution, i32> for ThresholdFilter {
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
pub trait UniFilter<S, A>: Send + Sync {
    /// Returns true if the entity passes the filter given the solution context.
    fn test(&self, solution: &S, a: &A) -> bool;
}

/// A filter over pairs of entities with access to the solution.
pub trait BiFilter<S, A, B>: Send + Sync {
    /// Returns true if the pair passes the filter given the solution context.
    fn test(&self, solution: &S, a: &A, b: &B) -> bool;
}

/// A filter over triples of entities with access to the solution.
pub trait TriFilter<S, A, B, C>: Send + Sync {
    /// Returns true if the triple passes the filter given the solution context.
    fn test(&self, solution: &S, a: &A, b: &B, c: &C) -> bool;
}

/// A filter over quadruples of entities with access to the solution.
pub trait QuadFilter<S, A, B, C, D>: Send + Sync {
    /// Returns true if the quadruple passes the filter given the solution context.
    fn test(&self, solution: &S, a: &A, b: &B, c: &C, d: &D) -> bool;
}

/// A filter over quintuples of entities with access to the solution.
pub trait PentaFilter<S, A, B, C, D, E>: Send + Sync {
    /// Returns true if the quintuple passes the filter given the solution context.
    fn test(&self, solution: &S, a: &A, b: &B, c: &C, d: &D, e: &E) -> bool;
}
