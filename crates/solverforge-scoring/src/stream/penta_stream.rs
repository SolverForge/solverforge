//! Zero-erasure penta-constraint stream for five-entity constraint patterns.
//!
//! A `PentaConstraintStream` operates on quintuples of entities and supports
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
//! // Penalize when five tasks are on the same team
//! let constraint = ConstraintFactory::<Solution, SimpleScore>::new()
//!     .for_each(|s: &Solution| s.tasks.as_slice())
//!     .join_self(equal(|t: &Task| t.team))
//!     .join_self(equal(|t: &Task| t.team))
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
//!         Task { team: 1 },
//!         Task { team: 1 },
//!         Task { team: 2 },
//!     ],
//! };
//!
//! // One quintuple on team 1: (0, 1, 2, 3, 4) = -1 penalty
//! assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-1));
//! ```

use std::hash::Hash;

use solverforge_core::score::Score;

use crate::constraint::IncrementalPentaConstraint;

use super::collector::PentaCollector;
use super::filter::PentaFilter;
use super::grouped_penta_stream::GroupedPentaConstraintStream;

super::arity_stream_macros::impl_arity_stream!(
    penta,
    PentaConstraintStream,
    PentaConstraintBuilder,
    IncrementalPentaConstraint
);

// group_by method for penta stream
impl<S, A, K, E, KE, F, Sc> PentaConstraintStream<S, A, K, E, KE, F, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Hash + PartialEq + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync + 'static,
    E: Fn(&S) -> &[A] + Send + Sync,
    KE: Fn(&A) -> K + Send + Sync,
    F: PentaFilter<S, A, A, A, A, A>,
    Sc: Score + 'static,
{
    /// Groups quintuples by a key and aggregates using a collector.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_scoring::stream::ConstraintFactory;
    /// use solverforge_scoring::stream::joiner::equal;
    /// use solverforge_scoring::stream::collector::penta_count;
    /// use solverforge_scoring::api::constraint_set::IncrementalConstraint;
    /// use solverforge_core::score::SimpleScore;
    ///
    /// #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    /// struct Task { team: u32, priority: u32 }
    ///
    /// #[derive(Clone)]
    /// struct Solution { tasks: Vec<Task> }
    ///
    /// let constraint = ConstraintFactory::<Solution, SimpleScore>::new()
    ///     .for_each(|s: &Solution| s.tasks.as_slice())
    ///     .join_self(equal(|t: &Task| t.team))
    ///     .join_self(equal(|t: &Task| t.team))
    ///     .join_self(equal(|t: &Task| t.team))
    ///     .join_self(equal(|t: &Task| t.team))
    ///     .group_by(
    ///         |_a: &Task, _b: &Task, _c: &Task, _d: &Task, e: &Task| e.priority,
    ///         penta_count(),
    ///     )
    ///     .penalize_with(|count: &usize| SimpleScore::of(*count as i64))
    ///     .as_constraint("Priority clustering");
    ///
    /// let solution = Solution {
    ///     tasks: vec![
    ///         Task { team: 1, priority: 1 },
    ///         Task { team: 1, priority: 1 },
    ///         Task { team: 1, priority: 1 },
    ///         Task { team: 1, priority: 1 },
    ///         Task { team: 1, priority: 1 },
    ///     ],
    /// };
    ///
    /// // 1 quintuple -> 1 penalty
    /// assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-1));
    /// ```
    pub fn group_by<GK, KF, C>(
        self,
        key_fn: KF,
        collector: C,
    ) -> GroupedPentaConstraintStream<S, A, GK, K, E, KE, impl Fn(&S, &A, &A, &A, &A, &A) -> bool + Send + Sync, KF, C, Sc>
    where
        GK: Clone + Eq + Hash + Send + Sync + 'static,
        KF: Fn(&A, &A, &A, &A, &A) -> GK + Send + Sync,
        C: PentaCollector<A> + Send + Sync + 'static,
        C::Accumulator: Send + Sync,
        C::Result: Clone + Send + Sync,
        F: 'static,
    {
        let filter = self.filter;
        let combined_filter = move |s: &S, a: &A, b: &A, c: &A, d: &A, e: &A| filter.test(s, a, b, c, d, e);

        GroupedPentaConstraintStream::new(
            self.extractor,
            self.key_extractor,
            combined_filter,
            key_fn,
            collector,
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
    //!     .join_self(equal(|i: &Item| i.group))
    //!     .join_self(equal(|i: &Item| i.group))
    //!     .filter(|a: &Item, b: &Item, c: &Item, d: &Item, e: &Item| {
    //!         a.value + b.value + c.value + d.value + e.value > 20
    //!     })
    //!     .penalize(SimpleScore::of(1))
    //!     .as_constraint("High sum quintuples");
    //!
    //! let solution = Solution {
    //!     items: vec![
    //!         Item { group: 1, value: 3 },
    //!         Item { group: 1, value: 4 },
    //!         Item { group: 1, value: 5 },
    //!         Item { group: 1, value: 6 },
    //!         Item { group: 1, value: 7 },
    //!     ],
    //! };
    //!
    //! // 3+4+5+6+7=25 > 20, matches
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
    //!     .join_self(equal(|t: &Task| t.priority))
    //!     .join_self(equal(|t: &Task| t.priority))
    //!     .penalize(SimpleScore::of(5))
    //!     .as_constraint("Quintuple priority conflict");
    //!
    //! let solution = Solution {
    //!     tasks: vec![
    //!         Task { priority: 1 },
    //!         Task { priority: 1 },
    //!         Task { priority: 1 },
    //!         Task { priority: 1 },
    //!         Task { priority: 1 },
    //!     ],
    //! };
    //!
    //! // One quintuple = -5
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
    //!     .join_self(equal(|t: &Task| t.team))
    //!     .join_self(equal(|t: &Task| t.team))
    //!     .penalize_with(|a: &Task, b: &Task, c: &Task, d: &Task, e: &Task| {
    //!         SimpleScore::of(a.cost + b.cost + c.cost + d.cost + e.cost)
    //!     })
    //!     .as_constraint("Team cost");
    //!
    //! let solution = Solution {
    //!     tasks: vec![
    //!         Task { team: 1, cost: 1 },
    //!         Task { team: 1, cost: 2 },
    //!         Task { team: 1, cost: 3 },
    //!         Task { team: 1, cost: 4 },
    //!         Task { team: 1, cost: 5 },
    //!     ],
    //! };
    //!
    //! // Penalty: 1+2+3+4+5 = -15
    //! assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-15));
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
    //!         Person { team: 1 },
    //!         Person { team: 1 },
    //!     ],
    //! };
    //!
    //! // One quintuple = +10
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
    //!     .join_self(equal(|i: &Item| i.id))
    //!     .join_self(equal(|i: &Item| i.id))
    //!     .penalize(SimpleScore::of(1))
    //!     .as_constraint("Quintuple items");
    //!
    //! assert_eq!(constraint.name(), "Quintuple items");
    //! ```
}
