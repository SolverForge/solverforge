//! Zero-erasure quad-constraint stream for four-entity constraint patterns.
//!
//! A `QuadConstraintStream` operates on quadruples of entities and supports
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
//! // Penalize when four tasks are on the same team
//! let constraint = ConstraintFactory::<Solution, SimpleScore>::new()
//!     .for_each(|s: &Solution| s.tasks.as_slice())
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
//!         Task { team: 2 },
//!     ],
//! };
//!
//! // One quadruple on team 1: (0, 1, 2, 3) = -1 penalty
//! assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-1));
//! ```

use std::hash::Hash;

use solverforge_core::score::Score;

use crate::constraint::quad_incremental::IncrementalQuadConstraint;

use super::filter::{FnPentaFilter, QuadFilter};
use super::joiner::Joiner;
use super::penta_stream::PentaConstraintStream;

super::arity_stream_macros::impl_arity_stream!(quad, QuadConstraintStream, QuadConstraintBuilder, IncrementalQuadConstraint);

// join_self method - transitions to PentaConstraintStream
impl<S, A, K, E, KE, F, Sc> QuadConstraintStream<S, A, K, E, KE, F, Sc>
where
    S: Send + Sync + 'static,
    A: Clone + Hash + PartialEq + Send + Sync + 'static,
    K: Eq + Hash + Clone + Send + Sync,
    E: Fn(&S) -> &[A] + Send + Sync,
    KE: Fn(&A) -> K + Send + Sync,
    F: QuadFilter<A, A, A, A>,
    Sc: Score + 'static,
{
    /// Joins this stream with a fifth element to create quintuples.
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
    /// // Penalize when five tasks are on the same team
    /// let constraint = ConstraintFactory::<Solution, SimpleScore>::new()
    ///     .for_each(|s: &Solution| s.tasks.as_slice())
    ///     .join_self(equal(|t: &Task| t.team))
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
    ///         Task { team: 1 },
    ///         Task { team: 2 },
    ///     ],
    /// };
    ///
    /// // One quintuple on team 1: (0, 1, 2, 3, 4) = -1 penalty
    /// assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-1));
    /// ```
    pub fn join_self<J>(
        self,
        joiner: J,
    ) -> PentaConstraintStream<S, A, K, E, KE, impl super::filter::PentaFilter<A, A, A, A, A>, Sc>
    where
        J: Joiner<A, A> + 'static,
        F: 'static,
    {
        let filter = self.filter;
        let combined_filter = move |a: &A, b: &A, c: &A, d: &A, e: &A| {
            filter.test(a, b, c, d) && joiner.matches(a, e)
        };

        PentaConstraintStream::new_self_join_with_filter(
            self.extractor,
            self.key_extractor,
            FnPentaFilter::new(combined_filter),
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
    //!     .filter(|a: &Item, b: &Item, c: &Item, d: &Item| {
    //!         a.value + b.value + c.value + d.value > 15
    //!     })
    //!     .penalize(SimpleScore::of(1))
    //!     .as_constraint("High sum quadruples");
    //!
    //! let solution = Solution {
    //!     items: vec![
    //!         Item { group: 1, value: 3 },
    //!         Item { group: 1, value: 4 },
    //!         Item { group: 1, value: 5 },
    //!         Item { group: 1, value: 6 },
    //!     ],
    //! };
    //!
    //! // 3+4+5+6=18 > 15, matches
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
    //!     .penalize(SimpleScore::of(5))
    //!     .as_constraint("Quadruple priority conflict");
    //!
    //! let solution = Solution {
    //!     tasks: vec![
    //!         Task { priority: 1 },
    //!         Task { priority: 1 },
    //!         Task { priority: 1 },
    //!         Task { priority: 1 },
    //!     ],
    //! };
    //!
    //! // One quadruple = -5
    //! assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-5));
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
    //!     .penalize(SimpleScore::of(1))
    //!     .as_constraint("Quadruple items");
    //!
    //! assert_eq!(constraint.name(), "Quadruple items");
    //! ```
}
