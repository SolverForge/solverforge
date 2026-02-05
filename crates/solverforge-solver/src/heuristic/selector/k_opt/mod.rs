//! K-opt move selector for tour optimization.
//!
//! Generates k-opt moves by enumerating all valid cut point combinations
//! within selected entities and applying reconnection patterns.
//!
//! # Complexity
//!
//! For a route of length n and k-opt:
//! - Full enumeration: O(n^k) cut combinations × reconnection patterns
//! - Use `NearbyKOptMoveSelector` to reduce to O(n × m^(k-1)) with nearby selection
//!
//! # Example
//!
//! ```
//! use solverforge_solver::heuristic::selector::k_opt::{KOptMoveSelector, KOptConfig};
//! use solverforge_solver::heuristic::selector::entity::FromSolutionEntitySelector;
//! use solverforge_core::domain::PlanningSolution;
//! use solverforge_core::score::SimpleScore;
//!
//! #[derive(Clone, Debug)]
//! struct Tour { cities: Vec<i32>, score: Option<SimpleScore> }
//!
//! impl PlanningSolution for Tour {
//!     type Score = SimpleScore;
//!     fn score(&self) -> Option<Self::Score> { self.score }
//!     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
//! }
//!
//! fn list_len(s: &Tour, _: usize) -> usize { s.cities.len() }
//! fn sublist_remove(s: &mut Tour, _: usize, start: usize, end: usize) -> Vec<i32> {
//!     s.cities.drain(start..end).collect()
//! }
//! fn sublist_insert(s: &mut Tour, _: usize, pos: usize, items: Vec<i32>) {
//!     for (i, item) in items.into_iter().enumerate() {
//!         s.cities.insert(pos + i, item);
//!     }
//! }
//!
//! let config = KOptConfig::new(3); // 3-opt
//!
//! let selector = KOptMoveSelector::<Tour, i32, _>::new(
//!     FromSolutionEntitySelector::new(0),
//!     config,
//!     list_len,
//!     sublist_remove,
//!     sublist_insert,
//!     "cities",
//!     0,
//! );
//! ```

mod config;
mod distance_meter;
mod iterators;
mod nearby;
mod selector;
#[cfg(test)]
mod tests;

pub use config::KOptConfig;
pub use distance_meter::{DefaultDistanceMeter, ListPositionDistanceMeter};
pub use iterators::{binomial, count_cut_combinations, CutCombinationIterator};
pub use nearby::NearbyKOptMoveSelector;
pub use selector::KOptMoveSelector;
