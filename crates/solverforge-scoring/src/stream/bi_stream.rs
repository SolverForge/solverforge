// Zero-erasure bi-constraint stream for self-join patterns.
//
// A `BiConstraintStream` operates on pairs of entities from the same
// collection (self-join), such as comparing Shifts to each other.
// All type information is preserved at compile time - no Arc, no dyn.
//
// # Example
//
// ```
// use solverforge_scoring::stream::ConstraintFactory;
// use solverforge_scoring::stream::joiner::equal;
// use solverforge_scoring::api::constraint_set::IncrementalConstraint;
// use solverforge_core::score::SoftScore;
//
// #[derive(Clone, Debug, Hash, PartialEq, Eq)]
// struct Task { team: u32 }
//
// #[derive(Clone)]
// struct Solution { tasks: Vec<Task> }
//
// // Penalize when two tasks are on the same team
// let constraint = ConstraintFactory::<Solution, SoftScore>::new()
//     .for_each(|s: &Solution| s.tasks.as_slice())
//     .join(equal(|t: &Task| t.team))
//     .penalize(SoftScore::of(1))
//     .named("Team conflict");
//
// let solution = Solution {
//     tasks: vec![
//         Task { team: 1 },
//         Task { team: 1 },
//         Task { team: 2 },
//     ],
// };
//
// // One pair on team 1: (0, 1) = -1 penalty
// assert_eq!(constraint.evaluate(&solution), SoftScore::of(-1));
// ```

use std::hash::Hash;

use solverforge_core::score::Score;

use crate::constraint::IncrementalBiConstraint;

use super::filter::{BiFilter, FnTriFilter, TriFilter};
use super::joiner::Joiner;
use super::tri_stream::TriConstraintStream;

super::arity_stream_macros::impl_arity_stream!(
    bi,
    BiConstraintStream,
    BiConstraintBuilder,
    IncrementalBiConstraint
);

// join method - transitions to TriConstraintStream
impl<S, A, K, E, KE, F, Sc> BiConstraintStream<S, A, K, E, KE, F, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Hash + PartialEq + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    E: Fn(&S) -> &[A] + Send + Sync,
    KE: Fn(&S, &A, usize) -> K + Send + Sync,
    F: BiFilter<S, A, A>,
    Sc: Score + 'static,
{
    // Joins this stream with a third element to create triples.
    //
    // # Example
    //
    // ```
    // use solverforge_scoring::stream::ConstraintFactory;
    // use solverforge_scoring::stream::joiner::equal;
    // use solverforge_scoring::api::constraint_set::IncrementalConstraint;
    // use solverforge_core::score::SoftScore;
    //
    // #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    // struct Task { team: u32 }
    //
    // #[derive(Clone)]
    // struct Solution { tasks: Vec<Task> }
    //
    // // Penalize when three tasks are on the same team
    // let constraint = ConstraintFactory::<Solution, SoftScore>::new()
    //     .for_each(|s: &Solution| s.tasks.as_slice())
    //     .join(equal(|t: &Task| t.team))
    //     .join(equal(|t: &Task| t.team))
    //     .penalize(SoftScore::of(1))
    //     .named("Team clustering");
    //
    // let solution = Solution {
    //     tasks: vec![
    //         Task { team: 1 },
    //         Task { team: 1 },
    //         Task { team: 1 },
    //         Task { team: 2 },
    //     ],
    // };
    //
    // // One triple on team 1: (0, 1, 2) = -1 penalty
    // assert_eq!(constraint.evaluate(&solution), SoftScore::of(-1));
    // ```
    pub fn join<J>(
        self,
        joiner: J,
    ) -> TriConstraintStream<S, A, K, E, KE, impl TriFilter<S, A, A, A>, Sc>
    where
        J: Joiner<A, A> + 'static,
        F: 'static,
    {
        let filter = self.filter;
        let combined_filter =
            move |s: &S, a: &A, b: &A, c: &A| filter.test(s, a, b, 0, 0) && joiner.matches(a, c);

        TriConstraintStream::new_self_join_with_filter(
            self.extractor,
            self.key_extractor,
            FnTriFilter::new(combined_filter),
        )
    }
}

// Additional doctests for individual methods

#[cfg(doctest)]
mod doctests {
    // # Filter method
    //
    // ```
    // use solverforge_scoring::stream::ConstraintFactory;
    // use solverforge_scoring::stream::joiner::equal;
    // use solverforge_scoring::api::constraint_set::IncrementalConstraint;
    // use solverforge_core::score::SoftScore;
    //
    // #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    // struct Item { group: u32, value: i32 }
    //
    // #[derive(Clone)]
    // struct Solution { items: Vec<Item> }
    //
    // let constraint = ConstraintFactory::<Solution, SoftScore>::new()
    //     .for_each(|s: &Solution| s.items.as_slice())
    //     .join(equal(|i: &Item| i.group))
    //     .filter(|a: &Item, b: &Item| a.value + b.value > 10)
    //     .penalize(SoftScore::of(1))
    //     .named("High sum pairs");
    //
    // let solution = Solution {
    //     items: vec![
    //         Item { group: 1, value: 6 },
    //         Item { group: 1, value: 7 },
    //     ],
    // };
    //
    // // 6+7=13 > 10, matches
    // assert_eq!(constraint.evaluate(&solution), SoftScore::of(-1));
    // ```
    //
    // # Penalize method
    //
    // ```
    // use solverforge_scoring::stream::ConstraintFactory;
    // use solverforge_scoring::stream::joiner::equal;
    // use solverforge_scoring::api::constraint_set::IncrementalConstraint;
    // use solverforge_core::score::SoftScore;
    //
    // #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    // struct Task { priority: u32 }
    //
    // #[derive(Clone)]
    // struct Solution { tasks: Vec<Task> }
    //
    // let constraint = ConstraintFactory::<Solution, SoftScore>::new()
    //     .for_each(|s: &Solution| s.tasks.as_slice())
    //     .join(equal(|t: &Task| t.priority))
    //     .penalize(SoftScore::of(5))
    //     .named("Pair priority conflict");
    //
    // let solution = Solution {
    //     tasks: vec![
    //         Task { priority: 1 },
    //         Task { priority: 1 },
    //     ],
    // };
    //
    // // One pair = -5
    // assert_eq!(constraint.evaluate(&solution), SoftScore::of(-5));
    // ```
    //
    // # Penalize with dynamic weight
    //
    // ```
    // use solverforge_scoring::stream::ConstraintFactory;
    // use solverforge_scoring::stream::joiner::equal;
    // use solverforge_scoring::api::constraint_set::IncrementalConstraint;
    // use solverforge_core::score::SoftScore;
    //
    // #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    // struct Task { team: u32, cost: i64 }
    //
    // #[derive(Clone)]
    // struct Solution { tasks: Vec<Task> }
    //
    // let constraint = ConstraintFactory::<Solution, SoftScore>::new()
    //     .for_each(|s: &Solution| s.tasks.as_slice())
    //     .join(equal(|t: &Task| t.team))
    //     .penalize_with(|a: &Task, b: &Task| {
    //         SoftScore::of(a.cost + b.cost)
    //     })
    //     .named("Team cost");
    //
    // let solution = Solution {
    //     tasks: vec![
    //         Task { team: 1, cost: 2 },
    //         Task { team: 1, cost: 3 },
    //     ],
    // };
    //
    // // Penalty: 2+3 = -5
    // assert_eq!(constraint.evaluate(&solution), SoftScore::of(-5));
    // ```
    //
    // # Reward method
    //
    // ```
    // use solverforge_scoring::stream::ConstraintFactory;
    // use solverforge_scoring::stream::joiner::equal;
    // use solverforge_scoring::api::constraint_set::IncrementalConstraint;
    // use solverforge_core::score::SoftScore;
    //
    // #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    // struct Person { team: u32 }
    //
    // #[derive(Clone)]
    // struct Solution { people: Vec<Person> }
    //
    // let constraint = ConstraintFactory::<Solution, SoftScore>::new()
    //     .for_each(|s: &Solution| s.people.as_slice())
    //     .join(equal(|p: &Person| p.team))
    //     .reward(SoftScore::of(10))
    //     .named("Team synergy");
    //
    // let solution = Solution {
    //     people: vec![
    //         Person { team: 1 },
    //         Person { team: 1 },
    //     ],
    // };
    //
    // // One pair = +10
    // assert_eq!(constraint.evaluate(&solution), SoftScore::of(10));
    // ```
    //
    // # named method
    //
    // ```
    // use solverforge_scoring::stream::ConstraintFactory;
    // use solverforge_scoring::stream::joiner::equal;
    // use solverforge_scoring::api::constraint_set::IncrementalConstraint;
    // use solverforge_core::score::SoftScore;
    //
    // #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    // struct Item { id: usize }
    //
    // #[derive(Clone)]
    // struct Solution { items: Vec<Item> }
    //
    // let constraint = ConstraintFactory::<Solution, SoftScore>::new()
    //     .for_each(|s: &Solution| s.items.as_slice())
    //     .join(equal(|i: &Item| i.id))
    //     .penalize(SoftScore::of(1))
    //     .named("Pair items");
    //
    // assert_eq!(constraint.name(), "Pair items");
    // ```
}
