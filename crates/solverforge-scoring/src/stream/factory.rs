/* Constraint factory for creating typed constraint streams.

The factory is the entry point for the fluent constraint API.
*/

use std::marker::PhantomData;

use solverforge_core::score::Score;

use super::collection_extract::CollectionExtract;
use super::filter::TrueFilter;
use super::UniConstraintStream;

/* Factory for creating constraint streams.

`ConstraintFactory` is parameterized by the solution type `S` and score type `Sc`.
It serves as the entry point for defining constraints using the fluent API.

# Example

```
use solverforge_scoring::stream::ConstraintFactory;
use solverforge_scoring::api::constraint_set::IncrementalConstraint;
use solverforge_core::score::SoftScore;

#[derive(Clone)]
struct Solution {
values: Vec<Option<i32>>,
}

let factory = ConstraintFactory::<Solution, SoftScore>::new();

let constraint = factory
.for_each(|s: &Solution| &s.values)
.filter(|v: &Option<i32>| v.is_none())
.penalize(SoftScore::of(1))
.named("Unassigned");

let solution = Solution { values: vec![Some(1), None, None] };
assert_eq!(constraint.evaluate(&solution), SoftScore::of(-2));
```
*/
pub struct ConstraintFactory<S, Sc: Score> {
    _phantom: PhantomData<(fn() -> S, fn() -> Sc)>,
}

impl<S, Sc> ConstraintFactory<S, Sc>
where
    S: Send + Sync + 'static,
    Sc: Score + 'static,
{
    // Creates a new constraint factory.
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }

    /* Creates a zero-erasure uni-constraint stream over a collection source.

    For macro-generated models, pass the inherent solution source method:
    `ConstraintFactory::new().for_each(Schedule::shifts())`.
    Low-level callers can still pass any `CollectionExtract<S, Item = A>`.
    The extractor type is preserved as a concrete generic for full zero-erasure.
    */
    pub fn for_each<A, E>(self, extractor: E) -> UniConstraintStream<S, A, E, TrueFilter, Sc>
    where
        A: Clone + Send + Sync + 'static,
        E: CollectionExtract<S, Item = A>,
    {
        UniConstraintStream::new(extractor)
    }
}

impl<S, Sc> Default for ConstraintFactory<S, Sc>
where
    S: Send + Sync + 'static,
    Sc: Score + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S, Sc: Score> Clone for ConstraintFactory<S, Sc> {
    fn clone(&self) -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<S, Sc: Score> std::fmt::Debug for ConstraintFactory<S, Sc> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConstraintFactory").finish()
    }
}
