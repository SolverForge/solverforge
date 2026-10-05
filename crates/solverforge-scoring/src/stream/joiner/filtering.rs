// Filtering joiner for custom predicate matching.

use super::Joiner;

/* Creates a joiner that matches based on a custom predicate.

# Example

```
use solverforge_scoring::stream::joiner::{Joiner, filtering};

#[derive(Clone)]
struct Task { priority: i32, id: i32 }

// Match tasks where a has higher priority than b
let higher_priority = filtering(|a: &Task, b: &Task| a.priority > b.priority);

assert!(higher_priority.matches(
&Task { priority: 10, id: 1 },
&Task { priority: 5, id: 2 }
));

assert!(!higher_priority.matches(
&Task { priority: 5, id: 1 },
&Task { priority: 10, id: 2 }
));
```
*/
pub fn filtering<F>(predicate: F) -> FilteringJoiner<F>
where
    F: Send + Sync,
{
    FilteringJoiner { predicate }
}

/* Creates a joiner that matches a whole left row against the new right entity.

The predicate receives the borrowed left row plus the right entity, so a
later relationship can inspect any earlier binding or combine several.
Non-indexable by construction: plans compile this to an explicit scan
over retained opposite-input rows, never a fabricated equality key.
*/
pub fn filtering_on<F>(predicate: F) -> FilteringJoiner<F>
where
    F: Send + Sync,
{
    FilteringJoiner { predicate }
}

impl<F> FilteringJoiner<F> {
    /// Compose without prematurely binding a borrowed row lifetime.
    pub fn and<J>(self, other: J) -> super::AndJoiner<Self, J> {
        super::AndJoiner {
            first: self,
            second: other,
        }
    }
}

// A joiner that matches based on a custom predicate.
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
