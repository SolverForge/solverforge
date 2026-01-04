//! Zero-erasure tri-constraint stream for three-entity constraint patterns.
//!
//! A `TriConstraintStream` operates on triples of entities and supports
//! filtering, weighting, and constraint finalization. All type information
//! is preserved at compile time - no Arc, no dyn.
//!
//! # Example
//!
//! ```
//! use solverforge_scoring::stream::ConstraintFactory;
//! use solverforge_scoring::stream::joiner::equal;
//! use solverforge_scoring::api::constraint_set::IncrementalConstraint;
//! use solverforge_core::score::SimpleScore;
//!
//! #[derive(Clone, Debug, Hash, PartialEq, Eq)]
//! struct Task { team: u32 }
//!
//! #[derive(Clone)]
//! struct Solution { tasks: Vec<Task> }
//!
//! // Penalize when three tasks are on the same team
//! let constraint = ConstraintFactory::<Solution, SimpleScore>::new()
//!     .for_each(|s: &Solution| s.tasks.as_slice())
//!     .join_self(equal(|t: &Task| t.team))
//!     .join_self(equal(|t: &Task| t.team))
//!     .penalize(SimpleScore::of(1))
//!     .as_constraint("Team clustering");
//!
//! let solution = Solution {
//!     tasks: vec![
//!         Task { team: 1 },
//!         Task { team: 1 },
//!         Task { team: 1 },
//!         Task { team: 2 },
//!     ],
//! };
//!
//! // One triple on team 1: (0, 1, 2) = -1 penalty
//! assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-1));
//! ```

use std::hash::Hash;

use solverforge_core::score::Score;

use crate::constraint::tri_incremental::IncrementalTriConstraint;

use super::filter::{FnQuadFilter, TriFilter};
use super::joiner::Joiner;
use super::quad_stream::QuadConstraintStream;

super::arity_stream_macros::impl_arity_stream!(tri, TriConstraintStream, TriConstraintBuilder, IncrementalTriConstraint);

// join_self method - transitions to QuadConstraintStream
impl<S, A, K, E, KE, F, Sc> TriConstraintStream<S, A, K, E, KE, F, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Hash + PartialEq + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    E: Fn(&S) -> &[A] + Send + Sync,
    KE: Fn(&A) -> K + Send + Sync,
    F: TriFilter<A, A, A>,
    Sc: Score + 'static,
{
    /// Joins this stream with a fourth element to create quadruples.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_scoring::stream::ConstraintFactory;
    /// use solverforge_scoring::stream::joiner::equal;
    /// use solverforge_scoring::api::constraint_set::IncrementalConstraint;
    /// use solverforge_core::score::SimpleScore;
    ///
    /// #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    /// struct Task { team: u32 }
    ///
    /// #[derive(Clone)]
    /// struct Solution { tasks: Vec<Task> }
    ///
    /// // Penalize when four tasks are on the same team
    /// let constraint = ConstraintFactory::<Solution, SimpleScore>::new()
    ///     .for_each(|s: &Solution| s.tasks.as_slice())
    ///     .join_self(equal(|t: &Task| t.team))
    ///     .join_self(equal(|t: &Task| t.team))
    ///     .join_self(equal(|t: &Task| t.team))
    ///     .penalize(SimpleScore::of(1))
    ///     .as_constraint("Team clustering");
    ///
    /// let solution = Solution {
    ///     tasks: vec![
    ///         Task { team: 1 },
    ///         Task { team: 1 },
    ///         Task { team: 1 },
    ///         Task { team: 1 },
    ///         Task { team: 2 },
    ///     ],
    /// };
    ///
    /// // One quadruple on team 1: (0, 1, 2, 3) = -1 penalty
    /// assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-1));
    /// ```
    pub fn join_self<J>(
        self,
        joiner: J,
    ) -> QuadConstraintStream<S, A, K, E, KE, impl super::filter::QuadFilter<A, A, A, A>, Sc>
    where
        J: Joiner<A, A> + 'static,
        F: 'static,
    {
        let filter = self.filter;
        let combined_filter = move |a: &A, b: &A, c: &A, d: &A| {
            filter.test(a, b, c) && joiner.matches(a, d)
        };

        QuadConstraintStream::new_self_join_with_filter(
            self.extractor,
            self.key_extractor,
            FnQuadFilter::new(combined_filter),
        )
    }
}

// Additional doctests for individual methods

#[cfg(doctest)]
mod doctests {
    //! # Filter method
    //!
    //! ```
    //! use solverforge_scoring::stream::ConstraintFactory;
    //! use solverforge_scoring::stream::joiner::equal;
    //! use solverforge_scoring::api::constraint_set::IncrementalConstraint;
    //! use solverforge_core::score::SimpleScore;
    //!
    //! #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    //! struct Item { group: u32, value: i32 }
    //!
    //! #[derive(Clone)]
    //! struct Solution { items: Vec<Item> }
    //!
    //! let constraint = ConstraintFactory::<Solution, SimpleScore>::new()
    //!     .for_each(|s: &Solution| s.items.as_slice())
    //!     .join_self(equal(|i: &Item| i.group))
    //!     .join_self(equal(|i: &Item| i.group))
    //!     .filter(|a: &Item, b: &Item, c: &Item| a.value + b.value + c.value > 10)
    //!     .penalize(SimpleScore::of(1))
    //!     .as_constraint("High sum triples");
    //!
    //! let solution = Solution {
    //!     items: vec![
    //!         Item { group: 1, value: 3 },
    //!         Item { group: 1, value: 4 },
    //!         Item { group: 1, value: 5 },
    //!     ],
    //! };
    //!
    //! // 3+4+5=12 > 10, matches
    //! assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-1));
    //! ```
    //!
    //! # Penalize method
    //!
    //! ```
    //! use solverforge_scoring::stream::ConstraintFactory;
    //! use solverforge_scoring::stream::joiner::equal;
    //! use solverforge_scoring::api::constraint_set::IncrementalConstraint;
    //! use solverforge_core::score::SimpleScore;
    //!
    //! #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    //! struct Task { priority: u32 }
    //!
    //! #[derive(Clone)]
    //! struct Solution { tasks: Vec<Task> }
    //!
    //! let constraint = ConstraintFactory::<Solution, SimpleScore>::new()
    //!     .for_each(|s: &Solution| s.tasks.as_slice())
    //!     .join_self(equal(|t: &Task| t.priority))
    //!     .join_self(equal(|t: &Task| t.priority))
    //!     .penalize(SimpleScore::of(5))
    //!     .as_constraint("Triple priority conflict");
    //!
    //! let solution = Solution {
    //!     tasks: vec![
    //!         Task { priority: 1 },
    //!         Task { priority: 1 },
    //!         Task { priority: 1 },
    //!     ],
    //! };
    //!
    //! // One triple = -5
    //! assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-5));
    //! ```
    //!
    //! # Penalize with dynamic weight
    //!
    //! ```
    //! use solverforge_scoring::stream::ConstraintFactory;
    //! use solverforge_scoring::stream::joiner::equal;
    //! use solverforge_scoring::api::constraint_set::IncrementalConstraint;
    //! use solverforge_core::score::SimpleScore;
    //!
    //! #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    //! struct Task { team: u32, cost: i64 }
    //!
    //! #[derive(Clone)]
    //! struct Solution { tasks: Vec<Task> }
    //!
    //! let constraint = ConstraintFactory::<Solution, SimpleScore>::new()
    //!     .for_each(|s: &Solution| s.tasks.as_slice())
    //!     .join_self(equal(|t: &Task| t.team))
    //!     .join_self(equal(|t: &Task| t.team))
    //!     .penalize_with(|a: &Task, b: &Task, c: &Task| {
    //!         SimpleScore::of(a.cost + b.cost + c.cost)
    //!     })
    //!     .as_constraint("Team cost");
    //!
    //! let solution = Solution {
    //!     tasks: vec![
    //!         Task { team: 1, cost: 2 },
    //!         Task { team: 1, cost: 3 },
    //!         Task { team: 1, cost: 5 },
    //!     ],
    //! };
    //!
    //! // Penalty: 2+3+5 = -10
    //! assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-10));
    //! ```
    //!
    //! # Reward method
    //!
    //! ```
    //! use solverforge_scoring::stream::ConstraintFactory;
    //! use solverforge_scoring::stream::joiner::equal;
    //! use solverforge_scoring::api::constraint_set::IncrementalConstraint;
    //! use solverforge_core::score::SimpleScore;
    //!
    //! #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    //! struct Person { team: u32 }
    //!
    //! #[derive(Clone)]
    //! struct Solution { people: Vec<Person> }
    //!
    //! let constraint = ConstraintFactory::<Solution, SimpleScore>::new()
    //!     .for_each(|s: &Solution| s.people.as_slice())
    //!     .join_self(equal(|p: &Person| p.team))
    //!     .join_self(equal(|p: &Person| p.team))
    //!     .reward(SimpleScore::of(10))
    //!     .as_constraint("Team synergy");
    //!
    //! let solution = Solution {
    //!     people: vec![
    //!         Person { team: 1 },
    //!         Person { team: 1 },
    //!         Person { team: 1 },
    //!     ],
    //! };
    //!
    //! // One triple = +10
    //! assert_eq!(constraint.evaluate(&solution), SimpleScore::of(10));
    //! ```
    //!
    //! # as_constraint method
    //!
    //! ```
    //! use solverforge_scoring::stream::ConstraintFactory;
    //! use solverforge_scoring::stream::joiner::equal;
    //! use solverforge_scoring::api::constraint_set::IncrementalConstraint;
    //! use solverforge_core::score::SimpleScore;
    //!
    //! #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    //! struct Item { id: usize }
    //!
    //! #[derive(Clone)]
    //! struct Solution { items: Vec<Item> }
    //!
    //! let constraint = ConstraintFactory::<Solution, SimpleScore>::new()
    //!     .for_each(|s: &Solution| s.items.as_slice())
    //!     .join_self(equal(|i: &Item| i.id))
    //!     .join_self(equal(|i: &Item| i.id))
    //!     .penalize(SimpleScore::of(1))
    //!     .as_constraint("Triple items");
    //!
    //! assert_eq!(constraint.name(), "Triple items");
    //! ```
}
