/* Filter traits for different arities.

These traits are used internally for filter composition. The solution
parameter enables constraint evaluation but user-facing filter predicates
receive only the entity.
*/

/* A filter over a single entity type.

# Example

```
use solverforge_scoring::stream::filter::UniFilter;

struct ThresholdFilter {
threshold: i32,
}

impl UniFilter<(), i32> for ThresholdFilter {
fn test(&self, _solution: &(), entity: &i32) -> bool {
*entity > self.threshold
}
}

let filter = ThresholdFilter { threshold: 10 };
assert!(filter.test(&(), &15));
assert!(!filter.test(&(), &5));
```
*/
pub trait UniFilter<S, A>: Send + Sync {
    // Returns true if the entity passes the filter.
    fn test(&self, solution: &S, a: &A) -> bool;
}

/* A filter over pairs of stream rows.
Indices are the row positions in the joined sources, passed through from the
constraint so low-level filters can use index-addressed side data without
HashMap lookups.
*/
pub trait BiFilter<S, A, B>: Send + Sync {
    // Returns true if the pair passes the filter.
    fn test(&self, solution: &S, a: &A, b: &B, a_idx: usize, b_idx: usize) -> bool;
}

// A filter over triples of stream rows.
pub trait TriFilter<S, A, B, C>: Send + Sync {
    // Returns true if the triple passes the filter.
    #[allow(clippy::too_many_arguments)]
    fn test(
        &self,
        solution: &S,
        a: &A,
        b: &B,
        c: &C,
        a_idx: usize,
        b_idx: usize,
        c_idx: usize,
    ) -> bool;
}

// A filter over quadruples of stream rows.
pub trait QuadFilter<S, A, B, C, D>: Send + Sync {
    // Returns true if the quadruple passes the filter.
    #[allow(clippy::too_many_arguments)]
    fn test(
        &self,
        solution: &S,
        a: &A,
        b: &B,
        c: &C,
        d: &D,
        a_idx: usize,
        b_idx: usize,
        c_idx: usize,
        d_idx: usize,
    ) -> bool;
}

// A filter over quintuples of stream rows.
pub trait PentaFilter<S, A, B, C, D, E>: Send + Sync {
    // Returns true if the quintuple passes the filter.
    #[allow(clippy::too_many_arguments)]
    fn test(
        &self,
        solution: &S,
        a: &A,
        b: &B,
        c: &C,
        d: &D,
        e: &E,
        a_idx: usize,
        b_idx: usize,
        c_idx: usize,
        d_idx: usize,
        e_idx: usize,
    ) -> bool;
}
