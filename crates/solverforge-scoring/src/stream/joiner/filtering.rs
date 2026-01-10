//! Filtering joiner for custom predicate matching.

use super::Joiner;

/// Creates a joiner that matches based on a custom predicate.
///
/// # Example
///
/// ```
/// use solverforge_scoring::stream::joiner::{Joiner, filtering};
///
/// #[derive(Clone)]
/// struct Task { priority: i32, id: i32 }
///
/// // Match tasks where a has higher priority than b
/// let higher_priority = filtering(|a: &Task, b: &Task| a.priority > b.priority);
///
/// assert!(higher_priority.matches(
///     &Task { priority: 10, id: 1 },
///     &Task { priority: 5, id: 2 }
/// ));
///
/// assert!(!higher_priority.matches(
///     &Task { priority: 5, id: 1 },
///     &Task { priority: 10, id: 2 }
/// ));
/// ```
pub fn filtering<A, B, F>(predicate: F) -> FilteringJoiner<F>
where
    F: Fn(&A, &B) -> bool + Send + Sync,
{
    FilteringJoiner { predicate }
}

/// A joiner that matches based on a custom predicate.
pub struct FilteringJoiner<F> {
    predicate: F,
}

impl<A, B, F> Joiner<A, B> for FilteringJoiner<F>
where
    F: Fn(&A, &B) -> bool + Send + Sync,
{
    #[inline]
    fn matches(&self, a: &A, b: &B) -> bool {
        (self.predicate)(a, b)
    }
}
