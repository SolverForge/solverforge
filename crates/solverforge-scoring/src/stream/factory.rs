// Constraint factory for creating typed constraint streams.
//
// The factory is the entry point for the fluent constraint API.

use std::hash::Hash;
use std::marker::PhantomData;

use solverforge_core::score::Score;

use super::bi_stream::BiConstraintStream;
use super::filter::TrueFilter;
use super::joiner::EqualJoiner;
use super::UniConstraintStream;

// Factory for creating constraint streams.
//
// `ConstraintFactory` is parameterized by the solution type `S` and score type `Sc`.
// It serves as the entry point for defining constraints using the fluent API.
//
// # Example
//
// ```
// use solverforge_scoring::stream::ConstraintFactory;
// use solverforge_scoring::api::constraint_set::IncrementalConstraint;
// use solverforge_core::score::SimpleScore;
//
// #[derive(Clone)]
// struct Solution {
//     values: Vec<Option<i32>>,
// }
//
// let factory = ConstraintFactory::<Solution, SimpleScore>::new();
//
// let constraint = factory
//     .for_each(|s: &Solution| &s.values)
//     .filter(|v: &Option<i32>| v.is_none())
//     .penalize(SimpleScore::of(1))
//     .as_constraint("Unassigned");
//
// let solution = Solution { values: vec![Some(1), None, None] };
// assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-2));
// ```
pub struct ConstraintFactory<S, Sc: Score> {
    _phantom: PhantomData<(S, Sc)>,
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

    // Creates a zero-erasure uni-constraint stream over entities extracted from the solution.
    //
    // The extractor function receives a reference to the solution and returns
    // a slice of entities to iterate over. The extractor type is preserved
    // as a concrete generic for full zero-erasure.
    pub fn for_each<A, E>(self, extractor: E) -> UniConstraintStream<S, A, E, TrueFilter, Sc>
    where
        A: Clone + Send + Sync + 'static,
        E: Fn(&S) -> &[A] + Send + Sync,
    {
        UniConstraintStream::new(extractor)
    }

    // Creates a zero-erasure bi-constraint stream over unique pairs of entities.
    //
    // This is equivalent to `for_each(extractor).join_self(joiner)` but provides
    // a more concise API for the common case of self-joins with key-based grouping.
    //
    // Pairs are ordered (i, j) where i < j to avoid duplicates and self-pairs.
    //
    // # Example
    //
    // ```
    // use solverforge_scoring::stream::{ConstraintFactory, joiner::equal};
    // use solverforge_scoring::api::constraint_set::IncrementalConstraint;
    // use solverforge_core::score::SimpleScore;
    //
    // #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    // struct Task { team: u32, priority: u32 }
    //
    // #[derive(Clone)]
    // struct Solution { tasks: Vec<Task> }
    //
    // let factory = ConstraintFactory::<Solution, SimpleScore>::new();
    //
    // // Penalize when two tasks on the same team conflict
    // let constraint = factory
    //     .for_each_unique_pair(
    //         |s: &Solution| s.tasks.as_slice(),
    //         equal(|t: &Task| t.team)
    //     )
    //     .penalize(SimpleScore::of(1))
    //     .as_constraint("Team conflict");
    //
    // let solution = Solution {
    //     tasks: vec![
    //         Task { team: 1, priority: 1 },
    //         Task { team: 1, priority: 2 },  // Same team as first
    //         Task { team: 2, priority: 1 },
    //     ],
    // };
    //
    // // One pair on same team: (0, 1) = -1 penalty
    // assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-1));
    // ```
    pub fn for_each_unique_pair<A, E, K, KA>(
        self,
        extractor: E,
        joiner: EqualJoiner<KA, KA, K>,
    ) -> BiConstraintStream<S, A, K, E, KA, TrueFilter, Sc>
    where
        A: Clone + Hash + PartialEq + Send + Sync + 'static,
        E: Fn(&S) -> &[A] + Send + Sync,
        K: Eq + Hash + Clone + Send + Sync,
        KA: Fn(&A) -> K + Send + Sync,
    {
        let (key_extractor, _) = joiner.into_keys();
        BiConstraintStream::new_self_join(extractor, key_extractor)
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
