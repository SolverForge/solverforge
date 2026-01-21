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

use crate::constraint::IncrementalPentaConstraint;

super::arity_stream_macros::impl_arity_stream!(
    penta,
    PentaConstraintStream,
    PentaConstraintBuilder,
    IncrementalPentaConstraint
);

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
